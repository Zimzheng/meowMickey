# 米奇开发指南

## 环境

- 使用当前稳定版 Rust。虽然项目清单写有较早的最低版本，当前锁定的依赖有更高要求，请勿按旧 README 的 Rust 1.75 配置环境。
- 安装 Tauri CLI 2.11.4：`cargo install tauri-cli --version 2.11.4 --locked`。
- macOS：安装 Xcode Command Line Tools（`xcode-select --install`）。
- Windows：安装 Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载、Windows SDK，以及 WebView2 运行环境。
- 前端是静态 HTML / CSS / JavaScript，无需 npm 构建。仓库的 Windows CI 使用 Node.js 22 安装 npm 版 Tauri CLI，两种 CLI 为替代入口。

[Tauri 环境要求](https://v2.tauri.app/start/prerequisites/) · [Windows 安装包说明](https://v2.tauri.app/distribute/windows-installer/)

## 获取代码与运行

在已配置编译工具的终端执行：

```bash
git clone https://github.com/Zimzheng/meowMickey.git
cd meowMickey
cargo install tauri-cli --version 2.11.4 --locked
cargo tauri dev
```

在 macOS 上也可以从仓库根目录执行 `bash scripts/dev.sh`。它进入 `src-tauri/` 后启动开发模式。

应用实际加载 `ui/main-v121.js` 和 `ui/sprite-v121.js`；修改前端时先确认入口，避免只改到未加载的副本。

## 不持有正式更新私钥的本地打包

以下命令关闭更新产物签名，仅用于开发打包。不会生成可供正式更新使用的签名清单。

### macOS

在仓库根目录执行：

```bash
cargo tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
open src-tauri/target/release/bundle/macos/米奇.app
```

### Windows（PowerShell）

在仓库根目录执行：

```powershell
'{"bundle":{"createUpdaterArtifacts":false,"windows":{"nsis":{"languages":["SimpChinese"],"displayLanguageSelector":false,"installMode":"currentUser"}}}}' | Set-Content -Encoding utf8 windows-build.json
cargo tauri build --bundles nsis --config windows-build.json
Get-ChildItem src-tauri/target/release/bundle/nsis/*.exe
Remove-Item windows-build.json
```

运行生成的 EXE 安装包即可。仓库的 [Windows 工作流](../.github/workflows/windows-release.yml) 使用同样的中文 NSIS 配置，在 Windows 构建机测试、编译并上传对应版本附件。

## 正式版本与签名更新

正式更新需要维护者持有既有签名私钥，不能为现有发布随意更换公钥。

macOS 维护者在仓库根目录执行 `bash scripts/build.sh`：脚本要求本机 `.local/updater.key` 或 `TAURI_SIGNING_PRIVATE_KEY`，并将已签名产物整理到 `dist/macos/`。

Windows CI 不保存签名私钥。构建完成后由维护者在本机签名，合并 macOS / Windows 更新清单，再发布。完整流程见 [版本更新说明](UPDATES.md)。

## 验证

在仓库根目录执行核心测试：

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
```

macOS 更新集成测试：

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml --test updater_flow
```

v1.3.1 已通过 36 项核心测试和 macOS 更新集成测试；Windows 核心测试及正式构建通过。Windows 原生交互体验仍需在 Windows 桌面环境中人工验收。

改动后至少检查：启动及拖动、右键菜单、记录及撤销喝水、完整动画、日记明细换行、图片与文字复制、关闭弹窗后的残留、重启后记录，以及新版检查。

## 产品与架构资料

- [产品架构](PRODUCT_ARCHITECTURE.md)
- [v1.3.1 发布说明](RELEASE_1.3.1.md)
- [早期迁移设计](superpowers/specs/2026-09-18-mickey-tauri-port-design.md)（历史资料，当前行为以代码和发布版为准）

七天总结的前端测试（安装 Node.js 后执行）：

```bash
node --test tests/hydration-week.test.mjs
```
