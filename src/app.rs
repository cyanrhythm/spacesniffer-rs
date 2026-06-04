use std::path::PathBuf;
use std::sync::{atomic::Ordering, Arc, Mutex};

use egui::{Color32, CornerRadius, Rect, Sense};

use crate::colors;
use crate::scanner::{self, FileEntry, ScanPhase, ScanProgress};
use crate::treemap;
use crate::util;
use rust_i18n::t;

pub struct SpaceSnifferApp {
    scan_path: String,
    scan_progress: Arc<Mutex<ScanProgress>>,
    root: Option<Arc<FileEntry>>,
    nav_stack: Vec<usize>,
    nav_names: Vec<String>,
    treemap_rects: Vec<TreemapCell>,
    error_message: Option<String>,
    context_menu: Option<ContextMenuState>,
    delete_confirm: Option<Vec<usize>>,
    /// Tracks the breadcrumb layout so we can cache it across frames.
    breadcrumb_snapshot: Option<BreadcrumbSnapshot>,
    breadcrumb_dirty: bool,
}

struct TreemapCell {
    index: usize,
    rect: Rect,
}

#[derive(Clone)]
struct BreadcrumbSnapshot {
    names: Vec<String>,
    stack_len: usize,
}

struct ContextMenuState {
    nav_path: Vec<usize>,
    position: egui::Pos2,
}

