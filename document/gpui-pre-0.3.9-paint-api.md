# gpui-pre 0.3.9 (longbridge fork) — 自绘 / 命中测试 API

源码：`...\index.crates.io-1949cf8c6b5b557f\gpui-pre-0.3.9\src\`（行号均已核对）。根重导出见 `gpui.rs:110-215`：`elements::*` / `window::*` / `geometry::*` / `text_system::*` / `path_builder::*` / `style::*` / `scene::*`。

## 0. 先纠正三个前提

| 假设 | 源码事实 |
|---|---|
| `paint` 闭包 `(&mut Window, &mut App)` | **错**，是 `(Bounds<Pixels>, T, &mut Window, &mut App)` — `canvas.rs:12` |
| 有 `window.paint_line(...)` | **不存在**。只有 `text_system/line.rs:521` 的私有 fn；画文本用 `ShapedLine::paint(...)` — `line.rs:108` |
| `hover(\|style, window, cx\|)` | **错**，只 1 个参数 `FnOnce(StyleRefinement) -> StyleRefinement` — `div.rs:842` |

`shape_line` 挂在 **`WindowTextSystem`**（`text_system.rs:608` 的 impl，方法 638 行），必须 `window.text_system()`；`cx.text_system()` 返回 `&Arc<TextSystem>`（`app.rs:2340`），**没有** `shape_line`。

```rust
use gpui::prelude::*; // InteractiveElement, Styled, Context, Render, ParentElement
use gpui::{canvas, div, px, point, size, fill, outline, quad, rgb, font,
           Bounds, Pixels, Point, Window, App, MouseButton, SharedString,
           PathBuilder, TextRun, TextAlign, BorderStyle, ContentMask, DispatchPhase};
```

## 1. `canvas(prepaint, paint)`

【用途】低层绘制区：`prepaint` 测量/缓存，`paint` 往 scene 塞 primitive。
【签名】`elements/canvas.rs:10-19`
```rust
pub fn canvas<T>(
    prepaint: impl 'static + FnOnce(Bounds<Pixels>, &mut Window, &mut App) -> T,
    paint:     impl 'static + FnOnce(Bounds<Pixels>, T, &mut Window, &mut App),
) -> Canvas<T>
```
- `T` 存为 `Element::PrepaintState = Option<T>`（`canvas.rs:39`），原样传入 `paint` 第 2 参（`canvas.rs:71`、`84-87`）。
- `Canvas<T>: Styled`（`canvas.rs:91`）→ `.size_full()` / `.absolute()` / `.inset_0()` 可用。这些由 `gpui_macros` 生成：`absolute()` 见 `gpui-pre-macros-0.3.9/src/styles.rs:116-118`，`_full`（= `relative(1.)`）见 `styles.rs:1089-1091`，宏展开于 `styled.rs:26-34`。

【示例】
```rust
div().size_full().relative().child(
    canvas(
        move |bounds: Bounds<Pixels>, _w, _cx| bounds,      // T = Bounds<Pixels>
        move |bounds, _cached, window, cx| {
            let _ = cx;
            window.paint_quad(fill(bounds, rgb(0x202020)));
        },
    )
    .absolute().inset_0().size_full(),                       // 铺满父容器
)
```
> 实例：`gpui-component-0.8.0/src/popover.rs:363-401`（`.absolute().inset_0().size_full()`）、`separator.rs:92-115`。

## 2. `paint_quad` / `PaintQuad` / `fill` / `outline` / `quad`

【用途】实心/描边矩形。**treemap 每格用这个**，零 tessellation。
【签名】
```rust
pub fn paint_quad(&mut self, quad: PaintQuad)                       // window.rs:4684

pub struct PaintQuad {                                              // window.rs:7763-7776
    pub bounds: Bounds<Pixels>, pub corner_radii: Corners<Pixels>,
    pub background: Background, pub border_widths: Edges<Pixels>,
    pub border_color: Hsla, pub border_style: BorderStyle,
}
pub fn quad(bounds: Bounds<Pixels>,                                 // window.rs:7813（6 参数）
    corner_radii: impl Into<Corners<Pixels>>, background: impl Into<Background>,
    border_widths: impl Into<Edges<Pixels>>, border_color: impl Into<Hsla>,
    border_style: BorderStyle) -> PaintQuad
pub fn fill(bounds: impl Into<Bounds<Pixels>>, background: impl Into<Background>) -> PaintQuad   // :7832
pub fn outline(bounds: impl Into<Bounds<Pixels>>, border_color: impl Into<Hsla>,
    border_style: BorderStyle) -> PaintQuad                         // :7844，固定 1px 边框+透明背景
