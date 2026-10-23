# gpui-component / gpui-kit 0.8.0 API 速查表（源码行号实证）

路径前缀（根 `C:\Users\alice\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\`）：
`GK`=gpui-kit-0.8.0/src · `GC`=gpui-component-0.8.0/src · `GP`=gpui-pre-0.3.9/src · `PL`=gpui-pre-platform-0.3.9/src · `GB`=gpui-base-0.7.3/src
⚠ **本地没有 gpui-base-0.8.0 源码**（只有 0.7.3）。`GB` 行号是 0.7.3；凡 0.8.0 自身源码调用过该 API 处，我已用 `GC 证实` 标注。

## 0. 先修正 5 个假设（照抄前必读）

| 你的写法 | 源码真相 | 证据 |
|---|---|---|
| `input::TextInput` | **不存在**。元素叫 `Input`；`TextInputState` 是私有内部类型 | `GC/input/input.rs:111`；`GC/input/mod.rs` 无该导出 |
| `WindowExt::open_modal` | **不存在**。是 `open_dialog`/`open_alert_dialog`/`open_sheet` | `GC/window_ext.rs:75,101,20` |
| `progress::ProgressBar` | **不存在**。线性=`Progress`，圆形=`ProgressCircle` | `GC/progress/mod.rs:4-5` |
| `button::ButtonSize` | **不存在**（全库只在一句注释里出现）。用 `Size` + `Sizable::small()` | `GC/sizing.rs:6,213`；grep 仅 `GC/sizing.rs:193` 注释 |
| `WindowOptions.title` | 顶层无此字段，标题在 `titlebar.title` | `PL/platform.rs:2491-2560`、`2694` |

trait 方法（`.small()`/`.primary()`/`.h_flex()`/`.value()`）不 import 就不编译：`use gpui_kit::component::{Sizable as _, ButtonVariants as _, StyledExt as _};`

---

## 1. 应用入口

**use** `use gpui_kit::*;`（= gpui 全量；组件层 `gpui_kit::component`）

**签名**
- `GK/lib.rs:200` `pub use platform::application;` → `PL/gpui_platform.rs:13` `pub fn application() -> gpui::Application`
- `GP/app.rs:249` `pub fn run<F>(self, on_finish_launching: F) where F: 'static + FnOnce(&mut App)`
- `GK/lib.rs:207` `pub fn init(cx: &mut App)`
- `GK/lib.rs:181-185` `pub fn open_window<V: Render>(options: WindowOptions, cx: &mut App, build: impl FnOnce(&mut Window, &mut App) -> Entity<V>) -> Result<(AnyWindowHandle, Entity<V>)>` ← 内部已包 `base::Root`
- `PL/platform.rs:2491` `pub struct WindowOptions`：`:2495 window_bounds: Option<WindowBounds>`、`:2498 titlebar`、`:2507 kind: WindowKind`、`:2512 is_movable`、`:2533 is_resizable`、`:2546 app_id: Option<String>`、`:2549 window_min_size: Option<Size<Pixels>>`、`:2553 window_decorations`、`:2556 icon: Option<Arc<image::RgbaImage>>`、`:2559 tabbing_identifier`
- `PL/platform.rs:2662` `impl Default` · `:2692` `TitlebarOptions { :2694 title: Option<SharedString>, :2698 appears_transparent }` · `:2629` `WindowBounds::{Windowed,Maximized,Fullscreen}` · `:2657` `WindowBounds::centered(size, cx)`
- `GC/title_bar.rs:82` `pub fn window_options() -> WindowOptions`（自绘标题栏基座，`:73-81` 有官方示例）

```rust
use gpui_kit::*;
use gpui_kit::component::TitleBar;

struct DiskView;
impl Render for DiskView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { div().size_full().child("hi") }
}
fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx); // 必须：theme / Root 插件 / menu / tooltip / popover 全靠它
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1100.), px(720.)), cx)),
            window_min_size: Some(size(px(720.), px(480.))),
            app_id: Some("spacesnifferx".into()),
            ..TitleBar::window_options()          // 或 ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| DiskView))
            .expect("failed to open window");
    });
}
```

