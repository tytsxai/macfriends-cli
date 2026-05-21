# 运行与发布操作

## 发布前检查

生产发布前，至少确认以下命令全部成立：

```bash
make ready
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- status --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

其中 `make ready` 会执行：
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build`
- native agent 构建
- fixture smoke：`scripts/smoke-fixture.sh`，包含 CLI、agent、Web API 基础接口
- 发布包生成

Ready 判定以 `doctor --json` / `attach --json` 为准，必须满足：
- `target_supported = true`
- `runtime_ready = true`
- `fixture_enabled = false`
- `release_blockers = []`

只要 `primitive_resolution` 中仍出现 `blocked` / `unresolved` / `fixture`，都不能上线。

`status --json` 是日常巡检入口，适合在排障、交接和自动化脚本里先跑一次。重点字段：
- `lifecycle`：整体阶段，常见值为 `not_prepared` / `prepared` / `running_blocked` / `ready`
- `lifecycle_label`：中文阶段名，供 Web 控制台和中文脚本提示使用
- `supported_wechat_version` / `installed_wechat_version` / `managed_wechat_version`：用于判断微信升级后是否仍兼容
- `release_blockers`：当前不能视为生产 Ready 的明确原因
- `compatibility_warnings`：微信版本、默认安装路径、受控副本不匹配等兼容提示
- `next_actions`：下一步建议，优先按这里处理
- `last_production_scan`：最近一次正式扫描摘要，只有它存在时才有可导出的生产结果
- `paths`：结果、日志、socket 等本机路径

需要图形化操作时，运行：

```bash
macfriends serve --open
# 或
macfriends 控制台 --open
```

Web 控制台仍然以 `doctor --json` / `status --json` / `attach --json` 的 Ready 判断为准，不会绕过生产门禁。

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

## 备份与恢复

建议定期备份以下目录：

```bash
~/Library/Application Support/MacFriends/runtime
~/Library/Application Support/MacFriends/results
~/Library/Application Support/MacFriends/bundle
~/Library/Application Support/MacFriends/logs
```

恢复原则：
- `results/latest-scan.json` 只接受 `mode=production` 的结果。
- `results/latest-fixture-scan.json` 仅用于测试，不参与正式导出。
- `results/history/` 默认仅保留最近 100 份生产扫描；`results/fixture/` 默认仅保留最近 20 份 fixture 扫描。
- 恢复后先运行 `status --json` 和 `doctor --json`，再运行 `launch` / `attach` 进行活体验证。

运行目录约束：
- `prepare` 前若发现受控进程或 agent 仍在运行，会直接失败，避免在活跃进程上覆盖运行资产。
- `prepare --force` 会清理历史遗留的运行目录 bundle 碎片，并同步当前仓库里的最新 native 构建产物。

## 诊断入口

默认日志路径：
- `~/Library/Application Support/MacFriends/logs/cli.log`
- `/tmp/macfriends-$USER/agent.log`

默认 IPC 路径：
- `/tmp/macfriends-$USER/agent.sock`

日志默认会在单文件达到 10 MiB 时轮转为 `.1` 备份，避免长期运行无限增长。

打包产物会同时生成 `.sha256` 校验文件，发布前建议校验压缩包完整性。

发行包内的 `install.sh` 支持直接从解压目录安装；源码仓库内的 `scripts/install.sh` 支持从 `target/release` 和 `native/agent/build` 安装。两种模式都会原子替换当前二进制和 bundle，并保留上一版本备份。

重点排查：
- `agent 未运行`：先检查 `launch` 是否超时、socket 是否生成。
- `status.lifecycle = not_prepared`：先执行 `macfriends prepare`。
- `status.lifecycle = prepared_blocked`：通常是受控副本版本/架构不匹配，确认源微信版本后执行 `macfriends prepare --force`。
- `launch` 失败后：CLI 会尝试终止本次拉起的受控进程并回收 `run-state.json` / `wechat.pid` / `agent.sock`，随后可直接重试。
- `fixture_enabled = true`：说明当前不是生产运行态，必须退出后重新以正式链路启动。
- `runtime_ready = false` 且 `primitive_resolution` 为 `unresolved`：表示核心真实原语尚未闭环，项目仍不可上线。
