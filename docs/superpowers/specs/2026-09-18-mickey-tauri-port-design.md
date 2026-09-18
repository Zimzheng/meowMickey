# Mickey Companion — Tauri 2 跨平台重写设计

**作者**: 项目 owner + AI 工程师协作
**日期**: 2026-09-18
**状态**: 待审

## 1. 目标与范围

把当前 Swift/AppKit 的 macOS 桌宠 `MickeyCompanion` 迁移到一个能在 macOS 与 Windows 上运行的项目，结构清晰、易于扩展、打包脚本简单。

### 1.1 明确的目标

1. 跨平台：同一份代码可产出 macOS `.app` 与 Windows `.exe`（+ `.msi`）。
2. 行为等价：与当前 Swift 版本在交互、定时、动画上完全一致（单/双击、拖动、定时喷嚏/踩奶、菜单栏、规则热重载）。
3. 开发体验优：构建、调试、迭代速度快；新功能添加成本低。
4. 本地打包：`./scripts/build.sh` 一行命令产出当前平台的 `.app` 或 `.exe`，落到 `dist/<os>/`。
5. 内部使用：不申请 Developer ID、不公证、不上商店。Ad-hoc 签名（macOS）/ 不签名（Windows）即可。

### 1.2 非目标（明确不做）

- ❌ Mac App Store / Windows Store 上架
- ❌ Linux 桌面（Rust + Tauri 技术上能跑，但本次不做）
- ❌ 移动端（iOS/Android）
- ❌ 多语言 UI（保持中文菜单项，与原 Swift 版本一致）
- ❌ 自动更新框架（`tauri-plugin-updater` 不引入；用户自行下载新版本）
- ❌ 沙盒化（内部使用，不需要；沙盒会限制 `rules.json` 外部覆盖能力）
- ❌ Web 前端框架（不引入 React/Vue/Svelte；纯 HTML/CSS/JS）

## 2. 技术栈与版本

| 组件 | 选型 | 版本约束 |
|---|---|---|
| 桌面框架 | Tauri | 2.x 最新稳定 |
| 后端语言 | Rust | 1.75+（Tauri 2 要求） |
| 前端 | 原生 HTML/CSS/JS | 无构建步骤 |
| 异步运行时 | tokio（Tauri 自带） | — |
| 序列化 | serde + serde_json | — |
| 文件监听 | notify（可选） | 监听 rules.json 变化 |
| 图标处理 | tauri 内置 | — |

**Tauri 插件**：本次不引入任何官方插件。所有需要（如打开外部文件）由 Rust 直接调用系统 API 或 `std::process::Command` 完成。减少依赖、减少权限配置。

## 3. 项目目录结构

```
~/Developer/MickeyCompanion/
├── README.md                       # 英文开发说明
├── 使用说明.md                      # 中文用户手册（沿用原版）
├── rules.json                      # 运行时配置（仓库根 + 打包时复制）
├── 启动米奇.command                 # macOS 启动器
├── 启动米奇.bat                     # Windows 启动器（新增）
├── .gitignore
├── src-tauri/                      # Rust 后端
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   │   ├── tray.png                # 22×22 托盘图标
│   │   ├── icon.png                # 1024×1024 应用图标源
│   │   ├── icon.icns               # macOS 应用图标
│   │   └── icon.ico                # Windows 应用图标
│   ├── capabilities/
│   │   └── default.json            # Tauri 2 权限声明
│   ├── resources/                  # 编译时打包的资源
│   │   ├── rules.json              # bundled 兜底版
│   │   ├── idle.png
│   │   ├── kneading.png
│   │   └── sneezing.png
│   └── src/
│       ├── main.rs                 # 入口 + 窗口 + 托盘 + 事件循环
│       ├── config.rs               # Rules 结构 + 读 rules.json
│       ├── timer.rs                # 30Hz tick + 喷嚏/踩奶调度
│       └── window_state.rs         # 窗口位置持久化
├── ui/                             # Web 前端（无构建步骤）
│   ├── index.html
│   ├── styles.css
│   ├── main.js                     # 入口：监听事件、初始化
│   ├── sprite.js                   # SpriteSheet 类、动画循环
│   └── mouse.js                    # 拖动 + 单/双击判定
├── dist/                           # 打包产物（gitignore）
│   ├── macos/MickeyCompanion.app + 启动米奇.command
│   └── windows/MickeyCompanion.exe + 启动米奇.bat
└── scripts/
    ├── dev.sh                      # cargo tauri dev
    ├── build.sh                    # 当前 OS 构建
    └── clean.sh                    # 清理 target/ dist/
```

