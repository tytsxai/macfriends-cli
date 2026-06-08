# 架构说明

## 总体架构

MacFriends 由两部分组成：
- `crates/cli`：Rust CLI，负责环境检查、状态总览、受控副本准备、启动编排、target status、RPC 客户端、结果导出
- `native/agent`：Objective-C++ 动态库，通过 `DYLD_INSERT_LIBRARIES` 预加载进受控目标进程，提供本地 Unix Socket RPC

`serve` 命令会在本机启动一个轻量 HTTP 控制台。Web API 不直接重写业务逻辑，而是调用当前 `macfriends --json` 子命令，因此 CLI、Web、脚本自动化共享同一套后端判断、错误码和 Ready 门禁。

文档和代码的模块级映射见 [modules.md](modules.md)，路径、环境变量和状态文件见 [configuration.md](configuration.md)，部署方式见 [deployment.md](deployment.md)。

## 组件边界

| 组件 | 文件 | 边界 |
| --- | --- | --- |
| CLI contract | `crates/cli/src/cli.rs`, `model.rs` | 命令、参数、JSON schema 和错误输出 |
| Runtime orchestration | `crates/cli/src/app.rs`, `layout.rs`, `util.rs` | prepare/launch/status/scan/export/cleanup 和本机路径 |
| RPC client | `crates/cli/src/rpc.rs` | UDS newline JSON request/response |
| Web console | `crates/cli/src/web.rs` | 本地 HTTP API，复用 CLI 子命令 |
| Native agent | `native/agent/src/agent.mm` | socket server、日志、请求分发、安全边界 |
| Adapter | `native/agent/src/wechat_4_1_8_arm64.mm` | 单版本门禁、fixture 原语、unresolved primitive 错误 |
| Packaging | `Makefile`, `scripts/*.sh` | 构建、安装、beta gate、fixture smoke |
| Manifest | `fixtures/adapter.wechat-macos-arm64.json` | 静态兼容、release channel、primitive 元数据 |

## 版本化适配

当前项目只注册一个 adapter：`wechat_4_1_8_arm64`。

agent 内部职责分层：
- transport：socket 建连、请求读取、响应写回
- process probe：读取 bundle id / version / arch
- adapter registry：按 manifest 决定是否加载 `wechat_4_1_8_arm64`
- adapter implementation：处理 `profile / contacts / scan`

## 受控运行链路

1. `prepare` 复制 `/Applications/WeChat.app` 到 `~/Library/Application Support/MacFriends/runtime/WeChat.app`
2. `prepare` 构建 agent 与宿主，并执行 ad-hoc 签名
3. `prepare` 写入 `adapter.json` 与 `target-status.json`
4. `launch` 使用 `DYLD_INSERT_LIBRARIES` 启动受控副本
5. 生产运行态由 `WeChatAppEx 2.4.1.19024` 承接；主 `WeChat 4.1.8` 主要负责拉起运行时组件
6. agent 在受支持运行时组件内启动 Unix Domain Socket 服务
7. CLI 通过 JSON RPC 调用 `status/profile/contacts/scan/stop`

关键安全点：
- `prepare` 不修改 `/Applications/WeChat.app`，只复制到 runtime 目录。
- `prepare` 发现 live run-state 或 live socket 时会拒绝覆盖。
- `launch` 清理外部 DYLD/MacFriends 运行变量后再注入自己的环境。
- `launch` 失败会尝试终止本次启动的进程，并回收 socket、pid 和 run-state。
- `cleanup` 只有在受控进程不再运行且 socket 不响应时才删除运行态文件。

`macfriends status` 不依赖单一运行态来源，会同时读取：
- `target-status.json`
- `run-state.json`
- agent socket 探活结果
- 最近生产/fixture 扫描结果
- 日志与结果目录路径

因此它适合作为用户日常入口和自动化巡检入口。`doctor` 偏环境与 Ready 门禁，`attach` 偏活体 agent 详情，`status` 偏产品级闭环视图。

## 数据流

```text
WeChat.app source
  -> prepare
  -> runtime/WeChat.app + runtime/bin + runtime/adapter.json + target-status.json
  -> launch with DYLD_INSERT_LIBRARIES
  -> native agent socket
  -> CLI RPC calls
  -> scan report
  -> results/latest-scan.json or latest-fixture-scan.json
  -> export json/csv
```

正式链路和 fixture 链路分开保存：
- production: `results/latest-scan.json` 和 `results/history/scan-*.json`
- fixture: `results/latest-fixture-scan.json` 和 `results/fixture/fixture-scan-*.json`

`export` 只读取 production 的 `latest-scan.json`。这保证 fixture smoke 不会污染正式导出。

## Web 控制台链路

1. `macfriends serve` 监听 `127.0.0.1:8765` 或用户传入的 `--addr`
2. 服务端生成一次性内存 token，并注入内置 HTML 控制台
3. 浏览器加载内置 HTML 控制台
4. 控制台调用 `/api/status`、`/api/prepare`、`/api/launch` 等本地 HTTP API；写操作必须带 `X-MacFriends-Token`
5. Web 后端调用当前二进制的 `macfriends --json <command>`
6. API 将 CLI 输出包装为 `{ ok, command, exit_code, data|error, stderr }`

这个设计让 Web 页面不会绕过 CLI 的生产门禁，也避免其他网页跨站触发本机写操作。若 native agent 返回 `profile_primitive_unresolved` 等错误，页面和 API 都会如实呈现。

Web 控制台不是公网服务：
- 默认只监听 `127.0.0.1:8765`。
- 写操作需要本次 `serve` 生成的内存 token。
- 没有通配 CORS。
- Web 导出不接受自定义 output 路径。
- 不承担账号体系、TLS、远程任务队列或多人协作。

## 当前适配边界

当前已完成：
- 单版本 manifest 门禁
- adapter 注册与分发
- 固定错误码
- fixture 测试链路
- 运行态 Ready 门禁

当前尚未落地的部分，是特定微信版本内部私有原语的解析与调用逻辑；因此当目标满足 4.1.8 条件但原语未解析时，agent 会返回 `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved`。

需要特别注意：CLI 静态门禁读取主 `WeChat 4.1.8`，native agent 的真实运行态 Ready 判断发生在 `com.tencent.flue.WeChatAppEx 2.4.1.19024`。这是当前微信桌面版的运行结构决定的，不是两个互相矛盾的目标版本。

## 运行态 Ready 门禁

除了静态的 bundle/version/arch 门禁外，CLI 还会基于 agent `status` 判定运行态是否 Ready。

运行态必须同时满足：
- 非 fixture 模式
- agent socket 正常可探活
- 关键原语 `profile/contacts/scan` 全部为 `resolved`

只通过静态门禁、不通过运行态门禁时，项目仍视为 Not Ready。

Release guard 使用 adapter manifest 中的 `release_channel` 与 `primitive_resolution` 防止误发布。当前 manifest 为 beta 且 primitives unresolved，默认 `make ready` / `make package` 会阻塞；只有显式设置 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 才能生成 beta/testable artifact。

## 扩展原则

新增命令、Web API、状态字段或 adapter 时，应遵循：
- 先写 OpenSpec change，明确需求、设计和验收。
- 复用 `model.rs` 的结构化输出，不新增无法脚本化的文本-only 状态。
- 让 Web 后端继续复用 CLI 子命令，避免第二套业务逻辑。
- fixture 与 production 必须在状态、结果文件和导出策略上保持隔离。
- 新 adapter 在 primitives 未 resolved 前必须保留 release blocker。
