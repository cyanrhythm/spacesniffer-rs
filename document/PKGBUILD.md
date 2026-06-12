# PKGBUILD 参考手册

## 必须变量

| 变量 | 类型 | 说明 | 示例 |
|---|---|---|---|
| `pkgname` | 字符串 或 数组 | 包名。数组表示拆分包 | `pkgname=spacesniffer-rs` |
| `pkgver` | 字符串 | 上游版本号 | `pkgver=0.1.0` |
| `pkgrel` | 整数 | Arch 打包修订号，上游没变但打包脚本变了时递增 | `pkgrel=1` |
| `arch` | 数组 | 支持架构：`x86_64`, `aarch64`, `any`, `i686` 等 | `arch=('x86_64' 'aarch64')` |

## 可选变量

| 变量 | 类型 | 说明 | 示例 |
|---|---|---|---|
| `pkgdesc` | 字符串 | 包描述（不超过 80 字符） | `pkgdesc="A disk space analyzer"` |
| `url` | 字符串 | 上游项目主页 | `url="https://github.com/user/repo"` |
| `license` | 数组 | 许可证。可多选 | `license=('MIT')` |
| `depends` | 数组 | **运行时**依赖 | `depends=('gcc-libs' 'gtk3')` |
| `makedepends` | 数组 | **构建时**才需要的依赖 | `makedepends=('cargo' 'cmake' 'git')` |
| `checkdepends` | 数组 | 跑测试时才需要的依赖 | `checkdepends=('python-pytest')` |
| `optdepends` | 数组 | 可选依赖，格式 `'包名: 描述'` | `optdepends=('git: 获取开发版')` |
| `provides` | 数组 | 声明本包能替代哪些包 | `provides=('spacesniffer-rs')` |
| `conflicts` | 数组 | 声明本包与哪些包冲突 | `conflicts=('spacesniffer-rs')` |
| `replaces` | 数组 | 声明本包取代哪些老包 | `replaces=('old-name-bin')` |
| `backup` | 数组 | 哪些配置文件在删除包时保留 | `backup=('etc/myapp/config.toml')` |
| `source` | 数组 | 源码下载地址，可混合多种协议 | 见下文「source 协议」 |
| `validpgpkeys` | 数组 | PGP 签名验证所用公钥指纹 | `validpgpkeys=('AB1234CD...')` |
| `sha256sums` | 数组 | source 各条目对应的校验和 | `sha256sums=('SKIP')` |
| `noextract` | 数组 | 哪些 source 不解压 | `noextract=('vendor.tar.gz')` |
| `install` | 字符串 | `.install` 脚本路径 | `install=spacesniffer-rs.install` |
| `changelog` | 字符串 | changelog 文件路径 | `changelog=CHANGELOG` |
| `options` | 数组 | 控制 makepkg 行为 | 见下文「options 选项」 |

## source 协议

| 协议 | 说明 | 示例 |
|---|---|---|
| `http://` / `https://` | HTTP 下载 tar 包/文件 | `source=("${url}/archive/v${pkgver}.tar.gz")` |
| `git+https://` | Git 克隆，需 `makedepends=('git')` | `source=("git+${url}.git")` |
| `svn+http://` | Subversion 检出 | `source=("svn+http://path/to/repo")` |
| `bzr+http://` | Bazaar 分支 | `source=("bzr+http://path/to/repo")` |
| `hg+http://` | Mercurial 克隆 | `source=("hg+http://path/to/repo")` |
| `local://` 或相对路径 | 本地文件 | `source=("myapp.desktop")` |

### 重命名语法

用 `::` 分隔目标名和源地址：

```
source=("mydir::git+https://example.com/repo.git"
        "myfile.tar.gz::https://example.com/v1.0.tar.gz")
```

### 分支/标签/commit 指定

```
source=("myrepo::git+${url}.git#branch=develop"
        "myrepo::git+${url}.git#tag=v1.0"
        "myrepo::git+${url}.git#commit=abc1234")
```

## options 选项

| 选项 | 含义 |
|---|---|
| `!strip` | 不剥离二进制调试信息 |
| `!debug` | 不生成 debug 包 |
| `!makeflags` | 不使用 MAKEFLAGS |
| `!buildflags` | 不使用 CFLAGS/LDFLAGS |
| `staticlibs` | 保留静态库（.a） |
| `!lto` | 禁用链接时优化 |
| `!zipman` | 不压缩 man 手册页 |
| `!emptydirs` | 不保留空目录 |

