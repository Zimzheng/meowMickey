# Mickey · A little cat on your desktop

[简体中文](README.md) · [English](README.en.md)

A desktop cat that asks for a little attention, reminds you to drink water, and turns your local water and interaction records into a shareable companion diary.

**[Download for macOS](https://github.com/Zimzheng/meowMickey/releases/download/v1.2.5/Mickey-macOS-arm64-v1.2.5.zip)** · **[Download for Windows](https://github.com/Zimzheng/meowMickey/releases/download/v1.2.5/Mickey-v1.2.5-windows-x86_64-setup.exe)** · [All releases](https://github.com/Zimzheng/meowMickey/releases) · [Report an issue](https://github.com/Zimzheng/meowMickey/issues)

| A little company | A drink together |
| :---: | :---: |
| [![Mickey's idle animation](docs/assets/mickey-idle.gif)](https://github.com/Zimzheng/meowMickey/releases/tag/v1.2.5) | [![Mickey's complete six-frame drinking animation](docs/assets/mickey-drinking.gif)](https://github.com/Zimzheng/meowMickey/releases/tag/v1.2.5) |

These GIFs use the application's actual sprite assets and frame timings. The light blue background is for the preview; Mickey's installed window is transparent. Click either animation to open the release page.

## What Mickey can do

- **Keep you company**: a transparent floating cat you can drag around the desktop. Pet, double-click, or click rapidly for different reactions, alongside sneezing, kneading, head tilts, stretches, and yawns.
- **Make drinking water a shared moment**: Mickey gets thirsty and reminds you to take a drink. Pick 200 / 300 / 500 ml or enter a custom amount, watch Mickey drink, and see your daily total update. Mistakes can be undone.
- **Check in occasionally**: after a long period without interaction, Mickey may ask for attention. Water reminders and scheduled actions support nighttime quiet rules, and reminders can be postponed.
- **Share a companion diary**: preview an image with Mickey, today's water total, interaction count, and a breakdown of activities such as petting, double-clicking, and logging water. Copy the image or text to paste into another app.
- **Fit your routine**: change the size, pause scheduled actions, edit reminder rules, and install new versions from the context menu. Position and size persist across restarts.

## Download and install

Current release: **v1.2.5**. Install a release package directly; Rust and Node.js are not required. The application's interface and Windows installer are currently in Chinese.

| Platform | Download | Installation |
| --- | --- | --- |
| macOS · Apple Silicon (M series) | [ZIP package](https://github.com/Zimzheng/meowMickey/releases/download/v1.2.5/Mickey-macOS-arm64-v1.2.5.zip) | Extract, move `米奇.app` to Applications, and open it |
| Windows 10 / 11 · x64 | [EXE installer](https://github.com/Zimzheng/meowMickey/releases/download/v1.2.5/Mickey-v1.2.5-windows-x86_64-setup.exe) | Follow the Chinese installer; it installs for the current user and downloads WebView2 if needed |

Native Intel Mac and Windows ARM installers are not currently available. The macOS package is ad hoc signed and has not been notarized by Apple, so the system may display an unverified-developer notice. Please report installation issues with your OS version in [Issues](https://github.com/Zimzheng/meowMickey/issues).

The `.sig`, `.app.tar.gz`, and `latest.json` assets serve the in-app updater. For a regular installation, use the ZIP or EXE above.

### Already have Mickey?

Right-click Mickey → **更新米奇版本** (Update Mickey). When a newer version is found, confirm **更新并重启** (Update and restart). Mickey downloads the package, verifies its update signature, installs it, and restarts. Personal records are not included in release packages, and the update process does not intentionally clear them.

If your older version lacks this menu item, quit Mickey and install manually. On macOS, replace the existing `米奇.app` with the new one.

## Getting acquainted

| Goal | Action |
| --- | --- |
| Move Mickey | Click and drag the cat |
| Pet / sneeze | Single-click kneads and double-click sneezes by default; both are configurable |
| Log water | Right-click → **喝水与记录** → select an amount or enter 10–3000 ml |
| Undo a mistake | Right-click → **撤销最近一次喝水**, or use the undo button after logging |
| Share a diary | In the water panel, choose **分享今天的米奇记录**, then **复制图片** (Copy image) or **复制文字** (Copy text) |
| Resize | Right-click → **大小**, choose 50%–200%; 100% is the default size |
| Pause scheduled actions | Right-click → **暂停／继续定时动作** |
| Reveal a hidden cat | Click the paw icon in the menu bar / system tray |
| Quit | Right-click → **退出米奇** |

Water and diary panels open beside Mickey. You choose where to paste shared content; Mickey does not send it to other people automatically.

## Records and privacy

- **No account required**. Water records are stored in the local app data directory, and interactions in local WebView storage. The current implementation has no cloud sync.
- Water entries store the amount, date, time, and source, retaining up to the latest **10,000 entries**. Interaction history retains up to the latest **500 entries**. Diaries summarize the day's retained records.
- **Updates use the network**: version checks and package downloads contact GitHub. A first Windows installation may also download WebView2.
- Share previews do not automatically save images to Downloads. Keep your own copies of diaries and records you want to preserve; clearing application data or system storage may remove history.

## Customize reminders

Right-click → **打开规则文件** (Open rules file), then edit the existing `rules.json`. Changes reload automatically; **重新加载规则** provides a manual reload.

Water reminders default to every 60 minutes, with nighttime hours from 23:00 to 07:00. Merge the fields below into your existing configuration:

```json
{
  "hydrationEnabled": true,
  "hydrationEveryMinutes": 60,
  "sneezeEveryMinutes": 30,
  "kneadEveryMinutes": 5,
  "nightQuietEnabled": true,
  "nightStartHour": 23,
  "nightEndHour": 7,
  "singleClick": "kneading",
  "doubleClick": "sneezing"
}
```

See [default rules](rules.json) for the full configuration. A rules file beside the application takes priority; otherwise an editable copy is created in the user's app data directory. Water reminders are a companion feature, and users choose the amounts they record.

## Development and contributions

Built with **Tauri 2, Rust, and plain HTML / CSS / JavaScript**. The frontend needs no npm build step.

With stable Rust, Tauri CLI 2.11.4, and your platform's build tools installed:

```bash
git clone https://github.com/Zimzheng/meowMickey.git
cd meowMickey
cargo install tauri-cli --version 2.11.4 --locked
cargo tauri dev
```

See the [development guide (Chinese)](docs/DEVELOPMENT.md) for prerequisites, macOS / Windows packaging commands, and checks. Signing and publishing the combined update manifest are covered in [update documentation (Chinese)](docs/UPDATES.md).

| Directory | Purpose |
| --- | --- |
| `src-tauri/src/` | Windows, tray, scheduling, local water storage, native clipboard, and updates |
| `ui/` | Animations, interactions, water panel, and diary rendering |
| `ui/sprites/` | Mickey's animation assets |
| `rules.json` | Default interaction and reminder settings |
| `.github/workflows/` | Windows tests, builds, and installer publishing |

Use [Issues](https://github.com/Zimzheng/meowMickey/issues) for bugs and suggestions. Include your OS, Mickey version, reproduction steps, and screenshots checked for private information.

The repository currently has no LICENSE file. Contact the author to confirm permission before redistributing the code or Mickey's assets.