---

## 2. Root 与 WindowExt

**use** `use gpui_kit::component::{Root, WindowExt};`

**签名**
- `GC/lib.rs:101` `pub use gpui_base::Root;`（不是组件库自定义类型）
- `GB/root.rs:147` `pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self`
- `GB/root.rs:101/176/183` `register_plugin<V: RootPlugin>(cx, build)` / `read(window,cx) -> &Self` / `update(window, cx, f)`
- `GB/root.rs:280-309` `impl Render for Root`：按序渲染 `view` + 各 plugin 浮层（dialog/notification/tooltip 全挂这里）
- `GC/root.rs:22` `gpui_base::Root::register_plugin::<WindowState>(cx, WindowState::new);`（由 `gpui_component::init` 调用）
- `GC/dialog/dialog.rs:906` `crate::Root::new(view, window, cx)` ← 0.8.0 侧证实参数顺序

`WindowExt`（`GC/window_ext.rs:13` 定义，`:141 impl WindowExt for Window`）
```
:20  fn open_sheet<F>(&mut self, cx: &mut App, build: F) where F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static
:30  fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F) where F: Fn(Sheet,&mut Window,&mut App) -> Sheet + 'static
:35  fn has_active_sheet(&mut self, cx: &mut App) -> bool
:38  fn close_sheet(&mut self, cx: &mut App)
:75  fn open_dialog<F>(&mut self, cx: &mut App, build: F) where F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static
:101 fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F) where F: Fn(AlertDialog,&mut Window,&mut App) -> AlertDialog + 'static
:106 fn has_active_dialog(&mut self, cx: &mut App) -> bool
:109 fn close_dialog(&mut self, cx: &mut App)        :112 fn close_all_dialogs(&mut self, cx: &mut App)
:115 fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App)
:119 fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App)
:122 fn remove_notification1<T: Sized + 'static>(&mut self, key: impl Into<ElementId>, cx: &mut App)
:125 fn clear_notifications(&mut self, cx: &mut App)
:128 fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>
:136 fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState>
:138 fn has_focused_input(&mut self, cx: &mut App) -> bool
```

dialog/notification 能工作的三要素：`gpui_kit::init(cx)`；窗口根是 `Root`（`gpui_kit::open_window` 已自动包，**别自己再套一层**）；正常 `Render`。只有直接用 `cx.open_window` 时才需手写：
```rust
let view = cx.new(|cx| DiskView::new(window, cx));
cx.open_window(options, |window, cx| cx.new(|cx| Root::new(view.clone(), window, cx)))?;
```

---

## 3. 输入框

**use** `use gpui_kit::component::input::{Input, InputState, InputEvent};`

**签名**
- `GB/input/input/mod.rs:10` `pub type InputState = InputBaseState<InputMode>;`
- `GB/input/base/state.rs:10922` `pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self`
- 同文件 `:904 placeholder(impl Into<SharedString>)`、`:1331 default_value(impl Into<SharedString>)`、`:1347 value(&self) -> SharedString`、`:1038 set_value(&mut self, value: impl Into<InputContent>, window: &mut Window, cx: &mut Context<Self>)`
- `GB/input/base/state.rs:127` `pub enum InputEvent { Change, PressEnter { secondary: bool, shift: bool }, Focus, Blur }` ← **结构体变体**，必须 `PressEnter { .. }`
- `GC/input/input.rs:110` `#[derive(IntoElement)] pub struct Input`；`:213` `pub fn new(state: &Entity<InputState>) -> Self`；`:276/281/317` `prefix/suffix/cleanable`
- `GC/input/search.rs:103` `cx.subscribe(&search_input, |this: &mut Self, _, ev: &InputEvent, cx| ..)` ← 0.8.0 真实写法