## 4. 组件职责划分

### 4.1 Rust 后端

| 模块 | 职责 |
|---|---|
| `main.rs` | Tauri `Builder`：窗口创建、托盘创建、`setup` hook 启动定时器、注册 commands |
| `config.rs` | `Rules` 结构定义、`load_rules()` 从外部 rules.json 读（外部优先，bundled 兜底）、监听文件变化（notify crate）触发 `rules_changed` 事件 |
| `timer.rs` | 30Hz `tokio::time::interval`，维护 `next_sneeze_at` / `next_knead_at`，到点 emit `trigger` 事件 |
| `window_state.rs` | 启动时读 `window.json` 恢复位置，拖动结束 `save_position(x, y)` 写盘 |
| 托盘 | `TrayIconBuilder` + 菜单项回调：打喷嚏、踩奶、重新加载规则、打开规则文件、暂停/继续、退出 |
| Tauri Commands | `get_rules`、`start_drag`、`update_drag`、`end_drag`、`trigger_action`、`quit` |
| `--check` CLI | 启动时检查 assets/rules.json 完整性，失败则退出码非 0 |

### 4.2 JS 前端

| 模块 | 职责 |
|---|---|
| `main.js` | 启动时 `invoke('get_rules')` 拉初始配置；订阅 `trigger` / `rules_changed` / `paused` 事件 |
| `sprite.js` | `SpriteSheet` 类：预加载三张 PNG、维护 `currentAction` 与 `frameIndex`、`setTimeout` 推进帧（不同动作帧时长不同） |
| `mouse.js` | 在 sprite div 上挂 `mousedown/mousemove/mouseup`；实现 250ms 单/双击判定；调用 Tauri commands 移动窗口 |
| `styles.css` | `body { margin: 0; background: transparent; }`，sprite div 192×208，PNG 用 `background-image` + `background-position` |

### 4.3 通信契约

**Rust → JS（`emit_to("main", event, payload)`）**

| 事件 | payload | 触发时机 |
|---|---|---|
| `trigger` | `{action: "sneezing" \| "kneading"}` | 定时到点 |
| `rules_changed` | `Rules` 完整对象 | rules.json 文件变化 / 菜单"重新加载规则" |
| `paused` | `{paused: bool}` | 菜单"暂停/继续" |

**JS → Rust（`invoke`）**

| 命令 | 参数 | 返回 | 用途 |
|---|---|---|---|
| `get_rules` | — | `Rules` | 启动时拉初始规则 |
| `start_drag` | — | `{x, y}` | mousedown 时记录 Rust 端起始窗口位置 |
| `update_drag` | `{x, y}` | — | mousemove 时移动窗口 |
| `end_drag` | — | — | mouseup 时保存位置 |
| `trigger_action` | `{action: string}` | — | 单击/双击/托盘菜单触发动作；Rust 端推进 next_sneeze_at / next_knead_at |
| `show_context_menu` | — | — | 右键 sprite 时调用，Rust 在鼠标位置弹出托盘菜单 |
| `quit` | — | — | 退出 |

### 4.4 状态所有权

| 状态 | 所有者 | 理由 |
|---|---|---|
| `next_sneeze_at` / `next_knead_at` | Rust | 时间逻辑；webview 后台节流不影响 |
| 当前 action / frame_index | JS | 纯渲染状态；invoke 往返成本太高 |
| `paused` | Rust | 暂停是"全局"状态，菜单和定时器都要看 |
| 窗口位置 | Rust（持久化）/ JS（拖动中） | 拖动是高频事件，不宜每帧 invoke 存盘；只在 end_drag 时存一次 |
| rules.json 解析结果 | Rust（解析）/ JS（缓存展示） | Rust 是 source of truth；变更时通过 `rules_changed` 同步给 JS |

