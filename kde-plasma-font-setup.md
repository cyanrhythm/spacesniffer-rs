# KDE Plasma + Monaco + MiSans 字体配置指南

> 参考：[ArchWiki - Font configuration](https://wiki.archlinux.org/title/Font_configuration)

## 目标

在 KDE Plasma 桌面环境下：
- **Monaco** 为主字体（优先显示）
- **MiSans** 为回退字体（当 Monaco 缺少字符时 fallback）

---

## 1. 安装字体

### 1.1 复制字体到用户字体目录

```bash
# 创建用户字体目录
mkdir -p ~/.local/share/fonts

# 复制 Monaco 字体（按你的实际路径调整）
cp /path/to/Monaco*.ttf ~/.local/share/fonts/

# 复制 MiSans 字体
cp /path/to/MiSans*.ttf ~/.local/share/fonts/
```

### 1.2 刷新字体缓存

```bash
fc-cache -fv
```

### 1.3 验证字体已识别

```bash
fc-list | grep -i monaco
fc-list | grep -i misans
```

如果输出中包含字体名称，说明安装成功。

---

## 2. 配置 Fontconfig 回退字体

### 2.1 创建/编辑用户配置

文件路径：`~/.config/fontconfig/fonts.conf`

```xml
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>

  <!--
    Monaco 为主字体，MiSans 为回退字体。
    当 Monaco 中不存在某个字符（如中文）时，
    fontconfig 会自动从 MiSans 中加载该字符。
  -->
  <alias>
    <family>Monaco</family>
    <prefer>
      <family>Monaco</family>
      <family>MiSans</family>
    </prefer>
  </alias>

  <!-- 等宽字体也使用 Monaco + MiSans 组合 -->
  <alias>
    <family>monospace</family>
    <prefer>
      <family>Monaco</family>
      <family>MiSans</family>
    </prefer>
  </alias>

  <!-- 无衬线字体使用 MiSans -->
  <alias>
    <family>sans-serif</family>
    <prefer>
      <family>MiSans</family>
    </prefer>
  </alias>

</fontconfig>
```

> **提示**：也可将配置拆分为 `~/.config/fontconfig/conf.d/50-monaco-misans.conf`，
> 效果相同，更便于管理。

### 2.2 ⚠️ KDE Plasma 的特殊注意事项

ArchWiki 明确指出：

> For some desktop environments (such as GNOME and **KDE Plasma**) using the
> Font Control Panel will automatically create or overwrite the user font
> configuration file.

即 KDE 的**字体控制面板会覆盖** `~/.config/fontconfig/fonts.conf`。

**建议策略**：

- **方案 A**：先通过 KDE 字体面板把「通用」「等宽字体」设为 Monaco，
  再手动编辑 `fonts.conf` 补充 `<alias>` 回退规则，此后**不要再点击面板中的"应用"**。
- **方案 B**：完全放弃 KDE 字体面板，仅在 `fonts.conf` 中配置所有规则。

---

## 3. KDE Plasma GUI 字体设置

打开 **系统设置 → 外观 → 字体**，建议如下配置：

| 字体类型     | 设置值  |
|-------------|--------|
| 通用         | Monaco |
| 等宽字体     | Monaco |
| 工具提示     | Monaco 或 MiSans |
| 窗口标题     | Monaco 或 MiSans |
| 工具栏       | Monaco 或 MiSans |
| 菜单         | Monaco 或 MiSans |

设置完成后，中文字符会自动从 MiSans 中渲染，英文/代码字符由 Monaco 渲染。

---

## 4. 验证配置

### 4.1 查看 Monaco 的 fallback 链

```bash
fc-match -s Monaco | head -10
```

确认 `MiSans` 出现在 Monaco 的 fallback 列表中且靠近顶部。

### 4.2 查看中文字符的匹配结果

```bash
fc-match Monaco:lang=zh
```

如果输出为 `MiSans`，说明中文回退配置正确。

### 4.3 查看详细的 fontconfig 规则

```bash
fc-match --verbose Monaco
```

---

## 5. 常见问题

### 5.1 KDE 字体面板覆盖了我的配置

KDE 的「系统设置 → 外观 → 字体」保存时会重写 `~/.config/fontconfig/fonts.conf`。
解决方式：

1. 在 KDE 面板中设置好基础字体（Monaco）。
2. 手动将 `<alias>` 回退规则写回 `fonts.conf`。
3. 此后如需调整，直接编辑 `fonts.conf`，避免再使用 KDE 字体面板点击"应用"。

### 5.2 某些应用没有生效

- 需要**重启应用**或**重新登录**才能加载新的 fontconfig 配置。
- 极少数应用（如 URxvt）不支持 Fontconfig，需要通过 `~/.Xresources` 设置。

### 5.3 Emoji 或其他符号无法显示

可在 `<prefer>` 列表中继续追加 Emoji 字体：

```xml
<alias>
  <family>Monaco</family>
  <prefer>
    <family>Monaco</family>
    <family>MiSans</family>
    <family>Noto Color Emoji</family>
  </prefer>
</alias>
```

---

## 6. 参考链接

- [ArchWiki - Font configuration](https://wiki.archlinux.org/title/Font_configuration)
- [ArchWiki - Fonts](https://wiki.archlinux.org/title/Fonts)
- `man fonts.conf`
- `man fc-match`
