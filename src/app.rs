//! 应用状态与导航逻辑。
//!
//! 这个文件只关心「状态怎么变」，界面长什么样交给 `views` 下的模块。
//! 每个视图函数都是 `impl SpaceSnifferApp` 的方法，返回 `AnyElement`，
//! 方便在 `Render::render` 里顺序组合。

use std::{
    cell::{Cell, RefCell},
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use crate::gpui::{
    self, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity, Focusable as _,
    MouseMoveEvent, ParentElement as _, Pixels, Point, Render, Styled as _, Subscription, Task,
    Window,
};
use gpui_kit::component::{
    button::ButtonVariant, input::InputState, ActiveTheme as _, WindowExt as _,
};
use rust_i18n::t;

use crate::scanner::{self, FileEntry, ScanPhase, ScanProgress};
use crate::util;
use crate::views;

/// 跟随指针时最短的重绘间隔（40Hz）。
const HOVER_REDRAW_INTERVAL: Duration = Duration::from_millis(25);

/// 当前节点下所有子项的布局矩形（窗口坐标）。
pub type CellLayout = Rc<Vec<(usize, Bounds<Pixels>)>>;

pub struct SpaceSnifferApp {
    /// 路径输入框的状态实体。
    pub(crate) scan_path: Entity<InputState>,

    /// 与扫描线程共享的进度。
    pub(crate) scan_progress: Arc<Mutex<ScanProgress>>,
    pub(crate) root: Option<Arc<FileEntry>>,
    pub(crate) error_message: Option<String>,

    /// 从根节点一路下钻的下标栈，以及对应的显示名。
    pub(crate) nav_stack: Vec<usize>,
    pub(crate) nav_names: Vec<String>,

    /// 当前悬停的格子（`current_node().children` 的下标）及其绝对路径。
    ///
    /// 路径用来在树被扫描快照整体替换后，把悬停状态接回同一个条目——
    /// 只靠下标的话，指针不动时信息卡会莫名消失。
    pub(crate) hovered_cell: Option<usize>,
    pub(crate) hovered_path: Option<PathBuf>,
    /// 指针在画布内的窗口坐标：信息卡跟着它走。
    pub(crate) hover_pos: Option<Point<Pixels>>,
    /// 上次因为「指针移动」而重绘的时间，用来给跟随重绘限频。
    last_hover_redraw: Instant,

    /// 画布在本帧的实际边界，由 treemap 的 prepaint 回填。
    pub(crate) canvas_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// 本帧算出的格子布局，供命中测试复用。
    pub(crate) cell_layout: Rc<RefCell<CellLayout>>,

    /// 扫描统计缓存：每帧被 3 个视图读到，不要每次去抢扫描线程的锁。
    pub(crate) stats: ScanStats,

    /// 右键菜单（gpui-kit 的 PopupMenu 实体）及其锚点。
    pub(crate) context_menu: Option<Entity<gpui_kit::component::menu::PopupMenu>>,
    pub(crate) context_pos: Point<Pixels>,

    /// 用户主动取消扫描后，忽略扫描线程回写的"取消"错误。
    cancel_requested: bool,

    poll_task: Option<Task<()>>,
    _input_subscription: Option<Subscription>,
    _menu_subscription: Option<Subscription>,
}

impl SpaceSnifferApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 支持 `spacesniffer-rs [PATH]`：带路径启动就直接开扫。
        let cli_path = initial_path_from_args();
        let initial_path = cli_path.clone().unwrap_or_else(util::dirs_or_home);
        let scan_path = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("path_hint").to_string())
                .default_value(initial_path)
                .submit_on_enter(true)
                .clean_on_escape()
        });

        let mut this = Self {
            scan_path: scan_path.clone(),
            scan_progress: new_progress(),
            root: None,
            error_message: None,
            nav_stack: Vec::new(),
            nav_names: Vec::new(),
            hovered_cell: None,
            hovered_path: None,
            hover_pos: None,
            last_hover_redraw: Instant::now(),
            canvas_bounds: Rc::new(Cell::new(None)),
            cell_layout: Rc::new(RefCell::new(Rc::new(Vec::new()))),
            stats: ScanStats::default(),
            context_menu: None,
            context_pos: Point::default(),
            cancel_requested: false,
            poll_task: None,
            _input_subscription: None,
            _menu_subscription: None,
        };

        // 输入框里按下回车 = 开始扫描。
        this._input_subscription = Some(cx.subscribe_in(
            &scan_path,
            window,
            |this, _, event: &gpui_kit::component::input::InputEvent, window, cx| {
                if matches!(
                    event,
                    gpui_kit::component::input::InputEvent::PressEnter { .. }
                ) {
                    this.start_scan(window, cx);
                }
            },
        ));

        // 跟随系统的浅色/深色外观。
        gpui_kit::component::theme::Theme::sync_system_appearance(Some(window), cx);

        if cli_path.is_some() {
            this.start_scan(window, cx);
        }

        this
    }

    // ---------------------------------------------------------------- 扫描

    pub(crate) fn is_scanning(&self) -> bool {
        self.stats.scanning
    }

    /// 当前扫描统计。读的是缓存字段，不碰扫描线程的锁。
    pub(crate) fn scan_stats(&self) -> &ScanStats {
        &self.stats
    }

    pub(crate) fn start_scan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.scan_path.read(cx).value().to_string();
        let path_text = raw.trim().to_string();
        if path_text.is_empty() {
            return;
        }

        let path = PathBuf::from(&path_text);
        if !path.is_dir() {
            self.error_message = Some(t!("error_not_a_directory", path = &path_text).to_string());
            cx.notify();
            return;
        }

        // 归一化输入框内容，回填去掉首尾空格后的路径。
        if raw != path_text {
            self.scan_path.update(cx, |state, cx| {
                state.set_value(path_text.clone(), window, cx);
            });
        }

        self.cancel_scan();
        self.root = None;
        self.nav_stack.clear();
        self.nav_names.clear();
        self.hovered_cell = None;
        self.hovered_path = None;
        self.error_message = None;
        self.context_menu = None;
        self.cancel_requested = false;
        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());

        let progress = new_progress();
        self.scan_progress = progress.clone();
        scanner::start_scan(path, progress);

        // 扫描线程要过一会儿才会把 phase 写成 Scanning，
        // 这里先把状态置上，避免第一帧闪回欢迎页。
        self.stats = ScanStats {
            scanning: true,
            ..Default::default()
        };

        self.ensure_poller(cx);
        cx.notify();
    }

    pub(crate) fn cancel_scan(&mut self) {
        if let Ok(p) = self.scan_progress.lock() {
            p.cancelled.store(true, Ordering::Relaxed);
        }
    }

    /// 用户点击「取消」。
    pub(crate) fn cancel_scan_from_ui(&mut self, cx: &mut Context<Self>) {
        if !self.is_scanning() {
            return;
        }
        self.cancel_requested = true;
        self.cancel_scan();
        cx.notify();
    }

    /// 后台轮询扫描线程，把增量结果搬进视图状态。
    fn ensure_poller(&mut self, cx: &mut Context<Self>) {
        self.poll_task = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;

            let keep_polling = this
                .update(cx, |this, cx| {
                    let changed = this.sync_scan_state();
                    if changed {
                        cx.notify();
                    }
                    this.is_scanning()
                })
                .unwrap_or(false);

            if !keep_polling {
                break;
            }
        }));
    }

    /// 把扫描线程的增量/最终结果同步进来，返回是否有变化。
    ///
    /// 顺带刷新 [`Self::stats`]：整个应用只在这里抢一次扫描线程的锁，
    /// 渲染路径直接读缓存字段。
    fn sync_scan_state(&mut self) -> bool {
        let mut changed = false;

        let (partial, result, error) = {
            let mut p = match self.scan_progress.lock() {
                Ok(p) => p,
                Err(_) => return false,
            };

            let stats = ScanStats {
                scanning: p.phase == ScanPhase::Scanning,
                files: p.files_processed,
                dirs: p.dirs_processed,
                skipped: p.skipped_dirs,
                current_path: p.current_path.clone(),
            };
            if stats != self.stats {
                self.stats = stats;
                changed = true;
            }

            let partial = p.partial_root.take();
            let result = p.result.take();
            // 错误只消费一次，避免每帧都被判定为「有变化」。
            let error = if matches!(p.phase, ScanPhase::Error(_)) {
                match std::mem::replace(&mut p.phase, ScanPhase::Idle) {
                    ScanPhase::Error(msg) => Some(msg),
                    _ => None,
                }
            } else {
                None
            };
            (partial, result, error)
        };

        if let Some(partial) = partial {
            // 子项按大小排序，增量快照会让下标整体漂移，
            // 所以用当前节点的绝对路径把导航栈重新挂回去。
            let keep = self.current_node_path();
            if self.nav_names.is_empty() {
                self.nav_names.push(partial.name.clone());
            }
            self.root = Some(Arc::new(partial));
            if let Some(path) = keep {
                self.restore_navigation(&path);
            }
            changed = true;
        }

        if let Some(root) = result {
            let keep = self.current_node_path();
            if self.nav_names.is_empty() {
                self.nav_names.push(root.name.clone());
            }
            self.root = Some(root);
            match keep {
                Some(path) => self.restore_navigation(&path),
                None => self.restore_navigation_at_root(),
            }
            changed = true;
        }

        if let Some(msg) = error {
            if self.cancel_requested {
                // 用户自己取消的，不算错误。
                self.cancel_requested = false;
                self.error_message = None;
            } else {
                self.error_message = Some(msg);
            }
            changed = true;
        }

        changed
    }

    // ------------------------------------------------------------ 树与导航

    pub(crate) fn current_node(&self) -> Option<&FileEntry> {
        let root = self.root.as_ref()?;
        let mut node: &FileEntry = root;
        for &idx in &self.nav_stack {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    pub(crate) fn navigate_into(&mut self, child_index: usize, cx: &mut Context<Self>) {
        let Some(node) = self.current_node() else {
            return;
        };
        let Some(child) = node.children.get(child_index) else {
            return;
        };
        // 空目录也允许进入：进去后会显示「(空目录)」，点空白处返回上级。
        if !child.is_dir {
            return;
        }

        self.nav_names.push(child.name.clone());
        self.nav_stack.push(child_index);
        self.after_navigation(cx);
    }

    pub(crate) fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if self.nav_stack.pop().is_some() {
            self.nav_names.pop();
            self.after_navigation(cx);
        }
    }

    /// 跳到面包屑的第 `depth` 层（0 = 根）。
    pub(crate) fn navigate_to_level(&mut self, depth: usize, cx: &mut Context<Self>) {
        if depth >= self.nav_stack.len() {
            return;
        }
        self.nav_stack.truncate(depth);
        self.nav_names.truncate(depth + 1);
        self.after_navigation(cx);
    }

    fn after_navigation(&mut self, cx: &mut Context<Self>) {
        // 层级变了：旧层级的悬停没有意义，等下一次鼠标移动重新命中。
        self.hovered_cell = None;
        self.hovered_path = None;
        self.context_menu = None;
        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());
        cx.notify();
    }

    /// 当前节点的绝对路径，用来在树被替换/删除后重新定位。
    fn current_node_path(&self) -> Option<PathBuf> {
        self.current_node().map(|node| node.path.clone())
    }

    fn restore_navigation_at_root(&mut self) {
        self.nav_stack.clear();
        self.nav_names.truncate(1);
        self.hovered_cell = None;
        self.hovered_path = None;
        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());
    }

    /// 按绝对路径把导航栈接回新树上。
    ///
    /// 子项按大小排序，扫描增量或删除都会让下标整体漂移，
    /// 所以这里只认真实路径；路径不存在就停在最后一个有效层级。
    fn restore_navigation(&mut self, target: &Path) {
        let Some(root) = self.root.as_ref() else {
            self.restore_navigation_at_root();
            return;
        };

        let mut stack = Vec::new();
        let mut names = vec![root.name.clone()];
        let mut node: &FileEntry = root;
        let mut current = root.path.clone();

        while current != target {
            let Ok(relative) = target.strip_prefix(&current) else {
                break;
            };
            let Some(component) = relative.components().next() else {
                break;
            };
            let child_path = current.join(component);
            let Some(index) = node
                .children
                .iter()
                .position(|child| child.path == child_path)
            else {
                break;
            };

            stack.push(index);
            node = &node.children[index];
            names.push(node.name.clone());
            current = child_path;
        }

        self.nav_stack = stack;
        self.nav_names = names;

        // 树被替换了但层级没变：按路径把悬停状态接回去。
        // 否则指针不动时，信息卡会在每次快照到达时闪没。
        let hovered = self.hovered_path.as_ref().and_then(|path| {
            self.current_node()
                .and_then(|node| node.children.iter().position(|child| &child.path == path))
        });
        self.hovered_cell = hovered;

        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());
    }

    // ------------------------------------------------------------ 删除

    /// 请求删除一个条目。只记绝对路径：确认框打开期间树可能被扫描快照
    /// 整体替换、子项重排，按下标二次定位会指到别的条目。
    pub(crate) fn request_delete(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        // 只用于文案（删除文件夹 / 删除文件）。
        let is_dir = self
            .current_node()
            .and_then(|node| node.children.iter().find(|child| child.path == path))
            .map(|child| child.is_dir)
            .unwrap_or(false);

        let weak = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _window, _cx| {
            let title = t!("confirm_delete_title").to_string();
            let body = t!("confirm_delete_body", name = &name).to_string();
            let ok_text = if is_dir {
                t!("context_delete_folder").to_string()
            } else {
                t!("context_delete_file").to_string()
            };
            let cancel_text = t!("cancel_button").to_string();
            let weak_ok = weak.clone();
            let path_ok = path.clone();

            alert
                .title(title)
                .description(body)
                .ok_text(ok_text)
                .ok_variant(ButtonVariant::Danger)
                .cancel_text(cancel_text)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    let path = path_ok.clone();
                    let _ = weak_ok.update(cx, |this, cx| {
                        this.perform_delete(path, window, cx);
                    });
                    true
                })
        });
    }

    /// 按绝对路径删除，并在内存树里摘掉同一个条目。
    fn perform_delete(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // 用 symlink_metadata：链接要按链接本身删（remove_file），
        // 不能顺着链接 remove_dir_all 到目标目录里去。
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(err) => {
                window.push_notification(
                    gpui_kit::component::notification::Notification::error(
                        t!("delete_failed", name = &name, msg = err.to_string()).to_string(),
                    ),
                    cx,
                );
                return;
            }
        };

        let remove_dir = meta.is_dir() && !meta.file_type().is_symlink();
        let result = if remove_dir {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        if let Err(err) = result {
            window.push_notification(
                gpui_kit::component::notification::Notification::error(
                    t!("delete_failed", name = &name, msg = err.to_string()).to_string(),
                ),
                cx,
            );
            return;
        }

        let keep = self.current_node_path();
        if let Some(root) = self.root.as_mut() {
            let root = Arc::make_mut(root);
            if remove_entry(root, &path) {
                recalc_node(root);
            }
        }

        match keep {
            Some(path) => self.restore_navigation(&path),
            None => self.restore_navigation_at_root(),
        }
        cx.notify();
    }

    // ------------------------------------------------------------ 命中测试

    /// 窗口坐标 → 当前节点下的子项下标。
    pub(crate) fn hit_test(&self, position: Point<Pixels>) -> Option<usize> {
        let layout = self.cell_layout.borrow().clone();
        layout
            .iter()
            .find(|(_, bounds)| bounds.contains(&position))
            .map(|(index, _)| *index)
    }

    fn pointer_inside_canvas(&self, position: Point<Pixels>) -> bool {
        self.canvas_bounds
            .get()
            .map(|bounds| bounds.contains(&position))
            .unwrap_or(false)
    }

    pub(crate) fn on_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let inside = self.pointer_inside_canvas(event.position);
        let hovered = if inside {
            self.hit_test(event.position)
        } else {
            None
        };

        let mut needs_redraw = hovered != self.hovered_cell;

        if hovered != self.hovered_cell {
            self.hovered_cell = hovered;
            self.hovered_path = self
                .hovered_cell
                .and_then(|index| {
                    self.current_node()
                        .and_then(|node| node.children.get(index))
                })
                .map(|child| child.path.clone());
        }

        // 信息卡跟着指针走，所以悬停期间的移动也要重绘。
        //
        // 但一帧要提交上万个 quad，跟着每个像素重建整棵树太贵了：
        // 用 2px 位移 + 25ms（40Hz）双重节流，肉眼看不出来，
        // 却能把「移动鼠标就吃满一个核」压回去。
        let moved = match self.hover_pos {
            Some(previous) => (previous - event.position).magnitude() > 2.0,
            None => true,
        };
        self.hover_pos = inside.then_some(event.position);

        if moved && hovered.is_some() {
            let now = Instant::now();
            if now.duration_since(self.last_hover_redraw) >= HOVER_REDRAW_INTERVAL {
                self.last_hover_redraw = now;
                needs_redraw = true;
            }
        }

        if needs_redraw {
            cx.notify();
        }
    }

    /// 当前悬停的条目，供信息卡读取。
    pub(crate) fn hovered_entry(&self) -> Option<&FileEntry> {
        let index = self.hovered_cell?;
        self.current_node()?.children.get(index)
    }

    pub(crate) fn on_left_click(&mut self, event: &ClickEvent, cx: &mut Context<Self>) {
        let position = event.position();
        match self.hit_test(position) {
            Some(index) => {
                if let Some(child) = self.current_node().and_then(|n| n.children.get(index)) {
                    if child.is_dir {
                        self.navigate_into(index, cx);
                    }
                }
            }
            // 点击空白处 = 返回上级。
            None => self.navigate_up(cx),
        }
    }

    pub(crate) fn on_right_click(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.hit_test(position) else {
            return;
        };
        let Some((path, is_dir)) = self
            .current_node()
            .and_then(|node| node.children.get(index))
            .map(|child| (child.path.clone(), child.is_dir))
        else {
            return;
        };

        self.open_context_menu(path, is_dir, position, window, cx);
    }

    fn open_context_menu(
        &mut self,
        path: PathBuf,
        is_dir: bool,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let weak = cx.entity().downgrade();
        let open_label = if is_dir {
            t!("context_open_folder").to_string()
        } else {
            t!("context_open_file_location").to_string()
        };
        let delete_label = if is_dir {
            t!("context_delete_folder").to_string()
        } else {
            t!("context_delete_file").to_string()
        };

        // 菜单里只保存绝对路径：子项按大小重排会让下标漂移，
        // 真正动手时必须重新按路径定位。
        let menu_path = path.clone();
        let menu =
            gpui_kit::component::menu::PopupMenu::build(window, cx, move |menu, _window, _cx| {
                let path_open = menu_path.clone();
                let weak_delete = weak.clone();
                let path_delete = menu_path.clone();

                menu.item(
                    gpui_kit::component::menu::PopupMenuItem::new(open_label).on_click(
                        move |_, _window, _cx| {
                            util::open_file_location(&path_open, is_dir);
                        },
                    ),
                )
                .separator()
                .item(
                    gpui_kit::component::menu::PopupMenuItem::new(delete_label).on_click(
                        move |_, window, cx| {
                            let path = path_delete.clone();
                            let _ = weak_delete.update(cx, |this, cx| {
                                this.request_delete(path, window, cx);
                            });
                        },
                    ),
                )
            });

        // 菜单需要焦点才能响应键盘（Esc / 上下键）。
        // 关掉之后要还回去，否则输入框再也没法用键盘。
        let previous_focus = window.focused(cx);
        menu.focus_handle(cx).focus(window, cx);

        // 菜单关闭（点外面 / Esc / 选中条目）后清理状态并把焦点还回去。
        self._menu_subscription = Some(cx.subscribe_in(
            &menu,
            window,
            move |this, _, _: &DismissEvent, window, cx| {
                this.context_menu = None;
                if let Some(handle) = previous_focus.as_ref() {
                    handle.focus(window, cx);
                }
                cx.notify();
            },
        ));

        self.context_menu = Some(menu);
        self.context_pos = position;
        cx.notify();
    }

    // ------------------------------------------------------------ 视图入口

    /// 系统原生目录选择框（GPUI 自带，无需额外依赖）。
    pub(crate) fn browse_for_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t!("browse_button").to_string().into()),
        });

        cx.spawn_in(window, async move |this, cx| {
            let picked = match receiver.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                _ => None,
            };
            let Some(path) = picked else {
                return;
            };

            let _ = this.update_in(cx, |this, window, cx| {
                this.set_scan_path(path.display().to_string(), window, cx);
                this.start_scan(window, cx);
            });
        })
        .detach();
    }

    /// 把路径写进输入框。
    pub(crate) fn set_scan_path(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scan_path.update(cx, |state, cx| {
            state.set_value(path, window, cx);
        });
        cx.notify();
    }

    /// 出错后重试：清掉错误再扫一次。
    pub(crate) fn retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.error_message = None;
        self.start_scan(window, cx);
    }
}

