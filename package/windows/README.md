# Windows 安装包（cargo-packager + NSIS）

安装包由 [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) 的 `nsis` 格式生成，
与 Siren 项目同一套方案：配置写在 `Cargo.toml` 的 `[package.metadata.packager]`，
视觉素材与 NSIS 模板放在本目录。

## 打包

```bash
# 只需装一次（本仓库按 0.11.8 的默认模板定制，见下方“模板升级”）
cargo install cargo-packager --version 0.11.8 --locked

# 仓库根目录执行；before-packaging-command 会先跑 cargo build --release
cargo packager --release
```

产物：

```text
target/release/
├── spacesniffer-rs.exe
└── spacesniffer-rs_<version>_x64-setup.exe   # 分发这个
```

cargo-packager 自带 NSIS 工具链：首次打包会下载官方 NSIS 3.09 到
`%LOCALAPPDATA%\.cargo-packager\NSIS`（需要网络），**不会**使用 PATH / MSYS2 里的 `makensis`。

## 安装包特性

| 项 | 值 / 实现 |
|---|---|
| 产品名 | `SpaceSniffer-RS`（安装目录、开始菜单、卸载项都用它） |
| 安装范围 | 当前用户（默认 `currentUser`）：装到 `%LOCALAPPDATA%\SpaceSniffer-RS`，不弹 UAC |
| 界面语言 | 简体中文优先、英文兜底，按系统语言自动选择 |
| 高分屏 | 模板里开启 `ManifestDPIAware` / `ManifestSupportedOS`，150% 缩放不发虚 |
| 安装图标 | `installer.ico`（`installer.svg` 生成） |
| 卸载图标 | `uninstall.ico`（`uninstall.svg` 生成；模板按安装图标同目录推导） |
| 欢迎/完成页 | `install-sidebar.bmp`（2× 光栅化，MUI2 自动缩放） |
| 页眉 | `install-header.bmp`（2× 光栅化） |
| License | 根目录 `LICENSE`（MIT），安装时显示 |
| 版权信息 | `Copyright (c) 2026 CYANRHYTHM`：写入 setup 的 `LegalCopyright` 与安装向导底部 `BrandingText` |
| exe 版本信息 | `build.rs` 里 `tauri-winres` 写入，公司名/产品名需与 `Cargo.toml` 同步 |

## 素材

```text
package/windows/
├── installer.svg / uninstall.svg      # 安装 / 卸载图标源文件
├── install-header.svg                 # 页眉源文件（150×57 设计稿）
├── install-sidebar.svg                # 侧栏源文件（164×314 设计稿）
├── installer.ico / uninstall.ico      # 生成物（16/24/32/48/64/128/256）
├── install-header.bmp                 # 生成物（300×114，BMP3）
├── install-sidebar.bmp                # 生成物（328×628，BMP3）
├── gen-assets.sh                      # 由 svg 重新生成上面的 ico/bmp
└── installer.nsi                      # 自定义 NSIS 模板
```

改完 svg 后重跑（MSYS2 依赖：`pacman -S mingw-w64-x86_64-librsvg mingw-w64-x86_64-imagemagick`）：

```bash
bash package/windows/gen-assets.sh
```

生成的 ico/bmp 已入库，只有改视觉素材时才需要重跑；脚本产物写在 `build/gen-assets/`（已忽略），跑完自动清理。

## 模板升级

`installer.nsi` 是从 cargo-packager **0.11.8** 的默认模板复制的（cargo-packager 以
MIT / Apache-2.0 双许可发布，模板沿用其许可），只加了两处：

1. `ManifestDPIAware true` / `ManifestSupportedOS all` —— 高分屏下界面清晰；
2. 按安装图标所在目录推导 `uninstall.ico` —— cargo-packager 没有卸载图标字段。

升级 cargo-packager 后，先对照新版默认模板
（`~/.cargo/registry/src/*/cargo-packager-<版本>/src/package/nsis/installer.nsi`）
同步占位符（`{{...}}`），否则可能打包时报缺变量。

## 已知限制

- **未签名**：首次运行 SmartScreen 会提示“未知发布者”，正式分发前请签名；
- 安装包架构跟随当前构建目标（本机为 `x64`）；ARM64 需另配 target triple。
- 若删除/改名 `installer.ico`，模板推导出的 `uninstall.ico` 路径也会失效，记得同步改 `Cargo.toml` 与模板。
