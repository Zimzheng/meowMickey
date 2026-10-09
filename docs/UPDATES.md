# 米奇版本更新

右键米奇 → **更新米奇版本**。菜单会显示检查、下载和安装状态。
发现新版后，用户选择“更新并重启”，应用验证更新包签名后覆盖安装并重启。
取消操作、网络失败或签名错误均不删除喝水和互动数据。
更新安装包不会改变位于应用数据目录中的记录。

## 发布更新包

使用 Tauri 官方 updater 插件。远端清单为：
`https://github.com/Zimzheng/meowMickey/releases/latest/download/latest.json`

本地签名私钥保存在 `.local/updater.key`，目录已被 Git 忽略。
此私钥必须安全备份，不要提交到仓库，不要每次构建重新生成。
公钥已嵌入 `src-tauri/tauri.conf.json`。
Windows 构建由 `.github/workflows/windows-release.yml` 在 Windows 构建机完成，
自动测试并上传中文 NSIS 安装包。当前流水线不持有私钥；下载构建结果后，
在本机用同一私钥签名并合并更新清单。私钥始终保留在本机。
若以后使用受控 CI 签名，需通过秘密环境变量传入，不得写入工作流或日志。

1. 同步更新 Cargo.toml、tauri.conf.json 和版本测试中的版本号。
2. 运行 `bash scripts/build.sh`，生成已签名更新包、`.sig` 和 `latest.json`。
3. 为对应版本创建正式 GitHub Release，例如 `v1.3.0`。
4. 上传 `dist/macos/` 中 `Mickey-v1.3.0-darwin-aarch64.app.tar.gz`、对应 `.sig`、`latest.json`。
5. Windows 或 Intel Mac 需在对应平台构建，使用同一私钥；
   `python3 scripts/prepare-update.py --out dist/updates --target windows-x86_64 --merge <另一平台的latest.json>`
   合并清单。上传所有平台的更新包和**合并后的唯一 latest.json**。
6. 保持该正式 Release 为 GitHub 的 Latest，旧客户端才会发现更新。

没有 `latest.json` 的旧版本 Release 无法用于应用内更新。
应用会显示“目前暂无可安装的更新包”，不会刷新界面冒充安装更新。
实现支持 macOS 和 Windows；本机可验证 macOS，Windows 安装需在 Windows 上验收。

参考：[Tauri Updater](https://v2.tauri.app/plugin/updater/)。

## Windows 下载与本机签名

Windows 10/11 x64 安装包为 `Mickey-v1.3.0-windows-x86_64-setup.exe`，使用中文安装向导、当前用户安装和 WebView2 自动引导。

1. 等待 Windows release 工作流成功，下载对应 Release 的安装包。
2. 将其放入 `src-tauri/target/release/bundle/nsis/`。
3. 执行 `cargo tauri signer sign -f .local/updater.key -p "" <安装包路径>`。
4. 执行 `python3 scripts/prepare-update.py --out dist/windows --target windows-x86_64 --merge dist/macos/latest.json`。
5. 上传 `.sig`，将 Release 的 `latest.json` 替换为合并后的清单，保留所有 macOS 附件。

自动上传安装包不等于完成更新发布；签名与清单必须一起发布。

新版本的 Windows CI 会先创建 Release 草稿。下载 Actions 构建产物到本机签名，上传安装包签名、macOS 附件和合并清单后，才发布为 Latest。已发布安装包禁止直接覆盖，否则既有签名会失效。