impl SpaceSnifferApp {
    pub fn new() -> Self {
        let scan_progress = Arc::new(Mutex::new(ScanProgress {
            phase: ScanPhase::Idle,
            result: None,
            partial_root: None,
            files_processed: 0,
            dirs_processed: 0,
            skipped_dirs: 0,
            total_bytes: 0,
            current_path: String::new(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }));

        Self {
            scan_path: util::dirs_or_home(),
            scan_progress: scan_progress.clone(),
            root: None,
            nav_stack: Vec::new(),
            nav_names: Vec::new(),
            treemap_rects: Vec::new(),
            error_message: None,
            context_menu: None,
            delete_confirm: None,
            breadcrumb_snapshot: None,
            breadcrumb_dirty: true,
        }
    }

    fn start_scan(&mut self) {
        self.cancel_scan();
        self.root = None;
        self.nav_stack.clear();
        self.nav_names.clear();
        self.treemap_rects.clear();
        self.error_message = None;
        self.context_menu = None;
        self.delete_confirm = None;
        self.breadcrumb_dirty = true;

        let path = PathBuf::from(&self.scan_path);
        let progress = Arc::new(Mutex::new(ScanProgress {
            phase: ScanPhase::Idle,
            result: None,
            partial_root: None,
            files_processed: 0,
            dirs_processed: 0,
            skipped_dirs: 0,
            total_bytes: 0,
            current_path: String::new(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }));

        self.scan_progress = progress.clone();
        scanner::start_scan(path, progress);
    }

    fn cancel_scan(&mut self) {
        if let Ok(p) = self.scan_progress.lock() {
            p.cancelled.store(true, Ordering::Relaxed);
        }
    }

    fn check_scan_result(&mut self) {
        let mut p = match self.scan_progress.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        if let Some(partial) = p.partial_root.take() {
            if self.root.is_none() {
                self.nav_names.push(partial.name.clone());
                self.breadcrumb_dirty = true;
            }
            self.root = Some(Arc::new(partial));
        }

        if let Some(root) = p.result.take() {
            self.nav_names.clear();
            self.nav_names.push(root.name.clone());
            self.breadcrumb_dirty = true;
            self.root = Some(root);
        }

        if let ScanPhase::Error(ref msg) = p.phase {
            if self.error_message.is_none() {
                self.error_message = Some(msg.clone());
            }
        }
    }

    fn current_node(&self) -> Option<&FileEntry> {
        let root = self.root.as_ref()?;
        let mut node: &FileEntry = root;
        for &idx in &self.nav_stack {
            if idx >= node.children.len() {
                return None;
            }
            node = &node.children[idx];
        }
        Some(node)
    }

    pub fn navigate_into(&mut self, child_index: usize) {
        let node = match self.current_node() {
            Some(n) => n,
            None => return,
        };
        if child_index >= node.children.len() {
            return;
        }
        let child = &node.children[child_index];
        if !child.is_dir {
            return;
        }

        self.nav_names.push(child.name.clone());
        self.nav_stack.push(child_index);
        self.treemap_rects.clear();
        self.context_menu = None;
        self.breadcrumb_dirty = true;
    }

    pub fn navigate_up(&mut self) {
        if self.nav_stack.is_empty() {
            return;
        }
        self.nav_names.pop();
        self.nav_stack.pop();
        self.treemap_rects.clear();
        self.context_menu = None;
        self.breadcrumb_dirty = true;
    }

    pub fn navigate_to_root(&mut self) {
        self.nav_names.truncate(1);
        self.nav_stack.clear();
        self.treemap_rects.clear();
        self.context_menu = None;
        self.breadcrumb_dirty = true;
    }

    fn breadcrumb_data(&mut self) -> BreadcrumbSnapshot {
        if self.breadcrumb_dirty || self.breadcrumb_snapshot.is_none() {
            self.breadcrumb_snapshot = Some(BreadcrumbSnapshot {
                names: self.nav_names.clone(),
                stack_len: self.nav_stack.len(),
            });
            self.breadcrumb_dirty = false;
        }
        self.breadcrumb_snapshot.as_ref().unwrap().clone()
    }

    fn get_entry_by_path(&self, path: &[usize]) -> Option<&FileEntry> {
        let root = self.root.as_ref()?;
        let mut node: &FileEntry = root;
        for &idx in path {
            if idx >= node.children.len() {
                return None;
            }
            node = &node.children[idx];
        }
        Some(node)
    }

    fn perform_delete(&mut self, nav_path: &[usize]) {
        if nav_path.is_empty() {
            return;
        }

        let root_arc = match &mut self.root {
            Some(r) => r,
            None => return,
        };
        let root = Arc::make_mut(root_arc);

        let parent_depth = nav_path.len().saturating_sub(1);
        let mut node = root;
        for &idx in &nav_path[..parent_depth] {
            if idx >= node.children.len() {
                return;
            }
            node = &mut node.children[idx];
        }
        let target_idx = *nav_path.last().unwrap();
        if target_idx >= node.children.len() {
            return;
        }
        let target = &node.children[target_idx];

        let result = if target.is_dir {
            std::fs::remove_dir_all(&target.path)
        } else {
            std::fs::remove_file(&target.path)
        };

        if result.is_err() {
            return;
        }

        node.children.remove(target_idx);
        self.recalculate_sizes();
        self.fix_navigation();
        self.treemap_rects.clear();
        self.breadcrumb_dirty = true;
    }

    fn recalculate_sizes(&mut self) {
        if let Some(root_arc) = &mut self.root {
            let root = Arc::make_mut(root_arc);
            recalc_node(root);
        }
    }

    fn fix_navigation(&mut self) {
        let node = match self.root.as_ref() {
            Some(n) => n,
            None => {
                self.nav_stack.clear();
                self.nav_names.truncate(1);
                self.breadcrumb_dirty = true;
                return;
            }
        };

        let mut new_stack = Vec::new();
        let mut current: &FileEntry = node;
        for &idx in &self.nav_stack {
            if idx >= current.children.len() {
                break;
            }
            new_stack.push(idx);
            current = &current.children[idx];
        }

        if new_stack.len() != self.nav_stack.len() {
            self.nav_names.truncate(new_stack.len() + 1);
            self.nav_stack = new_stack;
            self.breadcrumb_dirty = true;
        }
    }
}

impl eframe::App for SpaceSnifferApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_scan_result();

        let is_scanning = self
            .scan_progress
            .lock()
            .map(|p| p.phase == ScanPhase::Scanning)
            .unwrap_or(false);

        if is_scanning {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }

        let breadcrumb_data = self.breadcrumb_data();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(t!("landing_heading").as_ref());
                ui.separator();

                let path_edit = ui.add(
                    egui::TextEdit::singleline(&mut self.scan_path)
                        .hint_text(t!("path_hint").as_ref())
                        .desired_width(300.0),
                );

                if path_edit.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    self.start_scan();
                }

                if ui.button(t!("scan_button").as_ref()).clicked() {
                    self.start_scan();
                }

                if ui.button(t!("cancel_button").as_ref()).clicked() {
                    self.cancel_scan();
                }

