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

## 打包发布

在项目根目录执行：

```bash
make package
```

输出物：

```bash
dist/macfriends-0.1.0-macos-arm64.tar.gz
```

打包内容包括：
- CLI 二进制
- native agent dylib
- agent host
- 锁定版本 manifest
- README / CHANGELOG / LICENSE / docs
- `install.sh`

## 安装后的使用

```bash
macfriends doctor
macfriends prepare
macfriends launch --login
macfriends attach
```

## Ready 校验

安装完成后，不要只看 `prepare` 成功；必须继续执行：

```bash
macfriends doctor --json
macfriends launch --login --json
macfriends attach --json
```

只有 `runtime_ready=true`、`fixture_enabled=false`、`release_blockers=[]` 才可视为可上线。安装脚本会自动保留上一版本到 `macfriends.previous` 和 `bundle.previous`。