impl Render for SpaceSnifferApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        // 扫描线程的结果可能刚好在两次轮询之间到达，这里做一次兜底同步。
        self.sync_scan_state();

        let background = cx.theme().background;
        let foreground = cx.theme().foreground;

        let body = if self.error_message.is_some() {
            views::error_state(self, cx)
        } else if self.root.is_some() {
            views::treemap_panel(self, cx)
        } else if self.is_scanning() {
            views::scanning_state(self, cx)
        } else {
            views::landing_state(self, window, cx)
        };

        views::v_flex()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(views::toolbar(self, window, cx))
            .child(gpui::div().flex_1().relative().child(body))
            .child(views::status_bar(self, cx))
            .children(views::overlays(self, cx))
    }
}

/// 扫描进度快照。
#[derive(Clone, Default, PartialEq)]
pub struct ScanStats {
    pub scanning: bool,
    pub files: u64,
    pub dirs: u64,
    pub skipped: u64,
    pub current_path: String,
}

/// `spacesniffer-rs <PATH>` 里的那个路径，没有就返回 `None`。
fn initial_path_from_args() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .filter(|arg| !arg.trim().is_empty())
}

fn new_progress() -> Arc<Mutex<ScanProgress>> {
    Arc::new(Mutex::new(ScanProgress {
        phase: ScanPhase::Idle,
        result: None,
        partial_root: None,
        files_processed: 0,
        dirs_processed: 0,
        skipped_dirs: 0,
        total_bytes: 0,
        current_path: String::new(),
        cancelled: Arc::new(AtomicBool::new(false)),
    }))
}