```
options=('!strip' '!debug')
```

## 路径变量

| 变量 | 含义 | 典型值（AUR 包示例） |
|---|---|---|
| `$startdir` | PKGBUILD 所在目录 | `/home/user/aur/mypkg/` |
| `$srcdir` | 源码目录，source 下载后放在这 | `/home/user/aur/mypkg/src/` |
| `$pkgdir` | 打包目录，在此模拟安装 | `/home/user/aur/mypkg/pkg/mypkg/` |
| `$pkgdirbase` | 不含子包名的 pkgdir | `/home/user/aur/mypkg/pkg/` |
| `$pkgname` | 当前包名（仅拆分包时有用） | `mypkg` |
| `$BUILDDIR` | 构建目录，默认 `$startdir` | `/home/user/aur/mypkg/` |
| `$PKGDEST` | 生成的 .pkg.tar.zst 存放位置 | 默认 `$startdir` |
| `$SRCDEST` | source 文件缓存位置 | 默认 `$startdir` |
| `$LOGDEST` | 构建日志存放位置 | 默认 `$startdir` |

## 必须函数

| 函数 | 说明 | 何时需要 |
|---|---|---|
| `package()` | 将文件安装到 `$pkgdir`，**唯一强制要求的函数**。拆分包时用 `package_<子包名>()` 代替 | 始终 |

## 可选函数

| 函数 | 说明 | 何时需要 |
|---|---|---|
| `pkgver()` | 动态生成 `pkgver`，用 echo/printf 输出版本号 | VCS 包（`-git`、`-svn`） |
| `prepare()` | 应用补丁、修改源码、设置编译选项 | 需要打补丁或修改源码时 |
| `build()` | 编译源码、执行 `make`/`cargo`/`cmake` | 需要编译时 |
| `check()` | 运行测试 | 需要跑测试套件时 |

### 执行顺序

```
1. pkgver()  ← 对 VCS 包最先执行，更新版本号
2. prepare() ← 应用补丁
3. pkgver()  ← 非 VCS 包在此处执行（如有）
4. build()   ← 编译
5. check()   ← 测试
6. package() ← 打包安装
```

## 最小 PKGBUILD 示例

```bash
pkgname=myapp
pkgver=1.0
pkgrel=1
pkgdesc="My Application"
arch=('any')
url="https://example.com"
license=('MIT')
source=("${url}/releases/download/v${pkgver}/${pkgname}-${pkgver}.tar.gz")
sha256sums=('abc123...')

package() {
  cd "${srcdir}/${pkgname}-${pkgver}"
  install -Dm755 myapp "${pkgdir}/usr/bin/myapp"
}
```

## Cargo/Rust 包示例

```bash
makedepends=('cargo' 'git')   # 构建依赖

build() {
  cd "${srcdir}/${pkgname}"
  cargo build --release --locked
}

package() {
  cd "${srcdir}/${pkgname}"
  install -Dm755 "target/release/${pkgname}" \
    "${pkgdir}/usr/bin/${pkgname}"
}
```

## 拆分包（Split Packages）

一个 PKGBUILD 打出多个包，核心变化：**`pkgname` 从字符串变成数组，函数从 `package()` 变成 `package_<子包名>()`。**

### 核心规则

| 变化点 | 单包 | 拆分包 |
|---|---|---|
| `pkgname` | 字符串 `pkgname=myapp` | 数组 `pkgname=('mybin' 'mylib')` |
| `package()` | `package()` | **不能有** `package()`，只用 `package_mybin()` |
| 子包函数 | 无 | `package_mybin()` / `package_mylib()` |
| 共享函数 | 无 | `build()` / `check()` 对所有子包共用 |

### 变量作用域

大部分变量可共享，也可用后缀限定给特定子包：

```
# 所有子包共享
arch=('x86_64')
source=("...")
build() { ... }          # 只构建一次

# 专门给 mybin 子包
depends_mybin=('gtk3')
pkgdesc_mybin="GUI application"

# 专门给 mylib 子包
depends_mylib=('glibc')
pkgdesc_mylib="Shared library"

# 全局默认，子包不覆盖则使用这个
depends=('gcc-libs')     # 两个子包都会带上
```