```rust
use gpui_kit::*;
use gpui_kit::component::input::{Input, InputEvent, InputState};

struct DiskView { path_input: Entity<InputState>, _subs: Vec<Subscription> }

impl DiskView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let path_input = cx.new(|cx| {
            InputState::new(window, cx)                  // 必须传 window
                .placeholder("输入要扫描的路径")
                .default_value("C:\\")
        });
        let subs = vec![cx.subscribe(&path_input, |this: &mut Self, _st, ev: &InputEvent, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                let v = this.path_input.read(cx).value(); // SharedString
                this.start_scan(v.to_string(), cx);
            }
        })];
        Self { path_input, _subs: subs }
    }
    fn start_scan(&mut self, _p: String, cx: &mut Context<Self>) { cx.notify(); }
    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.path_input.update(cx, |s, cx| s.set_value("D:\\", window, cx)); // (value, window, cx)
    }
    fn ui(&self) -> impl IntoElement {
        div().w_full().child(Input::new(&self.path_input).cleanable(true))
    }
}
```
读 `state.read(cx).value()`；写 `state.update(cx, |s,cx| s.set_value(v, window, cx))`。`set_value` **不发** `InputEvent::Change`（`GB/input/base/state.rs:1032` 注释）。

---

## 4. 按钮

**use** `use gpui_kit::component::button::{Button, ButtonVariants}; use gpui_kit::component::{Sizable, Disableable};`

**签名**
- `GC/button/button.rs:242 new(impl Into<ElementId>)` · `:373 label(impl Into<SharedString>)` · `:399 icon(impl Into<ButtonIcon>)` · `:445 loading(bool)` · `:451 compact()`
- `GC/button/button.rs:457` `pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self`
- `GC/button/button.rs:48` `pub trait ButtonVariants`：`:52 primary` `:57 secondary` `:62 danger` `:67 warning` `:72 success` `:77 info` `:82 ghost` `:87 link` `:92 text`
- `GC/button/button.rs:146` `pub enum ButtonVariant`
- `GC/sizing.rs:6` `pub enum Size { Size(Pixels), XSmall, Small, Medium, Large }`；`:190-221` `Sizable { with_size(impl Into<Size>), xsmall(), small(), large() }`；`:572 impl Sizable for Button`
- `GB/component_traits.rs:40` `fn disabled(mut self, disabled: bool) -> Self`（Disableable）
- `GP/app/context.rs:246` `pub fn listener<E: ?Sized>(&self, listener: impl Fn(&mut T, &E, &mut Window, &mut Context<T>) + 'static) -> impl Fn(&E, &mut Window, &mut App) + 'static` ★首选

⚠ `.on_click` 第三参是 `&mut App`（不是 `Context<Self>`）：用 `cx.listener(...)`，或 `let this = cx.entity()` + `this.update(cx, ..)`。

```rust
use gpui_kit::*;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{h_flex, Disableable as _, Icon, Sizable as _, WindowExt as _};

h_flex().gap_2()
    .child(Button::new("rescan").label("重新扫描").primary().small()
        // (&mut DiskView, &ClickEvent, &mut Window, &mut Context<DiskView>)
        .on_click(cx.listener(|this, _ev, window, cx| {
            this.start_scan("C:\\".into(), cx);
            window.push_notification(Notification::info("开始扫描"), cx);
        })))
    .child(Button::new("stop").icon(Icon::new(IconName::Search)).ghost().disabled(true));
```
`IconName` 由宏生成（`GC/icon.rs:38`），可确认变体 `IconName::Search`（`GC/icon.rs:285` 测试里 `Icon::new(IconName::Search)`）。不确认名字就用 `Icon::empty().path("icons/foo.svg")`（`GC/icon.rs:117`）。

---

## 5. 面包屑

**use** `use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};`

**签名**
- `GC/breadcrumb.rs:31 new(label: impl Into<SharedString>)` · `:42 disabled(bool)` · `:47 on_click(impl Fn(&ClickEvent,&mut Window,&mut App) + 'static)`（BreadcrumbItem）
- `GC/breadcrumb.rs:119 Breadcrumb::new()` · `:127 child(impl Into<BreadcrumbItem>)` · `:133 children(IntoIterator<Item = impl Into<BreadcrumbItem>>)`
- `:73/79/85` `impl From<&'static str / String / SharedString> for BreadcrumbItem`；末项自动用 `foreground`，其余 `muted_foreground`（`:101-102`）

