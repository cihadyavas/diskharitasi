use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use eframe::egui::{
    self, Align2, Button, Color32, FontId, Id, Modal, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2,
};
use serde::{Deserialize, Serialize};

use crate::i18n::{self, Lang, T};
use crate::layout::{self, HEADER_H, Item, Params};
use crate::scan::{self, MountEntry, Scan};
use crate::tree::{NONE, Tree};

/// Kökten mutlak derinliğe göre 8'li renk döngüsü.
const PALETTE: [Color32; 8] = [
    Color32::from_rgb(255, 127, 127),
    Color32::from_rgb(255, 191, 127),
    Color32::from_rgb(255, 255, 0),
    Color32::from_rgb(127, 255, 127),
    Color32::from_rgb(127, 255, 255),
    Color32::from_rgb(191, 191, 255),
    Color32::from_rgb(191, 191, 191),
    Color32::from_rgb(255, 127, 255),
];
const FREE_BG: Color32 = Color32::from_rgb(240, 240, 240);
const TIP_BG: Color32 = Color32::from_rgb(255, 255, 225);
const ANIM_SECS: f64 = 0.25;

fn depth_color(depth: u16) -> Color32 {
    PALETTE[((depth.max(1) - 1) % 8) as usize]
}

fn font() -> FontId {
    FontId::proportional(10.5)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    pub show_free: bool,
    pub one_fs: bool,
    pub min_px: f32,
    pub bias: i32,
    pub info_tips: bool,
    pub tip_full_path: bool,
    pub tip_size: bool,
    pub tip_date: bool,
    pub tip_counts: bool,
    pub tip_delay_ms: u32,
    pub rollover: bool,
    pub animated_zoom: bool,
    pub disable_delete: bool,
    pub last_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: Lang::Tr,
            show_free: true,
            one_fs: true,
            min_px: 3.0,
            bias: 0,
            info_tips: true,
            tip_full_path: false,
            tip_size: true,
            tip_date: true,
            tip_counts: true,
            tip_delay_ms: 250,
            rollover: false,
            animated_zoom: true,
            disable_delete: false,
            last_path: String::new(),
        }
    }
}

enum Action {
    Open,
    Reload,
    ZoomFull,
    ZoomIn,
    ZoomOut,
    ZoomTo(u32),
    RunOpen(u32),
    ShowInFm(u32),
    CopyPath(u32),
    AskDelete(u32),
    Scan(PathBuf),
}

#[derive(Clone, Copy)]
enum AnimKind {
    /// Yakınlaşma: yeni görünüm eski yerleşimdeki bu kutudan büyür.
    In(Rect),
    /// Uzaklaşma: eski görünümün yeni yerleşimdeki kutusu henüz bilinmiyor.
    OutNode(u32),
    /// Uzaklaşma: yeni yerleşim, bu kutu ekranı dolduracak şekilde başlar.
    Out(Rect),
}

struct Anim {
    start: f64,
    kind: AnimKind,
}

struct OpenDialog {
    mounts: Vec<MountEntry>,
    path: String,
}

type LayoutKey = (u32, Rect, i32, i32, u64);