后缀规则：`<变量名>_<子包名>`，比如 `depends_mylib`、`optdepends_mybin`。

### 在 package_xxx() 中区分子包

`$pkgname` 在子包函数内自动是当前子包名：

```bash
package_mybin() {
  cd "${srcdir}/${pkgbase}"
  install -Dm755 mybin "${pkgdir}/usr/bin/mybin"
  install -Dm644 mybin.desktop "${pkgdir}/usr/share/applications/mybin.desktop"
}

package_mylib() {
  cd "${srcdir}/${pkgbase}"
  install -Dm755 libfoo.so "${pkgdir}/usr/lib/libfoo.so"
  install -Dm644 foo.h "${pkgdir}/usr/include/foo.h"
}
```

`$pkgbase` 指向全局 `pkgbase` 变量（即源码目录名），默认等于 `pkgname[0]`。

### 完整示例：一个 Rust 项目分出 bin 和 lib 两个包

```bash
pkgbase=myproject                          # 可选，指定基名（源码目录、source 命名等）
pkgname=('myproject-bin' 'myproject-lib')  # 数组，定义子包名
pkgver=1.0.0
pkgrel=1
pkgdesc="My Project"                       # 全局描述（子包可覆盖）
arch=('x86_64')
url="https://example.com"
license=('MIT')
makedepends=('cargo')

# 各自子包的依赖和描述
depends_myproject-bin=('gtk3')
pkgdesc_myproject-bin="GUI application"
depends_myproject-lib=('glibc')
pkgdesc_myproject-lib="Core library"

provides=('libmyproject.so')
provides_myproject-bin=('myproject')
conflicts_myproject-bin=('myproject')

source=("${url}/archive/v${pkgver}.tar.gz")
sha256sums=('abc123...')

build() {
  cd "${srcdir}/${pkgbase}-${pkgver}"
  cargo build --release --locked
}

package_myproject-bin() {
  cd "${srcdir}/${pkgbase}-${pkgver}"
  install -Dm755 "target/release/myproject" \
    "${pkgdir}/usr/bin/myproject"
  install -Dm644 "resources/icon.svg" \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/myproject.svg"
  install -Dm644 "resources/myproject.desktop" \
    "${pkgdir}/usr/share/applications/myproject.desktop"
}

package_myproject-lib() {
  cd "${srcdir}/${pkgbase}-${pkgver}"
  install -Dm755 "target/release/libmyproject.so" \
    "${pkgdir}/usr/lib/libmyproject.so"
  install -Dm755 "target/release/libmyproject.a" \
    "${pkgdir}/usr/lib/libmyproject.a"
  install -Dm644 "include/myproject.h" \
    "${pkgdir}/usr/include/myproject.h"
}
```

### 拆分包与 provides/conflicts

`provides` / `conflicts` 后缀规则一致：

```bash
# 全局声明（或用后缀限定）
provides=('libmyproject.so')
conflicts=('old-myproject')

# 仅某个子包
provides_myproject-bin=('myproject')
conflicts_myproject-bin=('myproject-git')
```

### 打包结果

```bash
makepkg
# 生成：
#   myproject-bin-1.0.0-1-x86_64.pkg.tar.zst
#   myproject-lib-1.0.0-1-x86_64.pkg.tar.zst
```

## .install 安装/卸载钩子（类似 Debian 的 preinst/postinst/prerm/postrm）

Arch 不在 PKGBUILD 中直接写钩子，而是通过独立的 `.install` 文件定义。

### 基本用法

1. 创建 `包名.install` 文件
2. PKGBUILD 中声明：`install=spacesniffer-rs.install`
3. 在 `.install` 中定义以下函数：

### 函数映射表

| Debian | Arch `.install` | 触发时机 |
|---|---|---|
| `preinst` | `pre_install()` | 安装前（新包即将写入磁盘） |
| `postinst` | `post_install()` | 安装后（新包已写入磁盘） |
| `prerm` | `pre_remove()` | 卸载前（准备删除本包） |
| `postrm` | `post_remove()` | 卸载后（本包文件已删除） |
| — | `pre_upgrade()` | 升级：新包准备装，旧包还在。`$1`=新包路径，`$2`=旧包路径 |
| — | `post_upgrade()` | 升级：新包已装，旧包即将被删。`$1`=新包路径，`$2`=旧包路径 |

