# MacFriends · macOS WeChat Friend Relationship Inspector CLI

[![Release](https://img.shields.io/github/v/release/tytsxai/macfriends-cli)](https://github.com/tytsxai/macfriends-cli/releases) · [中文 README](README.md) · [llms.txt](llms.txt) · [Docs](docs/README.md) · [Issues](https://github.com/tytsxai/macfriends-cli/issues) · [MIT License](LICENSE)

MacFriends is a **local-first macOS WeChat friend relationship inspector CLI** for Apple Silicon. It uses a Rust CLI to orchestrate a controlled, ad-hoc-signed WeChat copy, injects an Objective-C++ agent into the supported runtime, exposes local `profile`, `contacts`, and `scan` primitives over Unix Domain Socket IPC, and exports raw results as JSON or CSV.

This project is not a cloud "who deleted me on WeChat" service. It is a maintainability-first toolkit for users and developers who need local execution, explicit version gates, scriptable output, fixed error codes, rollback paths, and honest readiness checks.

## Search Keywords

macOS WeChat friend checker, WeChat ghost contact macOS, who deleted me on WeChat macOS, WeChat relationship inspector CLI, Apple Silicon WeChat tool, Rust macOS CLI, Objective-C++ agent, Unix Domain Socket IPC, ad-hoc signed WeChat copy, 微信单删检测 macOS, 微信好友关系检测.

## Who It Is For

- Apple Silicon Mac users who need local WeChat contact and relationship inspection.
- Developers who prefer a scriptable CLI and JSON output over a click-only GUI.
- Maintainers studying macOS app-copy preparation, ad-hoc signing, DYLD injection, UDS IPC, and Rust + Objective-C++ agent design.
- Contributors who want to extend a fixed-version WeChat adapter.

Not a fit for:

- Intel Mac, Windows, or Linux users.
- Users who need arbitrary latest WeChat version support.
- Users expecting a one-click interpretation of "who removed me". MacFriends exports raw `contacts` and `scan` data; relationship interpretation is left to the user.
- Users expecting an official WeChat API or cloud service. MacFriends is not affiliated with Tencent or WeChat.

## Core Features

- Rust CLI commands: `doctor`, `status`, `prepare`, `launch`, `attach`, `profile`, `contacts`, `scan`, `export`, `detach`, `cleanup`, `serve`.
- Chinese command aliases: `状态`, `准备`, `启动`, `连接`, `资料`, `联系人`, `扫描`, `导出`, `断开`, `清理`, `控制台`.
- Controlled WeChat copy: `prepare` copies `/Applications/WeChat.app` into the MacFriends runtime directory and ad-hoc signs it without modifying the original app.
- Local Objective-C++ agent over Unix Domain Socket JSON RPC.
- Local web console: `macfriends serve --open`, listening on `127.0.0.1:8765` by default.
- Script-friendly `--json` success and failure output.
- JSON/CSV export for the latest production scan.
- Readiness gates: `runtime_ready`, `fixture_enabled`, `release_blockers`, and `primitive_resolution`.
- Stable error codes such as `version_mismatch`, `adapter_not_loaded`, `profile_primitive_unresolved`, `contacts_primitive_unresolved`, `scan_primitive_unresolved`, and `rpc_timeout`.

## Current Status and Limits

MacFriends is currently a `0.1.x` beta project. The repository already includes the CLI, controlled-copy flow, native agent, fixture smoke test, local web console, installer, packaging workflow, logs, and readiness gates.

Important limits:

- macOS + Apple Silicon (`arm64`) only.
- Current adapter is locked to `WeChat 4.1.8`, bundle id `com.tencent.xinWeChat`, `arm64`, and `signature_scan`.
- The production attach point is `WeChatAppEx 2.4.1.19024`.
- In the current source, unresolved private primitives fail conservatively with `profile_primitive_unresolved`, `contacts_primitive_unresolved`, or `scan_primitive_unresolved`. Do not treat those as production-ready scan results.
- Fixture mode is for testing and CI only.
- This repository does not distribute WeChat installers and does not automatically track the latest WeChat release.

## Quick Start

From source:

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

Local install:

```bash
make install-local
macfriends doctor
macfriends status
macfriends serve --open
```

Package:

```bash
make package
```

See [docs/install.md](docs/install.md), [docs/operations.md](docs/operations.md), and [docs/README.md](docs/README.md) for the full workflow.

## Tech Stack

| Layer | Technology | Role |
| --- | --- | --- |
| CLI | Rust 2024, clap, serde, anyhow | Commands, status aggregation, export, web orchestration |
| Native agent | Objective-C++ | Runtime attachment and local primitive handling |
| IPC | Unix Domain Socket | Local JSON RPC at `/tmp/macfriends-$USER/agent.sock` |
| Web console | Built-in Rust HTTP server | Local UI backed by the same CLI JSON commands |
| Packaging | Makefile + shell scripts | Native assets, tar.gz package, local installation |

## Compatibility

| Item | Current value |
| --- | --- |
| Platform | macOS |
| Architecture | Apple Silicon `arm64` |
| WeChat bundle | `com.tencent.xinWeChat` |
| WeChat version | `4.1.8` |
| Runtime component | `WeChatAppEx 2.4.1.19024` |
| Adapter | `wechat_4_1_8_arm64` |
| Resolver | `signature_scan` |

## Production Readiness Gate

A run is production-ready only when `doctor --json` or `attach --json` reports:

```text
runtime_ready = true
fixture_enabled = false
release_blockers = []
primitive_resolution.profile = resolved
primitive_resolution.contacts = resolved
primitive_resolution.scan = resolved
```

If `primitive_resolution` is `unresolved`, `blocked`, or `fixture`, the project may be buildable and testable, but it is not ready for real production scanning.

## Local Web Console

```bash
macfriends serve --open
```

Default URL:

```text
http://127.0.0.1:8765
```

The web console calls the same `macfriends --json` backend, so it does not bypass CLI readiness checks.

![MacFriends local web console for macOS WeChat friend checking](docs/assets/macfriends-console-cn.jpg)

![MacFriends status API JSON for WeChat friend checker CLI](docs/assets/macfriends-api-status-cn.jpg)

## Documentation

- [README.md](README.md): Chinese-first project overview, quick start, limits, readiness gate, FAQ.
- [docs/README.md](docs/README.md): Documentation map and reader paths.
- [docs/中文用户指南.md](docs/中文用户指南.md): Chinese user workflow.
- [docs/install.md](docs/install.md): Installation and package layout.
- [docs/operations.md](docs/operations.md): Release, rollback, backup, diagnostics.
- [docs/compatibility.md](docs/compatibility.md): WeChat version and adapter boundary.
- [docs/architecture.md](docs/architecture.md): CLI, agent, controlled copy, web console, readiness design.
- [llms.txt](llms.txt): LLM and AI-search-friendly project summary.

## GitHub Topics Suggestion

`wechat`, `macos`, `apple-silicon`, `rust-cli`, `objective-cpp`, `friend-checker`, `unix-domain-socket`, `local-first`, `wechat-tool`, `cli-tool`

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=tytsxai/macfriends-cli&type=Date)](https://www.star-history.com/#tytsxai/macfriends-cli&Date)

## License

MIT. See [LICENSE](LICENSE).