pub struct App {
    s: Settings,
    tree: Option<Tree>,
    scan: Option<Scan>,
    view: u32,
    selected: u32,
    items: Vec<Item>,
    layout_key: Option<LayoutKey>,
    version: u64,
    hover: u32,
    hover_since: f64,
    anim: Option<Anim>,
    open_dialog: Option<OpenDialog>,
    confirm_delete: Option<u32>,
    error: Option<String>,
    show_settings: bool,
    show_about: bool,
    title: String,
    #[cfg(debug_assertions)]
    dbg: debug::Script,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, arg: Option<PathBuf>) -> App {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let s: Settings = cc
            .storage
            .and_then(|st| eframe::get_value(st, eframe::APP_KEY))
            .unwrap_or_default();
        let mut app = App {
            s,
            tree: None,
            scan: None,
            view: 0,
            selected: NONE,
            items: Vec::new(),
            layout_key: None,
            version: 0,
            hover: NONE,
            hover_since: 0.0,
            anim: None,
            open_dialog: None,
            confirm_delete: None,
            error: None,
            show_settings: false,
            show_about: false,
            title: String::new(),
            #[cfg(debug_assertions)]
            dbg: debug::Script::from_env(),
        };
        if let Some(p) = arg {
            app.start_scan(p, &cc.egui_ctx);
        }
        app
    }

    fn start_scan(&mut self, path: PathBuf, ctx: &egui::Context) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if !path.is_dir() {
            self.error = Some(format!("{}: {}", i18n::t(self.s.lang).not_found, path.display()));
            return;
        }
        self.s.last_path = path.to_string_lossy().into_owned();
        self.scan = Some(Scan::start(path, self.s.one_fs, ctx.clone()));
    }

    fn poll_scan(&mut self, ctx: &egui::Context) {
        let Some(scan) = &mut self.scan else { return };
        if !scan.is_finished() {
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        let result = scan.take_result();
        self.scan = None;
        if let Some(tree) = result {
            self.tree = Some(tree);
            self.view = 0;
            self.selected = NONE;
            self.hover = NONE;
            self.anim = None;
            self.version += 1;
        }
    }

    /// Yakınlaş düğmesinin gideceği klasör.
    fn zoom_in_target(&self) -> Option<u32> {
        let tree = self.tree.as_ref()?;
        if self.selected == NONE {
            return None;
        }
        let n = &tree.nodes[self.selected as usize];
        let target = if n.is_dir { self.selected } else { n.parent };
        if target == NONE || target == self.view || tree.nodes[target as usize].children.is_empty() {
            return None;
        }
        Some(target)
    }

    fn zoom_to(&mut self, target: u32, now: f64) {
        let Some(tree) = &self.tree else { return };
        if target == self.view || target == NONE {
            return;
        }
        self.anim = None;
        if self.s.animated_zoom {
            if tree.is_ancestor_or_self(self.view, target) {
                if let Some(it) = self.items.iter().find(|it| it.node == target) {
                    self.anim = Some(Anim { start: now, kind: AnimKind::In(it.rect) });
                }
            } else if tree.is_ancestor_or_self(target, self.view) {
                self.anim = Some(Anim { start: now, kind: AnimKind::OutNode(self.view) });
            }
        }
        self.view = target;
        self.selected = NONE;
    }

    fn apply(&mut self, action: Action, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        match action {
            Action::Open => {
                let path = if self.s.last_path.is_empty() {
                    std::env::var("HOME").unwrap_or_else(|_| "/".into())
                } else {
                    self.s.last_path.clone()
                };
                self.open_dialog = Some(OpenDialog { mounts: scan::mounts(), path });
            }
            Action::Reload => {
                if let Some(t) = &self.tree {
                    let p = t.root_path.clone();
                    self.start_scan(p, ctx);
                }
            }
            Action::Scan(p) => self.start_scan(p, ctx),
            Action::ZoomFull => self.zoom_to(0, now),
            Action::ZoomIn => {
                if let Some(t) = self.zoom_in_target() {
                    self.zoom_to(t, now);
                }
            }
            Action::ZoomOut => {
                if let Some(t) = &self.tree {
                    let p = t.nodes[self.view as usize].parent;
                    self.zoom_to(p, now);
                }
            }
            Action::ZoomTo(id) => self.zoom_to(id, now),
            Action::RunOpen(id) => {
                if let Some(t) = &self.tree {
                    spawn_quiet("xdg-open", vec![t.path(id).into_os_string()]);
                }
            }
            Action::ShowInFm(id) => {
                if let Some(t) = &self.tree {
                    show_in_file_manager(t.path(id));
                }
            }
            Action::CopyPath(id) => {
                if let Some(t) = &self.tree {
                    ctx.copy_text(t.path(id).to_string_lossy().into_owned());
                }
            }
            Action::AskDelete(id) => {
                if !self.s.disable_delete {
                    self.confirm_delete = Some(id);
                }
            }
        }
    }

    fn delete_confirmed(&mut self, id: u32) {
        let tr = i18n::t(self.s.lang);
        let Some(tree) = &mut self.tree else { return };
        let path = tree.path(id);
        match trash::delete(&path) {
            Ok(()) => {
                tree.detach(id);
                if tree.is_ancestor_or_self(id, self.selected) || self.selected == id {
                    self.selected = NONE;
                }
                self.hover = NONE;
                self.version += 1;
            }
            Err(e) => self.error = Some(format!("{}\n{}\n\n{e}", tr.trash_error, path.display())),
        }
    }

    fn toolbar(&self, ui: &mut egui::Ui, tr: &T, actions: &mut Vec<Action>) {
        let idle = self.scan.is_none();
        let ready = idle && self.tree.is_some();
        let zoomed = ready && self.view != 0;
        let sel = ready && self.selected != NONE;
        ui.horizontal(|ui| {
            let mut b = |ui: &mut egui::Ui, enabled: bool, text: &str, a: Action| {
                if ui.add_enabled(enabled, Button::new(text)).clicked() {
                    actions.push(a);
                }
            };
            b(ui, idle, tr.open, Action::Open);
            b(ui, ready, tr.reload, Action::Reload);
            ui.separator();
            b(ui, zoomed, tr.zoom_full, Action::ZoomFull);
            b(ui, ready && self.zoom_in_target().is_some(), tr.zoom_in, Action::ZoomIn);
            b(ui, zoomed, tr.zoom_out, Action::ZoomOut);
            ui.separator();
            ui.add_enabled_ui(ready, |ui| {
                // Aç/kapa düğmesi: ayarı doğrudan değiştirir, eylem gerekmez.
                let r = ui.add(Button::new(tr.free_space).selected(self.s.show_free));
                if r.clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(Id::new("bos-alan-degis"), true));
                }
            });
            ui.separator();
            b(ui, sel, tr.run_open, Action::RunOpen(self.selected));
            b(ui, sel && !self.s.disable_delete, tr.delete, Action::AskDelete(self.selected));
            ui.separator();
            if ui.button(tr.setup).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(Id::new("ayarlar-ac"), true));
            }
            if ui.button(tr.about).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(Id::new("hakkinda-ac"), true));
            }
        });
    }

    fn map(&mut self, ui: &mut egui::Ui, tr: &T, actions: &mut Vec<Action>) {
        let ctx = ui.ctx().clone();
        let lang = self.s.lang;
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::WHITE);

        let Some(tree) = &self.tree else {
            if self.scan.is_none() {
                painter.text(rect.center(), Align2::CENTER_CENTER, tr.empty_hint, FontId::proportional(14.0), Color32::GRAY);
            }
            return;
        };

        // Kökte boş alan solda, harita sağda.
        let mut free_rect = None;
        let mut map_rect = rect;
        if self.view == 0 && self.s.show_free && tree.is_mount && tree.fs_total > 0 {
            let used = tree.nodes[0].size;
            let frac = tree.fs_free as f64 / (tree.fs_free + used).max(1) as f64;
            let fw = (rect.width() as f64 * frac).round() as f32;
            free_rect = Some(Rect::from_min_max(rect.min, pos2(rect.min.x + fw, rect.max.y)));
            map_rect = Rect::from_min_max(pos2(rect.min.x + fw, rect.min.y), rect.max);
        }
        // Sağ/alt kenar çizgileri dışarı taşmasın.
        let lay_rect = Rect::from_min_max(map_rect.min, map_rect.max - vec2(1.0, 1.0));

        let key: LayoutKey = (self.view, lay_rect, self.s.bias, (self.s.min_px * 4.0) as i32, self.version);
        if self.layout_key != Some(key) {
            let params = Params { min_px: self.s.min_px, bias: self.s.bias };
            self.items = layout::build(tree, self.view, lay_rect, &params);
            self.layout_key = Some(key);
        }

        let now = ctx.input(|i| i.time);
        if let Some(Anim { kind: AnimKind::OutNode(id), start }) = self.anim {
            self.anim = self
                .items
                .iter()
                .find(|it| it.node == id)
                .map(|it| Anim { start, kind: AnimKind::Out(it.rect) });
        }
        let mut xf: Option<(Rect, Rect)> = None;
        if let Some(anim) = &self.anim {
            let t = ((now - anim.start) / ANIM_SECS).clamp(0.0, 1.0) as f32;
            let e = 1.0 - (1.0 - t) * (1.0 - t);
            let lerp = |a: Rect, b: Rect| Rect::from_min_max(a.min.lerp(b.min, e), a.max.lerp(b.max, e));
            xf = match anim.kind {
                AnimKind::In(from) => Some((lay_rect, lerp(from, lay_rect))),
                AnimKind::Out(focus) => Some((lerp(focus, lay_rect), lay_rect)),
                AnimKind::OutNode(_) => None,
            };
            if t >= 1.0 {
                self.anim = None;
                xf = None;
            } else {
                ctx.request_repaint();
            }
        }
        let animating = xf.is_some();
        let tx = |r: Rect| match xf {
            None => r,
            Some((a, b)) => {
                let sx = b.width() / a.width().max(1.0);
                let sy = b.height() / a.height().max(1.0);
                Rect::from_min_max(
                    pos2(b.min.x + (r.min.x - a.min.x) * sx, b.min.y + (r.min.y - a.min.y) * sy),
                    pos2(b.min.x + (r.max.x - a.min.x) * sx, b.min.y + (r.max.y - a.min.y) * sy),
                )
            }
        };

        if let Some(fr) = free_rect {
            painter.rect_filled(fr, 0.0, FREE_BG);
            let pct = tree.fs_free as f64 * 100.0 / tree.fs_total as f64;
            let pct = if lang == Lang::Tr { format!("{pct:.1}").replace('.', ",") } else { format!("{pct:.1}") };
            let lines = [
                format!("<{}: {pct}%>", tr.free_space),
                format!("{} {}", i18n::human(tree.fs_free, lang), tr.free),
                format!("{}: {}", tr.files_total, i18n::thousands(tree.nodes[0].files, lang)),
                format!("{}: {}", tr.folders_total, i18n::thousands(tree.nodes[0].dirs.saturating_sub(1), lang)),
            ];
            let p = painter.with_clip_rect(fr);
            let y0 = fr.center().y - 2.0 * 12.0;
            for (i, l) in lines.iter().enumerate() {
                p.text(pos2(fr.center().x, y0 + i as f32 * 12.0), Align2::CENTER_TOP, l, font(), Color32::BLACK);
            }
        }

        let mp = painter.with_clip_rect(map_rect);
        for it in &self.items {
            let r = tx(it.rect);
            if !r.intersects(map_rect) || r.width() < 1.0 || r.height() < 1.0 {
                continue;
            }
            let node = &tree.nodes[it.node as usize];
            draw_box(&mp, r, depth_color(node.depth));
            if animating {
                continue;
            }
            if it.header {
                let clip = Rect::from_min_max(r.min + vec2(1.0, 1.0), pos2(r.max.x - 1.0, r.min.y + HEADER_H));
                mp.with_clip_rect(clip.intersect(map_rect))
                    .text(r.min + vec2(3.0, 1.0), Align2::LEFT_TOP, &*node.name, font(), Color32::BLACK);
            } else if !node.is_dir || r.height() < HEADER_H + 2.0 {
                draw_file_label(&mp, r, map_rect, node, lang, tr);
            }
        }

        // Seçim ve üzerine gelme vurgusu.
        let hovered_pos = if animating { None } else { resp.hover_pos() };
        #[cfg(debug_assertions)]
        let hovered_pos = hovered_pos.or(self.dbg.hover);
        let hit = |pos: Pos2| {
            self.items.iter().rev().find(|it| it.rect.contains(pos)).map(|it| it.node).unwrap_or(NONE)
        };
        let hover = hovered_pos.map(hit).unwrap_or(NONE);
        if !animating {
            if self.s.rollover && hover != NONE && hover != self.selected {
                if let Some(it) = self.items.iter().find(|it| it.node == hover) {
                    mp.rect_stroke(outline(it.rect), 0.0, Stroke::new(2.0, Color32::from_rgb(0, 0, 160)), StrokeKind::Inside);
                }
            }
            if let Some(it) = self.items.iter().find(|it| it.node == self.selected) {
                let r = it.rect;
                if it.header {
                    let strip = Rect::from_min_max(r.min, pos2(r.max.x + 1.0, r.min.y + HEADER_H));
                    mp.rect_filled(strip, 0.0, Color32::BLACK);
                    mp.with_clip_rect(strip.shrink(1.0).intersect(map_rect)).text(
                        r.min + vec2(3.0, 1.0),
                        Align2::LEFT_TOP,
                        &*tree.nodes[it.node as usize].name,
                        font(),
                        Color32::WHITE,
                    );
                }
                mp.rect_stroke(outline(r), 0.0, Stroke::new(2.0, Color32::BLACK), StrokeKind::Inside);
            }
        }

        // Tıklamalar.
        if resp.clicked() || resp.secondary_clicked() {
            self.selected = resp.interact_pointer_pos().map(hit).unwrap_or(NONE);
        }
        if resp.double_clicked() && self.selected != NONE {
            let n = &tree.nodes[self.selected as usize];
            if n.is_dir && !n.children.is_empty() {
                actions.push(Action::ZoomTo(self.selected));
            }
        }
        let sel = self.selected;
        let disable_delete = self.s.disable_delete;
        let sel_is_dir = sel != NONE && tree.nodes[sel as usize].is_dir;
        resp.context_menu(|ui| {
            if sel == NONE {
                ui.close();
                return;
            }
            let mut item = |ui: &mut egui::Ui, enabled: bool, text: &str, a: Action| {
                if ui.add_enabled(enabled, Button::new(text)).clicked() {
                    actions.push(a);
                    ui.close();
                }
            };
            item(ui, sel_is_dir, tr.zoom_in, Action::ZoomTo(sel));
            item(ui, true, tr.run_open, Action::RunOpen(sel));
            item(ui, true, tr.show_in_fm, Action::ShowInFm(sel));
            item(ui, true, tr.copy_path, Action::CopyPath(sel));
            ui.separator();
            item(ui, !disable_delete, tr.delete, Action::AskDelete(sel));
        });

        // Bilgi ipucu.
        if hover != self.hover {
            self.hover = hover;
            self.hover_since = now;
        }
        if self.s.info_tips && hover != NONE && !resp.context_menu_opened() {
            let wait = self.s.tip_delay_ms as f64 / 1000.0 - (now - self.hover_since);
            if wait > 0.0 {
                ctx.request_repaint_after(Duration::from_secs_f64(wait));
            } else if let Some(pos) = hovered_pos {
                self.info_tip(&ctx, pos, hover, tr);
            }
        }
    }

    fn info_tip(&self, ctx: &egui::Context, pos: Pos2, id: u32, tr: &T) {
        let Some(tree) = &self.tree else { return };
        let n = &tree.nodes[id as usize];
        let lang = self.s.lang;
        egui::Area::new(Id::new("bilgi-ipucu"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos + vec2(14.0, 18.0))
            .interactable(false)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(TIP_BG)
                    .stroke(Stroke::new(1.0, Color32::BLACK))
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.style_mut().visuals.override_text_color = Some(Color32::BLACK);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(if n.is_dir { "📁" } else { "📄" }).size(22.0));
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                let name = if self.s.tip_full_path { tree.display_path(id) } else { n.name.to_string() };
                                ui.label(egui::RichText::new(name).strong());
                                if self.s.tip_size {
                                    ui.label(format!(
                                        "{} {}  ({})",
                                        i18n::thousands(n.size, lang),
                                        tr.bytes,
                                        i18n::human(n.size, lang)
                                    ));
                                }
                                if self.s.tip_date {
                                    ui.label(i18n::date(n.mtime, lang));
                                }
                                if self.s.tip_counts && n.is_dir {
                                    ui.label(format!(
                                        "{} {}, {} {}",
                                        i18n::thousands(n.files, lang),
                                        tr.files,
                                        i18n::thousands(n.dirs.saturating_sub(1), lang),
                                        tr.folders
                                    ));
                                }
                            });
                        });
                    });
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context, tr: &T, actions: &mut Vec<Action>) {
        if ctx.data_mut(|d| d.remove_temp::<bool>(Id::new("bos-alan-degis"))).unwrap_or(false) {
            self.s.show_free = !self.s.show_free;
        }
        if ctx.data_mut(|d| d.remove_temp::<bool>(Id::new("ayarlar-ac"))).unwrap_or(false) {
            self.show_settings = true;
        }
        if ctx.data_mut(|d| d.remove_temp::<bool>(Id::new("hakkinda-ac"))).unwrap_or(false) {
            self.show_about = true;
        }

        if let Some(scan) = &self.scan {
            let p = &scan.progress;
            use std::sync::atomic::Ordering::Relaxed;
            Modal::new(Id::new("tarama")).show(ctx, |ui| {
                ui.set_width(480.0);
                ui.heading(tr.scanning);
                let cur = p.current.lock().map(|c| c.clone()).unwrap_or_default();
                ui.add(egui::Label::new(if cur.is_empty() { scan.root.display().to_string() } else { cur }).truncate());
                egui::Grid::new("sayac").num_columns(2).show(ui, |ui| {
                    ui.label(tr.found_files);
                    ui.label(i18n::thousands(p.files.load(Relaxed), self.s.lang));
                    ui.end_row();
                    ui.label(tr.found_folders);
                    ui.label(i18n::thousands(p.dirs.load(Relaxed), self.s.lang));
                    ui.end_row();
                });
                let bytes = p.bytes.load(Relaxed);
                let bar = match scan.expected_bytes {
                    Some(exp) if exp > 0 => egui::ProgressBar::new((bytes as f32 / exp as f32).min(1.0)).show_percentage(),
                    _ => egui::ProgressBar::new(0.0).animate(true),
                };
                ui.add(bar.text(i18n::human(bytes, self.s.lang)));
                ui.horizontal(|ui| {
                    if ui.button(tr.cancel).clicked() {
                        p.cancel.store(true, Relaxed);
                    }
                });
            });
        }

        let mut close_open = false;
        if let Some(dlg) = &mut self.open_dialog {
            let lang = self.s.lang;
            let resp = Modal::new(Id::new("ac")).show(ctx, |ui| {
                ui.set_width(520.0);
                ui.heading(tr.select_title);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for m in &dlg.mounts {
                        let p = m.path.to_string_lossy().into_owned();
                        let info = match m.space {
                            Some((total, free)) => format!(
                                "{}  ·  {} {} / {} {}",
                                m.fstype,
                                i18n::human(free, lang),
                                tr.free,
                                i18n::human(total, lang),
                                tr.total
                            ),
                            None => format!("{}  ·  {}", m.fstype, tr.network),
                        };
                        let r = ui.add(Button::selectable(dlg.path == p, format!("{p}\n{info}")).min_size(vec2(500.0, 0.0)));
                        if r.clicked() {
                            dlg.path = p.clone();
                        }
                        if r.double_clicked() {
                            actions.push(Action::Scan(m.path.clone()));
                            close_open = true;
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(tr.path);
                    let r = ui.add(egui::TextEdit::singleline(&mut dlg.path).desired_width(330.0));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        actions.push(Action::Scan(PathBuf::from(&dlg.path)));
                        close_open = true;
                    }
                    if ui.button(tr.home).clicked() {
                        dlg.path = std::env::var("HOME").unwrap_or_else(|_| "/".into());
                    }
                });
                ui.checkbox(&mut self.s.one_fs, tr.one_fs);
                ui.horizontal(|ui| {
                    if ui.button(tr.scan).clicked() {
                        actions.push(Action::Scan(PathBuf::from(&dlg.path)));
                        close_open = true;
                    }
                    if ui.button(tr.cancel).clicked() {
                        close_open = true;
                    }
                });
            });
            if resp.should_close() {
                close_open = true;
            }
        }
        if close_open {
            self.open_dialog = None;
        }

        if let (Some(id), Some(tree)) = (self.confirm_delete, &self.tree) {
            let n = &tree.nodes[id as usize];
            let mut answer = None;
            let resp = Modal::new(Id::new("sil")).show(ctx, |ui| {
                ui.set_width(440.0);
                ui.heading(tr.confirm_trash);
                ui.add(egui::Label::new(egui::RichText::new(tree.display_path(id)).strong()).wrap());
                ui.label(format!("{}  ·  {}", i18n::human(n.size, self.s.lang), i18n::date(n.mtime, self.s.lang)));
                ui.horizontal(|ui| {
                    if ui.button(tr.delete).clicked() {
                        answer = Some(true);
                    }
                    if ui.button(tr.cancel).clicked() {
                        answer = Some(false);
                    }
                });
            });
            if resp.should_close() && answer.is_none() {
                answer = Some(false);
            }
            if let Some(yes) = answer {
                self.confirm_delete = None;
                if yes {
                    self.delete_confirmed(id);
                }
            }
        }

        if let Some(msg) = &self.error {
            let mut close = false;
            let resp = Modal::new(Id::new("hata")).show(ctx, |ui| {
                ui.set_max_width(480.0);
                ui.label(msg);
                if ui.button(tr.ok).clicked() {
                    close = true;
                }
            });
            if close || resp.should_close() {
                self.error = None;
            }
        }

        let s = &mut self.s;
        egui::Window::new(tr.settings)
            .open(&mut self.show_settings)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::ComboBox::from_label(tr.language)
                    .selected_text(if s.lang == Lang::Tr { "Türkçe" } else { "English" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.lang, Lang::Tr, "Türkçe");
                        ui.selectable_value(&mut s.lang, Lang::En, "English");
                    });
                ui.separator();
                ui.strong(tr.layout);
                ui.horizontal(|ui| {
                    ui.label(format!("{}:  {}", tr.density, tr.density_many));
                    ui.add(egui::Slider::new(&mut s.min_px, 2.0..=12.0).show_value(false));
                    ui.label(tr.density_few);
                });
                ui.horizontal(|ui| {
                    ui.label(format!("{}:  {}", tr.bias, tr.horz));
                    ui.add(egui::Slider::new(&mut s.bias, -4..=4).show_value(false));
                    ui.label(tr.vert);
                });
                ui.checkbox(&mut s.show_free, tr.free_space);
                ui.checkbox(&mut s.one_fs, tr.one_fs);
                ui.separator();
                ui.strong(tr.tooltips);
                ui.checkbox(&mut s.info_tips, tr.info_tips);
                ui.add_enabled_ui(s.info_tips, |ui| {
                    ui.indent("ipucu", |ui| {
                        ui.checkbox(&mut s.tip_full_path, tr.full_path);
                        ui.checkbox(&mut s.tip_size, tr.file_size);
                        ui.checkbox(&mut s.tip_date, tr.date_time);
                        ui.checkbox(&mut s.tip_counts, tr.counts);
                        ui.horizontal(|ui| {
                            ui.label(tr.delay_ms);
                            ui.add(egui::DragValue::new(&mut s.tip_delay_ms).range(0..=3000).speed(10));
                        });
                    });
                });
                ui.checkbox(&mut s.rollover, tr.rollover);
                ui.separator();
                ui.strong(tr.misc);
                ui.checkbox(&mut s.animated_zoom, tr.animated_zoom);
                ui.checkbox(&mut s.disable_delete, tr.disable_delete);
            });

        egui::Window::new(tr.about)
            .open(&mut self.show_about)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading(format!("{} {}", tr.app, env!("CARGO_PKG_VERSION")));
                ui.label(tr.about_text);
            });
    }

    fn update_title(&mut self, ctx: &egui::Context, tr: &T) {
        let lang = self.s.lang;
        let title = match (&self.tree, &self.scan) {
            (_, Some(_)) => format!("{} - {}", tr.scanning, tr.app),
            (None, None) => tr.app.to_string(),
            (Some(t), None) if self.selected != NONE => {
                let n = &t.nodes[self.selected as usize];
                let pct = n.size as f64 * 100.0 / t.fs_total.max(1) as f64;
                let pct = if lang == Lang::Tr { format!("%{pct:.1}").replace('.', ",") } else { format!("{pct:.1}%") };
                format!("{} - {pct} - {} - {}", t.display_path(self.selected), i18n::human(n.size, lang), tr.app)
            }
            (Some(t), None) => format!(
                "{} - {} {} - {} {} - {}",
                t.display_path(self.view),
                i18n::human(t.nodes[self.view as usize].size, lang),
                tr.total,
                i18n::human(t.fs_free, lang),
                tr.free,
                tr.app
            ),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let tr = i18n::t(self.s.lang);
        self.poll_scan(&ctx);
        let mut actions = Vec::new();
        egui::Panel::top("arac-cubugu").show(ui, |ui| self.toolbar(ui, tr, &mut actions));
        egui::CentralPanel::no_frame().show(ui, |ui| self.map(ui, tr, &mut actions));
        self.dialogs(&ctx, tr, &mut actions);
        for a in actions {
            self.apply(a, &ctx);
        }
        self.update_title(&ctx, tr);
        #[cfg(debug_assertions)]
        self.debug_step(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }
}

/// Komşu kutuların siyah çizgileri tek piksel olsun diye çerçeve sağ/alt kenardan bir piksel taşar.
fn outline(r: Rect) -> Rect {
    Rect::from_min_max(r.min, r.max + vec2(1.0, 1.0))
}

fn draw_box(p: &egui::Painter, r: Rect, fill: Color32) {
    p.rect_filled(outline(r), 0.0, fill);
    if r.width() >= 4.0 && r.height() >= 4.0 {
        let light = |c: u8| (c as u16 + (255 - c as u16) * 3 / 4) as u8;
        let dark = |c: u8| (c as u16 * 3 / 4) as u8;
        let light = Color32::from_rgb(light(fill.r()), light(fill.g()), light(fill.b()));
        let dark = Color32::from_rgb(dark(fill.r()), dark(fill.g()), dark(fill.b()));
        let (w, h) = (r.width(), r.height());
        p.rect_filled(Rect::from_min_size(r.min + vec2(1.0, 1.0), vec2(w - 1.0, 1.0)), 0.0, light);
        p.rect_filled(Rect::from_min_size(r.min + vec2(1.0, 1.0), vec2(1.0, h - 1.0)), 0.0, light);
        p.rect_filled(Rect::from_min_size(pos2(r.min.x + 2.0, r.max.y - 1.0), vec2(w - 2.0, 1.0)), 0.0, dark);
        p.rect_filled(Rect::from_min_size(pos2(r.max.x - 1.0, r.min.y + 2.0), vec2(1.0, h - 2.0)), 0.0, dark);
    }
    p.rect_stroke(outline(r), 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
}

fn draw_file_label(p: &egui::Painter, r: Rect, clip: Rect, node: &crate::tree::Node, lang: Lang, tr: &T) {
    let inner = r.shrink(2.0).intersect(clip);
    if r.width() >= 70.0 && r.height() >= 44.0 {
        let lp = p.with_clip_rect(inner);
        let c = r.center();
        let lines = [
            node.name.to_string(),
            format!("{} {}", i18n::thousands(node.size, lang), tr.bytes),
            i18n::date(node.mtime, lang),
        ];
        for (i, l) in lines.iter().enumerate() {
            lp.text(pos2(c.x, c.y + (i as f32 - 1.0) * 12.0), Align2::CENTER_CENTER, l, font(), Color32::BLACK);
        }
    } else if r.width() >= 14.0 && r.height() >= 12.0 {
        p.with_clip_rect(inner)
            .text(pos2(r.min.x + 3.0, r.center().y), Align2::LEFT_CENTER, &*node.name, font(), Color32::BLACK);
    }
}

fn spawn_quiet(cmd: &'static str, args: Vec<std::ffi::OsString>) {
    std::thread::spawn(move || {
        let _ = Command::new(cmd).args(args).status();
    });
}

/// Dosyayı dosya yöneticisinde seçili gösterir (FreeDesktop FileManager1); olmazsa klasörü açar.
fn show_in_file_manager(path: PathBuf) {
    std::thread::spawn(move || {
        let uri = file_uri(&path);
        let ok = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.FileManager1",
                "--object-path",
                "/org/freedesktop/FileManager1",
                "--method",
                "org.freedesktop.FileManager1.ShowItems",
                &format!("['{uri}']"),
                "",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            let dir = path.parent().unwrap_or(Path::new("/")).to_path_buf();
            let _ = Command::new("xdg-open").arg(dir).status();
        }
    });
}

fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut s = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// Geliştirme için betikli gezinti + ekran görüntüsü (yalnız debug derlemesi).
/// `DH_SCRIPT="select;hover;shot:/tmp/a.ppm;zoomin;shot:/tmp/b.ppm;exit"`
#[cfg(debug_assertions)]
mod debug {
    use eframe::egui::Pos2;

    #[derive(Default)]
    pub struct Script {
        pub steps: std::collections::VecDeque<String>,
        pub next_at: f64,
        pub hover: Option<Pos2>,
        pub pending_shot: Option<String>,
    }

    impl Script {
        pub fn from_env() -> Script {
            let steps = std::env::var("DH_SCRIPT")
                .map(|s| s.split(';').map(str::to_string).collect())
                .unwrap_or_default();
            Script { steps, ..Default::default() }
        }
    }

    pub fn save_ppm(path: &str, img: &eframe::egui::ColorImage) {
        let mut out = format!("P6 {} {} 255\n", img.size[0], img.size[1]).into_bytes();
        for c in &img.pixels {
            out.extend_from_slice(&[c.r(), c.g(), c.b()]);
        }
        let _ = std::fs::write(path, out);
    }
}

#[cfg(debug_assertions)]
impl App {
    fn debug_step(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.dbg.pending_shot.clone() {
            let img = ctx.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = img {
                debug::save_ppm(&path, &img);
                self.dbg.pending_shot = None;
            } else {
                ctx.request_repaint();
                return;
            }
        }
        if self.dbg.steps.is_empty() || self.tree.is_none() || self.scan.is_some() {
            if !self.dbg.steps.is_empty() {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            return;
        }
        let now = ctx.input(|i| i.time);
        if now < self.dbg.next_at {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        }
        self.dbg.next_at = now + 0.8;
        ctx.request_repaint_after(Duration::from_millis(850));
        let step = self.dbg.steps.pop_front().unwrap_or_default();
        let biggest_dir = self.items.iter().find(|it| it.header).copied();
        match step.as_str() {
            "select" => self.selected = biggest_dir.map(|it| it.node).unwrap_or(NONE),
            "selectfile" => {
                self.selected = self
                    .items
                    .iter()
                    .filter(|it| !self.tree.as_ref().unwrap().nodes[it.node as usize].is_dir)
                    .max_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
                    .map(|it| it.node)
                    .unwrap_or(NONE)
            }
            "hover" => {
                self.dbg.hover = self.items.iter().find(|it| it.node == self.selected).map(|it| it.rect.center())
            }
            "unhover" => self.dbg.hover = None,
            "zoomin" => self.apply(Action::ZoomIn, ctx),
            "zoomout" => self.apply(Action::ZoomOut, ctx),
            "free" => self.s.show_free = !self.s.show_free,
            "settings" => self.show_settings = !self.show_settings,
            "open" => self.apply(Action::Open, ctx),
            "menu" => {
                if let Some(it) = self.items.iter().find(|it| it.node == self.selected) {
                    self.dbg.hover = Some(it.rect.center());
                }
            }
            "delete" => {
                if self.selected != NONE {
                    self.delete_confirmed(self.selected);
                }
            }
            "exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            s if s.starts_with("shot:") => {
                self.dbg.pending_shot = Some(s[5..].to_string());
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            _ => {}
        }
    }
}