/// 自底向上重算目录大小（删除后调用）。
fn recalc_node(node: &mut FileEntry) -> u64 {
    let mut total = 0u64;
    for child in &mut node.children {
        if child.is_dir {
            total += recalc_node(child);
        } else {
            total += child.size;
        }
    }
    node.size = total;
    total
}

/// 在树里按绝对路径摘掉一个条目，返回是否找到并删掉了。
///
/// 用路径而不是下标定位：扫描快照和删除都会让 `children` 顺序变化，
/// 下标在两次操作之间是不可靠的。
fn remove_entry(node: &mut FileEntry, target: &Path) -> bool {
    if let Some(index) = node.children.iter().position(|child| child.path == target) {
        node.children.remove(index);
        return true;
    }

    for child in &mut node.children {
        if child.is_dir && target.starts_with(&child.path) && remove_entry(child, target) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, size: u64) -> FileEntry {
        FileEntry {
            name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
            path: PathBuf::from(path),
            size,
            children: Vec::new(),
            is_dir: false,
            is_symlink: false,
        }
    }

    fn dir(path: &str, children: Vec<FileEntry>) -> FileEntry {
        let size = children.iter().map(|child| child.size).sum();
        FileEntry {
            name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
            path: PathBuf::from(path),
            size,
            children,
            is_dir: true,
            is_symlink: false,
        }
    }

    /// 扫描快照会按大小重排 children。删除必须跟着路径走，
    /// 否则「右键 A、树重排、确认」会摘掉 B。
    #[test]
    fn remove_entry_survives_sibling_reorder() {
        let small = file("/tmp/root/small.txt", 10);
        let big = file("/tmp/root/big.txt", 90);

        let mut tree = dir("/tmp/root", vec![big, small]);
        // 模拟一次快照：顺序反过来
        tree.children.reverse();

        assert!(remove_entry(&mut tree, Path::new("/tmp/root/small.txt")));

        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["big.txt"], "被删的应该是 small.txt");
    }

    #[test]
    fn remove_entry_reaches_nested_paths() {
        let deep = file("/tmp/root/a/b/deep.bin", 42);
        let mut tree = dir(
            "/tmp/root",
            vec![dir("/tmp/root/a", vec![dir("/tmp/root/a/b", vec![deep])])],
        );

        assert!(remove_entry(&mut tree, Path::new("/tmp/root/a/b/deep.bin")));
        assert!(!remove_entry(
            &mut tree,
            Path::new("/tmp/root/a/b/deep.bin")
        ));

        // 目录大小由调用方重算，这里显式跟一遍 perform_delete 的流程。
        recalc_node(&mut tree);
        assert_eq!(tree.size, 0);
    }

    #[test]
    fn remove_entry_reports_missing_paths() {
        let mut tree = dir("/tmp/root", vec![file("/tmp/root/keep.txt", 1)]);
        assert!(!remove_entry(&mut tree, Path::new("/tmp/root/gone.txt")));
        assert_eq!(tree.children.len(), 1);
    }
}
