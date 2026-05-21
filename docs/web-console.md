# 本地 Web 控制台

## 启动

```bash
macfriends serve --open
# 或
macfriends 控制台 --open
```

默认监听：

```text
127.0.0.1:8765
```

也可以指定端口：

```bash
macfriends serve --addr 127.0.0.1:8787 --open
```

控制台只面向本机使用。不要把监听地址改成公网地址；当前接口会执行本地 `prepare`、`launch`、`scan`、`export` 等操作。

## 页面能力

页面提供：
- 生命周期、Ready、fixture、target supported 状态
- 本机微信版本、受控副本版本、当前 adapter 锁定版本与兼容提示
- release blockers 与 next actions
- 中文按钮：准备受控微信副本、强制重新准备、启动并登录、全量扫描联系人、导出结果、断开 agent、清理运行状态
- CLI log 与 agent log 查看
- 原始 API 输出窗口

## API 契约

所有命令接口都会返回统一包装：

```json
{
  "ok": true,
  "command": "status",
  "exit_code": 0,
  "data": {},
  "error": null,
  "stderr": ""
}
```

命令失败时：

```json
{
  "ok": false,
  "command": "attach",
  "exit_code": 21,
  "data": null,
  "error": {
    "ok": false,
    "command": "attach",
    "error_code": "agent_unreachable",
    "message": "..."
  },
  "stderr": ""
}
```

## 接口列表

| Method | Path | Body | 说明 |
| --- | --- | --- | --- |
| `GET` | `/api/health` | - | Web 服务活性 |
| `GET` | `/api/status` | - | 产品级状态总览 |
| `GET` | `/api/compatibility` | - | 版本兼容状态，返回同一份 status 数据 |
| `GET` | `/api/doctor` | - | 环境与 Ready 门禁检查 |
| `GET` | `/api/attach` | - | 当前 agent 运行态详情 |
| `GET` | `/api/profile` | - | 当前登录资料 |
| `GET` | `/api/contacts` | - | 联系人列表 |
| `GET` | `/api/logs?kind=cli&lines=200` | - | CLI 日志 |
| `GET` | `/api/logs?kind=agent&lines=200` | - | agent 日志 |
| `POST` | `/api/prepare` | `{ "force": true, "source_app": "/Applications/WeChat.app" }` | 准备受控副本 |
| `POST` | `/api/launch` | `{ "login": true }` | 启动受控副本 |
| `POST` | `/api/scan` | `{ "all": true }` | 扫描并保存结果 |
| `POST` | `/api/export` | `{ "format": "csv" }` | 导出最近生产扫描 |
| `POST` | `/api/detach` | `{}` | 请求 agent 停止 |
| `POST` | `/api/cleanup` | `{}` | 清理本地运行态文件 |

## 生产边界

Web 控制台不能绕过 native agent 的真实能力边界。当前生产链路仍必须满足：
- `runtime_ready=true`
- `fixture_enabled=false`
- `release_blockers=[]`
- `primitive_resolution.profile/contacts/scan = resolved`

如果 agent 返回 `profile_primitive_unresolved`、`contacts_primitive_unresolved` 或 `scan_primitive_unresolved`，页面会显示失败。这是正确行为，不能改成伪成功。