```
链式 builder：`.corner_radii()` / `.border_widths()` / `.border_color()` / `.background()`（`window.rs:7780-7809`）。

配套：`Corners<T>` `geometry.rs:2270`，`Corners::all` `:2309`，`From<Pixels> for Corners<Pixels>` `:2579`；`Edges<T>` `:1753`，`Edges::all` `:1820`，`From<Pixels> for Edges<Pixels>` `:2155`；`BorderStyle::{Solid,Dashed}` `scene.rs:597-603`。

【示例】
```rust
window.paint_quad(fill(rect, bg));                                       // 纯填充
window.paint_quad(outline(rect, border, BorderStyle::Solid));            // 1px 描边
window.paint_quad(quad(rect, px(2.), bg, px(1.), border, BorderStyle::Solid)); // 填充+描边+圆角，1 个 primitive
window.paint_quad(fill(rect, bg).corner_radii(px(2.)));                  // builder 写法
```
> `paint_quad` 对 bounds/border_widths 做 snap（`window.rs:4688-4689`），纯边框 quad 会拆成 4 条 strip 避免整块透明着色（`window.rs:4706-4749`）——上万矩形友好。

## 3. `paint_path` / `PathBuilder`

【用途】多边形/折线/高亮描边。
【签名】
```rust
pub fn paint_path(&mut self, mut path: Path<Pixels>, color: impl Into<Background>)  // window.rs:4755
// path_builder.rs:25
pub fn stroke(width: Pixels) -> Self        // :88      pub fn fill() -> Self          // :96
pub fn dash_array(self, &[Pixels]) -> Self  // :108     pub fn move_to(&mut self, to: Point<Pixels>)  // :125
pub fn line_to(&mut self, to: Point<Pixels>)// :131     pub fn curve_to(&mut self, to: Point<Pixels>, ctrl: Point<Pixels>) // :137 ← (终点, 控制点)
pub fn close(&mut self)                     // :199     pub fn build(self) -> Result<Path<Pixels>, Error> // :244
```
【示例】抄自 `separator.rs:92-113`
```rust
canvas(move |_, _, _| {}, move |bounds, _, window, _| {
    let mut b = PathBuilder::stroke(px(1.)).dash_array(&[px(4.), px(2.)]);
    b.move_to(point(bounds.origin.x, bounds.origin.y));
    b.line_to(point(bounds.origin.x + bounds.size.width, bounds.origin.y));
    if let Ok(line) = b.build() { window.paint_path(line, rgb(0x555555)); }
})
```
填充多边形（`popover.rs:373-380`）：`PathBuilder::fill()` → `move_to` → `line_to`×n → `close()` → `build()`。

## 4. canvas 里画文本

【用途】每格标签。**没有** `window.paint_line`；流程 = shape 一次 → paint 一次。
【签名】
```rust
pub fn text_system(&self) -> &Arc<WindowTextSystem>                 // window.rs:2462
pub fn shape_line(&self, text: SharedString, font_size: Pixels,     // text_system.rs:638-644
    runs: &[TextRun], force_width: Option<Pixels>) -> ShapedLine
pub struct TextRun { pub len: usize, pub font: Font, pub color: Hsla,   // text_system.rs:1228-1241
    pub background_color: Option<Hsla>, pub underline: Option<UnderlineStyle>,
    pub strikethrough: Option<StrikethroughStyle> }
pub struct Font { family, features, fallbacks, weight, style }      // text_system.rs:1292-1309
pub fn font(family: impl Into<SharedString>) -> Font                // :1318（默认 ".SystemUIFont"）
pub fn paint(&self, origin: Point<Pixels>, line_height: Pixels,     // line.rs:108-116
    align: TextAlign, align_width: Option<Pixels>, window, cx) -> Result<()>
