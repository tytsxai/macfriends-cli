# MacFriends 文档导航 / Documentation

这个目录记录 MacFriends 的真实使用、安装、架构、兼容性和排障信息。MacFriends 是一个 **macOS Apple Silicon 微信好友关系检测 CLI / local-first WeChat friend relationship inspector**，核心由 Rust CLI、Objective-C++ agent、Unix Domain Socket IPC、本地 Web 控制台和固定版本 adapter 组成。

## 推荐阅读路径

新用户建议按这个顺序阅读：

1. [../README.md](../README.md): 项目是什么、适合谁、快速开始、限制和 Ready 门禁。
2. [中文用户指南.md](中文用户指南.md): 中文用户如何用控制台或 CLI 完成状态检查、准备、启动、扫描和导出。
3. [install.md](install.md): 本地安装、发布包安装、打包产物和安装后的校验。
4. [compatibility.md](compatibility.md): 当前只支持 `WeChat 4.1.8 + arm64 + signature_scan` 的原因和边界。
5. [troubleshooting.md](troubleshooting.md): `version_mismatch`、`primitive_unresolved`、`rpc_timeout` 等常见问题。

开发者和维护者建议继续阅读：

1. [architecture.md](architecture.md): Rust CLI、native agent、受控副本、Web 控制台和 Ready 门禁的整体设计。
2. [operations.md](operations.md): 发布前检查、回滚、备份恢复、日志路径和生产 Ready 判断。
3. [web-console.md](web-console.md): 本地控制台和 `/api/*` HTTP 接口。

## 文档覆盖范围

| 文档 | 读者 | 内容 |
| --- | --- | --- |
| [中文用户指南.md](中文用户指南.md) | 中文终端用户 | 控制台、中文命令别名、状态字段、真实生产边界 |
| [install.md](install.md) | 安装者 / 发布者 | `make install-local`, `make package`, `install.sh`, Ready 校验 |
| [compatibility.md](compatibility.md) | 使用者 / adapter 开发者 | macOS、Apple Silicon、WeChat 4.1.8、manifest 边界 |
| [architecture.md](architecture.md) | 开发者 | CLI、agent、DYLD 注入、UDS RPC、Web API 复用 CLI |
| [operations.md](operations.md) | 维护者 | `make ready`、回滚、备份、日志、故障诊断 |
| [web-console.md](web-console.md) | Web 控制台用户 / 集成者 | `macfriends serve --open` 和本地 HTTP API |
| [troubleshooting.md](troubleshooting.md) | 排障者 | 错误码、阻塞项和处理路径 |

## 关键事实

- 平台：macOS + Apple Silicon (`arm64`)。
- 当前锁定微信：`WeChat 4.1.8`，bundle id 为 `com.tencent.xinWeChat`。
- 运行态附着点：`WeChatAppEx 2.4.1.19024`。
- adapter：`fixtures/adapter.wechat-macos-arm64.json`，`adapter_name=wechat_4_1_8_arm64`。
- 本地控制台：`macfriends serve --open`，默认 `http://127.0.0.1:8765`。
- 生产 Ready 条件：`runtime_ready=true`、`fixture_enabled=false`、`release_blockers=[]`，且 `profile/contacts/scan` 原语均为 `resolved`。
- 当前源码中的真实私有原语未闭环时会返回 `*_primitive_unresolved`，这是有意保守失败，不是可忽略警告。

## 常用命令

```bash
cargo build
make -C native/agent artifacts
cargo run -p macfriends -- status
cargo run -p macfriends -- serve --open
make install-local
make ready
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
