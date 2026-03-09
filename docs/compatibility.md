# 兼容矩阵

## 当前默认范围

- 平台：macOS
- 架构：Apple Silicon (`arm64`)
- 发行形态：CLI
- 注入方式：`DYLD_INSERT_LIBRARIES`
- 分发方式：压缩包 / 安装脚本

## 锁定版本

当前项目唯一受支持的 manifest：
- `fixtures/adapter.wechat-macos-arm64.json`

当前值：
- Bundle ID: `com.tencent.xinWeChat`
- Build Target: `4.1.8`
- Arch: `arm64`
- Resolver Mode: `signature_scan`
- Adapter Name: `wechat_4_1_8_arm64`

只有当目标微信满足以上条件，agent 才会将该目标视为可支持版本。

## 上线前附加条件

即使静态兼容条件全部满足，仍必须满足运行态 Ready 条件后才可以上线：
- `runtime_ready = true`
- `fixture_enabled = false`
- `primitive_resolution` 中 `profile / contacts / scan` 全部为 `resolved`
