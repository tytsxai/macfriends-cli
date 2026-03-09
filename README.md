# MacFriends

MacFriends 是一个**面向 Apple Silicon 的 macOS 微信好友关系检测 CLI 新仓库**。

它与旧的 Windows 注入式项目完全分离：
- 新名称、新文档、新目录结构
- Rust CLI + Objective-C++ agent
- 只面向 macOS / Apple Silicon / 锁定微信 4.1.8
- 不做 App Store 分发，不依赖云端服务

## 当前状态

本仓库已经提供：
- 完整 CLI：`doctor` / `prepare` / `launch` / `attach` / `profile` / `contacts` / `scan` / `export` / `detach` / `cleanup`
- 单版本 adapter manifest：`WeChat 4.1.8 + arm64 + signature_scan`
- 受控副本准备、ad-hoc 签名、Unix Domain Socket IPC、结果导出
- 版本门禁、target status 持久化、运行态 Ready 门禁、固定错误码
- 本地安装脚本、回滚备份位与发布打包脚本

**注意：**
当前仓库已将主流程收敛到 `WeChat 4.1.8 (arm64)`，并实现了 adapter registry 与门禁。

只有当 `doctor --json` 或 `attach --json` 同时满足 `runtime_ready=true`、`fixture_enabled=false`、`release_blockers=[]` 时，才可视为达到生产 Ready。

若目标不满足 bundle/version/arch 条件，命令会明确失败，错误码固定为：
- `version_mismatch`
- `adapter_not_loaded`
- `resolver_validation_failed`
- `profile_primitive_unresolved`
- `contacts_primitive_unresolved`
- `scan_primitive_unresolved`
- `rpc_timeout`

## 目录

- `crates/cli`：命令行、配置、导出、IPC 客户端
- `native/agent`：macOS agent 动态库、受控宿主、版本 adapter
- `docs`：架构、兼容性、排障、安装文档
- `fixtures`：锁定版本 adapter 模板
- `scripts`：安装与打包脚本

## 快速开始

```bash
cargo build
make -C native/agent artifacts
cargo run -p macfriends -- doctor
cargo run -p macfriends -- prepare
cargo run -p macfriends -- launch --login
cargo run -p macfriends -- attach
```

## 安装与发布

```bash
make install-local
make package
```

详细说明见 `docs/install.md`。

## 命令

```bash
macfriends doctor
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
```

## 版本策略

- 仅支持 `Apple Silicon`
- 仅支持 `WeChat 4.1.8`
- 仅支持 `fixtures/adapter.wechat-macos-arm64.json` 中定义的 `arm64 + signature_scan`
- 若源微信版本与目标不匹配，`prepare` 会记录 `target-status.json`，后续 `attach/profile/contacts/scan` 将明确失败

## 本地 Smoke Test

fixture 模式只用于测试和 CI，不属于默认用户路径：

```bash
make -C native/agent artifacts
MACFRIENDS_AGENT_SOCKET="$HOME/Library/Application Support/MacFriends/runtime/agent.sock" \
MACFRIENDS_ADAPTER_PATH="$PWD/fixtures/adapter.wechat-macos-arm64.json" \
MACFRIENDS_ENABLE_FIXTURE=1 \
DYLD_INSERT_LIBRARIES="$PWD/native/agent/build/libmacfriends_agent.dylib" \
"$PWD/native/agent/build/macfriends-host"
```

另一个终端执行：

```bash
cargo run -p macfriends -- attach
cargo run -p macfriends -- profile
cargo run -p macfriends -- contacts
cargo run -p macfriends -- scan --all
```

## 许可

MIT，见 `LICENSE`。

## 生产发布门槛

```bash
make ready
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

若 `primitive_resolution` 仍是 `unresolved`，则说明核心真实原语尚未闭环，项目仍不可上线。详细操作见 `docs/operations.md`。
