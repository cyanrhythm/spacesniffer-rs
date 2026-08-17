# SpaceSniffer-RS

跨平台磁盘空间分析工具，采用实时 Treemap 可视化，灵感来自 [SpaceSniffer](http://www.uderzo.it/main_products/space_sniffer/)。

## 功能

- **Treemap 可视化** — 目录内容按文件大小比例渲染为嵌套矩形（squarified treemap 算法）
- **实时扫描** — 矩形随扫描进度逐个出现并增长，无需等待进度条
- **目录钻入** — 点击任意矩形进入子目录；面包屑导航返回上级
- **右键菜单** — 右键点击文件/文件夹可**在文件管理器中定位**或**删除**（带确认对话框）
- **颜色编码** — 目录（蓝色）、文档（红色）、代码（绿色）、图片（紫色）、视频（深红）、音频（蓝紫）、压缩包（黄色）、可执行文件（灰色）、其他（自动哈希着色）
- **跨平台** — Windows、macOS、Linux
- **内置 CJK 字体** — 自动加载系统 CJK 字体，中文/日文/韩文路径正常显示
- **SVG 窗口图标** — 矢量图标编译进二进制

## 构建

环境要求：
- Rust 1.80+
- Cargo

```bash
git clone https://gitee.com/purplesakura/spacesniffer-rs
cd spacesniffer-rs
cargo build --release
```

编译产物位于 `target/release/spacesniffer-rs`（Windows 下为 `.exe`）。

## 使用方式

1. 启动程序 — 初始为空闲状态，显示路径输入框
2. 输入路径或点击 **Browse...** 选择目录
3. 点击 **Scan** — 矩形实时出现并增长
4. **左键** 点击目录矩形进入子目录
5. **左键** 点击空白区域返回上级
6. **右键** 点击任意矩形弹出菜单：
   - **Open file location** — 在系统文件管理器中定位该文件
   - **Delete** — 删除文件/文件夹（弹出确认对话框）
7. 使用顶部的**面包屑导航**快速跳转层级

### 扫描建议

快速体验可扫描某个项目目录或 `~/Documents`。扫描整个用户主目录耗时较长但完全可以。

## 项目结构

```
spacesniffer-rs/
├── Cargo.toml
├── build.rs
├── icon.svg                  # 矢量窗口图标
├── ui/
│   └── main.slint            # Slint 声明式 UI（实验性）
├── src/
│   ├── main.rs               # 入口，CJK 字体加载，图标加载
│   ├── app.rs                # UI 状态管理，egui 渲染，导航逻辑
│   ├── scanner.rs            # 递归目录扫描器
│   └── treemap.rs            # squarified treemap 布局算法
```

### 核心算法

- **Squarified Treemap**（Bruls, Huizing, van Wijk）— 生成的矩形长宽比尽可能接近 1:1
- **增量扫描** — 扫描线程每 150ms 发送部分树快照，实现实时渲染

## 依赖

| Crate | 用途 |
|-------|------|
| `eframe` / `egui` | GUI 框架（即时模式） |
| `rfd` | 系统原生文件/目录选择对话框 |
| `resvg` / `tiny-skia` | SVG 图标渲染 |
| `walkdir` | （预留，用于后续优化扫描性能） |

## License

MIT

## 后续工作

### 性能优化

- **扫描快照浅拷贝** — 当前扫描线程每 250ms 深拷贝整棵已扫描树（`snapshot.clone()`），对大型目录为 O(n²) 开销。子树改为 `Arc<FileEntry>` 共享，快照只做浅拷贝。
- **减少扫描线程锁竞争** — 当前每个文件需锁 2~3 次进度互斥量并分配路径字符串，建议合并为单次锁操作并局部累加计数，每 N 个文件再更新一次。
- **treemap 布局缓存** — 当前每帧重建 items 并重算布局（O(n log n)），且每个矩形创建独立 clip 上下文；改为仅在画布尺寸变化时重算，文本用 `painter.galley` 预排版缓存。

### 正确性

- **符号链接循环防护** — 扫描使用 `metadata()` 会跟随符号链接，目录链接可能导致无限递归或重复扫描，改用 `symlink_metadata()` 或跳过链接目录。
- **空目录返回导航** — 空目录视图未处理点击空白返回上级，用户只能靠面包屑导航。
- **删除失败反馈** — `perform_delete` 删除失败时静默返回，应提示用户原因。

### 体验改进

- **扫描中导航连贯性** — 部分树中子目录为空，扫描期间无法点击进入，且最终结果到达时整体替换根节点会弹回导航位置。
- **删除确认按钮本地化** — 删除按钮硬编码英文 "Delete"，未走 i18n。
- **右键菜单边界检测** — 菜单位置靠近屏幕边缘时会超出可视区域。

### 代码清理

- 移除 `treemap_rects` 死字段（仅赋值从未读取）。
- 更新过时的依赖表（`walkdir` 未在 Cargo.toml 中声明）。
- 删除操作基于数组索引定位节点，删除后索引失效，考虑改用稳定标识。
