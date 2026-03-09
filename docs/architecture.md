# 架构说明

## 总体架构

MacFriends 由两部分组成：
- `crates/cli`：Rust CLI，负责环境检查、受控副本准备、启动编排、target status、RPC 客户端、结果导出
- `native/agent`：Objective-C++ 动态库，通过 `DYLD_INSERT_LIBRARIES` 预加载进受控目标进程，提供本地 Unix Socket RPC

## 版本化适配

当前项目只注册一个 adapter：`wechat_4_1_8_arm64`。

agent 内部职责分层：
- transport：socket 建连、请求读取、响应写回
- process probe：读取 bundle id / version / arch
- adapter registry：按 manifest 决定是否加载 `wechat_4_1_8_arm64`
- adapter implementation：处理 `profile / contacts / scan`

## 受控运行链路

1. `prepare` 复制 `/Applications/WeChat.app` 到 `~/Library/Application Support/MacFriends/runtime/WeChat.app`
2. `prepare` 构建 agent 与宿主，并执行 ad-hoc 签名
3. `prepare` 写入 `adapter.json` 与 `target-status.json`
4. `launch` 使用 `DYLD_INSERT_LIBRARIES` 启动受控副本
5. agent 在构造函数里启动 Unix Domain Socket 服务
6. CLI 通过 JSON RPC 调用 `status/profile/contacts/scan/stop`

## 当前适配边界

当前已完成：
- 单版本 manifest 门禁
- adapter 注册与分发
- 固定错误码
- fixture 测试链路
- 运行态 Ready 门禁

当前尚未落地的部分，是特定微信版本内部私有原语的解析与调用逻辑；因此当目标满足 4.1.8 条件但原语未解析时，agent 会返回 `profile_primitive_unresolved` / `contacts_primitive_unresolved` / `scan_primitive_unresolved`。

## 运行态 Ready 门禁

除了静态的 bundle/version/arch 门禁外，CLI 还会基于 agent `status` 判定运行态是否 Ready。

运行态必须同时满足：
- 非 fixture 模式
- agent socket 正常可探活
- 关键原语 `profile/contacts/scan` 全部为 `resolved`

只通过静态门禁、不通过运行态门禁时，项目仍视为 Not Ready。
