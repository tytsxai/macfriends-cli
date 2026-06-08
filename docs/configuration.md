# 配置说明

MacFriends 的配置面很小，核心原则是本机私有目录、显式 adapter manifest、结构化状态文件和少量环境变量。日常用户优先通过 CLI 参数配置；环境变量主要给测试、fixture、adapter 开发和故障定位使用。

## 默认目录

运行目录由 `crates/cli/src/layout.rs` 定义。

| 类型 | 默认路径 | 说明 |
| --- | --- | --- |
| root | `~/Library/Application Support/MacFriends` | MacFriends 本机数据根目录 |
| runtime | `~/Library/Application Support/MacFriends/runtime` | 受控 WeChat 副本、运行态文件和 runtime agent |
| managed app | `~/Library/Application Support/MacFriends/runtime/WeChat.app` | `prepare` 复制出的受控副本 |
| runtime bin | `~/Library/Application Support/MacFriends/runtime/bin` | `launch` 注入使用的 dylib 和 host |
| bundle | `~/Library/Application Support/MacFriends/bundle` | 安装后的 native 资产和 adapter 模板 |
| results | `~/Library/Application Support/MacFriends/results` | 扫描结果、导出文件和历史记录 |
| logs | `~/Library/Application Support/MacFriends/logs` | CLI 日志 |
| IPC | `/tmp/macfriends-$USER` | agent socket 和默认 agent 日志 |

这些目录会以 owner-private 方式创建。CLI 和 agent 都会确保 socket 父目录权限为 `0700`。

## 生成文件

| 文件 | 写入者 | 说明 |
| --- | --- | --- |
| `runtime/adapter.json` | `prepare` | 从 adapter 模板复制出的运行态 manifest |
| `runtime/target-status.json` | `prepare` | 受控副本的 bundle、version、arch 和支持状态 |
| `runtime/run-state.json` | `launch` / `detach` | 受控进程 PID、runtime PID、socket、adapter 和启动时间 |
| `runtime/wechat.pid` | `launch` | 受控主进程 PID |
| `results/latest-scan.json` | `scan` production 模式 | 最近一次正式链路扫描 |
| `results/history/scan-*.json` | `scan` production 模式 | 正式链路历史，最多保留 100 份 |
| `results/latest-fixture-scan.json` | `scan` fixture 模式 | 最近一次 fixture 扫描 |
| `results/fixture/fixture-scan-*.json` | `scan` fixture 模式 | fixture 历史，最多保留 20 份 |
| `results/latest-scan-export.json` | `export --format json` | 默认 JSON 导出 |
| `results/latest-scan-export.csv` | `export --format csv` | 默认 CSV 导出 |
| `logs/cli.log` | CLI | JSON Lines 命令事件日志，10 MiB 轮转 |
| `/tmp/macfriends-$USER/agent.log` | agent | native agent 文本日志，10 MiB 轮转 |
| `/tmp/macfriends-$USER/agent.sock` | agent | Unix Domain Socket |

`export` 只读取 `latest-scan.json`，并要求 `mode=production`。fixture 结果不会被导出为正式结果。

## CLI 参数

| 命令 | 参数 | 默认值 | 说明 |
| --- | --- | --- | --- |
| 全局 | `--json` | false | 输出结构化 JSON；失败时也输出错误对象 |
| `prepare` | `--source-app` | `/Applications/WeChat.app` | 指定源 WeChat.app |
| `prepare` | `--force` | false | 强制重新复制并重签名受控副本 |
| `launch` | `--login` | false | 显式表示准备登录，并设置 `MACFRIENDS_LOGIN_MODE=1` |
| `scan` | `--all` | false | 全量扫描联系人 |
| `export` | `--format json|csv` | `json` | 导出格式 |
| `export` | `--output` | results 默认文件 | CLI 自定义导出路径；Web API 不允许传自定义路径 |
| `serve` | `--addr` | `127.0.0.1:8765` | Web 控制台监听地址 |
| `serve` | `--open` | false | 启动后用默认浏览器打开 |

中文命令别名只替换命令名，不替换参数名。

## 环境变量