```rust
use gpui_kit::*;
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};

Breadcrumb::new()
    .child(BreadcrumbItem::new("C:").on_click(|_, _, _| { /* 回根 */ }))
    .child(BreadcrumbItem::new("Users").on_click(|_, _, _| {}))
    .child("alice")   // 末项=当前目录，可省 on_click
```

---

## 6. 对话框 / 删除确认

**use** `use gpui_kit::component::dialog::{Dialog, DialogButtonProps}; use gpui_kit::component::{WindowExt, button::ButtonVariant};`（`AlertDialog` 也在 `dialog` 下）

**签名**
- `GC/dialog/dialog.rs:351 Dialog::new(cx: &mut App)` · `:387 title(impl IntoElement)` · `:406 footer(impl IntoElement)` · `:422 button_props(DialogButtonProps)` · `:483 w(impl Into<Pixels>)`（默认 448px）· `:547 impl ParentElement for Dialog`（→ `.child()`）
- `:447 on_ok(impl Fn(&ClickEvent,&mut Window,&mut App) -> bool + 'static)` · `:458 on_cancel(同)` · `:436 on_close(-> ())`
- `:47 DialogButtonProps`：`:60 ok_text` `:66 ok_variant(ButtonVariant)` `:72 cancel_text` `:78 cancel_variant` `:84 show_cancel(bool)` `:92 on_ok` `:103 on_cancel`
- `GC/dialog/alert_dialog.rs:80 AlertDialog::new(cx)` · `:96 confirm()` · `:169 title` · `:177 description` · `:240 show_cancel` · `:296 on_ok`
- `GC/window_ext.rs:75 open_dialog(cx, build)`；⚠ builder 是**渲染闭包**（可能重复执行），`InputState` 之类持久实体要在 `open_dialog` 之前建好（`GC/window_ext.rs:40-56`）

```rust
use gpui_kit::*;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::WindowExt as _;

fn confirm_delete(&mut self, target: String, window: &mut Window, cx: &mut Context<Self>) {
    let this = cx.entity();                 // Context<Self> 才有 entity()（Deref→App：GP/app/context.rs:26）
    window.open_dialog(cx, move |dialog, _w, _cx| {
        let this = this.clone();            // 外层闭包是 Fn，Entity 必须在这里克隆
        dialog.title("删除确认")
            .child(format!("确定删除 “{target}”？不可撤销。"))
            .button_props(DialogButtonProps::default().show_cancel(true)
                .ok_text("删除").ok_variant(ButtonVariant::Danger).cancel_text("取消"))
            .on_ok(move |_, _, cx| { this.update(cx, |this, cx| this.delete_now(cx)); true })
    });   // on_ok 返回 true=关闭，false=留在对话框
}
```

---

## 7. 右键菜单 / 下拉菜单

**use** `use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem, DropdownMenu};`

**签名**
- `GC/menu/context_menu.rs:17` `pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled`
- `:23` `fn context_menu(mut self, f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static) -> ContextMenu<Self>`
- `:113` `impl<E: ..> IntoElement for ContextMenu<E>`（返回可直接当元素）
- `GC/menu/popup_menu.rs:71 PopupMenuItem::new(impl Into<SharedString>)` · `:196 on_click<F>(handler: F) where F: Fn(&ClickEvent,&mut Window,&mut App) + 'static` · `:113 separator()` · `:126 icon(impl Into<Icon>)` · `:161 disabled(bool)`
- `GC/menu/popup_menu.rs:730 PopupMenu::item(impl Into<PopupMenuItem>)` · `:682 separator()` · `:467 menu(label, Box<dyn Action>)` · `:358 build(window, cx, f) -> Entity<Self>`
- `GC/menu/dropdown_menu.rs:12` `pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static`；`:14 dropdown_menu(f)`；`:33 impl DropdownMenu for Button`

