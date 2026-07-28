# 关键模块与核心逻辑

本文面向接手维护者，按文件说明 MacFriends 的核心模块、数据流和扩展点。

## 顶层结构

| 路径 | 职责 |
| --- | --- |
| `crates/cli` | Rust CLI、状态聚合、Web 后端、导出、运行编排 |
| `native/agent` | Objective-C++ agent、UDS 服务、adapter 分发、fixture 原语 |
| `fixtures` | 当前 adapter manifest 模板 |
| `scripts` | 安装、打包、release guard、fixture smoke |
| `docs` | 用户、维护、部署、配置和排障文档 |
| `openspec/changes` | 规格驱动变更记录 |

## Rust CLI

| 文件 | 关键职责 |
| --- | --- |
| `src/main.rs` | CLI 入口，调用 `app::run`，失败时输出固定错误码 |
| `src/cli.rs` | clap 命令和参数定义，包含中文命令别名 |
| `src/app.rs` | 核心业务编排：doctor/status/prepare/launch/attach/profile/contacts/scan/export/detach/cleanup |
| `src/model.rs` | JSON 输出 schema、manifest、扫描记录、运行态状态模型 |
| `src/layout.rs` | 所有本机路径、socket、结果和日志位置 |
| `src/rpc.rs` | Unix Domain Socket JSON RPC 客户端 |
| `src/export.rs` | JSON/CSV 导出，CSV 公式字段中和 |
| `src/web.rs` | 内置 HTTP 控制台和 `/api/*` 包装层 |
| `src/util.rs` | plist、codesign、atomic write、日志轮转、PID 和进程工具 |

### 命令生命周期

1. `doctor`
   - 读取默认源 `/Applications/WeChat.app`、受控副本和 adapter。
   - 探测 live agent 状态。
   - 汇总 `runtime_ready`、`fixture_enabled`、`primitive_resolution` 和 `release_blockers`。

2. `status`
   - 聚合 `target-status.json`、`run-state.json`、agent ping、最近扫描、路径、兼容提示和下一步。
   - 是日常巡检和 Web 控制台的主入口。

3. `prepare`
   - 确认没有 live 受控进程或 agent socket。
   - 准备私有目录。
   - 复制源 WeChat.app 到 runtime。
   - 同步 bundled agent/host/adapter。
   - 对 agent dylib 和受控 WeChat.app 做 ad-hoc signing。
   - 写入 `runtime/adapter.json` 与 `runtime/target-status.json`。

4. `launch`
   - 清理 stale run-state。
   - 移除外部污染的 DYLD 和 MacFriends 运行变量。
   - 通过 `DYLD_INSERT_LIBRARIES` 启动受控副本。
   - 写入 PID 和 run-state。
   - 等待 agent socket 建立并连续 ping 成功。
   - 检测 runtime PID，通常是 WeChatAppEx。
   - 若启动失败，终止本次进程并清理 pid/socket/run-state。

5. `attach/profile/contacts/scan/detach`
   - 通过 `rpc.rs` 调用 agent 的 `status/profile/contacts/scan/stop`。
   - 单次请求/响应是 newline-delimited JSON。

6. `scan`
   - 调用 agent `scan`。
   - 按 agent 状态设置 `mode=production` 或 `mode=fixture`。
   - production 写 `latest-scan.json` 和 `history/scan-*.json`。
   - fixture 写 `latest-fixture-scan.json` 和 `fixture/fixture-scan-*.json`。

7. `export`
   - 只读取 `latest-scan.json`。
   - 如果最近结果不是 `mode=production`，返回 `fixture_export_forbidden`。
   - 默认输出到 results 目录；CLI 可指定 `--output`，Web 不允许。

8. `cleanup`
   - 确认 run-state 中的进程不再运行，且 socket 不再响应。
   - 删除 socket、pid 和 run-state。

## Native Agent

| 文件 | 职责 |
| --- | --- |
| `native/agent/src/agent.mm` | 注入后启动 UDS server、读取请求、写响应、日志、socket 目录权限、runtime debug dump |
| `native/agent/src/wechat_4_1_8_arm64.mm` | 单版本 adapter 状态判断、fixture profile/contacts/scan、unresolved primitive 错误 |
| `native/agent/src/adapter.hpp` | adapter C 接口 |
| `native/agent/src/host.mm` | fixture smoke 使用的简单宿主进程 |
| `native/agent/Makefile` | 构建 `libmacfriends_agent.dylib` 和 `macfriends-host` |

### Agent 启动逻辑

agent 通过 Objective-C `constructor` 在目标进程加载时启动。它会：

