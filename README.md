# Glance

一款轻量级桌面截图与文本翻译工具。快捷键截屏，框选即翻译；也可以在双栏窗口里直接进行文本互译。

使用 Tauri 2 + 原生窗口渲染，启动快、体积小。

## 演示

![demo](assets/demo.jpg)

## 使用方法

| 操作 | 说明 |
|------|------|
| `Ctrl+Shift+X` | 全局快捷键，启动截图（可在设置中修改） |
| **鼠标左键拖拽** | 框选要翻译的区域 |
| `Esc` | 取消截图 |
| **鼠标右键** | 取消截图 |
| **点击托盘图标** | 显示主窗口 |
| **右键托盘图标** | 显示菜单（显示窗口 / 退出） |

> 关闭主窗口不会退出程序，Glance 会常驻系统托盘（右下角）。

## 功能

- 全局快捷键一键截图
- 框选区域后自动调用有道 OCR 翻译
- 翻译结果直接覆盖在原图位置
- 双栏文本互译，输入任意一侧即可向另一侧翻译
- 支持必应、Google、微软、腾讯、Yandex、词霸和 OpenAI 兼容的大模型接口
- 截图目标语言和文本翻译语言可独立设置
- 支持多语言互译（中/英/日/韩/法/德/俄/西）
- 可选的文本翻译历史与本地结果缓存
- 支持系统代理和自定义代理
- 窗口置顶、语言交换、输入法组合输入保护和应用内更新检查
- 系统托盘常驻，关闭窗口不退出
- 开机自启并静默进入系统托盘（可选）

## 隐私与密钥

- 文本翻译历史默认关闭；开启后会把原文和译文写入本机应用数据目录。
- 跨服务商备用引擎默认关闭；开启后，主引擎失败时文本可能会发送到另一个翻译服务商。
- 大模型 API Key 在 Windows 使用 DPAPI 保护，在 macOS 保存到登录钥匙串。Linux 当前仍保存在本地设置文件中。
- 翻译缓存保存在本机，可在设置中清空或关闭（将缓存数量设为 `0`）。

## 技术栈

- **后端**：Rust + Tauri 2
- **前端**：原生 HTML / CSS / JavaScript（无框架）
- **截图**：Windows BitBlt（通过 `screenshots` crate）
- **选区窗口**：winit + softbuffer 原生渲染（无 WebView 开销）
- **翻译 API**：有道智云 OCR 图片翻译

## 项目结构

```
src-tauri/
├── src/
│   ├── main.rs            # 应用入口，托盘 & 窗口管理
│   ├── commands.rs        # Tauri 命令（截图、翻译、设置）
│   ├── capture.rs         # 屏幕截图（BitBlt）
│   ├── capture_window.rs  # 原生全屏选区窗口（winit + softbuffer）
│   ├── api.rs             # 有道翻译 API 客户端
│   ├── translate_engine.rs # 文本翻译引擎调度
│   ├── translate_cache.rs # 文本翻译缓存
│   ├── secure.rs          # API Key 本地保护
│   ├── app_state.rs       # 全局状态管理
│   ├── config.rs          # 配置持久化
│   ├── models.rs          # 数据类型定义
│   └── error.rs           # 错误处理
├── icons/                 # 应用图标
└── Cargo.toml
ui/
├── index.html             # 主窗口
├── overlay.html           # 翻译结果浮层
├── styles.css             # 样式
└── app.js                 # 前端逻辑
```

## 开发

需要 Rust 工具链和 Tauri CLI：

```bash
cargo install tauri-cli --version "^2"
```

开发运行：

```bash
cargo tauri dev
```

打包：

```bash
cargo tauri build
```

产物在 `src-tauri/target/release/bundle/` 下。

## Windows 便携版

在 GitHub Release 下载 `Glance-windows-x64-portable.zip`，解压后直接运行 `Glance.exe`，无需安装。Windows 10/11 通常已经内置 Microsoft Edge WebView2 Runtime；若程序无法启动，请安装 [Evergreen WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) 后重试。

## macOS 安装说明

由于 Glance 尚未启用付费开发者证书签名（不走 App Store），在 macOS 上首次下载安装后会触发系统安全拦截。

### 处理“已损坏，无法打开”报错
如果您通过浏览器下载后提示“已损坏，无法打开”，这通常是因为 macOS Gatekeeper 对未签名应用的严格隔离。请在终端执行以下命令：

```bash
sudo xattr -cr /Applications/Glance.app
```

### 首次运行提示“无法验证开发者”
1. 将 `Glance.app` 拖入 `Applications`（应用程序）文件夹。
2. **不要直接双击打开**，而是在访达（Finder）中**右键点击** `Glance.app`，选择“打开”。
3. 在弹出的对话框中再次选择“打开”。
4. 如果依然无法运行，请前往 **系统设置 > 隐私与安全性**，点击下方的“仍要打开”。

### 更新后截图全黑

Glance 目前使用临时签名，macOS 可能会把更新后的版本识别为一个新应用，导致旧版本的屏幕录制授权不再生效。请前往 **系统设置 > 隐私与安全性 > 屏幕与系统音频录制**（旧版 macOS 为“屏幕录制”），将 Glance 的开关关闭后重新开启，再完全退出并重新打开 Glance。

如果设置中没有 Glance，先运行一次截图功能触发授权提示；仍无法恢复时，可在终端执行 `tccutil reset ScreenCapture com.harukaon.glance`，重新打开 Glance 并再次授权。

## 许可证

MIT

## 友链

- [Linux.do](https://linux.do) — 一个真实、自由、纯粹的技术社区