```rust
use gpui_kit::*;
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};

let this = cx.entity();
h_flex().id("row-C:\\Users").px_3().py_2().child("C:\\Users")
    .context_menu(move |menu, _w, _cx| {
        let this = this.clone();     // 闭包是 Fn
        menu.item(PopupMenuItem::new("打开").on_click(move |_, _, cx| {
                this.update(cx, |this, cx| this.enter_dir("C:\\Users", cx));
            }))
            .separator()
            .item(PopupMenuItem::new("删除").on_click(|_, _, _| {}))
            .item(PopupMenuItem::new("属性").disabled(true))
    });
// 下拉：Button::new("more").label("更多").dropdown_menu(|menu, _, _| menu.item(PopupMenuItem::new("导出 CSV")))
```

---

## 8. Tooltip

**use** `use gpui_kit::component::tooltip::Tooltip; use gpui_kit::StatefulInteractiveElement as _;`

**签名**
- `GC/tooltip.rs:48 Tooltip::new(text: impl Into<Text>)`（`Text: From<&str>/String/SharedString`：`GC/text/compat.rs:419-433`）· `:58 element(builder)` · `:86 build(self, _: &mut Window, cx: &mut App) -> AnyView`
- `GP/elements/div.rs:1682` `fn tooltip(self, build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static) -> Self`（在 `StatefulInteractiveElement`，`GP/elements/div.rs:1306` → **元素必须先 `.id()`**）
- `GC/attachment.rs:491` `.tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))` ← 0.8.0 真实用法
- 组件内部 `ManagedTooltipExt` 是 `pub(crate)`（`GC/tooltip.rs:246`），外部用不了

```rust
div().id("usage-bar").child(Progress::new("p").value(61.0))
    .tooltip(|window, cx| Tooltip::new("已用 61%").build(window, cx));
```

---

## 9. 进度 / 加载

**use** `use gpui_kit::component::{Progress, ProgressCircle, Spinner};`（crate 根导出：`GC/progress/mod.rs:4-5`）

**签名**
- `GC/progress/progress.rs:26 Progress::new(impl Into<ElementId>)` · `:56 value(f32)` ⚠ **0.0..=100.0**（内部 clamp）· `:42 loading(bool)`（true=不确定态，忽略 value）· `:48 color(impl Into<Hsla>)`
- `GC/progress/progress_circle.rs:30 new(..)` · `:61 value(f32)`
- `GC/spinner.rs:20 Spinner::new()` · `:35 icon(impl Into<Icon>)` · `:41 color(Hsla)`
- 三者都 `impl Sizable`（可 `.small()`/`.large()`）

```rust
v_flex().gap_2()
    .child(Progress::new("scan-progress").value(37.0).h(px(6.)).w_full())
    .child(Progress::new("indeterminate").loading(true).w_full())
    .child(h_flex().gap_2().child(Spinner::new().small()).child("正在扫描…"));
```

---

## 10. 主题

**use** `use gpui_kit::component::theme::{Theme, ThemeMode}; use gpui_kit::component::ActiveTheme;`

**签名**
- `GC/theme/mod.rs:44` `pub trait ActiveTheme { fn theme(&self) -> &Theme; }`；`:48 impl ActiveTheme for App`（`cx.theme()`，`Context` Deref 到 App）
- `GC/theme/mod.rs:147` `pub struct Theme { :148 colors: ThemeColor, :159 mode, :166 font_family: SharedString, :168 font_size: Pixels, :181 mono_font_family, :185 radius }`
- `:220` `impl Deref for Theme { type Target = ThemeColor; }` ← 所以能直接写 `theme.background`
- `:239 Theme::global(cx: &App) -> &Theme` · `:251 global_mut(cx)` · `:288 update<R>(cx, edit)` · `:389 change(mode: impl Into<ThemeMode>, _window: Option<&mut Window>, cx)`
- `:807 pub enum ThemeMode { Light, Dark }`
- `GC/theme/theme_color.rs:59 ThemeColor`：`:67 background` `:165 foreground` `:69 border` `:195 muted` `:197 muted_foreground` `:203 primary` `:221 secondary` `:149 danger` `:199 popover` `:213 ring` `:229 selection` `:231 sidebar` `:295 title_bar` `:175 input` `:177 link`

```rust
use gpui_kit::*;
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};

fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.theme();  // == Theme::global(cx)
    v_flex().size_full().bg(t.background).text_color(t.foreground)
        .child(div().border_1().border_color(t.border).child("card"))
}
Theme::change(ThemeMode::Dark, Some(window), cx); // 切主题
```