## 5. 行为规范

### 5.1 窗口属性

- **大小**：固定 192×208（与原 Swift 版本一致）
- **样式**：`transparent: true`、`decorations: false`、`resizable: false`、`maximizable: false`、`minimizable: false`、`skipTaskbar: true`
- **层级**：`alwaysOnTop: true`、`visibleOnAllWorkspaces: true`
- **位置**：首次启动放到主屏幕右下角（`screen.width - 192 - 35, 35`）；之后从 `window.json` 恢复
- **焦点**：`focus: false`（不抢窗口焦点）

### 5.2 Sprite 动画

**资源规格**（沿用原 Swift 版本）：
- 三张 PNG：`idle.png`、`kneading.png`、`sneezing.png`
- 每张 6 帧横排，单帧 192×208，整图 1152×208
- 帧时长（秒，每帧独立）：
  - `idle`: `[0.28, 0.11, 0.11, 0.14, 0.14, 0.32]`
  - `sneezing`: `[0.18, 0.15, 0.12, 0.14, 0.17, 0.25]`
  - `kneading`: `[0.18, 0.18, 0.18, 0.18, 0.18, 0.24]`

**渲染方式**（前端）：
- 一个 `<div id="sprite">` 元素，宽高 192×208
- `background-image` 切换为当前 action 对应的 PNG（asset URL 由 Rust 在启动时通过 `convertFileSrc` 或资源路径暴露）
- 切帧时改 `background-position-x = -frameIndex * 192`
- 用 `setTimeout(nextDuration)` 推进下一帧，不是 `setInterval`（每帧时长不同）
- 单帧 PNG 比整图 sprite 性能略好但资源多 6 倍；保持整图 sprite（与原版一致），简单

**校验**：Rust 启动时校验 PNG 尺寸必须为 1152×208，否则 alert 报错退出。

### 5.3 鼠标交互

**事件流**（JS → Rust）：

1. `mousedown` on `#sprite`：
   - 记录 `dragStartClient = {x, y}`、`moved = false`
   - `invoke('start_drag')` → Rust 返回 `{startX, startY}`（窗口当前位置）
2. `mousemove`：
   - 若已 mousedown 且超过 3px 阈值：标记 `moved = true`
   - `invoke('update_drag', {x: clientX, y: clientY})` → Rust 移动窗口
3. `mouseup`：
   - 若 `moved`：调用 `invoke('end_drag')` 保存位置；return
   - 否则进入单/双击判定：
     - 启动 250ms `setTimeout` 等待第二次 click
     - 250ms 内再次 click → `trigger_action(doubleClick 配置的动作)`，取消 timeout
     - 250ms 后未再次 click → `trigger_action(singleClick 配置的动作)`

**右键**：
- `contextmenu` 事件 → `preventDefault` + `invoke('show_context_menu')` → Rust 在鼠标位置弹出托盘菜单

**阈值**：拖动距离 > 3px 视为拖动（与原 Swift 版本一致）

### 5.4 定时器

**Rust 端**：
- 启动 `tokio::time::interval(Duration::from_millis(33))`（≈30Hz）
- 每个 tick 检查：
  - 若 `now >= next_sneeze_at` → emit `trigger {action: "sneezing"}`，重置 `next_sneeze_at`
  - 否则若 `now >= next_knead_at` → emit `trigger {action: "kneading"}`，重置 `next_knead_at`
- `next_*_at` 重置规则：`Date.now() + max(1, rule.minutes) * 60 * 1000`
- `paused = true` 时跳过触发检查
- 用户通过 `trigger_action` 主动触发的动作也推进 `next_*_at`（避免刚手动喷嚏又被定时喷嚏打断）

**前端**：
- 不维护定时器，只在收到 `trigger` 事件时切换 action 并重启动画循环

### 5.5 系统托盘菜单

**macOS**：状态栏右上角图标 + 菜单
**Windows**：系统托盘图标 + 右键菜单

