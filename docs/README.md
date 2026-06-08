# MacFriends 文档导航 / Documentation

这个目录记录 MacFriends 的真实使用、安装、部署、配置、架构、模块、运维和排障信息。MacFriends 是一个 **macOS Apple Silicon 微信好友关系检测 CLI / local-first WeChat friend relationship inspector**，核心由 Rust CLI、Objective-C++ agent、Unix Domain Socket IPC、本地 Web 控制台和固定版本 adapter 组成。

## 推荐阅读路径

新用户建议按这个顺序阅读：

1. [../README.md](../README.md): 项目是什么、适合谁、快速开始、限制和 Ready 门禁。
2. [中文用户指南.md](中文用户指南.md): 中文用户如何用控制台或 CLI 完成状态检查、准备、启动、扫描和导出。
3. [install.md](install.md): 本地安装、发布包安装、打包产物和安装后的校验。
4. [deployment.md](deployment.md): 本地源码、beta 包、容器验证和服务器边界。
5. [configuration.md](configuration.md): 路径、环境变量、adapter manifest、状态文件和保留策略。
6. [compatibility.md](compatibility.md): 当前只支持 `WeChat 4.1.8 + arm64 + signature_scan` 的原因和边界。
7. [troubleshooting.md](troubleshooting.md): `version_mismatch`、`primitive_unresolved`、`rpc_timeout` 等常见问题。

开发者和维护者建议继续阅读：

1. [architecture.md](architecture.md): Rust CLI、native agent、受控副本、Web 控制台和 Ready 门禁的整体设计。
2. [modules.md](modules.md): 文件级模块职责、核心逻辑、错误边界和 adapter 扩展路径。
3. [operations.md](operations.md): 发布前检查、回滚、备份恢复、日志路径和生产 Ready 判断。
4. [web-console.md](web-console.md): 本地控制台和 `/api/*` HTTP 接口。

## 文档覆盖范围

| 文档 | 读者 | 内容 |
| --- | --- | --- |
| [中文用户指南.md](中文用户指南.md) | 中文终端用户 | 控制台、中文命令别名、状态字段、真实生产边界 |
| [install.md](install.md) | 安装者 / 发布者 | `make install-local`, `make package`, `install.sh`, Ready 校验 |
| [deployment.md](deployment.md) | 维护者 / 发布者 | 本地源码、beta 包、容器验证、服务器边界、部署验收 |
| [configuration.md](configuration.md) | 维护者 / 集成者 | 默认目录、状态文件、环境变量、adapter manifest、Web 配置 |
| [compatibility.md](compatibility.md) | 使用者 / adapter 开发者 | macOS、Apple Silicon、WeChat 4.1.8、manifest 边界 |
| [architecture.md](architecture.md) | 开发者 | CLI、agent、DYLD 注入、UDS RPC、Web API 复用 CLI |
| [modules.md](modules.md) | 接手维护者 | Rust/native/scripts 文件级职责、核心流程、扩展规则 |
| [operations.md](operations.md) | 维护者 | `make ready`、回滚、备份、日志、故障诊断 |
| [web-console.md](web-console.md) | Web 控制台用户 / 集成者 | `macfriends serve --open` 和本地 HTTP API |
| [troubleshooting.md](troubleshooting.md) | 排障者 | 错误码、阻塞项和处理路径 |

## 关键事实

- 平台：macOS + Apple Silicon (`arm64`)。
- 当前锁定微信：`WeChat 4.1.8`，bundle id 为 `com.tencent.xinWeChat`。
- 运行态附着点：`WeChatAppEx 2.4.1.19024`。
- adapter：`fixtures/adapter.wechat-macos-arm64.json`，`adapter_name=wechat_4_1_8_arm64`。
- 本地控制台：`macfriends serve --open`，默认 `http://127.0.0.1:8765`。
- 真实 Ready 条件：`runtime_ready=true`、`fixture_enabled=false`、`release_blockers=[]`，且 `profile/contacts/scan` 原语均为 `resolved`。
- 当前源码中的真实私有原语未闭环时会返回 `*_primitive_unresolved`，这是有意保守失败，不是可忽略警告。
- 当前 adapter 是 `release_channel=beta`；默认 `make ready` / `make package` 会阻止出包，只有设置 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 才会生成 beta/testable artifact。

## 维护同步规则

任何开发任务只要改动以下内容，必须在同一个变更中更新 `docs/`：

- CLI 命令、参数、中文别名、JSON 输出字段或错误码。
- 默认路径、环境变量、安装目录、socket、日志、结果文件或保留策略。
- Web API、token、导出行为或本地控制台页面能力。
- adapter manifest、兼容版本、release channel、primitive 状态或 Ready 门禁。
- 打包、安装、release guard、smoke、回滚和部署流程。

本仓库采用 OpenSpec 作为规格驱动基线。新增功能、缺陷修复、架构调整、发布变更和文档体系调整都应在 `openspec/changes/` 下保留 proposal/design/tasks 上下文；紧急止血可以先处理，但事后必须补齐规格和文档。

## 常用命令

```bash
cargo build
make -C native/agent artifacts
cargo run -p macfriends -- status
cargo run -p macfriends -- serve --open
make install-local
MACFRIENDS_ALLOW_BETA_RELEASE=1 make ready
```

安装后：

```bash
macfriends 状态
macfriends 准备
macfriends 启动 --login
macfriends 连接
macfriends 扫描 --all
macfriends 导出 --format csv
macfriends 控制台 --open
```

## AI 搜索与引用

面向 AI 搜索引擎、LLM crawler 和自动摘要系统，请优先读取：

- [../llms.txt](../llms.txt)
- [../README.md](../README.md)
- [compatibility.md](compatibility.md)
- [operations.md](operations.md)

这些文件明确描述了项目定位、限制、快速开始、真实能力边界和生产 Ready 判断，避免把 MacFriends 误解为云端服务、官方微信接口或任意版本通用工具。