                if ui.button(t!("browse_button").as_ref()).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.scan_path = path.display().to_string();
                        self.start_scan();
                    }
                }

                ui.separator();
                render_breadcrumbs(ui, &breadcrumb_data, self);
            });
        });

        self.render_context_menu(ctx);
        self.render_delete_confirm(ctx);
        self.render_treemap_area(ctx, is_scanning);
        self.render_bottom_panel(ctx);
    }
}

impl SpaceSnifferApp {
    fn render_treemap_area(&mut self, ctx: &egui::Context, is_scanning: bool) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(msg) = self.error_message.clone() {
                let needs_retry = ui
                    .vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.colored_label(Color32::RED, t!("error_label", msg = &msg).as_ref());
                        ui.button(t!("retry_button").as_ref()).clicked()
                    })
                    .inner;
                if needs_retry {
                    self.error_message = None;
                    self.start_scan();
                }
                return;
            }

            if self.root.is_some() {
                self.render_treemap(ui);
                if is_scanning {
                    if let Ok(p) = self.scan_progress.lock() {
                        egui::Area::new(egui::Id::new("scan_overlay"))
                            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 0.0])
                            .show(ctx, |ui| {
                                egui::Frame::NONE
                                    .fill(Color32::from_black_alpha(180))
                                    .corner_radius(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(format!(
                                                "Scanning... {} files, {} dirs",
                                                p.files_processed, p.dirs_processed
                                            ));
                                        });
                                    });
                            });
                    }
                }
                return;
            }

            if is_scanning {
                if let Ok(p) = self.scan_progress.lock() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(t!("scanning_heading").as_ref());
                        ui.add_space(10.0);
                        ui.label(format!(
                            "Files: {}  |  Dirs: {}",
                            p.files_processed, p.dirs_processed
                        ));
                        if !p.current_path.is_empty() {
                            ui.label(t!("scanning_current", path = &p.current_path).as_ref());
                        }
                        ui.add_space(20.0);
                        ui.add(
                            egui::ProgressBar::new(0.0)
                                .animate(true)
                                .desired_width(300.0),
                        );
                    });
                }
                return;
            }

            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading(t!("landing_heading").as_ref());
                ui.add_space(10.0);
                ui.label(t!("landing_instruction").as_ref());
                ui.add_space(20.0);
                if ui.button(t!("landing_browse_button").as_ref()).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.scan_path = path.display().to_string();
                        self.start_scan();
                    }
                }
            });
        });
    }

    fn render_treemap(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        if available.x <= 0.0 || available.y <= 0.0 {
            return;
        }

        let (response, painter) = ui.allocate_painter(available, Sense::click());
        let canvas_rect = response.rect;
        let padding = 2.0;
        let inner_rect = canvas_rect.shrink(padding);

        let node = match self.current_node() {
            Some(n) => n,
            None => return,
        };

        if node.children.is_empty() {
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                t!("empty_directory").as_ref(),
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
            return;
        }

        let items: Vec<(usize, f64)> = node
            .children
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.size as f64))
            .collect();

        let raw_layout = treemap::compute_treemap(&items, inner_rect);

        let new_treemap_rects: Vec<TreemapCell> = raw_layout
            .into_iter()
            .filter(|(_, rect)| rect.width() > 0.5 && rect.height() > 0.5)
            .map(|(index, rect)| TreemapCell { index, rect })
            .collect();

        let hover_pos = ui.input(|i| i.pointer.hover_pos());
        let mut new_hovered = None;

        let depth = self.nav_stack.len();
        let darken = 1.0 - (depth as f32 * 0.06).min(0.4);

        let mut right_click_target: Option<(Vec<usize>, egui::Pos2)> = None;

        for cell in &new_treemap_rects {
            let child = &node.children[cell.index];
            let base_color = colors::color_for_entry(child);

            let color = Color32::from_rgb(
                (base_color.r() as f32 * darken).min(255.0) as u8,
                (base_color.g() as f32 * darken).min(255.0) as u8,
                (base_color.b() as f32 * darken).min(255.0) as u8,
            );

            let fill_color = if let Some(pos) = hover_pos {
                if cell.rect.contains(pos) {
                    new_hovered = Some(cell.index);
                    Color32::from_rgb(
                        color.r().saturating_add(40),
                        color.g().saturating_add(40),
                        color.b().saturating_add(40),
                    )
                } else {
                    color
                }
            } else {
                color
            };

            painter.rect_filled(cell.rect, CornerRadius::ZERO, fill_color);
            painter.rect_stroke(
                cell.rect,
                CornerRadius::ZERO,
                egui::Stroke::new(1.0, Color32::from_gray(15)),
                egui::StrokeKind::Middle,
            );

            if cell.rect.width() > 30.0 && cell.rect.height() > 18.0 {
                let font_size = (cell.rect.height() / 4.0).clamp(10.0, 18.0);
                let text = if cell.rect.width() > 60.0 && cell.rect.height() > 36.0 {
                    format!(
                        "{}\n{}",
                        util::truncate_name(&child.name, 30),
                        util::format_size(child.size)
                    )
                } else if cell.rect.width() > 40.0 && cell.rect.height() > 20.0 {
                    util::truncate_name(&child.name, 25)
                } else {
                    String::new()
                };

                if !text.is_empty() {
                    let clip = painter.with_clip_rect(cell.rect);
                    let est_width = util::est_text_width(&text, font_size);
                    let padding = 8.0;
                    if est_width <= cell.rect.width() - padding {
                        clip.text(
                            cell.rect.center(),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::proportional(font_size),
                            Color32::WHITE,
                        );
                    } else {
                        clip.text(
                            egui::pos2(cell.rect.min.x + 4.0, cell.rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            text,
                            egui::FontId::proportional(font_size),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }

        let right_clicked =
            ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));

        if let Some(hovered) = new_hovered {
            if right_clicked {
                let mut nav_path = self.nav_stack.clone();
                nav_path.push(hovered);
                right_click_target = Some((nav_path, hover_pos.unwrap_or_default()));
            } else if response.clicked() {
                let child = &node.children[hovered];
                if child.is_dir && !child.children.is_empty() {
                    self.navigate_into(hovered);
                    return;
                }
            }
        } else if response.clicked() {
            self.navigate_up();
            return;
        }

        if let Some(hovered) = new_hovered {
            let child = &node.children[hovered];
            egui::show_tooltip_at_pointer(
                ui.ctx(),
                egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("treemap_tooltip")),
                egui::Id::new("treemap_tooltip"),
                |ui: &mut egui::Ui| {
                    ui.strong(&child.name);
                    ui.label(t!("tooltip_size", size = &util::format_size(child.size)).as_ref());
                    if child.is_dir {
                        ui.label(t!("tooltip_dir_type", items = child.children.len()).as_ref());
                    } else {
                        ui.label(t!("tooltip_file_type").as_ref());
                        if let Some(ext) = child.path.extension().and_then(|e| e.to_str()) {
                            ui.label(t!("tooltip_extension", ext = ext).as_ref());
                        }
                    }
                    ui.label(t!("tooltip_path", path = &child.path.display().to_string()).as_ref());
                },
            );
        }

        self.treemap_rects = new_treemap_rects;

        if let Some((nav_path, pos)) = right_click_target {
            self.context_menu = Some(ContextMenuState {
                nav_path,
                position: pos,
            });
        }
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(root) = &self.root {
                    ui.label(t!("total_label", total = &util::format_size(root.size)).as_ref());
                }

                ui.separator();

                let skipped = self
                    .scan_progress
                    .lock()
                    .map(|p| p.skipped_dirs)
                    .unwrap_or(0);
                if skipped > 0 {
                    ui.label(t!("dirs_skipped_label", skipped = skipped).as_ref());
                    ui.separator();
                }

                ui.spacing_mut().item_spacing.x = 4.0;
                render_legend_item(ui, Color32::from_rgb(70, 130, 180), t!("legend_dir").as_ref());
                render_legend_item(ui, Color32::from_rgb(200, 50, 50), t!("legend_doc").as_ref());
                render_legend_item(ui, Color32::from_rgb(80, 200, 120), t!("legend_code").as_ref());
                render_legend_item(ui, Color32::from_rgb(200, 100, 200), t!("legend_img").as_ref());
                render_legend_item(ui, Color32::from_rgb(220, 80, 80), t!("legend_vid").as_ref());
                render_legend_item(ui, Color32::from_rgb(100, 100, 220), t!("legend_aud").as_ref());
                render_legend_item(ui, Color32::from_rgb(200, 200, 80), t!("legend_arc").as_ref());
                render_legend_item(ui, Color32::from_rgb(160, 160, 160), t!("legend_exe").as_ref());
                render_legend_item(ui, Color32::from_rgb(140, 140, 140), t!("legend_oth").as_ref());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let node = self.current_node();
                    let count = node.map(|n| n.children.len()).unwrap_or(0);
                    ui.label(t!("items_count_label", count = count).as_ref());
                });
            });
        });
    }

    fn render_context_menu(&mut self, ctx: &egui::Context) {
        let (nav_path, position, path, is_dir) = {
            let menu = match self.context_menu.as_ref() {
                Some(m) => m,
                None => return,
            };

            let entry = self.get_entry_by_path(&menu.nav_path);
            if entry.is_none() {
                self.context_menu = None;
                return;
            }
            let entry = entry.unwrap();
            (
                menu.nav_path.clone(),
                menu.position,
                entry.path.clone(),
                entry.is_dir,
            )
        };

        let area_response = egui::Area::new(egui::Id::new("file_context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ctx, |ui| {
                egui::Frame::popup(&ctx.style()).show(ui, |ui| {
                    ui.set_min_width(180.0);
                    let open_label = if is_dir { t!("context_open_folder").to_string() } else { t!("context_open_file_location").to_string() };
                    if ui.button(&open_label).clicked() {
                        util::open_file_location(&path, is_dir);
                        self.context_menu = None;
                    }
                    let delete_label = if is_dir { t!("context_delete_folder").to_string() } else { t!("context_delete_file").to_string() };
                    if ui
                        .button(&delete_label)
                        .clicked()
                    {
                        self.delete_confirm = Some(nav_path);
                        self.context_menu = None;
                    }
                });
            });

        if area_response.response.clicked_elsewhere() {
            self.context_menu = None;
        }
    }

    fn render_delete_confirm(&mut self, ctx: &egui::Context) {
        let (nav_path, entry_name) = {
            let nav = match self.delete_confirm.clone() {
                Some(p) => p,
                None => return,
            };

            let entry = self.get_entry_by_path(&nav);
            if entry.is_none() {
                self.delete_confirm = None;
                return;
            }
            let entry = entry.unwrap();
            (nav, entry.name.clone())
        };

        let mut show = true;
        egui::Window::new(t!("confirm_delete_title").as_ref())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut show)
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label(t!("confirm_delete_body", name = &entry_name));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(t!("cancel_button").as_ref()).clicked() {
                        self.delete_confirm = None;
                    }
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("Delete").color(Color32::RED),
                        ))
                        .clicked()
                    {
                        self.perform_delete(&nav_path);
                        self.delete_confirm = None;
                    }
                });
            });

        if !show {
            self.delete_confirm = None;
        }
    }
}