### 完整示例

`spacesniffer-rs.install`：

```bash
pre_install() {
  echo "准备安装 $1 ..."
}

post_install() {
  update-desktop-database -q
  echo "安装完成，运行: spacesniffer-rs"
}

pre_upgrade() {
  echo "正在从旧版本升级..."
}

post_upgrade() {
  post_install
}

pre_remove() {
  echo "谢谢使用 spacesniffer-rs"
}

post_remove() {
  update-desktop-database -q
}
```

### 函数参数

| 参数 | 含义 |
|---|---|
| `$1` | 当前操作的 .pkg.tar.zst 包路径 |
| `$2` | 仅 `pre_upgrade()` 和 `post_upgrade()`：旧包的 .pkg.tar.zst 路径 |

### 拆分包中的 install

不同子包可指定不同 `.install`，后缀规则与 `depends` 等变量一致：

```bash
install=common.install                  # 所有子包共用
install_mybin=mybin.install             # 仅 mybin 子包
install_mylib=mylib.install             # 仅 mylib 子包
```

## install 命令参数

`package()` 中常用 `install -DmXXX src dst`：

| 参数 | 含义 |
|---|---|
| `-D` | 自动创建目标路径的上级目录 |
| `-m755` | 可执行文件权限 |
| `-m644` | 普通文件（如 .desktop、图标）权限 |

## FHS 规范目录参考

PKGBUILD 没有预制的路径宏，所有路径需硬编码。以下是 Linux FHS（Filesystem Hierarchy Standard）和 freedesktop.org 定义的标准目录及 PKGBUILD 中的写法。

### FHS 核心目录

| 目录 | 用途 | PKGBUILD 典型写法 |
|---|---|---|
| `/usr/bin/` | 可执行文件 | `"${pkgdir}/usr/bin/myapp"` |
| `/usr/lib/` | 共享库（.so） | `"${pkgdir}/usr/lib/libfoo.so"` |
| `/usr/include/` | C/C++ 头文件（.h） | `"${pkgdir}/usr/include/foo.h"` |
| `/usr/share/` | 架构无关数据 | `"${pkgdir}/usr/share/myapp/"` |
| `/usr/share/doc/` | 文档 | `"${pkgdir}/usr/share/doc/${pkgname}/"` |
| `/usr/share/man/` | man 手册 | `"${pkgdir}/usr/share/man/man1/myapp.1"` |
| `/usr/share/locale/` | 翻译文件（.mo） | `"${pkgdir}/usr/share/locale/zh_CN/LC_MESSAGES/myapp.mo"` |
| `/usr/lib/systemd/` | systemd 单元 | `"${pkgdir}/usr/lib/systemd/system/myapp.service"` |
| `/usr/lib/sysusers.d/` | systemd 系统用户定义 | `"${pkgdir}/usr/lib/sysusers.d/myapp.conf"` |
| `/usr/lib/tmpfiles.d/` | systemd 临时文件规则 | `"${pkgdir}/usr/lib/tmpfiles.d/myapp.conf"` |
| `/etc/` | 系统级配置 | `"${pkgdir}/etc/myapp/config.toml"` |
| `/opt/` | 第三方独立软件包 | `"${pkgdir}/opt/myapp/"` |
| `/usr/lib/modules-load.d/` | 内核模块自动加载 | `"${pkgdir}/usr/lib/modules-load.d/myapp.conf"` |
| `/usr/lib/udev/rules.d/` | udev 规则 | `"${pkgdir}/usr/lib/udev/rules.d/99-myapp.rules"` |
| `/var/lib/` | 可变状态数据 | `"${pkgdir}/var/lib/myapp/"` |
| `/var/log/` | 日志（归档用） | `"${pkgdir}/var/log/myapp/"` |

### freedesktop.org 规范目录