pub fn width(&self) -> Pixels    // line.rs:88      pub fn ascent(&self) -> Pixels   // line_layout.rs:321
pub fn size(&self, line_height: Pixels) -> Size<Pixels>             // line_layout.rs:313
pub enum TextAlign { Left, Center, Right }                          // style.rs:423-433（Left 默认）
pub fn with_content_mask<R>(&mut self, mask: Option<ContentMask<Pixels>>, f) -> R  // window.rs:4163 ← 裁剪
```
【示例】居中 + 裁剪
```rust
canvas(
    move |bounds: Bounds<Pixels>, window: &mut Window, _cx: &mut App| {
        let text: SharedString = "src/main.rs".into();
        let line_height = px(14.);
        let run = TextRun {
            len: text.len(), font: font(".SystemUIFont"), color: rgb(0xdddddd).into(),
            background_color: None, underline: None, strikethrough: None,
        };
        let line = window.text_system().shape_line(text, px(11.), &[run], None);
        (line, bounds, line_height)                       // T
    },
    move |_b, (line, bounds, line_height), window, cx| {
        let y = bounds.origin.y.as_f32()                  // 垂直居中
              + (bounds.size.height.as_f32() - line_height.as_f32()) * 0.5;
        window.with_content_mask(Some(ContentMask { bounds }), |window| {   // 裁剪到格子内
            line.paint(point(bounds.origin.x, px(y)), line_height,
                       TextAlign::Center, Some(bounds.size.width), window, cx).ok();
        });
    },
)
```
> 真实 `TextRun` 构造：`gpui-component-0.8.0/src/diff/mod.rs:832-856`。`shape_line` 有 `debug_assert!` 禁止 `\n`（`text_system.rs:645-648`），多行用 `shape_text`（`:750`）；要省缓存开销用 `shape_line_by_hash`（`:689`）。

## 5. 命中测试：右键落在哪个矩形

【用途】canvas 只有一块 element，无 per-rect hitbox → 自己把窗口坐标转局部坐标再查索引。
【签名】
```rust
pub fn mouse_position(&self) -> Point<Pixels>                        // window.rs:3362
pub fn on_mouse_event<Event: MouseEvent>(&mut self,                  // window.rs:5432
    listener: impl FnMut(&Event, DispatchPhase, &mut Window, &mut App) + 'static)
pub struct MouseDownEvent { button: MouseButton, position: Point<Pixels>,  // interactive.rs:148-163
                            modifiers, click_count: usize, first_mouse: bool }
pub struct MouseMoveEvent { position: Point<Pixels>, pressed_button, modifiers } // interactive.rs:494-503
pub enum MouseButton { Left, Right, Middle, Navigate(..) }           // interactive.rs:453-466
fn on_mouse_down(self, MouseButton, impl Fn(&MouseDownEvent, &mut Window, &mut App)) -> Self // div.rs:869
fn on_click(...) -> Self  // div.rs:1590（参数 &ClickEvent，enum 见 interactive.rs:290）
fn id(self, impl Into<ElementId>) -> Stateful<Self>                  // div.rs:779
pub fn listener<E: ?Sized>(&self, impl Fn(&mut T, &E, &mut Window, &mut Context<T>)) -> impl Fn(&E, ...) // context.rs:246
pub fn contains(&self, point: &Point<T>) -> bool                     // geometry.rs:1468（左上闭/右下开）
pub fn position(&self) -> Option<Point<Pixels>>                      // ClickEvent，interactive.rs:343-345
```
【推荐 A：外层 div 挂监听】
```rust
let cb = Rc::new(Cell::new(None::<Bounds<Pixels>>));   // prepaint 里填入
let cb2 = cb.clone();
div().size_full().relative()
    .on_mouse_down(MouseButton::Right, cx.listener(
        move |this: &mut Self, ev: &MouseDownEvent, _w, cx| {
            let Some(b) = cb.get() else { return };
            let local = ev.position - b.origin;        // 窗口坐标 → 局部坐标
            this.selected = this.hit_test(local);      // 你的 treemap 下钻
            cx.notify();
        }))
    .child(canvas(move |bounds, _w, _cx| cb2.set(Some(bounds)), move |..| { }).absolute().inset_0().size_full())
```
【推荐 B：canvas paint 里注册窗口监听】
```rust
let rects = self.rects.clone();                        // Rc<Vec<(Bounds<Pixels>, usize)>>
window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, _cx| {
    if phase == DispatchPhase::Bubble && bounds.contains(&ev.position) {
        let local = ev.position - bounds.origin;
        // 存 selected → 重绘
        window.refresh();                              // window.rs:2376
    }
});
```
> `on_mouse_event` 的监听**只活一帧**（`window.rs:5427-5429`），必须每帧 paint 重注册 —— 这是 gpui 自绘组件标准做法，真实模式见 `elements/div.rs:2789-2815`、`elements/text.rs:1189-1222`（`phase == DispatchPhase::Bubble && hitbox.is_hovered(window)`）。`DispatchPhase::Bubble` 见 `window.rs:94-100`。
> 执行式 `Div::on_mouse_down` 在 `div.rs:126`，返回 `()`，不能链式。

## 6. hover 高亮

【用途】自绘 canvas 的 hover 必须自己存 index 再换色 —— `hover()` 只改 div 样式，改不了你 paint 的像素（`div.rs:842` 只有 1 个参数，拿不到 window/cx）。
【签名】
```rust
fn hover(self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self      // div.rs:842
fn on_hover(self, impl Fn(&bool, &mut Window, &mut App)) -> Self               // div.rs:1661
pub fn request_animation_frame(&self)          // window.rs:2730（内部 cx.notify(current_view)）
pub fn refresh(&mut self)                      // window.rs:2376（标 dirty）
pub fn on_next_frame(&self, impl FnOnce(&mut Window, &mut App))  // window.rs:2710
```
【示例】
```rust
let hovered = self.hovered.clone();   // Rc<Cell<Option<usize>>>
let rects = self.rects.clone();
window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, _cx| {
    if phase != DispatchPhase::Bubble || !bounds.contains(&ev.position) { return }
    let local = ev.position - bounds.origin;
    let new = rects.iter().find(|(b, _)| b.contains(&local)).map(|(_, id)| *id);
    if hovered.get() != new {                  // 只在变化时重绘
        hovered.set(new);
        window.request_animation_frame();
    }
});
```
【性能要点】
1. 只在 hovered 真变化时请求重绘（对照 `text.rs:1233-1236` 的 `if current != updated`），否则每次 move 都整帧重绘。
2. 命中别线性扫上万矩形：treemap 是嵌套的，递归 `contains` 下钻 O(depth)，或排序后二分。`Bounds::contains` 只 4 次比较（`geometry.rs:1468-1473`）。
3. hover 时**别重新 shape_line**：缓存 `ShapedLine`，或用 `shape_line_by_hash`（`text_system.rs:689`）。
4. 别写 `loop { request_animation_frame() }`；`window.rs:2725-2729` 建议装饰性动画检查 `App::reduce_motion`。

## 7. 持续重绘（100ms 轮询扫描进度）

【签名】
```rust
pub fn spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R>               // context.rs:230-238
  where AsyncFn: AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R + 'static
