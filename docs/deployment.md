# 部署方式

MacFriends 的真实运行目标是本机 macOS Apple Silicon。它需要复制并启动本机 WeChat 受控副本，通过 `DYLD_INSERT_LIBRARIES` 注入本地 agent，因此“部署”主要分为源码本地部署、发行包部署、容器内静态验证和服务器侧边界说明。

## 部署矩阵

| 方式 | 是否支持真实扫描链路 | 适用场景 | 关键限制 |
| --- | --- | --- | --- |
| 本地源码部署 | 支持，但当前 primitives 未 resolved 时仍 Not Ready | 开发、调试、adapter 维护 | 必须是 macOS arm64，且本机有支持版本 WeChat |
| beta 发行包部署 | 支持同一套本机链路 | 给测试用户安装 CLI 与 native 资产 | 当前需要 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 构建 |
| 容器部署 | 不支持真实扫描链路 | Rust 静态检查、文档检查、部分单元测试 | 容器无法运行 macOS WeChat GUI 与 DYLD 注入目标 |
| 服务器部署 | 不支持真实扫描链路 | 发布制品托管、CI、文档站、issue triage | 不要把 `serve` 暴露到公网，也不要声称服务器能代跑微信扫描 |

## 本地源码部署

适合维护者和 adapter 开发者。

```bash
cargo build
make -C native/agent artifacts
cargo run -p macfriends -- doctor
cargo run -p macfriends -- status
cargo run -p macfriends -- prepare
cargo run -p macfriends -- launch --login
cargo run -p macfriends -- attach
```

安装到当前用户：

```bash
make install-local
```

`make install-local` 会执行 release 构建、native agent 构建，并调用 `scripts/install.sh`。安装位置：

| 资产 | 位置 |
| --- | --- |
| CLI | `~/.local/bin/macfriends` |
| bundled agent | `~/Library/Application Support/MacFriends/bundle/bin/libmacfriends_agent.dylib` |
| agent host | `~/Library/Application Support/MacFriends/bundle/bin/macfriends-host` |
| adapter 模板 | `~/Library/Application Support/MacFriends/bundle/adapter.wechat-macos-arm64.json` |

源码调试时，如果改了 `native/agent` 或 `fixtures/adapter.wechat-macos-arm64.json`，执行：

```bash
make -C native/agent artifacts
macfriends cleanup
macfriends prepare --force
```

`prepare --force` 会同步仓库里的最新 native 构建产物到 runtime 目录。若受控进程或 agent socket 仍在运行，`prepare` 会拒绝覆盖。

## 发行包部署

当前 adapter 是 beta 通道，真实私有原语仍未 resolved。默认打包会被 `scripts/release-guard.sh` 阻止。明确要生成 beta/testable 产物时：

```bash
MACFRIENDS_ALLOW_BETA_RELEASE=1 make package
```

输出：

```text
dist/macfriends-0.1.2-macos-arm64.tar.gz
dist/macfriends-0.1.2-macos-arm64.tar.gz.sha256
```

在目标机器上：

```bash
tar -xzf macfriends-0.1.2-macos-arm64.tar.gz
cd macfriends-0.1.2-macos-arm64
./install.sh
macfriends doctor --json
macfriends status --json
```

安装脚本会保留上一版：

| 备份 | 位置 |
| --- | --- |
| CLI 备份 | `~/.local/bin/macfriends.previous` |
| bundle 备份 | `~/Library/Application Support/MacFriends/bundle.previous` |

## 容器内验证

容器只能用于不依赖 macOS GUI、codesign、WeChat.app 和 DYLD 注入的验证。可以做：

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

不能做：

- 真实 `prepare`，因为没有 `/Applications/WeChat.app`。
- 真实 `launch` / `attach`，因为没有 macOS WeChat 进程。
- native Objective-C++ agent 的完整 macOS 运行验证。
- Web 控制台真实扫描，因为它最终仍调用本机 CLI 和 agent。

如果 CI 运行在 macOS runner，可以执行 `make ready` 的更多步骤；但当前 beta adapter 仍需要 `MACFRIENDS_ALLOW_BETA_RELEASE=1` 才能通过打包门禁。

## 服务器部署边界

MacFriends 不是云服务，也没有服务端账号、队列或远程扫描架构。服务器侧只建议承担：

- 发布 GitHub Release 和校验文件。
- 托管 README/docs。
- 跑不触碰真实微信的 CI 检查。
- 收集 issue、崩溃日志片段和 adapter 维护信息。

不要这样部署：

```bash
macfriends serve --addr 0.0.0.0:8765
```

`serve` 会执行本机 `prepare`、`launch`、`scan`、`export` 等操作，只应监听 `127.0.0.1`。Web 写操作有会话 token，但这不是公网认证系统。

## 生产 Ready 验收

无论通过哪种本机方式部署，最终验收必须看运行态字段：

```bash
macfriends doctor --json
macfriends status --json
macfriends launch --login --json
macfriends attach --json
```

必须同时满足：

```text
runtime_ready = true
fixture_enabled = false
release_blockers = []
primitive_resolution.profile = resolved
primitive_resolution.contacts = resolved
primitive_resolution.scan = resolved
```

当前源码默认返回 unresolved primitives，因此 beta 包只能作为工程链路和测试资产，不能描述为真实生产可用。
