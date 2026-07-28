# MacFriends · macOS 微信好友关系检测 CLI / WeChat Friend Inspector

[![Release](https://img.shields.io/github/v/release/tytsxai/macfriends-cli)](https://github.com/tytsxai/macfriends-cli/releases) · [English README](README.en.md) · [llms.txt](llms.txt) · [Docs](docs/README.md) · [Issues](https://github.com/tytsxai/macfriends-cli/issues) · [MIT License](LICENSE)

MacFriends 是一个面向 **macOS Apple Silicon** 的本地微信好友关系检测与扫描框架，英文定位是 **local-first macOS WeChat friend relationship inspector CLI**。它用 Rust CLI 编排受控微信副本，用 Objective-C++ agent 通过 Unix Domain Socket 暴露 `profile`、`contacts`、`scan` 等本地原语，并提供 JSON/CSV 导出、本地 Web 控制台、Ready 门禁和可诊断错误码。

它解决的问题不是“云端一键查僵尸粉”，而是给需要在本机、可审计、可脚本化地检查微信联系人关系的开发者和高级用户一个明确边界的工具链：固定支持版本、失败原因可见、原始数据可导出、线上可用性有门禁。

> **SEO / Search keywords**: macOS 微信好友关系检测, 微信单删检测 macOS, 微信清理僵尸粉 macOS, WeChat friend checker CLI, WeChat ghost contact macOS, Apple Silicon WeChat tool, Rust macOS CLI, Objective-C++ agent, Unix Domain Socket IPC, ad-hoc signed WeChat copy.

## 适合谁使用 / Who It Is For

- 使用 Apple Silicon Mac，并且希望在本机检测微信好友关系、联系人状态或单删线索的高级用户。
- 需要可脚本化、可导出、可排障的 `macfriends` CLI，而不是只能手点 GUI 的使用者。
- 研究 macOS WeChat 本地自动化、受控副本、动态库注入、UDS IPC、Rust + Objective-C++ agent 架构的开发者。
- 想基于固定版本 adapter 扩展 WeChat 原语解析、扫描结果导出或本地控制台的人。

不适合：

- Intel Mac、Windows、Linux 用户。
- 需要支持任意最新版微信的用户。
- 期待项目代替你解释“谁删了我”的用户。MacFriends 导出原始 `contacts` / `scan` 数据，关系判断需要你自行处理。
- 期待云端服务、商业背书或官方微信接口的用户。本项目不是腾讯/微信官方项目。

## 核心功能 / Core Features

- **Rust CLI**: `doctor`, `status`, `prepare`, `launch`, `attach`, `profile`, `contacts`, `scan`, `export`, `detach`, `cleanup`, `serve`。
- **中文命令别名**: `状态`, `准备`, `启动`, `连接`, `资料`, `联系人`, `扫描`, `导出`, `断开`, `清理`, `控制台`。
- **受控微信副本**: `prepare` 会复制 `/Applications/WeChat.app` 到 MacFriends runtime 目录并 ad-hoc 签名，不修改原始微信。
- **本地 agent 与 IPC**: Objective-C++ agent 通过 Unix Domain Socket 提供本地 JSON RPC。
- **本地 Web 控制台**: `macfriends serve --open` / `macfriends 控制台 --open` 启动只监听 `127.0.0.1` 的中文控制台。
- **结构化输出**: 支持 `--json`，成功和失败都返回可脚本处理的结构。
- **结果导出**: 最近一次正式链路扫描可导出 JSON 或 CSV，CSV 会中和疑似表格公式字段。
- **运维门禁**: `runtime_ready`, `fixture_enabled`, `release_blockers`, `primitive_resolution` 明确区分可用、测试和阻塞状态。
- **固定错误码**: `version_mismatch`, `adapter_not_loaded`, `profile_primitive_unresolved`, `contacts_primitive_unresolved`, `scan_primitive_unresolved`, `rpc_timeout` 等。
- **可打包安装**: `make install-local`, `make package`, `scripts/install.sh` 支持本地安装、beta 包安装和上一版本备份；当前原语未 resolved 时打包必须显式 opt-in。

## 技术栈 / Tech Stack

| 层 | 技术 | 说明 |
| --- | --- | --- |
| CLI | Rust 2024, clap, serde, anyhow | 命令、状态聚合、导出、Web 后端编排 |
| native agent | Objective-C++ | 注入受控微信运行态并处理本地原语请求 |
| IPC | Unix Domain Socket | 默认 socket 在 `/tmp/macfriends-$USER/agent.sock` |
| Web console | Rust 内置轻量 HTTP 服务 | 默认 `127.0.0.1:8765`，复用 CLI JSON 子命令 |
| packaging | Makefile + shell scripts | 构建 native 资产、打包 tar.gz、安装到本机 |

## 当前状态与限制 / Current Status and Limits

当前仓库是 `0.1.x` beta 阶段，已具备 CLI、受控副本、agent、fixture smoke、本地控制台、安装打包、日志和 Ready 门禁等工程闭环。

重要限制：

- 只支持 **macOS + Apple Silicon (`arm64`)**。
- 当前 adapter 锁定 **WeChat `4.1.8` + `com.tencent.xinWeChat` + `arm64` + `signature_scan`**。
- 生产运行态实际附着点是 **WeChatAppEx `2.4.1.19024`**。
- 当前源码中的真实私有原语解析仍以 `primitive_resolution=unresolved` 作为安全失败状态；看到 `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved` 时，不要把结果当成生产可用。
- 当前 adapter manifest 标记为 `release_channel=beta`。默认 `make ready` / `make package` 会被 release guard 阻止，除非你明确设置 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 构建 beta/testable artifact。
- fixture 模式仅用于测试和 CI，不代表真实微信扫描结果。
- 本项目不分发 WeChat 安装包，也不会自动跟进最新版微信。

## 快速开始 / Quick Start

源码开发路径：

```bash
cargo build
make -C native/agent artifacts
cargo run -p macfriends -- doctor
cargo run -p macfriends -- status
cargo run -p macfriends -- prepare
cargo run -p macfriends -- launch --login
cargo run -p macfriends -- attach
cargo run -p macfriends -- serve --open
```

本地安装：

```bash
make install-local
macfriends doctor
macfriends status
macfriends serve --open
```

常用中文路径：

```bash
macfriends 状态
macfriends 准备
macfriends 启动 --login
macfriends 连接
macfriends 扫描 --all
macfriends 导出 --format csv
macfriends 控制台 --open
```

beta 打包：

```bash
MACFRIENDS_ALLOW_BETA_RELEASE=1 make package
```

输出示例：

```text
dist/macfriends-0.1.3-macos-arm64.tar.gz
dist/macfriends-0.1.3-macos-arm64.tar.gz.sha256
```

当前 adapter 仍是 beta 通道；不带 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 时，发布门禁会阻止生成看起来像生产可用的包。

安装和发布细节见 [docs/install.md](docs/install.md) 与 [docs/operations.md](docs/operations.md)。

## 使用场景 / Usage Scenarios

- 在本机检查微信联系人关系，导出 JSON/CSV 后自行做差集、状态分析或审计。
- 用 `macfriends status --json` 做自动化巡检，确认微信版本、受控副本、agent、阻塞项和下一步动作。
- 用 `macfriends 控制台 --open` 给中文用户提供更直观的本地操作入口。
- 基于 `fixtures/adapter.wechat-macos-arm64.json` 开发新的 WeChat 版本 adapter。
- 研究受控 macOS App 副本、ad-hoc signing、DYLD 注入、Unix Socket agent 与 Rust CLI 的组合架构。

## 命令速查 / Commands

```bash
macfriends doctor
macfriends status
macfriends prepare [--source-app /Applications/WeChat.app] [--force]
macfriends launch --login
macfriends attach
macfriends profile
macfriends contacts
macfriends scan --all
macfriends export --format json
macfriends export --format csv
macfriends detach
macfriends cleanup
macfriends serve [--addr 127.0.0.1:8765] [--open]
```

`status` 是推荐日常入口，会汇总：

- 生命周期：`not_prepared` / `prepared` / `running_blocked` / `ready`
- 本机微信版本、受控副本版本、adapter 锁定版本和兼容提示
- agent socket、PID、fixture 状态、Ready 状态和 release blockers
- 最近一次正式链路扫描与 fixture 扫描摘要
- 结果目录、CLI 日志、agent 日志、socket 路径
- 下一步动作建议

## 版本策略 / Compatibility

| 项 | 当前值 |
| --- | --- |
| 平台 | macOS |
| 架构 | Apple Silicon `arm64` |
| WeChat bundle | `com.tencent.xinWeChat` |
| WeChat 版本 | `4.1.8` |
| 运行态 WeChatAppEx | `2.4.1.19024` |
| adapter | `wechat_4_1_8_arm64` |
| resolver | `signature_scan` |

如果 `prepare` / `doctor` 报 `reason=version_mismatch`：

1. 确认当前微信版本：
   ```bash
   defaults read /Applications/WeChat.app/Contents/Info CFBundleShortVersionString
   ```
2. 确认是否能使用 `WeChat 4.1.8 arm64`。本仓库不提供 WeChat 安装包。
3. 不愿降级时，等待或自行开发新 adapter。参考 [docs/compatibility.md](docs/compatibility.md)。

## Ready 门禁 / Production Readiness

只有 `doctor --json` 或 `attach --json` 同时满足以下条件，才可视为生产 Ready：

```text
runtime_ready = true
fixture_enabled = false
release_blockers = []
primitive_resolution.profile = resolved
primitive_resolution.contacts = resolved
primitive_resolution.scan = resolved
```

只要 `primitive_resolution` 仍是 `unresolved`、`blocked` 或 `fixture`，都不能把扫描结果当成真实生产结果。

当前源码的真实原语仍未 resolved，因此默认发布门禁会拒绝生成看起来像正式版的产物：

```bash
make ready
make package
```

如果你明确要生成 beta/testable artifact，必须显式 opt-in：

```bash
MACFRIENDS_ALLOW_BETA_RELEASE=1 make ready
MACFRIENDS_ALLOW_BETA_RELEASE=1 make package
```

真实生产发布前还需要执行：

```bash
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- status --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

`make ready` 会先执行 release guard；通过后再执行格式检查、clippy、测试、构建、native agent 构建、fixture smoke 和打包。

## 本地 Web 控制台 / Local Web Console

```bash
macfriends serve --open
# 或
macfriends 控制台 --open
```

默认监听：

```text
http://127.0.0.1:8765
```

Web 控制台调用同一个 `macfriends --json` 后端，不维护第二套业务逻辑。常用接口包括：

写操作会由控制台页面自动携带本次 `serve` 会话 token；没有 token 的跨站 `POST` 会被拒绝。Web 导出固定写入默认结果目录，如需自定义路径请使用 CLI `macfriends export --output`。

- `GET /api/status`
- `GET /api/compatibility`
- `GET /api/doctor`
- `GET /api/attach`
- `GET /api/profile`
- `GET /api/contacts`
- `GET /api/logs?kind=cli|agent`
- `POST /api/prepare`
- `POST /api/launch`
- `POST /api/scan`
- `POST /api/export`
- `POST /api/detach`
- `POST /api/cleanup`

更多说明见 [docs/web-console.md](docs/web-console.md)。

## 截图 / Screenshots

### 中文本地控制台

![MacFriends 中文本地控制台 - macOS 微信好友关系检测 Web UI](docs/assets/macfriends-console-cn.jpg)

### 状态 API 与兼容提示

![MacFriends 状态 API - WeChat friend checker CLI JSON output](docs/assets/macfriends-api-status-cn.jpg)

## 文档 / Documentation

- [docs/README.md](docs/README.md): 文档导航和读者路径。
- [docs/中文用户指南.md](docs/中文用户指南.md): 中文用户从控制台到 CLI 的完整路径。
- [docs/install.md](docs/install.md): 安装、打包、发布包结构。
- [docs/deployment.md](docs/deployment.md): 本地源码、beta 包、容器验证和服务器边界。
- [docs/configuration.md](docs/configuration.md): 路径、环境变量、adapter manifest、状态文件和保留策略。
- [docs/operations.md](docs/operations.md): 发布前检查、回滚、备份、诊断。
- [docs/compatibility.md](docs/compatibility.md): 版本兼容矩阵与 adapter 边界。
- [docs/architecture.md](docs/architecture.md): Rust CLI、native agent、Web 控制台和 Ready 门禁架构。
- [docs/modules.md](docs/modules.md): 文件级模块职责、核心逻辑和 adapter 扩展路径。
- [docs/troubleshooting.md](docs/troubleshooting.md): 常见失败、错误码和排障路径。
- [llms.txt](llms.txt): 给 AI 搜索引擎、LLM crawler 和引用系统使用的项目摘要。

## FAQ

**Q: 能不能用来做“微信单删检测”或“清理僵尸粉”？**

可以作为本地原始数据采集工具使用，但它不替你解释关系。你需要基于导出的 `contacts` 和 `scan` 数据自行判断。

**Q: 会修改我安装的微信吗？**

不会。`prepare` 在 MacFriends runtime 目录创建受控副本并 ad-hoc 重签名，`/Applications/WeChat.app` 不会被修改。

**Q: 为什么只支持 WeChat 4.1.8？**

这是当前 adapter 的安全边界。微信版本、bundle、架构或运行态不匹配时，工具会明确失败，而不是冒险假装兼容。

**Q: `primitive_resolution=unresolved` 是什么意思？**

说明真实私有原语尚未解析闭环。此时 CLI/Web 会返回明确错误，不能视为生产扫描可用。

**Q: 数据会上传吗？**

不会。CLI、agent、Web 控制台都在本机运行。Web 控制台默认只监听 `127.0.0.1`。

**Q: 支持 Intel Mac 吗？**

不支持。当前只支持 Apple Silicon `arm64`。

## GitHub Topics 建议

如果你在 GitHub 上维护本项目，可考虑添加这些 topics 以增强搜索发现：

`wechat`, `macos`, `apple-silicon`, `rust-cli`, `objective-cpp`, `friend-checker`, `unix-domain-socket`, `local-first`, `wechat-tool`, `cli-tool`

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=tytsxai/macfriends-cli&type=Date)](https://www.star-history.com/#tytsxai/macfriends-cli&Date)

## License

MIT，见 [LICENSE](LICENSE)。
