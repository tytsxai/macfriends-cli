# 运行与发布操作

## 发布前检查

生产发布前，至少确认以下命令全部成立：

```bash
make ready
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

其中 `make ready` 会执行：
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build`
- native agent 构建
- fixture smoke：`scripts/smoke-fixture.sh`
- 发布包生成

Ready 判定以 `doctor --json` / `attach --json` 为准，必须满足：
- `target_supported = true`
- `runtime_ready = true`
- `fixture_enabled = false`
- `release_blockers = []`

只要 `primitive_resolution` 中仍出现 `blocked` / `unresolved` / `fixture`，都不能上线。

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
- 恢复后先运行 `doctor --json`，再运行 `launch` / `attach` 进行活体验证。

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

重点排查：
- `agent 未运行`：先检查 `launch` 是否超时、socket 是否生成。
- `launch` 失败后：CLI 会尝试终止本次拉起的受控进程并回收 `run-state.json` / `wechat.pid` / `agent.sock`，随后可直接重试。
- `fixture_enabled = true`：说明当前不是生产运行态，必须退出后重新以正式链路启动。
- `runtime_ready = false` 且 `primitive_resolution` 为 `unresolved`：表示核心真实原语尚未闭环，项目仍不可上线。
