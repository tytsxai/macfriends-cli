# 排障

排障时先按 `status --json` 的事实走，不要只看单个命令的文本输出。配置和路径参考 [configuration.md](configuration.md)，模块实现参考 [modules.md](modules.md)。

## 先看总状态

排障优先执行：

```bash
macfriends status --json
# 或
macfriends 状态 --json
```

重点看：
- `lifecycle`：当前卡在哪个阶段
- `release_blockers`：不能视为真实 Ready 的直接原因
- `next_actions`：下一步建议
- `paths.cli_log` / `paths.agent_log`：继续定位时要看的日志

## `prepare` 报找不到 WeChat.app

默认源路径是 `/Applications/WeChat.app`。如果你的安装位置不同，请使用：

```bash
macfriends prepare --source-app /path/to/WeChat.app
```

## `doctor` 或 `prepare` 显示 `version_mismatch`

当前项目只支持 `WeChat 4.1.8 (arm64)`。如果你的源 app 不是该版本，CLI 会允许准备受控副本，但会在 `target-status.json` 中记录不匹配状态，后续 `attach/profile/contacts/scan` 默认不可用。

确认版本：

```bash
defaults read /Applications/WeChat.app/Contents/Info CFBundleShortVersionString
defaults read /Applications/WeChat.app/Contents/Info CFBundleIdentifier
lipo -archs /Applications/WeChat.app/Contents/MacOS/WeChat
```

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
- `runtime_ready=false`：表示 agent 已连通但真实运行态仍未满足可用要求。
- `fixture_enabled=true`：表示当前链路只适合测试，不能导出正式结果。
- `primitive_resolution=unresolved`：表示真实 `profile/contacts/scan` 原语尚未闭环，必须先修复；打 beta 包也需要显式设置 `MACFRIENDS_ALLOW_BETA_RELEASE=1`。
- `cleanup` 失败且提示 PID 仍在运行：先退出受控 WeChat，再执行清理。
- `launch` 超时或失败：CLI 会自动回收本次失败启动留下的 pid/socket/runtime 状态；若仍失败，再检查注入链路与 agent 日志。

## 固定错误码

| error_code | 常见原因 | 处理 |
| --- | --- | --- |
| `version_mismatch` | bundle/version 不符合 adapter | 使用支持版本重新 `prepare --force`，或开发新 adapter |
| `adapter_not_loaded` | 架构或目标不匹配，agent 未加载目标 adapter | 检查 arm64、manifest、agent 日志 |
| `resolver_validation_failed` | 原语解析校验失败 | 查看 native adapter 解析逻辑和 agent 日志 |
| `profile_primitive_unresolved` | profile 原语未闭环 | 修复 adapter 原语，不要伪造成功 |
| `contacts_primitive_unresolved` | contacts 原语未闭环 | 修复 adapter 原语，不要伪造空列表 |
| `scan_primitive_unresolved` | scan 原语未闭环 | 修复 adapter 原语，不要导出伪结果 |
| `rpc_timeout` | agent 响应超时 | 看 socket、agent 日志、目标进程是否卡死 |
| `agent_boot_timeout` | launch 后 socket 未就绪 | 看 DYLD 注入、WeChatAppEx 是否启动、agent 日志 |
| `managed_app_missing` | runtime 受控副本不存在 | 执行 `macfriends prepare` |
| `agent_socket_conflict` | socket 已被 live agent 占用 | `macfriends detach` 或退出受控 WeChat |
| `agent_process_conflict` | run-state 中进程仍运行 | 退出受控 WeChat 后再操作 |
| `agent_unreachable` | socket 不存在或不可连接 | `launch --login` 后重试，检查 `/tmp/macfriends-$USER` |
| `web_bad_request` | Web API 请求体不完整或 JSON 格式错误 | 修正调用方请求；内置控制台正常不会触发 |
| `request_too_large` | RPC 或 Web 请求超过限制 | 缩小请求体，检查调用方 |
| `production_scan_missing` | 没有正式链路扫描 | 先让 Ready 通过，再 `scan --all` |
| `fixture_export_forbidden` | 尝试导出 fixture 结果 | 改用正式链路生成 production scan |

## 日志定位

CLI 日志是 JSON Lines：

```bash
tail -n 200 "$HOME/Library/Application Support/MacFriends/logs/cli.log"
```

agent 日志默认在：

```bash
tail -n 200 "/tmp/macfriends-$USER/agent.log"
```

若需要定位 Objective-C runtime 类或方法，可临时设置：

```bash
MACFRIENDS_DEBUG_CLASS_FILTER=Contact macfriends launch --login
```

该变量只用于 native 原语定位，输出量会被 agent 限制。排查完后不要保留在日常启动环境中。

## Web 控制台写操作返回 `web_csrf_required`

这表示请求没有携带当前 `serve` 会话生成的 `X-MacFriends-Token`。直接使用内置控制台页面时会自动带上 token；如果你用脚本调用 `POST /api/*`，需要改用 CLI 子命令，或在同一个控制台会话中显式带上该 header。

Web 控制台的 `export` 不支持传入自定义输出路径，默认导出到 MacFriends 结果目录。需要指定路径时使用 CLI：

```bash
macfriends export --format csv --output /path/to/file.csv
```

## 误删或重置

如果 runtime 状态混乱但还要保留扫描结果：

```bash
macfriends detach || true
macfriends cleanup || true
rm -rf "$HOME/Library/Application Support/MacFriends/runtime"
macfriends prepare --force
```

如果要完整重置本机数据，先备份 `results` 和 `logs`，再删除 `~/Library/Application Support/MacFriends` 与 `/tmp/macfriends-$USER`。
