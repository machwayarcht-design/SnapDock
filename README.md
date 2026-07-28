# SnapDock

> Fast Window Layout Manager for Windows

SnapDock 是一款轻量级的 Windows 窗口布局管理工具，用全局快捷键即可在多个窗口排布方案之间瞬时切换，并支持保存你常用的空间布局。

![SnapDock 面板](assets/screenshots/desktop.png)

---

## ✨ 核心特性

- **开箱即用** —— 解压即运行，无需安装、无运行时依赖（仅使用系统自带的 WebView2）。
- **轻量化后台运行** —— 关闭窗口不退出，自动最小化到系统托盘常驻；内存占用极低，常驻无感。
- **快捷键切换响应迅速** —— 全局热键即时响应，窗口排布瞬间完成，不打断当前工作。
- **可保存常用空间布局** —— 4 个槽位，一键保存 / 恢复你的工作区。
- **4 种内置布局** —— 单窗口 / 左右分屏 / 三窗 / 四宫格，开箱即用。
- **中英双语** —— 简体中文 / English，托盘菜单一键切换。
- **开机自启** —— 可选写入 `HKCU\...\Run\SnapDock`，无需管理员权限。
- **基于 Tauri + Rust 构建** —— 体积小、性能好，没有 Electron 的臃肿。

---

## 🚀 快速开始（开箱即用）

1. 在 [Releases](../../releases) 页面下载最新版 `SnapDock_v1.7.1_portable.zip`。
2. 右键解压到任意文件夹。
3. 双击 `SnapDock.exe`。

完成。无需安装、不写注册表（除可选的开机自启项），也不捆绑任何运行时。它仅依赖 Windows 10/11 绝大多数机器上已预装的 WebView2 组件。

---

## 🔆 后台运行（轻量化常驻）

SnapDock 设计为**常驻后台**的窗口工具：

- 点击窗口关闭按钮 —— 程序**不会退出**，而是最小化到系统托盘。
- 系统托盘图标右键菜单：
  - **SnapDock**（标题）
  - **布局** ▸ 单窗口 / 左右分屏 / 三窗 / 四宫格（点击即应用）
  - **键盘快捷键** —— 打开快捷键指南
  - **语言** ▸ 简体中文 / English
  - **开机自启** —— 勾选后下次登录自动启动
  - **关于**
  - **退出**

![托盘菜单](assets/screenshots/tray.png)

> 常驻托盘意味着它始终待命，随时用快捷键呼出，几乎不占用系统资源。

---

## ⌨️ 快捷键（响应迅速）

全局热键即时生效，无需聚焦 SnapDock 窗口：

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl` + `~` | 显示 / 隐藏 SnapDock 面板 |
| `1` / `2` / `3` / `4` | 切换布局：单窗口 / 左右分屏 / 三窗 / 四宫格 |
| `Ctrl` + `Tab` | 轮换窗口顺序 |
| `Ctrl` + `←` / `→` / `↑` / `↓` | 将窗口向左 / 右 / 上 / 下互换位置 |
| `Ctrl` + `Shift` + `1`–`4` | 保存当前工作区到对应槽位 |
| `Ctrl` + `1`–`4` | 恢复对应槽位保存的工作区 |
| `Esc` | 关闭面板 / 弹窗 |

---

## 💾 保存常用空间布局

你可以把**当前所有窗口的位置与大小**存成布局，之后一键还原：

1. 手动把窗口拖到你习惯的位置（例如：左边浏览器、右边编辑器、下方终端）。
2. 按下 `Ctrl` + `Shift` + `1` 保存为「布局 1」。
3. 之后无论窗口怎么乱，按 `Ctrl` + `1` 即可**瞬间还原**这套排列。
4. 共 4 个独立槽位（`1`–`4`），可分别保存不同场景：工作、摸鱼、演示、阅读……

![布局排布示意](assets/screenshots/layout1.png)
![布局排布示意](assets/screenshots/layout2.png)

---

## 📸 截图预览

| | |
| --- | --- |
| ![主面板](assets/screenshots/desktop.png) | ![布局一](assets/screenshots/layout1.png) |
| ![布局二](assets/screenshots/layout2.png) | ![布局三](assets/screenshots/layout3.png) |
| ![布局四](assets/screenshots/layout4.png) | ![快捷键指南](assets/screenshots/shortcuts.png) |
| ![托盘菜单](assets/screenshots/tray.png) | ![关于页](assets/screenshots/about.png) |

---

## 🛠 技术栈

- **[Tauri 2](https://tauri.app/)** —— 应用框架
- **Rust** —— 核心引擎与系统交互
- **原生 WebView（HTML / CSS / JavaScript）** —— 无前端框架，纯原生 JS

---

## 🗺 后续规划

详见 [ROADMAP.md](ROADMAP.md)，包括多显示器支持、自定义快捷键、布局导入导出等。

---

## 📄 许可证

SnapDock 基于 [MIT License](LICENSE) 开源发布。
