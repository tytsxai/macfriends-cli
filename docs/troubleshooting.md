# 排障

## 先看总状态

排障优先执行：

```bash
macfriends status --json
# 或
macfriends 状态 --json
```

重点看：
- `lifecycle`：当前卡在哪个阶段
- `release_blockers`：不能视为生产 Ready 的直接原因
- `next_actions`：下一步建议
- `paths.cli_log` / `paths.agent_log`：继续定位时要看的日志

## `prepare` 报找不到 WeChat.app

默认源路径是 `/Applications/WeChat.app`。如果你的安装位置不同，请使用：

```bash
macfriends prepare --source-app /path/to/WeChat.app
```

## `doctor` 或 `prepare` 显示 `version_mismatch`

当前项目只支持 `WeChat 4.1.8 (arm64)`。如果你的源 app 不是该版本，CLI 会允许准备受控副本，但会在 `target-status.json` 中记录不匹配状态，后续 `attach/profile/contacts/scan` 默认不可用。

## `attach` 连接不到 socket

请确认：
- 已执行 `macfriends launch --login`
- agent 动态库存在于 `~/Library/Application Support/MacFriends/runtime/bin/`
- 目标进程没有被系统安全策略阻止预加载
- socket 父目录可写；新版本 agent 会自动补建该目录，但若仍失败请检查磁盘权限与路径长度

如果你刚改过 `native/agent` 代码却发现受控副本行为没有变化，请重新执行：

```bash
macfriends cleanup
macfriends prepare --force
```

新版本会在 `prepare --force` 时覆盖旧的 bundle 资产；如果进程仍在运行，`prepare` 会直接拒绝执行。

如果 `launch --json` 失败，请优先查看返回的 `error_code` 与 `causes`，以及：
- `~/Library/Application Support/MacFriends/logs/cli.log`
- `/tmp/macfriends-$USER/agent.log`

## `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved`

这表示当前目标已经通过 4.1.8 版本门禁，但具体原语解析链尚未成功建立。CLI 不会回退到伪检测，也不会返回空跑结果。

## Ready 相关故障

- `lifecycle=not_prepared`：先执行 `macfriends prepare`。
- `lifecycle=prepared_blocked`：受控副本已存在但版本、bundle 或架构不满足门禁，确认源微信后执行 `macfriends prepare --force`。
- `lifecycle=process_without_agent`：存在受控进程但 socket 不可用，先退出受控 WeChat，再执行 `macfriends cleanup` 后重试。
- `runtime_ready=false`：表示 agent 已连通但真实运行态仍未满足上线要求。
- `fixture_enabled=true`：表示当前链路只适合测试，不能导出正式结果。
- `primitive_resolution=unresolved`：表示真实 `profile/contacts/scan` 原语尚未闭环，必须先修复再上线。
- `cleanup` 失败且提示 PID 仍在运行：先退出受控 WeChat，再执行清理。
- `launch` 超时或失败：CLI 会自动回收本次失败启动留下的 pid/socket/runtime 状态；若仍失败，再检查注入链路与 agent 日志。
