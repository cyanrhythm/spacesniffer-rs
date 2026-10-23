//! 应用状态与导航逻辑。
//!
//! 这个文件只关心「状态怎么变」，界面长什么样交给 `views` 下的模块。
//! 每个视图函数都是 `impl SpaceSnifferApp` 的方法，返回 `AnyElement`，
//! 方便在 `Render::render` 里顺序组合。

use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::gpui::{self, 
    AppContext as _, Focusable as _, ParentElement as _, Styled as _, 
    Bounds, ClickEvent, Context, DismissEvent, Entity, MouseMoveEvent,
    Pixels, Point, Render, Subscription, Task, Window,
};
use gpui_kit::component::{
    button::ButtonVariant, input::InputState, ActiveTheme as _, WindowExt as _,
};
use rust_i18n::t;

use crate::scanner::{self, FileEntry, ScanPhase, ScanProgress};
use crate::util;
use crate::views;

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

    /// 当前悬停的格子（`current_node().children` 的下标）。
    pub(crate) hovered_cell: Option<usize>,

    /// 画布在本帧的实际边界，由 treemap 的 prepaint 回填。
    pub(crate) canvas_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// 本帧算出的格子布局，供命中测试复用。
    pub(crate) cell_layout: Rc<RefCell<CellLayout>>,

    /// 右键菜单（gpui-kit 的 PopupMenu 实体）及其锚点。
    pub(crate) context_menu: Option<Entity<gpui_kit::component::menu::PopupMenu>>,
    pub(crate) context_pos: Point<Pixels>,
    pub(crate) context_target: Option<Vec<usize>>,

    /// 等待确认删除的目标。
    pub(crate) delete_target: Option<Vec<usize>>,

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
            canvas_bounds: Rc::new(Cell::new(None)),
            cell_layout: Rc::new(RefCell::new(Rc::new(Vec::new()))),
            context_menu: None,
            context_pos: Point::default(),
            context_target: None,
            delete_target: None,
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
        self.scan_progress
            .lock()
            .map(|p| p.phase == ScanPhase::Scanning)
            .unwrap_or(false)
    }

    /// 当前扫描统计，`render` 每帧取一次，避免重复加锁。
    pub(crate) fn scan_stats(&self) -> ScanStats {
        let p = match self.scan_progress.lock() {
            Ok(p) => p,
            Err(_) => return ScanStats::default(),
        };
        ScanStats {
            scanning: p.phase == ScanPhase::Scanning,
            files: p.files_processed,
            dirs: p.dirs_processed,
            skipped: p.skipped_dirs,
            current_path: p.current_path.clone(),
        }
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
        self.error_message = None;
        self.context_menu = None;
        self.context_target = None;
        self.delete_target = None;
        self.cancel_requested = false;
        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());

        let progress = new_progress();
        self.scan_progress = progress.clone();
        scanner::start_scan(path, progress);

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
        self.poll_task = Some(cx.spawn(async move |this, cx| {
            loop {
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
            }
        }));
    }

    /// 把扫描线程的增量/最终结果同步进来，返回是否有变化。
    fn sync_scan_state(&mut self) -> bool {
        let mut changed = false;

        let (partial, result, error) = {
            let mut p = match self.scan_progress.lock() {
                Ok(p) => p,
                Err(_) => return false,
            };

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

    pub(crate) fn get_entry_by_path(&self, path: &[usize]) -> Option<&FileEntry> {
        let root = self.root.as_ref()?;
        let mut node: &FileEntry = root;
        for &idx in path {
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
        if !child.is_dir || child.children.is_empty() {
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
        self.hovered_cell = None;
        self.context_menu = None;
        self.context_target = None;
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
            let Some(index) = node.children.iter().position(|child| child.path == child_path)
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
        self.hovered_cell = None;
        *self.cell_layout.borrow_mut() = Rc::new(Vec::new());
    }

    // ------------------------------------------------------------ 删除

    pub(crate) fn request_delete(&mut self, target: Vec<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.get_entry_by_path(&target) else {
            return;
        };
        let name = entry.name.clone();
        let is_dir = entry.is_dir;
        self.delete_target = Some(target.clone());

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
            let weak_cancel = weak.clone();
            let target_ok = target.clone();

            alert
                .title(title)
                .description(body)
                .ok_text(ok_text)
                .ok_variant(ButtonVariant::Danger)
                .cancel_text(cancel_text)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    let _ = weak_ok.update(cx, |this, cx| {
                        this.delete_target = None;
                        this.perform_delete(&target_ok, window, cx);
                    });
                    true
                })
                .on_cancel(move |_, _, cx| {
                    let _ = weak_cancel.update(cx, |this, _| this.delete_target = None);
                    true
                })
        });
    }

    fn perform_delete(&mut self, target: &[usize], window: &mut Window, cx: &mut Context<Self>) {
        let Some(last) = target.last().copied() else {
            return;
        };
        let parent_path = &target[..target.len() - 1];

        let Some(entry) = self.get_entry_by_path(target) else {
            return;
        };
        let entry_path = entry.path.clone();
        let is_dir = entry.is_dir;
        let entry_name = entry.name.clone();

        let result = if is_dir {
            std::fs::remove_dir_all(&entry_path)
        } else {
            std::fs::remove_file(&entry_path)
        };

        if let Err(err) = result {
            window.push_notification(
                gpui_kit::component::notification::Notification::error(
                    t!("delete_failed", name = &entry_name, msg = err.to_string()).to_string(),
                ),
                cx,
            );
            return;
        }

        // 重新走到父节点，摘掉这一项。
        let keep = self.current_node_path();
        if let Some(root) = self.root.as_mut() {
            let root = Arc::make_mut(root);
            if remove_child(root, parent_path, last) {
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
        let hovered = if self.pointer_inside_canvas(event.position) {
            self.hit_test(event.position)
        } else {
            None
        };

        if hovered != self.hovered_cell {
            self.hovered_cell = hovered;
            cx.notify();
        }
    }

    pub(crate) fn on_left_click(&mut self, event: &ClickEvent, cx: &mut Context<Self>) {
        let position = event.position();
        match self.hit_test(position) {
            Some(index) => {
                if let Some(child) = self.current_node().and_then(|n| n.children.get(index)) {
                    if child.is_dir && !child.children.is_empty() {
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
        let mut target = self.nav_stack.clone();
        target.push(index);
        self.open_context_menu(target, position, window, cx);
    }

    fn open_context_menu(
        &mut self,
        target: Vec<usize>,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.get_entry_by_path(&target) else {
            return;
        };
        let path = entry.path.clone();
        let is_dir = entry.is_dir;

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

        let menu_target = target.clone();
        let menu = gpui_kit::component::menu::PopupMenu::build(window, cx, move |menu, _window, _cx| {
            let path_open = path.clone();
            let weak_delete = weak.clone();
            let target_delete = menu_target.clone();

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
                        let target = target_delete.clone();
                        let _ = weak_delete.update(cx, |this, cx| {
                            this.request_delete(target, window, cx);
                        });
                    },
                ),
            )
        });

        // 菜单需要焦点才能响应键盘（Esc / 上下键）。
        menu.focus_handle(cx).focus(window, cx);

        // 菜单关闭（点外面 / Esc / 选中条目）后清理状态。
        self._menu_subscription = Some(cx.subscribe_in(
            &menu,
            window,
            |this, _, _: &DismissEvent, _window, cx| {
                this.context_menu = None;
                this.context_target = None;
                cx.notify();
            },
        ));

        self.context_menu = Some(menu);
        self.context_pos = position;
        self.context_target = Some(target);
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
#[derive(Clone, Default)]
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

fn new_progress() -> Arc<Mutex<ScanProgress>> {    Arc::new(Mutex::new(ScanProgress {
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

/// 从树上摘掉 `parent_path` 下的第 `index` 个子项，返回是否真的删掉了。
fn remove_child(root: &mut FileEntry, parent_path: &[usize], index: usize) -> bool {
    let mut node = root;
    for &parent in parent_path {
        match node.children.get_mut(parent) {
            Some(child) => node = child,
            None => return false,
        }
    }

    if index < node.children.len() {
        node.children.remove(index);
        true
    } else {
        false
    }
}
