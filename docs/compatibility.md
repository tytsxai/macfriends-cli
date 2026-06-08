# 兼容矩阵

## 当前默认范围

- 平台：macOS
- 架构：Apple Silicon (`arm64`)
- 发行形态：CLI
- 注入方式：`DYLD_INSERT_LIBRARIES`
- 分发方式：压缩包 / 安装脚本
- Web 控制台：仅本机 `127.0.0.1`

不支持：
- Intel Mac
- Windows / Linux
- 任意最新版微信自动适配
- 云端或服务器代跑扫描
- 容器内真实扫描链路

## 锁定版本

当前项目唯一受支持的 manifest：
- `fixtures/adapter.wechat-macos-arm64.json`

当前值：
- Bundle ID: `com.tencent.xinWeChat`
- Build Target: `4.1.8`
- Arch: `arm64`
- Resolver Mode: `signature_scan`
- Adapter Name: `wechat_4_1_8_arm64`
- Release Channel: `beta`
- Primitive Resolution: `profile/contacts/scan = unresolved`

只有当目标微信满足以上条件，agent 才会将该目标视为可支持版本。

运行态附着点：
- Runtime Bundle ID: `com.tencent.flue.WeChatAppEx`
- Runtime Version: `2.4.1.19024`

主 WeChat 4.1.8 是受控副本入口；真实 agent Ready 发生在 WeChatAppEx 运行时组件中。

## 上线前附加条件

即使静态兼容条件全部满足，仍必须满足运行态 Ready 条件后才可以上线：
- `runtime_ready = true`
- `fixture_enabled = false`
- `primitive_resolution` 中 `profile / contacts / scan` 全部为 `resolved`
- `release_blockers = []`

当前 manifest 仍是 beta 且 primitives unresolved，所以只能构建 beta/testable artifact。新增或升级 adapter 时，必须先更新 OpenSpec、manifest、native adapter、release guard、fixture smoke 和 docs。
