# 运行与发布操作

本文是维护者 runbook。部署方式见 [deployment.md](deployment.md)，配置项和路径见 [configuration.md](configuration.md)，模块职责见 [modules.md](modules.md)。

## 日常健康检查

建议排障和交接时先收集以下输出：

```bash
macfriends status --json
macfriends doctor --json
macfriends attach --json
```

如果 `attach` 失败，继续看：

```bash
tail -n 200 "$HOME/Library/Application Support/MacFriends/logs/cli.log"
tail -n 200 "/tmp/macfriends-$USER/agent.log"
ls -la "$HOME/Library/Application Support/MacFriends/runtime"
ls -la "/tmp/macfriends-$USER"
```

判断优先级：
1. `status.lifecycle` 确认卡在哪个阶段。
2. `release_blockers` 确认可恢复问题还是真实能力缺口。
3. `compatibility_warnings` 确认微信版本和受控副本状态。
4. `paths` 确认日志、结果和 socket 位置。

## 发布前检查

真实生产发布前，至少确认以下命令全部成立：

```bash
make ready
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- status --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

当前 adapter 仍有真实原语未 resolved，因此默认 `make ready` 会先被 release guard 阻止，避免产物看起来像正式可用版本。只有明确生成 beta/testable artifact 时，才运行：

```bash
MACFRIENDS_ALLOW_BETA_RELEASE=1 make ready
MACFRIENDS_ALLOW_BETA_RELEASE=1 make package
```

通过 release guard 后，`make ready` 会执行：
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build`
- native agent 构建
- fixture smoke：`scripts/smoke-fixture.sh`，包含 CLI、agent、Web API 基础接口
- beta 包生成或真实生产包生成，取决于 adapter release metadata

真实 Ready 判定以 `doctor --json` / `attach --json` 为准，必须满足：
- `target_supported = true`
- `runtime_ready = true`
- `fixture_enabled = false`
- `release_blockers = []`
- `primitive_resolution.profile/contacts/scan = resolved`

只要 `primitive_resolution` 中仍出现 `blocked` / `unresolved` / `fixture`，都不能上线。

`status --json` 是日常巡检入口，适合在排障、交接和自动化脚本里先跑一次。重点字段：
- `lifecycle`：整体阶段，常见值为 `not_prepared` / `prepared` / `running_blocked` / `ready`
- `lifecycle_label`：中文阶段名，供 Web 控制台和中文脚本提示使用
- `supported_wechat_version` / `installed_wechat_version` / `managed_wechat_version`：用于判断微信升级后是否仍兼容
- `release_blockers`：当前不能视为真实 Ready 的明确原因
- `compatibility_warnings`：微信版本、默认安装路径、受控副本不匹配等兼容提示
- `next_actions`：下一步建议，优先按这里处理
- `last_production_scan`：最近一次正式链路扫描摘要，只有它存在时才有可导出的正式链路结果
- `paths`：结果、日志、socket 等本机路径

需要图形化操作时，运行：

```bash
macfriends serve --open
# 或
macfriends 控制台 --open
```

Web 控制台仍然以 `doctor --json` / `status --json` / `attach --json` 的 Ready 判断为准，不会绕过真实运行态门禁。

## 变更类型与必跑检查

| 变更 | 必跑 |
| --- | --- |
| Rust CLI 参数、状态、导出 | `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings` |
| Web API 或控制台 | 上述 Rust 检查，加 `scripts/smoke-fixture.sh` |
| native agent | `make -C native/agent artifacts`, `scripts/smoke-fixture.sh`，真实机上 `prepare --force` 后验证 |
| adapter manifest | `scripts/release-guard.sh`，`macfriends doctor --json`，兼容文档同步 |
| 打包/安装脚本 | `MACFRIENDS_ALLOW_BETA_RELEASE=1 make package`，解压后执行 `./install.sh` 验证 |
| 文档 | 链接检查、README/docs 导航同步、OpenSpec tasks 更新 |

如果当前 adapter 仍 unresolved，`make ready` 默认阻塞是正确结果。只有明确验证 beta 产物时才带 `MACFRIENDS_ALLOW_BETA_RELEASE=1`。

## 升级与回滚

本地安装会自动保留上一版本：
- `~/.local/bin/macfriends.previous`
- `~/Library/Application Support/MacFriends/bundle.previous`

回滚步骤：
1. 停止当前受控进程，并确认 `macfriends cleanup` 可执行。
2. 用 `macfriends.previous` 覆盖当前二进制。
3. 用 `bundle.previous` 覆盖当前 `bundle/`。
4. 重新执行 `macfriends prepare --force`。
5. 再次执行 `doctor --json` 和 `attach --json` 验证 Ready 状态。

快速回滚命令示例：

```bash
macfriends detach || true
macfriends cleanup || true
cp ~/.local/bin/macfriends.previous ~/.local/bin/macfriends
rm -rf "$HOME/Library/Application Support/MacFriends/bundle"
/usr/bin/ditto "$HOME/Library/Application Support/MacFriends/bundle.previous" "$HOME/Library/Application Support/MacFriends/bundle"
macfriends prepare --force
macfriends status --json
```

如果 `cleanup` 提示受控进程仍在运行，先从 UI 退出受控 WeChat，或用 `run-state.json` 中记录的 PID 定位进程后再处理。

## 备份与恢复