| 变量 | 读取方 | 用途 | 默认值 / 说明 |
| --- | --- | --- | --- |
| `MACFRIENDS_AGENT_SOCKET` | CLI / agent | 指定 agent UDS socket 路径 | `/tmp/macfriends-$USER/agent.sock` |
| `MACFRIENDS_LOG_FILE` | CLI launch 注入 / agent | 指定 agent 日志路径 | `/tmp/macfriends-$USER/agent.log` |
| `MACFRIENDS_ADAPTER_TEMPLATE` | CLI | 覆盖 `prepare` 读取的 adapter 模板 | 主要用于 adapter 开发 |
| `MACFRIENDS_ADAPTER_PATH` | agent / release guard | agent 运行态 manifest；release guard 可指定检查 manifest | CLI `launch` 会设置为 `runtime/adapter.json` |
| `MACFRIENDS_ENABLE_FIXTURE` | agent / smoke | fixture 模式开关 | `1` 时返回 mock profile/contacts/scan |
| `MACFRIENDS_ALLOW_BETA_RELEASE` | release guard | 允许 beta/testable 打包 | 当前 beta adapter 打包必须设为 `1` |
| `MACFRIENDS_LOGIN_MODE` | launch 注入 / agent 环境 | 标记用户预期登录 | `launch --login` 时为 `1` |
| `MACFRIENDS_DEBUG_CLASS_FILTER` | agent | Objective-C runtime 类/方法调试 dump 过滤词 | 仅用于 native 原语定位 |
| `PREFIX` | install.sh | 安装根目录 | `~/.local` |

`launch` 会先清理外部传入的敏感运行变量，再写入自己的 `DYLD_INSERT_LIBRARIES`、socket、adapter、log 和 login 配置，避免继承污染的 DYLD 环境。

## Adapter Manifest

当前模板是 `fixtures/adapter.wechat-macos-arm64.json`，安装后复制到 `bundle/`，`prepare` 时再写入 `runtime/adapter.json`。

| 字段 | 当前值 | 说明 |
| --- | --- | --- |
| `bundle_id` | `com.tencent.xinWeChat` | 主 WeChat bundle id |
| `bundle_version` | `4.1.8` | 静态版本标识 |
| `build_target` | `4.1.8` | CLI 静态门禁使用的目标版本 |
| `arch` | `arm64` | 目标架构 |
| `resolver_mode` | `signature_scan` | adapter 解析模式标记 |
| `release_channel` | `beta` | release guard 使用的发布通道 |
| `primitive_resolution.profile` | `unresolved` | profile 原语状态 |
| `primitive_resolution.contacts` | `unresolved` | contacts 原语状态 |
| `primitive_resolution.scan` | `unresolved` | scan 原语状态 |
| `executable_name` | `WeChat` | 主可执行名 |
| `adapter_name` | `wechat_4_1_8_arm64` | adapter 标识 |
| `scan_status_codes` | `0xB1/0xB2/0xB3/0x00` | 扫描状态码到状态名映射 |

注意：native agent 当前真实附着 Ready 门禁在 `WeChatAppEx 2.4.1.19024` 上完成。主 WeChat 4.1.8 是 prepare 和 launch 的受控副本入口。

## Web 控制台配置

`macfriends serve` 默认监听：

```text
127.0.0.1:8765
```

每次启动生成一个内存态 token，并注入内置页面。所有 `POST /api/*` 必须携带：

```text
X-MacFriends-Token: <current-session-token>
```

Web 后端复用当前二进制执行 `macfriends --json <command>`，所以不会绕过 CLI 的错误码、Ready 门禁和导出限制。

## 日志与保留策略

- CLI 日志是 JSON Lines，便于脚本按 `command`、`status`、`detail` 过滤。
- agent 日志是文本行，记录 socket、request method、fixture 状态、runtime dump 等 native 信息。
- 单个日志文件达到 10 MiB 后轮转到 `.1`。
- 正式扫描历史保留最近 100 份，fixture 历史保留最近 20 份。
- CSV 导出会中和以 `=`, `+`, `-`, `@` 开头的单元格，降低表格公式注入风险。

## 配置变更维护规则

凡是改动以下内容，必须同步更新本文档和相关用户文档：

- CLI 命令、参数、JSON 字段或错误码。
- runtime、results、logs、socket、bundle 路径。
- adapter manifest 字段、发布通道、primitive 状态。
- Web API 路径、请求体、token 或导出策略。
- 扫描结果 schema、历史保留上限、日志轮转策略。
- 安装、打包、release guard 行为。