fn render_legend_item(ui: &mut egui::Ui, color: Color32, label: &str) {
    let size = egui::vec2(14.0, 14.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(2), color);
    ui.label(label);
}

fn render_breadcrumbs(
    ui: &mut egui::Ui,
    data: &BreadcrumbSnapshot,
    app: &mut SpaceSnifferApp,
) {
    let button = |ui: &mut egui::Ui, text: &str, is_active: bool| -> egui::Response {
        let label = if is_active {
            egui::RichText::new(text).strong()
        } else {
            egui::RichText::new(text).underline()
        };
        ui.selectable_label(is_active, label)
    };

    if !data.names.is_empty() {
        let is_root = data.stack_len == 0;
        if button(ui, &data.names[0], is_root).clicked() {
            app.navigate_to_root();
        }
    }

    for (i, name) in data.names.iter().enumerate().skip(1) {
        ui.label(r"/");
        let is_last = i == data.names.len() - 1;
        if button(ui, name, is_last).clicked() && !is_last {
            let pop_count = data.names.len() - i - 1;
            for _ in 0..pop_count {
                app.nav_names.pop();
                app.nav_stack.pop();
            }
            app.treemap_rects.clear();
            app.context_menu = None;
            app.breadcrumb_dirty = true;
            return;
        }
    }
}

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
