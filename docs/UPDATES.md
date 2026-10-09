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
CI 构建时，将私钥和可选密码通过 `TAURI_SIGNING_PRIVATE_KEY`、
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 环境变量传入。

1. 同步更新 Cargo.toml、tauri.conf.json 和版本测试中的版本号。
2. 运行 `bash scripts/build.sh`，生成已签名更新包、`.sig` 和 `latest.json`。
3. 为对应版本创建正式 GitHub Release，例如 `v1.2.5`。
4. 上传 `dist/macos/` 中 `Mickey-v1.2.5-darwin-aarch64.app.tar.gz`、对应 `.sig`、`latest.json`。
5. Windows 或 Intel Mac 需在对应平台构建，使用同一私钥；
   `python3 scripts/prepare-update.py --out dist/updates --target windows-x86_64 --merge <另一平台的latest.json>`
   合并清单。上传所有平台的更新包和**合并后的唯一 latest.json**。
6. 保持该正式 Release 为 GitHub 的 Latest，旧客户端才会发现更新。

没有 `latest.json` 的旧版本 Release 无法用于应用内更新。
应用会显示“目前暂无可安装的更新包”，不会刷新界面冒充安装更新。
实现支持 macOS 和 Windows；本机可验证 macOS，Windows 安装需在 Windows 上验收。

参考：[Tauri Updater](https://v2.tauri.app/plugin/updater/)。