---

## 11. CJK 字体注册 + font_family

**use** `use std::borrow::Cow;`（`App::text_system` 自带）

**签名**
- `GP/text_system.rs:295` `pub fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()>` ⚠ **既不是 `Vec<Arc<str>>` 也不是 `Vec<&[u8]>`**；`Result` 是 `anyhow::Result`（`GP/gpui.rs:101 pub use anyhow::Result;`）
- `GP/text_system.rs:284` `pub fn all_font_names(&self) -> Vec<String>`（验证是否装上）
- `GP/app.rs:2340` `pub fn text_system(&self) -> &Arc<TextSystem>`（在 `GP/app.rs:918 impl App` 内）
- `GP/style.rs:443` `pub font_family: SharedString`（`TextStyle`，`:438` 起）· `GP/styled.rs:708` `fn font_family(mut self, family_name: impl Into<SharedString>) -> Self`
- `GC/theme/mod.rs:166` `pub font_family: SharedString`（全局 UI 字体）· `:181 mono_font_family`

```rust
use std::borrow::Cow;

gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    static ZH: &[u8] = include_bytes!("../assets/NotoSansSC-Regular.otf");
    cx.text_system().add_fonts(vec![Cow::Borrowed(ZH)]).expect("install CJK font");
    gpui_kit::component::theme::Theme::global_mut(cx).font_family = "Noto Sans SC".into();
    gpui_kit::open_window(WindowOptions::default(), cx, |w, cx| cx.new(|cx| DiskView::new(w, cx))).unwrap();
});
// 局部：div().font_family("Noto Sans SC").child("磁盘 使用率")
```

---

## 12. 文本 / 标题样式

**use** `use gpui_kit::component::label::Label; use gpui_kit::component::{Sizable, StyledExt, h_flex, v_flex};`（`GC/styled.rs:2,7` 均 crate 根）

**签名**
- `GC/label.rs:63 Label::new(impl Into<SharedString>)` · `:76 secondary(impl Into<SharedString>)` · `:82 masked(bool)` · `:88 highlights(impl Into<HighlightsMatch>)`
- `GB/styled.rs:78` `pub trait StyledExt: Styled + Sized`：`:105 h_flex()`、`:113 v_flex()`、`:192 font_semibold`（`:187-195` 宏生成 thin…black）
- `GP/styled.rs:545/552/566/573` `fn text_xs/text_sm/text_lg/text_xl`；`:514 text_color(impl Into<Hsla>)`；`:538 text_size(impl Into<AbsoluteLength>)`；`:740 line_height(impl Into<DefiniteLength>)`
- `GP/elements/div.rs:1590` `fn on_click(mut self, listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self`
- `GC/title_bar.rs:16` `pub const TITLE_BAR_HEIGHT: Pixels = px(34.);`；`:43 TitleBar`（`:51 new()`、`:60 title_bar_options()`、`:82 window_options()`）；`:299/305 impl Styled/ParentElement`

```rust
v_flex().gap_1()
    .child(TitleBar::new().child("SpaceSnifferX"))
    .child(Label::new("磁盘 使用率").text_lg().font_semibold())
    .child(Label::new("C:\\").secondary("已用 128 GB / 512 GB").text_sm()
        .text_color(cx.theme().muted_foreground))
    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("扫描耗时 3.2s"))
```

---

## 13. 迁移自检

1. `Entity<InputState>` 只在 `new(window, cx)` 里创建；`render` 里只 `Input::new(&self.state)`。
2. 所有组件 `.on_click`/`.on_ok` 的 `cx` 都是 `&mut App`：用 `cx.listener(..)` 或 `cx.entity()` + `update`。
3. `Progress::value` 用 0..=100；`Spinner::new()` 无参。
4. dialog/notification/tooltip/menu 需要 `gpui_kit::init(cx)` + 窗口根是 `Root`（`open_window` 自动给）。
5. `.id()` 是 `.tooltip()`/`.context_menu()` 稳定的前提（tooltip 强制要求）。