| 目录 | 规范 | 用途 | PKGBUILD 典型写法 |
|---|---|---|---|
| `/usr/share/applications/` | Desktop Entry | `.desktop` 启动器 | `"${pkgdir}/usr/share/applications/myapp.desktop"` |
| `/usr/share/icons/hicolor/` | Icon Theme | 图标文件 | `"${pkgdir}/usr/share/icons/hicolor/48x48/apps/myapp.png"` |
| `/usr/share/icons/hicolor/scalable/apps/` | Icon Theme | SVG 矢量图标 | `"${pkgdir}/usr/share/icons/hicolor/scalable/apps/myapp.svg"` |
| `/usr/share/metainfo/` | AppStream | 软件中心元数据 | `"${pkgdir}/usr/share/metainfo/myapp.metainfo.xml"` |
| `/usr/share/dbus-1/services/` | D-Bus | D-Bus 服务定义 | `"${pkgdir}/usr/share/dbus-1/services/org.example.MyApp.service"` |
| `/usr/share/dbus-1/system-services/` | D-Bus | 系统级 D-Bus 服务 | `"${pkgdir}/usr/share/dbus-1/system-services/org.example.MyApp.service"` |
| `/usr/share/polkit-1/actions/` | PolicyKit | 权限策略 | `"${pkgdir}/usr/share/polkit-1/actions/org.example.myapp.policy"` |
| `/usr/share/mime/packages/` | Shared MIME | MIME 类型注册 | `"${pkgdir}/usr/share/mime/packages/myapp.xml"` |
| `/usr/lib/systemd/user/` | systemd | 用户级 systemd 服务 | `"${pkgdir}/usr/lib/systemd/user/myapp.service"` |
| `/usr/share/fish/vendor_completions.d/` | Fish Shell | Fish 补全脚本 | `"${pkgdir}/usr/share/fish/vendor_completions.d/myapp.fish"` |
| `/usr/share/zsh/site-functions/` | Zsh | Zsh 补全脚本 | `"${pkgdir}/usr/share/zsh/site-functions/_myapp"` |
| `/usr/share/bash-completion/completions/` | Bash | Bash 补全脚本 | `"${pkgdir}/usr/share/bash-completion/completions/myapp"` |

### 常用 install 模板

```bash
package() {
  cd "${srcdir}/${pkgname}-${pkgver}"

  # 二进制
  install -Dm755 myapp "${pkgdir}/usr/bin/myapp"

  # .desktop 启动器
  install -Dm644 resources/myapp.desktop \
    "${pkgdir}/usr/share/applications/myapp.desktop"

  # 图标（多分辨率）
  for size in 16 24 32 48 64 128 256; do
    install -Dm644 "resources/icons/${size}x${size}.png" \
      "${pkgdir}/usr/share/icons/hicolor/${size}x${size}/apps/myapp.png"
  done

  # SVG 图标
  install -Dm644 resources/icon.svg \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/myapp.svg"

  # AppStream 元数据
  install -Dm644 resources/myapp.metainfo.xml \
    "${pkgdir}/usr/share/metainfo/myapp.metainfo.xml"

  # 文档（LICENSE、README 等）
  install -Dm644 LICENSE "${pkgdir}/usr/share/doc/${pkgname}/LICENSE"
  install -Dm644 README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"

  # man 手册
  install -Dm644 doc/myapp.1 "${pkgdir}/usr/share/man/man1/myapp.1"

  # Shell 补全
  install -Dm644 completions/myapp.bash \
    "${pkgdir}/usr/share/bash-completion/completions/myapp"
  install -Dm644 completions/myapp.zsh \
    "${pkgdir}/usr/share/zsh/site-functions/_myapp"
  install -Dm644 completions/myapp.fish \
    "${pkgdir}/usr/share/fish/vendor_completions.d/myapp.fish"

  # systemd 服务
  install -Dm644 resources/myapp.service \
    "${pkgdir}/usr/lib/systemd/system/myapp.service"

  # 配置文件
  install -Dm644 config/default.toml \
    "${pkgdir}/etc/myapp/config.toml"
}
```

### update-desktop-database 等缓存刷新

安装了 `.desktop`、图标、MIME 类型的包需要在 `.install` 钩子中刷新缓存：

```bash
# 在 post_install / post_upgrade / post_remove 中调用
post_install() {
  update-desktop-database -q       # .desktop 文件
  update-mime-database /usr/share/mime  # MIME 类型
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor  # GTK 图标缓存
}
```
