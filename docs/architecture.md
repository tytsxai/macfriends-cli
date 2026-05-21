# 架构说明

## 总体架构

MacFriends 由两部分组成：
- `crates/cli`：Rust CLI，负责环境检查、状态总览、受控副本准备、启动编排、target status、RPC 客户端、结果导出
- `native/agent`：Objective-C++ 动态库，通过 `DYLD_INSERT_LIBRARIES` 预加载进受控目标进程，提供本地 Unix Socket RPC

`serve` 命令会在本机启动一个轻量 HTTP 控制台。Web API 不直接重写业务逻辑，而是调用当前 `macfriends --json` 子命令，因此 CLI、Web、脚本自动化共享同一套后端判断、错误码和 Ready 门禁。

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

`macfriends status` 不依赖单一运行态来源，会同时读取：
- `target-status.json`
- `run-state.json`
- agent socket 探活结果
- 最近生产/fixture 扫描结果
- 日志与结果目录路径

因此它适合作为用户日常入口和自动化巡检入口。`doctor` 偏环境与 Ready 门禁，`attach` 偏活体 agent 详情，`status` 偏产品级闭环视图。

## Web 控制台链路

1. `macfriends serve` 监听 `127.0.0.1:8765` 或用户传入的 `--addr`
2. 浏览器加载内置 HTML 控制台
3. 控制台调用 `/api/status`、`/api/prepare`、`/api/launch` 等本地 HTTP API
4. Web 后端调用当前二进制的 `macfriends --json <command>`
5. API 将 CLI 输出包装为 `{ ok, command, exit_code, data|error, stderr }`

这个设计让 Web 页面不会绕过 CLI 的生产门禁。若 native agent 返回 `profile_primitive_unresolved` 等错误，页面和 API 都会如实呈现。

## 当前适配边界

当前已完成：
- 单版本 manifest 门禁
- adapter 注册与分发
- 固定错误码
- fixture 测试链路
- 运行态 Ready 门禁

当前尚未落地的部分，是特定微信版本内部私有原语的解析与调用逻辑；因此当目标满足 4.1.8 条件但原语未解析时，agent 会返回 `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved`。

## 运行态 Ready 门禁

除了静态的 bundle/version/arch 门禁外，CLI 还会基于 agent `status` 判定运行态是否 Ready。

运行态必须同时满足：
- 非 fixture 模式
- agent socket 正常可探活
- 关键原语 `profile/contacts/scan` 全部为 `resolved`

只通过静态门禁、不通过运行态门禁时，项目仍视为 Not Ready。
