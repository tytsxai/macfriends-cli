# 安装与打包

## 本地安装

在项目根目录执行：

```bash
make install-local
```

这会完成：
- `cargo build --release`
- `make -C native/agent artifacts`
- 将 `macfriends` 安装到 `~/.local/bin/macfriends`
- 将 agent 与 manifest 安装到 `~/Library/Application Support/MacFriends/bundle/`

如果你是在仓库里开发而不是使用发布包，`prepare --force` 会优先同步当前仓库构建出的 native agent/host，而不是继续复用旧的 `bundle/` 资产。

从源码仓库执行 `make install-local` 时，安装脚本会读取 `target/release/macfriends` 与 `native/agent/build/`。从发布 tar 包解压后执行 `./install.sh` 时，安装脚本会读取包内 `bin/` 与 `bundle/`，不要求用户保留源码树。

## 打包发布

在项目根目录执行：

```bash
make package
```

输出物：

```bash
dist/macfriends-0.1.0-macos-arm64.tar.gz
dist/macfriends-0.1.0-macos-arm64.tar.gz.sha256
```

打包内容包括：
- CLI 二进制
- native agent dylib
- agent host
- 锁定版本 manifest
- README / CHANGELOG / LICENSE / llms.txt / docs
- `install.sh`

## 安装后的使用

```bash
macfriends doctor
macfriends status
macfriends 状态
macfriends prepare
macfriends launch --login
macfriends attach
macfriends serve --open
```

中文用户完整路径见 `docs/中文用户指南.md`。

## Ready 校验

安装完成后，不要只看 `prepare` 成功；必须继续执行：

```bash
macfriends doctor --json
macfriends status --json
macfriends launch --login --json
macfriends attach --json
```

只有 `runtime_ready=true`、`fixture_enabled=false`、`release_blockers=[]` 才可视为可上线。安装脚本会自动保留上一版本到 `macfriends.previous` 和 `bundle.previous`。

如果命令以 `--json` 运行，即使失败也会输出结构化错误对象，包含：
- `command`
- `error_code`
- `message`
- `causes`

日常排障建议优先运行 `macfriends status --json`，它会汇总生命周期、最近生产扫描、日志路径、socket 路径和下一步动作。

安装脚本会先校验构建产物是否齐全，再以临时文件/目录完成原子替换，避免安装过程中留下半更新状态。