pub fn background_executor(&self) -> &BackgroundExecutor            // async_context.rs:159
pub fn timer(&self, duration: Duration) -> Task<()>                 // executor.rs:183（可 .await）
pub fn update<C: AppContext, R>(&self, cx: &mut C, impl FnOnce(&mut T, &mut Context<T>) -> R) -> Result<R> // entity_map.rs:803
pub fn notify(&mut self)                                            // context.rs:221
```
【骨架】
```rust
use std::time::Duration;
struct ScanView { scanned: usize, total: usize, poll_task: Option<Task<()>> } // 必须持有，否则 future 被 drop

impl ScanView {
    fn start_polling(&mut self, cx: &mut Context<Self>, state: Arc<Mutex<ScanState>>) {
        self.poll_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                let (scanned, total) = { let s = state.lock().unwrap(); (s.scanned, s.total) };
                let alive = this.update(cx, |this, cx| {
                    if this.scanned == scanned && this.total == total { return; } // 没变不 notify
                    this.scanned = scanned; this.total = total;
                    cx.notify();
                });
                if alive.is_err() { break; }     // 视图已释放
            }
        }));
    }
}
```
要点：`cx.background_executor()` 来自 `AsyncApp`（`async_context.rs:159`），别跨 await 持有 `&mut Self`；`this.update` 返回 `Result`，`Err` = 实体已释放 → break；变化判断放 `update` 内避免空 notify。

## 8. 几何类型确认

```rust
pub const fn px(pixels: f32) -> Pixels                    // geometry.rs:3748
pub const fn point<T>(x: T, y: T) -> Point<T>             // :111
pub const fn size<T>(w: T, h: T) -> Size<T>               // :425
pub struct Point<T> { pub x: T, pub y: T }                // :85-90
pub struct Bounds<T> { pub origin: Point<T>, pub size: Size<T> }   // :723-727
pub fn new(origin: Point<T>, size: Size<T>) -> Self       // :785
pub fn from_corners(top_left: Point<T>, bottom_right: Point<T>) -> Self  // :821
pub fn center(&self) -> Point<T>                          // :1001
pub fn contains(&self, point: &Point<T>) -> bool          // :1468
impl<T> Add<Point<T>> for Bounds<T>                       // :1250
pub fn top/bottom/left/right(&self) -> T                  // :1296/:1305/:1314/:1323
```
- `Point` 的 `Add/AddAssign/Sub/SubAssign/Neg` 是 **derive** 的（`geometry.rs:66-82`），能用：`ev.position - bounds.origin`、`bounds.origin + point(px(4.), px(0.))`。
- 字段：`bounds.origin.x/.y`、`bounds.size.width/.height`。
- `Pixels` ↔ `f32`：`.as_f32()`、`Pixels::MAX`。

## 附：treemap 绘制核心循环

```rust
window.paint_quad(fill(rect, bg));                                    // 填色，零 tessellation
if rect.size.width > px(2.) {
    window.paint_quad(outline(rect, border, BorderStyle::Solid));     // 1px 描边
}
if rect.size.height > px(16.) && rect.size.width > px(40.) {          // 放得下才画字
    line.paint(point(rect.origin.x, y), line_height, TextAlign::Left,
               Some(rect.size.width), window, cx).ok();
}
```
优先级：`paint_quad` >> `paint_path`（要 tessellate）；文本只在格子够大时画且复用 `ShapedLine`。