建议在升级、改 adapter 或大改配置前做冷备份。优先先 `detach` / 退出受控 WeChat，再执行：

```bash
./scripts/backup.sh
# 或指定目录
./scripts/backup.sh /path/to/backup-root
```

脚本会把以下内容复制到 `~/Library/Application Support/MacFriends/backups/macfriends-backup-<UTC时间>/`：

```text
runtime/   results/   bundle/   logs/   （以及 /tmp 下的 agent.log，若存在）
```

也可手工备份同样目录：

```bash
~/Library/Application Support/MacFriends/runtime
~/Library/Application Support/MacFriends/results
~/Library/Application Support/MacFriends/bundle
~/Library/Application Support/MacFriends/logs
```

恢复原则：
- `results/latest-scan.json` 只接受 `mode=production` 的结果。
- `results/latest-fixture-scan.json` 仅用于测试，不参与正式导出。
- `results/history/` 默认仅保留最近 100 份正式链路扫描；`results/fixture/` 默认仅保留最近 20 份 fixture 扫描。
- 恢复后先运行 `status --json` 和 `doctor --json`，再运行 `launch` / `attach` 进行活体验证。
- 敏感状态与扫描结果文件按 owner-private（`0600`）写入；恢复后若权限被放宽，建议重新 `chmod 600` 关键文件。

运行目录约束：
- `prepare` 前若发现受控进程或 agent 仍在运行，会直接失败，避免在活跃进程上覆盖运行资产。
- `prepare --force` 会清理历史遗留的运行目录 bundle 碎片，并同步当前仓库里的最新 native 构建产物。

不建议备份 `/tmp/macfriends-$USER/agent.sock`，socket 是运行态临时文件。恢复后如果 socket 残留但 agent 不可 ping，执行 `macfriends cleanup` 或删除该 socket。

## 诊断入口

默认日志路径：
- `~/Library/Application Support/MacFriends/logs/cli.log`
- `/tmp/macfriends-$USER/agent.log`

默认 IPC 路径：
- `/tmp/macfriends-$USER/agent.sock`

日志默认会在单文件达到 10 MiB 时轮转为 `.1` 备份，避免长期运行无限增长。

当前 adapter 仍处于 beta 通道，打包时需要显式执行 `MACFRIENDS_ALLOW_BETA_RELEASE=1 make package`。不带该变量时，`scripts/release-guard.sh` 会阻止生成发行包，避免把 unresolved primitives 误发布成生产可用版本。打包产物会同时生成 `.sha256` 校验文件，发布前建议校验压缩包完整性。

beta/发行包内的 `install.sh` 支持直接从解压目录安装；源码仓库内的 `scripts/install.sh` 支持从 `target/release` 和 `native/agent/build` 安装。两种模式都会原子替换当前二进制和 bundle，并保留上一版本备份。

重点排查：
- `agent 未运行`：先检查 `launch` 是否超时、socket 是否生成。
- `status.lifecycle = not_prepared`：先执行 `macfriends prepare`。
- `status.lifecycle = prepared_blocked`：通常是受控副本版本/架构不匹配，确认源微信版本后执行 `macfriends prepare --force`。
- `launch` 失败后：CLI 会尝试终止本次拉起的受控进程并回收 `run-state.json` / `wechat.pid` / `agent.sock`，随后可直接重试。
- `fixture_enabled = true`：说明当前不是真实运行态，必须退出后重新以正式链路启动。
- `runtime_ready = false` 且 `primitive_resolution` 为 `unresolved`：表示核心真实原语尚未闭环，项目仍不能作为真实扫描可用版本发布。

## 上线 Go / No-Go

在“马上生产上线并长期运行”场景下，按下面清单判定，不要只看编译通过。

| 检查项 | Go 条件 | 当前默认状态 |
| --- | --- | --- |
| 真实原语 | `primitive_resolution.profile/contacts/scan = resolved` | **No-Go**：源码仍为 `unresolved` |
| 运行态 | `runtime_ready=true` 且 `fixture_enabled=false` | **No-Go**（依赖上一项） |
| 版本锁定 | 本机/受控 WeChat 与 adapter `build_target` 一致 | 取决于用户本机微信版本 |
| 发布门禁 | 生产包不需 `MACFRIENDS_ALLOW_BETA_RELEASE` | **No-Go**：默认 release guard 拦截 |
| 工程门禁 | `cargo test` / clippy / fixture smoke / CI 绿 | 工程链路可 Go |
| 回滚 | `macfriends.previous` + `bundle.previous` 可用 | 安装脚本已支持 |
| 备份 | 升级前有 results/runtime/bundle 备份 | `scripts/backup.sh` |

**结论规则**：只要表中任一项为 No-Go，只能发 beta/testable 包或内部工程包，不能对外宣称“生产可用扫描工具”。

## 文档发布要求

每次发布前检查：

```bash
rg -n "0\\.1\\.|macfriends-.*macos-arm64|primitive_resolution|release_channel|WeChat 4\\.1\\.8|WeChatAppEx" README.md README.en.md docs llms.txt
```

确认：
- 版本号和包名与 `crates/cli/Cargo.toml` 一致。
- beta / production 文案与 manifest 的 `release_channel` 一致。
- Ready 条件和 unresolved primitive 文案未被淡化。
- 新命令、新路径、新 API 已写入 docs 导航和对应专题页。
