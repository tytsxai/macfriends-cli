# 排障

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

## `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved`

这表示当前目标已经通过 4.1.8 版本门禁，但具体原语解析链尚未成功建立。CLI 不会回退到伪检测，也不会返回空跑结果。

## Ready 相关故障

- `runtime_ready=false`：表示 agent 已连通但真实运行态仍未满足上线要求。
- `fixture_enabled=true`：表示当前链路只适合测试，不能导出正式结果。
- `primitive_resolution=unresolved`：表示真实 `profile/contacts/scan` 原语尚未闭环，必须先修复再上线。
- `cleanup` 失败且提示 PID 仍在运行：先退出受控 WeChat，再执行清理。