**菜单项**（顺序固定）：
1. `打喷嚏` → `trigger_action("sneezing")`
2. `踩奶` → `trigger_action("kneading")`
3. — 分隔符 —
4. `重新加载规则` → 调用 `config::reload_rules()`，emit `rules_changed`
5. `打开规则文件` → `tauri-plugin-opener` 打开 `rules.json` 所在路径
6. `暂停/继续定时动作` → 切换 `paused`，emit `paused`
7. — 分隔符 —
8. `退出米奇` → `app.exit(0)`

**托盘图标**：
- 同一份 `tray.png`（22×22 PNG）作为源，build 时 `cargo tauri icon` 生成对应平台格式
- macOS: Tauri 2 的 `TrayIconBuilder` 设置 `icon_as_template(true)`，图标自动适应 light/dark menu bar
- Windows: ICO 格式由 `cargo tauri icon` 从源 PNG 生成

### 5.6 持久化

**窗口位置**：
- 路径：`{app_data_dir}/window.json`（Tauri 2 `app.path().app_data_dir()`，macOS: `~/Library/Application Support/local.codex.mickey-companion/window.json`，Windows: `%APPDATA%\local.codex.mickey-companion\window.json`），内容 `{x: number, y: number}`
- 写入时机：`end_drag` 时
- 读取时机：`setup` 时，若无文件则用默认右下角位置

**rules.json 读取优先级**：
1. **外部**（与二进制同级目录的 `rules.json`）：用户可编辑
2. **bundled**（`src-tauri/resources/rules.json`）：打包时内置的默认值
3. **硬编码 defaults**（在 Rust 代码里）：仅前两者都不可用时

读取策略用 `config::load_rules()`：
```rust
fn load_rules() -> Rules {
    // 1. 尝试二进制同目录的 rules.json
    if let Ok(data) = read(exe_dir().join("rules.json")) { return parse(data); }
    // 2. 尝试 bundled resource
    if let Ok(data) = read(resource_dir().join("rules.json")) { return parse(data); }
    // 3. 硬编码
    Rules::default()
}
```

**rules.json 监听**：用 `notify` crate 在 setup 时启动 watcher，发现文件变化自动 reload + emit `rules_changed`。Windows 上 notify 需配置 `ReadDirectoryChangesW`，macOS 用 FSEvents。

### 5.7 Rules 数据模型

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rules {
    pub sneeze_every_minutes: f64,
    pub knead_every_minutes: f64,
    pub single_click: String,    // "sneezing" | "kneading"
    pub double_click: String,    // "sneezing" | "kneading"
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            sneeze_every_minutes: 30.0,
            knead_every_minutes: 5.0,
            single_click: "kneading".to_string(),
            double_click: "sneezing".to_string(),
        }
    }
}
```

**JSON 字段名用 snake_case**，与原 `rules.json` 一致：`sneezeEveryMinutes` → `sneeze_every_minutes` 等。serde 用 `#[serde(rename_all = "camelCase")]` 保持 JSON 兼容。

## 6. 构建与打包

### 6.1 开发模式

```bash
./scripts/dev.sh
# 等价于：
cd src-tauri && cargo tauri dev
```

Tauri dev 模式自动监视 Rust 文件 + `ui/` 文件变化并热重载。

### 6.2 生产构建

```bash
./scripts/build.sh
```

脚本逻辑：
1. 检测 OS（`uname -s`，mac = Darwin，win = MINGW*/MSYS*/CYGWIN*）
2. macOS：`cd src-tauri && cargo tauri build`
3. Windows：同上（脚本在 Windows 上跑）
4. 复制产物到 `dist/<os>/`：
   - mac：`src-tauri/target/release/bundle/macos/MickeyCompanion.app` → `dist/macos/`
   - win：`src-tauri/target/release/bundle/msi/*.msi` + `bundle/nsis/*.exe` → `dist/windows/`
5. 复制启动器脚本：
   - mac：`启动米奇.command` → `dist/macos/`
   - win：`启动米奇.bat` → `dist/windows/`

### 6.3 签名（不做）

