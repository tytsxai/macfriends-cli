# 运行与发布操作

## 发布前检查

生产发布前，至少确认以下命令全部成立：

```bash
make ready
cargo run -p macfriends -- doctor --json
cargo run -p macfriends -- launch --login --json
cargo run -p macfriends -- attach --json
```

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
- 恢复后先运行 `doctor --json`，再运行 `launch` / `attach` 进行活体验证。

## 诊断入口

默认日志路径：
- `~/Library/Application Support/MacFriends/logs/cli.log`
- `~/Library/Application Support/MacFriends/logs/agent.log`

重点排查：
- `agent 未运行`：先检查 `launch` 是否超时、socket 是否生成。
- `fixture_enabled = true`：说明当前不是生产运行态，必须退出后重新以正式链路启动。
- `runtime_ready = false` 且 `primitive_resolution` 为 `unresolved`：表示核心真实原语尚未闭环，项目仍不可上线。