1. 读取 `MACFRIENDS_ADAPTER_PATH`。
2. 用 `MFBuildAdapterStatus` 判断当前进程是否应启动 agent server。
3. 仅当 fixture 模式或目标 runtime 支持时启动 UDS server。
4. 创建 socket 父目录并 chmod `0700`，socket 文件本身 chmod `0600`。
5. 监听 `MACFRIENDS_AGENT_SOCKET`。
6. 对每个请求调用 `MFHandleAdapterRequest`。

真实运行态支持条件当前在 adapter 内硬编码为：

```text
bundle_id = com.tencent.flue.WeChatAppEx
bundle_version = 2.4.1.19024
arch = arm64
```

如果是主 WeChat 进程或其他版本，agent 会记录 skip 或返回 `version_mismatch` / `adapter_not_loaded`。

### Primitive 状态

当前 adapter 的 private primitives 尚未闭环：

| 模式 | profile | contacts | scan |
| --- | --- | --- | --- |
| fixture | `fixture` | `fixture` | `fixture` |
| unsupported target | `blocked` | `blocked` | `blocked` |
| supported target but unresolved | `unresolved` | `unresolved` | `unresolved` |
| production ready 目标态 | `resolved` | `resolved` | `resolved` |

只有最后一种才允许被文档和 release 描述为生产 Ready。

## Web 控制台

`crates/cli/src/web.rs` 是单文件轻量 HTTP server。

核心逻辑：

- `serve` 绑定 `127.0.0.1:8765` 或 `--addr`。
- 启动时用 `/usr/bin/uuidgen` 生成内存 token。
- `GET /` 返回内置 HTML。
- `GET /api/*` 调用当前二进制的 `macfriends --json <command>`。
- `POST /api/*` 必须带 `X-MacFriends-Token`。
- 不开放通配 CORS。
- `/api/export` 只接收 `{ "format": "csv|json" }`，不能传 `output`。

Web API 是 CLI 的包装层，不拥有第二套业务逻辑。

## 打包与安装脚本

| 文件 | 职责 |
| --- | --- |
| `Makefile` | `build`、`agent`、`test`、`package`、`ready`、`install-local` |
| `scripts/release-guard.sh` | 检查 adapter release channel 和 primitive 状态，阻止误打生产包 |
| `scripts/package.sh` | 使用 Cargo 版本生成 `dist/macfriends-<version>-macos-arm64.tar.gz` |
| `scripts/install.sh` | 从源码构建产物或 release 包安装 CLI/bundle，并保留上一版 |
| `scripts/smoke-fixture.sh` | 启动 fixture host，验证 CLI、agent、Web API 和 fixture/export 边界 |

`scripts/package.sh` 从 `crates/cli/Cargo.toml` 读取版本，所以 Cargo package version 是发行包命名的单一事实来源。

## 数据和错误边界

关键输出 schema 在 `model.rs`。维护时要保持：

- `--json` 成功输出必须可被脚本稳定解析。
- 失败输出必须包含 `ok=false`、`command`、`error_code`、`message`、`causes`。
- `status` 比 `doctor` 更适合产品级巡检，因为它包含生命周期、路径、扫描摘要和下一步动作。
- `release_blockers` 不是警告，是不能发布或不能视为真实 Ready 的原因。

固定错误码由 `classify_error_code` 和 `error_exit_code` 维护。新增 native 或 CLI 错误时，需要同时更新：

- `docs/troubleshooting.md`
- `docs/configuration.md` 或 `docs/modules.md` 中相关字段
- README 中的错误码或 Ready 边界描述
- 相关单元测试或 fixture smoke

## 扩展新 WeChat Adapter

新增 adapter 时，最小路径是：

1. 新增或更新 `fixtures/*.json` manifest，明确 bundle/version/arch/resolver/release_channel/primitive_resolution。
2. 在 `native/agent/src` 增加 adapter 实现或 registry 分发。
3. 更新 `assess_path` / agent probe 逻辑，避免只认旧版本。
4. 更新 release guard，让它检查正确 manifest。
5. 增加 fixture 或 smoke 测试。
6. 更新 `docs/compatibility.md`、`docs/configuration.md`、`docs/modules.md` 和 README。

如果真实 primitive 尚未 resolved，必须保持 beta channel 和 release blocker，不得用 fixture 结果代替生产结果。

## 文档同步清单

改动以下代码时必须同步文档：

- `cli.rs`: 命令、别名、参数。
- `model.rs`: JSON schema、状态字段、扫描记录。
- `layout.rs`: 路径和环境变量。
- `app.rs`: 生命周期、Ready 门禁、scan/export、错误码。
- `web.rs`: API、token、安全策略。
- `native/agent/src/*.mm`: runtime 支持条件、primitive 状态、agent 日志。
- `scripts/*.sh` / `Makefile`: 安装、打包、release、smoke。
- `fixtures/*.json`: adapter 元数据、状态码、release channel。