- macOS：保持 ad-hoc 签名（与原 Swift 版本一致）。用户首次运行时需右键打开绕过 Gatekeeper。
- Windows：默认不签名。如后续要发给团队外，再考虑自签名证书。

### 6.4 .gitignore

```
target/
dist/
node_modules/
.DS_Store
*.swp
*.log
```

## 7. Swift → Tauri 概念映射

| Swift/AppKit | Tauri 对应 |
|---|---|
| `SpriteView: NSView` | `<div id="sprite">` + JS 类 |
| `draw(_:)` 重绘 | CSS `background-image` + `background-position` |
| `mouseDown/Dragged/Up` | DOM `mousedown/mousemove/mouseup` |
| `window.setFrameOrigin` | `window.setPosition(LogicalPosition)` |
| `Timer.scheduledTimer(0.03)` | `tokio::time::interval(33ms)` |
| `NSStatusItem` + `NSMenu` | `TrayIconBuilder` + 菜单 |
| `Bundle.main.resourceURL` | `app.path().resource_dir()` |
| `UserDefaults` (windowOrigin) | 自管 `window.json` |
| `JSONDecoder().decode(Rules.self)` | `serde_json::from_str` |
| `--check` 入口 | 启动时 `std::env::args().any(|a| a == "--check")` 分支 |
| `cellWidth = 192`、`cellHeight = 208` | `ui/sprite.js` 中的常量（与 Rust 一致） |
| `[Action.idle, .sneezing, .kneading]` 枚举 | Rust `enum Action` + JS 字符串字面量 |

## 8. 测试策略

### 8.1 单元测试（Rust）

`cargo test` 覆盖：
- `config::load_rules` 三层优先级
- `Rules::default` 字段值
- 文件路径解析（外部 vs bundled）

### 8.2 资产校验

启动时 + `--check` CLI 模式下：
- 三张 PNG 存在
- PNG 尺寸 == 1152×208
- `rules.json`（bundled）可解析

### 8.3 手动验收

- [ ] 启动：图标出现在右下角
- [ ] 单击：踩奶动画播放
- [ ] 双击：喷嚏动画播放
- [ ] 拖动：图标跟随鼠标，松手后位置保留
- [ ] 等 5 分钟：自动踩奶
- [ ] 等 30 分钟：自动喷嚏
- [ ] 菜单"重新加载规则"：修改 rules.json 后菜单触发，新间隔生效
- [ ] 菜单"暂停/继续"：定时动作停止/恢复
- [ ] 重启应用：窗口位置恢复
- [ ] Windows 上同样功能验证

## 9. 实施步骤（高层）

1. 在 `~/Developer/MickeyCompanion/` 下手动初始化 `src-tauri/`（`Cargo.toml` + `tauri.conf.json` + `src/main.rs`）与 `ui/` 目录，**不用** `cargo create-tauri-app`（那个会引入 npm/Vite，与本设计不符）
2. 写 `.gitignore`、`README.md`、`使用说明.md`、仓库根 `rules.json`
3. 复制三个 PNG sprite 与 `rules.json` 到 `src-tauri/resources/`
4. 写 Rust 模块：`config.rs`（含单元测试）→ `window_state.rs` → `timer.rs` → `main.rs`
5. 写前端：`index.html` → `styles.css` → `sprite.js` → `mouse.js` → `main.js`
6. 写 `scripts/dev.sh` / `build.sh` / `clean.sh`
7. macOS 上 `cargo tauri dev` 跑通交互
8. macOS 上 `./scripts/build.sh` 出 `.app`，按 §8.3 手动验收清单逐项确认
9. 在 Windows 机器上重复 7-8（本次不远程验证；交付后在 Windows 上由用户执行）

## 10. 未来可考虑（非本次范围）

- 多语言 UI（中/英切换）
- 主题/配色支持（蓝/绿/粉等虹膜变体）
- 配置文件中支持更多动作（打哈欠、伸懒腰、睡觉）
- 自定义 sprite 上传
- 自动更新（`tauri-plugin-updater`）
- GitHub Actions 自动构建 + Release
- Linux 支持
