use crate::{
    assets::{AssetCatalog, AssetKind},
    behavior::{self, ActionTemplate},
    document::{Document, History, Result},
    preview::Preview,
    runtime::{ResolvedStyle, StyleBook, VisualState, is_style_definition, resolve_asset},
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const ACCENT: Color32 = Color32::from_rgb(45, 206, 183);
const PANEL: Color32 = Color32::from_rgb(22, 31, 40);
const PANEL_RAISED: Color32 = Color32::from_rgb(28, 39, 50);
const CANVAS: Color32 = Color32::from_rgb(13, 21, 28);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Workspace {
    Interface,
    Behaviors,
    Resources,
    Test,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomPanel {
    Resources,
    History,
    Problems,
    Output,
}
struct Page {
    doc: Document,
    history: History,
    selected: usize,
    source: String,
    draft: bool,
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Settings {
    root: PathBuf,
    executable: PathBuf,
}
pub struct Studio {
    context: egui::Context,
    pages: Vec<Page>,
    active: usize,
    settings: Settings,
    settings_path: PathBuf,
    log_path: PathBuf,
    status: String,
    workspace: Workspace,
    bottom_panel: BottomPanel,
    project_filter: String,
    hierarchy_filter: String,
    files: HashMap<PathBuf, Vec<PathBuf>>,
    styles: StyleBook,
    assets: AssetCatalog,
    asset_filter: String,
    texture_cache: HashMap<PathBuf, egui::TextureHandle>,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    snap: bool,
    grid_size: f32,
    code: bool,
    show_native: bool,
    interact: bool,
    forced_state: String,
    live: bool,
    preview: Preview,
    texture: Option<egui::TextureHandle>,
    asset: Option<egui::TextureHandle>,
    property_key: String,
    property_value: String,
    add_dialog: bool,
    new_type: String,
    new_id: String,
    drag: Option<(usize, Rect, bool)>,
    boxes: Vec<Rect>,
    behavior_event: String,
    behavior_action: ActionTemplate,
    behavior_target: String,
    behavior_argument: String,
    simulation_log: Vec<String>,
    pending: Option<Instant>,
    smoke: bool,
    frames: u32,
    closing: bool,
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, log_path: PathBuf, smoke: bool) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.global_style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8.0, 6.0);
            s.spacing.button_padding = Vec2::new(12.0, 7.0);
            s.spacing.interact_size.y = 28.0;
            s.visuals.panel_fill = PANEL;
            s.visuals.window_fill = PANEL_RAISED;
            s.visuals.extreme_bg_color = CANVAS;
            s.visuals.faint_bg_color = Color32::from_rgb(29, 41, 52);
            s.visuals.widgets.inactive.bg_fill = Color32::from_rgb(29, 41, 52);
            s.visuals.widgets.hovered.bg_fill = Color32::from_rgb(36, 55, 66);
            s.visuals.widgets.active.bg_fill = Color32::from_rgb(31, 103, 109);
            s.visuals.widgets.noninteractive.bg_stroke =
                Stroke::new(1.0, Color32::from_rgb(48, 63, 76));
            s.visuals.selection.bg_fill = Color32::from_rgb(30, 91, 101);
            s.visuals.selection.stroke = Stroke::new(1.5, ACCENT);
        });
        let settings_path = log_path.with_file_name("nextgen-studio.settings.json");
        let settings = if smoke {
            Settings::default()
        } else {
            std::fs::read(&settings_path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default()
        };
        let mut s = Self {
            context: cc.egui_ctx.clone(),
            pages: vec![],
            active: 0,
            settings,
            settings_path,
            log_path,
            status: "Pronto • nenhum arquivo do cliente foi alterado".into(),
            workspace: Workspace::Interface,
            bottom_panel: BottomPanel::Resources,
            project_filter: String::new(),
            hierarchy_filter: String::new(),
            files: HashMap::new(),
            styles: StyleBook::default(),
            assets: AssetCatalog::default(),
            asset_filter: String::new(),
            texture_cache: HashMap::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            grid: true,
            snap: true,
            grid_size: 8.0,
            code: false,
            show_native: false,
            interact: false,
            forced_state: "Automático".into(),
            live: true,
            preview: Preview::default(),
            texture: None,
            asset: None,
            property_key: String::new(),
            property_value: String::new(),
            add_dialog: false,
            new_type: "Button".into(),
            new_id: "novoElemento".into(),
            drag: None,
            boxes: vec![],
            behavior_event: "@onClick".into(),
            behavior_action: ActionTemplate::Show,
            behavior_target: String::new(),
            behavior_argument: String::new(),
            simulation_log: Vec::new(),
            pending: None,
            smoke,
            frames: 0,
            closing: false,
        };
        if s.settings.root.is_dir() {
            let root = s.settings.root.clone();
            s.load_project(root);
        }
        s.new_document();
        if !smoke {
            if let Some(p) = std::env::args().skip(1).find(|a| !a.starts_with('-')) {
                s.open(Path::new(&p));
            }
        }
        s
    }
    fn load_project(&mut self, path: PathBuf) {
        self.settings.root = path;
        self.files.clear();
        self.texture_cache.clear();
        self.styles = StyleBook::load(&self.settings.root);
        self.assets = AssetCatalog::scan(&self.settings.root);
        self.status = format!(
            "Projeto indexado: {} estilos em {} arquivos • {} recursos",
            self.styles.style_count(),
            self.styles.files,
            self.assets.entries.len()
        );
    }
    fn report(&mut self, result: Result<()>) {
        if let Err(e) = result {
            log::error!("{e}");
            self.status = e;
        } else {
            self.status = "Alteração aplicada".into();
            self.pending = Some(Instant::now());
        }
    }
    fn new_document(&mut self) {
        if self.pages.len() >= 8 {
            self.status = "Limite de oito documentos.".into();
            return;
        }
        let mut doc = Document::parse(include_bytes!("../examples/welcome.otui"), true).unwrap();
        doc.mark_new();
        self.pages.push(Page {
            source: doc.text(),
            doc,
            history: History::default(),
            selected: 0,
            draft: false,
        });
        self.active = self.pages.len() - 1;
    }
    fn open(&mut self, p: &Path) {
        if let Ok(c) = std::fs::canonicalize(p) {
            if let Some(n) = self
                .pages
                .iter()
                .position(|p| p.doc.path.as_ref() == Some(&c))
            {
                self.active = n;
                return;
            }
        }
        if self.pages.len() >= 8 {
            self.status = "Feche uma aba antes de abrir outro documento.".into();
            return;
        }
        match Document::load(p) {
            Ok(doc) => {
                self.code = !doc.is_otui;
                self.pages.push(Page {
                    source: doc.text(),
                    doc,
                    history: History::default(),
                    selected: 0,
                    draft: false,
                });
                self.active = self.pages.len() - 1;
                self.status = format!("Aberto: {}", p.display());
                log::info!("{}", self.status);
            }
            Err(e) => self.report(Err(e)),
        }
    }
    fn mutate(&mut self, label: &str, op: impl FnOnce(&mut Document) -> Result<()>) {
        if let Some(p) = self.pages.get_mut(self.active) {
            if p.draft && label != "Editar código" {
                self.status = "Aplique o código pendente antes de editar o design.".into();
                return;
            }
            let result = p.history.apply(&mut p.doc, label, op);
            if result.is_ok() {
                p.source = p.doc.text();
                p.draft = false;
            }
            self.report(result);
        }
    }
    fn undo(&mut self, redo: bool) {
        if !self.commit_fields() {
            return;
        }
        if self.pages.get(self.active).is_some_and(|p| p.draft) {
            self.status = "Aplique o código pendente antes de usar o histórico.".into();
            return;
        }
        if let Some(p) = self.pages.get_mut(self.active) {
            let r = if redo {
                p.history.redo(&mut p.doc)
            } else {
                p.history.undo(&mut p.doc)
            };
            p.source = p.doc.text();
            self.report(r);
        }
    }
    fn commit_fields(&mut self) -> bool {
        let mut edits = Vec::new();
        if let Some(p) = self.pages.get(self.active) {
            for (n, node) in p.doc.nodes.iter().enumerate() {
                for prop in &node.properties {
                    let id = egui::Id::new(("property", self.active, n, &prop.key));
                    if let Some(value) = self.context.memory(|m| m.data.get_temp::<String>(id)) {
                        if value != prop.value {
                            edits.push((id, n, prop.key.clone(), value));
                        } else {
                            self.context.memory_mut(|m| m.data.remove::<String>(id));
                        }
                    }
                }
            }
        }
        for (id, n, key, value) in edits {
            let p = &mut self.pages[self.active];
            let result = p.history.apply(&mut p.doc, "Alterar propriedade", |d| {
                d.set(n, &key, &value)
            });
            if let Err(e) = result {
                self.report(Err(e));
                return false;
            }
            p.source = p.doc.text();
            self.context.memory_mut(|m| m.data.remove::<String>(id));
            self.pending = Some(Instant::now());
        }
        true
    }
    fn save(&mut self, save_as: bool) -> bool {
        if !self.commit_fields() {
            return false;
        }
        if self.pages.get(self.active).is_some_and(|p| p.draft) {
            let text = self.pages[self.active].source.clone();
            self.mutate("Editar código", |d| d.replace_text(&text));
            if self.pages[self.active].draft {
                return false;
            }
        }
        let Some(p) = self.pages.get(self.active) else {
            return true;
        };
        let target = if save_as || p.doc.path.is_none() {
            rfd::FileDialog::new()
                .set_directory(&self.settings.root)
                .set_file_name(
                    p.doc
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|x| x.to_str())
                        .unwrap_or("interface.otui"),
                )
                .save_file()
        } else {
            p.doc.path.clone()
        };
        let Some(target) = target else {
            return false;
        };
        if self.pages.iter().enumerate().any(|(i, p)| {
            i != self.active
                && p.doc
                    .path
                    .as_ref()
                    .is_some_and(|p| std::fs::canonicalize(&target).is_ok_and(|t| t == *p))
        }) {
            self.report(Err("Destino aberto em outra aba.".into()));
            return false;
        }
        let r = self.pages[self.active].doc.save(&target, true);
        let ok = r.is_ok();
        if ok {
            self.status = format!("Salvo: {}", target.display());
            log::info!("{}", self.status);
            self.pending = Some(Instant::now());
        } else {
            self.report(r);
        }
        ok
    }
    fn confirm_page(&mut self) -> bool {
        if !self.commit_fields() {
            return false;
        }
        if !self
            .pages
            .get(self.active)
            .is_some_and(|p| p.doc.dirty() || p.draft)
        {
            return true;
        }
        match rfd::MessageDialog::new()
            .set_title("Alterações pendentes")
            .set_description("Salvar as alterações deste documento?")
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show()
        {
            rfd::MessageDialogResult::Yes => self.save(false),
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }
    fn close_page(&mut self) {
        if self.confirm_page() {
            self.pages.remove(self.active);
            self.active = self.active.saturating_sub(1);
        }
    }
    fn image(&mut self, ctx: &egui::Context, p: &Path, native: bool) {
        let result = (|| {
            let reader = image::ImageReader::open(p)
                .map_err(|e| e.to_string())?
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let size = reader.into_dimensions().map_err(|e| e.to_string())?;
            if u64::from(size.0) * u64::from(size.1) > 16_000_000 {
                return Err("Imagem excede 16 milhões de pixels.".into());
            }
            let decoded = image::ImageReader::open(p)
                .map_err(|e| e.to_string())?
                .decode()
                .map_err(|e| e.to_string())?;
            let resized = if native {
                decoded.thumbnail(1600, 1200)
            } else {
                decoded.thumbnail(320, 240)
            };
            let rgba = resized.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            );
            Ok(ctx.load_texture(
                if native { "motor" } else { "asset" },
                color,
                egui::TextureOptions::NEAREST,
            ))
        })();
        match result {
            Ok(t) => {
                if native {
                    self.preview.image_loaded(p);
                    self.texture = Some(t)
                } else {
                    self.asset = Some(t)
                }
            }
            Err(e) => self.status = e,
        }
    }
    fn cached_texture(&mut self, path: &Path) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.texture_cache.get(path) {
            return Some(texture.clone());
        }
        let decoded = image::ImageReader::open(path).ok()?.decode().ok()?;
        if u64::from(decoded.width()) * u64::from(decoded.height()) > 16_000_000 {
            return None;
        }
        if self.texture_cache.len() >= 128 {
            self.texture_cache.clear();
        }
        let rgba = decoded.to_rgba8();
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        );
        let texture =
            self.context
                .load_texture(path.to_string_lossy(), color, egui::TextureOptions::NEAREST);
        self.texture_cache.insert(path.to_owned(), texture.clone());
        Some(texture)
    }
    fn tree(&mut self, ui: &mut egui::Ui, dir: &Path, depth: usize) {
        if depth > 12 {
            return;
        }
        if !self.files.contains_key(dir) {
            let mut list: Vec<PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_type().is_ok_and(|t| t.is_symlink()))
                .map(|e| e.path())
                .filter(|p| {
                    !p.file_name()
                        .is_some_and(|s| s.to_string_lossy().starts_with('.'))
                })
                .take(2000)
                .collect();
            list.sort();
            self.files.insert(dir.into(), list);
        }
        for p in self.files.get(dir).cloned().unwrap_or_default() {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if p.is_dir() {
                egui::CollapsingHeader::new(name)
                    .id_salt(&p)
                    .show(ui, |ui| self.tree(ui, &p, depth + 1));
            } else {
                let ext = p
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                if [
                    "otui", "lua", "otmod", "html", "css", "txt", "png", "jpg", "jpeg",
                ]
                .contains(&ext.as_str())
                    && ui.selectable_label(false, name).double_clicked()
                {
                    if ["png", "jpg", "jpeg"].contains(&ext.as_str()) {
                        self.image(ui.ctx(), &p, false);
                    } else {
                        self.open(&p);
                    }
                }
            }
        }
    }
    fn start_preview(&mut self) {
        if self.settings.root.as_os_str().is_empty() {
            self.status = "Abra um projeto antes de iniciar a prévia.".into();
            return;
        }
        if !self.preview.running() {
            if !self.settings.executable.is_file() {
                let Some(p) = rfd::FileDialog::new()
                    .set_directory(&self.settings.root)
                    .set_title("Executável NextGen")
                    .pick_file()
                else {
                    return;
                };
                self.settings.executable = p;
            }
            if rfd::MessageDialog::new().set_title("Prévia isolada").set_description("Instalar a integração dev_studio_bridge e iniciar uma janela separada do NextGen? Ela executará os scripts do projeto. Use somente projetos confiáveis.").set_buttons(rfd::MessageButtons::YesNo).show()!=rfd::MessageDialogResult::Yes{return;}
            let r = self
                .preview
                .start(&self.settings.root, &self.settings.executable);
            if r.is_err() {
                self.report(r);
                return;
            }
        }
        self.send_preview();
        self.show_native = true;
    }
    fn send_preview(&mut self) {
        if let Some(p) = self.pages.get(self.active) {
            if p.doc.is_otui {
                self.texture = None;
                let r = self.preview.send(&p.doc.text());
                if let Err(e) = r {
                    self.status = e;
                }
            }
        }
    }
    fn geometry(&self) -> Vec<Rect> {
        let Some(page) = self.pages.get(self.active) else {
            return vec![];
        };
        let d = &page.doc;
        let mut boxes = vec![Rect::NOTHING; d.nodes.len().min(3000)];
        for (i, n) in d.nodes.iter().take(3000).enumerate() {
            let under_definition = std::iter::successors(n.parent, |parent| {
                d.nodes.get(*parent).and_then(|node| node.parent)
            })
            .any(|parent| {
                d.nodes
                    .get(parent)
                    .is_some_and(|node| is_style_definition(&node.name))
            });
            if !n.widget || is_style_definition(&n.name) || under_definition {
                continue;
            }
            let resolved = self.styles.resolve(d, i, VisualState::default());
            let num = |key: &str, default: f32| {
                let raw = if d.value(i, key).is_empty() {
                    resolved.get(key)
                } else {
                    d.value(i, key)
                };
                raw.parse::<f32>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .unwrap_or(default)
                    .clamp(-8192.0, 8192.0)
            };
            let root = n.parent.is_none();
            let mut w = num("width", if root { 620.0 } else { 140.0 });
            let mut h = num("height", if root { 400.0 } else { 36.0 });
            let size_value = if d.value(i, "size").is_empty() {
                resolved.get("size")
            } else {
                d.value(i, "size")
            };
            let size: Vec<_> = size_value
                .split_whitespace()
                .filter_map(|n| n.parse::<f32>().ok())
                .collect();
            if size.len() == 2 {
                w = size[0].clamp(1.0, 8192.0);
                h = size[1].clamp(1.0, 8192.0);
            }
            let parent = n
                .parent
                .and_then(|p| boxes.get(p).copied())
                .filter(|r| r.is_finite())
                .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 640.0)));
            let mut x = num("margin-left", if root { 0.0 } else { 16.0 });
            let mut y = num(
                "margin-top",
                if root {
                    0.0
                } else {
                    40.0 + (i % 5) as f32 * 40.0
                },
            );
            let anchor = |key: &str| {
                let value = d.value(i, key);
                if value.is_empty() {
                    resolved.get(key)
                } else {
                    value
                }
            };
            let left = anchor("anchors.left") == "parent.left";
            let right = anchor("anchors.right") == "parent.right";
            let top = anchor("anchors.top") == "parent.top";
            let bottom = anchor("anchors.bottom") == "parent.bottom";
            if left && right {
                x = num("margin-left", 0.0);
                w = (parent.width() - x - num("margin-right", 0.0)).max(1.0);
            } else if right {
                x = parent.width() - w - num("margin-right", 0.0);
            }
            if top && bottom {
                y = num("margin-top", 0.0);
                h = (parent.height() - y - num("margin-bottom", 0.0)).max(1.0);
            } else if bottom {
                y = parent.height() - h - num("margin-bottom", 0.0);
            }
            if anchor("anchors.horizontalCenter") == "parent.horizontalCenter" {
                x = (parent.width() - w) / 2.0 + num("margin-left", 0.0) - num("margin-right", 0.0);
            }
            if anchor("anchors.verticalCenter") == "parent.verticalCenter" {
                y = (parent.height() - h) / 2.0 + num("margin-top", 0.0)
                    - num("margin-bottom", 0.0);
            }
            if anchor("anchors.fill") == "parent" {
                x = 0.0;
                y = 0.0;
                w = parent.width();
                h = parent.height();
            }
            if anchor("anchors.centerIn") == "parent" {
                x = (parent.width() - w) / 2.0;
                y = (parent.height() - h) / 2.0;
            }
            boxes[i] = Rect::from_min_size(
                parent.min + Vec2::new(x, y),
                Vec2::new(w.max(1.0), h.max(1.0)),
            );
        }
        boxes
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(if self.interact {
                "PRÉVIA INTERATIVA"
            } else {
                "EDIÇÃO VISUAL OTUI"
            });
            if ui.selectable_label(!self.interact, "Editar").clicked() {
                self.interact = false;
            }
            if ui.selectable_label(self.interact, "Interagir").clicked() {
                self.interact = true;
            }
            egui::ComboBox::from_id_salt("visual-state")
                .selected_text(&self.forced_state)
                .show_ui(ui, |ui| {
                    for state in [
                        "Automático",
                        "Normal",
                        "Hover",
                        "Pressionado",
                        "Desabilitado",
                        "Marcado",
                        "Ligado",
                        "Foco",
                    ] {
                        ui.selectable_value(&mut self.forced_state, state.into(), state);
                    }
                });
            ui.checkbox(&mut self.grid, "Grid");
            ui.checkbox(&mut self.snap, "Snapping");
            ui.add(
                egui::DragValue::new(&mut self.grid_size)
                    .range(1.0..=64.0)
                    .suffix(" px"),
            );
            ui.add(egui::Slider::new(&mut self.zoom, 0.25..=2.5).text("Zoom"));
            if ui.button("Centralizar").clicked() {
                self.pan = Vec2::ZERO;
                self.zoom = 1.0;
            }
        });
        ui.label(egui::RichText::new(format!("Runtime offline: {} estilos carregados • imagens, estados e eventos funcionam sem abrir o client.", self.styles.style_count())).small().color(Color32::GRAY));
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let area = response.rect;
        let origin = area.min + Vec2::new(28.0, 24.0) + self.pan;
        painter.rect_filled(area, 0.0, Color32::from_rgb(15, 23, 31));
        if self.grid {
            let step = (self.grid_size * self.zoom).max(6.0);
            let mut x = area.left() + (origin.x - area.left()).rem_euclid(step);
            while x < area.right() {
                let mut y = area.top() + (origin.y - area.top()).rem_euclid(step);
                while y < area.bottom() {
                    painter.circle_filled(Pos2::new(x, y), 0.6, Color32::from_rgb(46, 63, 77));
                    y += step;
                }
                x += step;
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if response.hovered() {
            let wheel = ui.input(|i| {
                if i.modifiers.ctrl {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            });
            if wheel != 0.0 {
                self.zoom = (self.zoom * (1.0 + wheel * 0.002)).clamp(0.25, 2.5);
            }
        }
        self.boxes = self.geometry();
        if self.pages.get(self.active).is_none() {
            return;
        }
        if let Some(pos) = response.interact_pointer_pos() {
            let local = Pos2::ZERO + (pos - origin) / self.zoom;
            if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(n) = self.boxes.iter().rposition(|b| b.contains(local)) {
                    self.pages[self.active].selected = n;
                    if self.interact && response.clicked() {
                        let events = behavior::events(&self.pages[self.active].doc, n);
                        let id = self.pages[self.active].doc.value(n, "id").to_owned();
                        let handler = events
                            .iter()
                            .find(|(event, _)| event.eq_ignore_ascii_case("@onClick"));
                        self.simulation_log
                            .push(if let Some((_, expression)) = handler {
                                format!(
                                    "Clique em {} → {}",
                                    if id.is_empty() { "widget" } else { &id },
                                    expression
                                )
                            } else {
                                format!(
                                    "Clique em {} → nenhum @onClick",
                                    if id.is_empty() { "widget" } else { &id }
                                )
                            });
                        if self.simulation_log.len() > 100 {
                            self.simulation_log.remove(0);
                        }
                    }
                    let r = self.boxes[n];
                    let resize = local.distance(r.max) < 12.0 / self.zoom;
                    let d = &self.pages[self.active].doc;
                    let node = &d.nodes[n];
                    let managed = node
                        .properties
                        .iter()
                        .any(|p| p.key.starts_with("anchors."))
                        || node.parent.is_some_and(|p| {
                            d.nodes[p].properties.iter().any(|p| p.key == "layout")
                        });
                    if !self.interact
                        && response.drag_started_by(egui::PointerButton::Primary)
                        && !managed
                        && (resize || node.parent.is_some())
                    {
                        self.drag = Some((n, r, resize));
                    } else if !self.interact && managed && response.drag_started() {
                        self.status="Geometria controlada por anchors/layout: edite as propriedades sem quebrar o vínculo.".into();
                    }
                }
            }
        }
        if let Some((n, original, resize)) = self.drag {
            let delta = response.drag_delta() / self.zoom;
            let snap = |v: f32| {
                if self.snap {
                    (v / self.grid_size).round() * self.grid_size
                } else {
                    v.round()
                }
            };
            let mut r = original;
            if resize {
                r.max = r.min
                    + Vec2::new(
                        snap(original.width() + delta.x).max(8.0),
                        snap(original.height() + delta.y).max(8.0),
                    );
            } else {
                r = r.translate(Vec2::new(snap(delta.x), snap(delta.y)));
            }
            if let Some(b) = self.boxes.get_mut(n) {
                *b = r;
            }
            if response.drag_stopped() {
                let parent = self.pages[self.active].doc.nodes[n]
                    .parent
                    .and_then(|p| self.boxes.get(p))
                    .map_or(Pos2::ZERO, |r| r.min);
                if resize {
                    self.mutate("Redimensionar", |d| {
                        d.set(
                            n,
                            "size",
                            &format!("{} {}", r.width() as i32, r.height() as i32),
                        )
                    });
                } else {
                    let p = r.min - parent;
                    self.mutate("Mover", |d| {
                        d.set(n, "margin-left", &format!("{}", p.x as i32))?;
                        d.set(n, "margin-top", &format!("{}", p.y as i32))
                    });
                }
                self.drag = None;
            }
        }
        let pointer = response
            .hover_pos()
            .map(|position| Pos2::ZERO + (position - origin) / self.zoom);
        let pressed = ui.input(|input| input.pointer.primary_down());
        let render_items: Vec<_> = {
            let page = &self.pages[self.active];
            self.boxes
                .iter()
                .enumerate()
                .filter_map(|(i, b)| {
                    if !b.is_finite() || !page.doc.nodes.get(i).is_some_and(|node| node.widget) {
                        return None;
                    }
                    let automatic_hover =
                        self.interact && pointer.is_some_and(|position| b.contains(position));
                    let state = visual_state(
                        &self.forced_state,
                        automatic_hover,
                        pressed && automatic_hover,
                    );
                    let style = self.styles.resolve(&page.doc, i, state);
                    let label = if style.get("text").is_empty() {
                        if page.doc.value(i, "text").is_empty() {
                            page.doc.value(i, "id").to_owned()
                        } else {
                            page.doc.value(i, "text").to_owned()
                        }
                    } else {
                        style.get("text").trim_matches(['\'', '"']).to_owned()
                    };
                    Some((
                        i,
                        *b,
                        page.doc.nodes[i].name.clone(),
                        label,
                        style,
                        i == page.selected,
                    ))
                })
                .collect()
        };
        for (_i, b, name, label, style, selected) in render_items {
            if !b.is_finite() {
                continue;
            }
            let r = Rect::from_min_size(origin + b.min.to_vec2() * self.zoom, b.size() * self.zoom);
            if !r.intersects(area) {
                continue;
            }
            let opacity = style
                .get("opacity")
                .parse::<f32>()
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let background = parse_color(style.get("background-color"))
                .unwrap_or_else(|| {
                    if name.contains("Label") {
                        Color32::TRANSPARENT
                    } else {
                        Color32::from_rgb(34, 49, 63)
                    }
                })
                .gamma_multiply(opacity);
            if background != Color32::TRANSPARENT {
                painter.rect_filled(r, 3.0, background);
            }
            if let Some(path) = resolve_asset(&self.settings.root, style.get("image-source")) {
                if let Some(texture) = self.cached_texture(&path) {
                    paint_otui_image(&painter, &texture, r, &style, opacity);
                }
            } else if !name.contains("Label") {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(1.0, Color32::from_rgb(68, 89, 106)),
                    StrokeKind::Inside,
                );
            }
            let offset = pair(style.get("text-offset")).unwrap_or(Vec2::ZERO) * self.zoom;
            painter.with_clip_rect(r.intersect(area)).text(
                r.center() + offset,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(13.0 * self.zoom),
                parse_color(style.get("color"))
                    .unwrap_or(Color32::from_rgb(220, 231, 239))
                    .gamma_multiply(opacity),
            );
            if selected && !self.interact {
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, ACCENT), StrokeKind::Inside);
                painter.rect_filled(Rect::from_center_size(r.max, Vec2::splat(7.0)), 0.0, ACCENT);
            }
        }
    }

    fn behavior_canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Fluxo visual do módulo");
            ui.separator();
            ui.label("Eventos e ações gravados diretamente no OTUI");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{}%", (self.zoom * 100.0) as i32));
            });
        });
        let selected = self.pages.get(self.active).map_or(0, |page| page.selected);
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("behavior-event")
                .selected_text(&self.behavior_event)
                .show_ui(ui, |ui| {
                    for event in [
                        "@onClick",
                        "@onDoubleClick",
                        "@onHoverChange",
                        "@onFocusChange",
                        "@onTextChange",
                        "@onEnter",
                        "@onEscape",
                    ] {
                        ui.selectable_value(&mut self.behavior_event, event.into(), event);
                    }
                });
            egui::ComboBox::from_id_salt("behavior-action")
                .selected_text(self.behavior_action.label())
                .show_ui(ui, |ui| {
                    for action in ActionTemplate::ALL {
                        ui.selectable_value(&mut self.behavior_action, action, action.label());
                    }
                });
            ui.add(
                egui::TextEdit::singleline(&mut self.behavior_target)
                    .hint_text("ID do widget alvo"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.behavior_argument).hint_text(
                    if self.behavior_action == ActionTemplate::Custom {
                        "expressão Lua"
                    } else {
                        "mensagem/opcional"
                    },
                ),
            );
            if ui.button("Adicionar ação").clicked() {
                let key = self.behavior_event.clone();
                let target = behavior::widget_reference(&self.behavior_target);
                let expression = self
                    .behavior_action
                    .expression(&target, &self.behavior_argument);
                if expression.is_empty() {
                    self.status = "Informe uma expressão Lua válida.".into();
                } else {
                    self.mutate("Criar comportamento visual", |document| {
                        document.set(selected, &key, &expression)
                    });
                }
            }
        });
        if let Some(page) = self.pages.get(self.active) {
            let current = behavior::events(&page.doc, selected);
            if current.is_empty() {
                ui.label(
                    egui::RichText::new("O widget selecionado ainda não possui eventos.")
                        .small()
                        .color(Color32::GRAY),
                );
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Eventos atuais:");
                    for (event, expression) in current {
                        ui.label(
                            egui::RichText::new(format!("{event} → {expression}"))
                                .small()
                                .color(ACCENT),
                        );
                    }
                });
            }
        }
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        painter.rect_filled(response.rect, 0.0, CANVAS);
        let step = 18.0;
        let mut x = response.rect.left();
        while x < response.rect.right() {
            let mut y = response.rect.top();
            while y < response.rect.bottom() {
                painter.circle_filled(Pos2::new(x, y), 0.7, Color32::from_rgb(43, 58, 70));
                y += step;
            }
            x += step;
        }
        let Some(page) = self.pages.get(self.active) else {
            return;
        };
        let nodes: Vec<_> = page
            .doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.widget)
            .take(24)
            .map(|(index, node)| {
                let depth = node.indent.min(5) as f32;
                let column = depth;
                let row = index as f32;
                let rect = Rect::from_min_size(
                    response.rect.min + Vec2::new(32.0 + column * 190.0, 34.0 + (row % 7.0) * 82.0),
                    Vec2::new(156.0, 58.0),
                );
                let title = if page.doc.value(index, "id").is_empty() {
                    node.name.clone()
                } else {
                    page.doc.value(index, "id").to_owned()
                };
                (index, node.parent, rect, title, index == page.selected)
            })
            .collect();
        let rect_by_index: HashMap<usize, Rect> = nodes
            .iter()
            .map(|(index, _, rect, _, _)| (*index, *rect))
            .collect();
        for (_, parent, rect, _, _) in &nodes {
            if let Some(parent_rect) = parent.and_then(|p| rect_by_index.get(&p)) {
                painter.line_segment(
                    [parent_rect.right_center(), rect.left_center()],
                    Stroke::new(1.5, ACCENT.gamma_multiply(0.75)),
                );
            }
        }
        let mut selected = None;
        for (index, _, rect, title, active) in nodes {
            let visible = rect.intersect(response.rect);
            if visible.is_negative() {
                continue;
            }
            let node_response =
                ui.interact(rect, egui::Id::new(("flow", index)), egui::Sense::click());
            if node_response.clicked() {
                selected = Some(index);
            }
            painter.rect_filled(
                rect,
                6.0,
                if active {
                    Color32::from_rgb(27, 84, 88)
                } else {
                    PANEL_RAISED
                },
            );
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(
                    1.0,
                    if active {
                        ACCENT
                    } else {
                        Color32::from_rgb(63, 82, 96)
                    },
                ),
                StrokeKind::Inside,
            );
            painter.text(
                rect.left_top() + Vec2::new(12.0, 10.0),
                Align2::LEFT_TOP,
                title,
                FontId::proportional(14.0),
                Color32::from_rgb(226, 235, 241),
            );
            painter.text(
                rect.left_bottom() + Vec2::new(12.0, -10.0),
                Align2::LEFT_BOTTOM,
                "Widget OTUI",
                FontId::proportional(11.0),
                Color32::from_rgb(143, 161, 174),
            );
        }
        if let Some(index) = selected {
            if let Some(page) = self.pages.get_mut(self.active) {
                page.selected = index;
            }
        }
    }

    fn bottom_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, label) in [
                (BottomPanel::Resources, "Recursos"),
                (BottomPanel::History, "Histórico"),
                (BottomPanel::Problems, "Problemas"),
                (BottomPanel::Output, "Saída"),
            ] {
                if ui
                    .selectable_label(self.bottom_panel == tab, label)
                    .clicked()
                {
                    self.bottom_panel = tab;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Local do log").clicked() {
                    self.status = self.log_path.display().to_string();
                }
            });
        });
        ui.separator();
        match self.bottom_panel {
            BottomPanel::Resources => {
                let counts = self.assets.counts();
                ui.label(format!(
                    "{} imagens • {} fontes • {} OTUI • {} Lua • {} OTMOD",
                    counts[AssetKind::Image as usize],
                    counts[AssetKind::Font as usize],
                    counts[AssetKind::Interface as usize],
                    counts[AssetKind::Script as usize],
                    counts[AssetKind::Module as usize]
                ));
                if let Some(texture) = &self.asset {
                    ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(Vec2::new(180.0, 90.0))
                            .maintain_aspect_ratio(true),
                    );
                }
            }
            BottomPanel::History => {
                if let Some(page) = self.pages.get(self.active) {
                    for label in page.history.labels() {
                        ui.label(label);
                    }
                }
            }
            BottomPanel::Problems => {
                if let Some(page) = self.pages.get(self.active) {
                    if page.doc.issues.is_empty() {
                        ui.colored_label(ACCENT, "Nenhum problema detectado no documento.");
                    }
                    for issue in &page.doc.issues {
                        ui.colored_label(Color32::from_rgb(237, 177, 72), issue);
                    }
                }
            }
            BottomPanel::Output => {
                ui.label(&self.preview.status);
                if let Some(page) = self.pages.get(self.active) {
                    ui.label(format!(
                        "{} • {} nós • {:.1} KiB",
                        page.doc.encoding(),
                        page.doc.nodes.len(),
                        page.doc.bytes().len() as f32 / 1024.0
                    ));
                }
            }
        }
    }

    fn resource_workspace(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Biblioteca do projeto");
            ui.add(
                egui::TextEdit::singleline(&mut self.asset_filter)
                    .hint_text("Filtrar nome, pasta ou tipo…"),
            );
            if ui.button("Reindexar").clicked() {
                let root = self.settings.root.clone();
                self.assets = AssetCatalog::scan(&root);
                self.texture_cache.clear();
            }
        });
        let counts = self.assets.counts();
        ui.label(format!("{} recursos indexados sob demanda • imagens {} • fontes {} • interfaces {} • scripts {} • módulos {}{}",
            self.assets.entries.len(), counts[0], counts[1], counts[2], counts[3], counts[4], if self.assets.truncated { " • limite de segurança atingido" } else { "" }));
        ui.separator();
        let needle = self.asset_filter.to_ascii_lowercase();
        let entries: Vec<_> = self
            .assets
            .entries
            .iter()
            .filter(|entry| {
                needle.is_empty()
                    || entry.relative.to_ascii_lowercase().contains(&needle)
                    || entry.kind.label().to_ascii_lowercase().contains(&needle)
            })
            .take(2_000)
            .cloned()
            .collect();
        let mut open = None;
        egui::ScrollArea::vertical()
            .id_salt("asset-catalog")
            .show(ui, |ui| {
                egui::Grid::new("asset-grid")
                    .num_columns(3)
                    .spacing(Vec2::new(18.0, 8.0))
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in entries {
                            if ui.selectable_label(false, entry.kind.label()).clicked() {
                                open = Some(entry.path.clone());
                            }
                            if ui.selectable_label(false, &entry.relative).double_clicked() {
                                open = Some(entry.path.clone());
                            }
                            ui.label(format_size(entry.bytes));
                            ui.end_row();
                        }
                    });
            });
        if let Some(path) = open {
            if path.extension().is_some_and(|extension| {
                ["png", "jpg", "jpeg", "bmp"]
                    .iter()
                    .any(|value| extension.eq_ignore_ascii_case(value))
            }) {
                self.image(ui.ctx(), &path, false);
            } else {
                self.open(&path);
            }
        }
    }
    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriedades");
        let Some(page) = self.pages.get(self.active) else {
            return;
        };
        let n = page.selected;
        let Some(node) = page.doc.nodes.get(n).cloned() else {
            ui.label("Este documento é editado em Código.");
            return;
        };
        ui.label(egui::RichText::new(&node.name).color(ACCENT));
        egui::ScrollArea::vertical()
            .id_salt("properties")
            .max_height(ui.available_height() * 0.6)
            .show(ui, |ui| {
                for property in node.properties {
                    ui.horizontal(|ui| {
                        ui.label(&property.key);
                        if ui
                            .small_button("×")
                            .on_hover_text("Remover propriedade")
                            .clicked()
                        {
                            self.mutate("Remover propriedade", |d| {
                                d.remove_property(n, &property.key)
                            });
                        }
                    });
                    let id = egui::Id::new(("property", self.active, n, &property.key));
                    let mut value = ui
                        .memory(|m| m.data.get_temp::<String>(id))
                        .unwrap_or_else(|| property.value.clone());
                    let r = ui.add_enabled(
                        !property.block,
                        egui::TextEdit::singleline(&mut value)
                            .id(id)
                            .desired_width(f32::INFINITY),
                    );
                    if r.changed() {
                        ui.memory_mut(|m| m.data.insert_temp(id, value.clone()));
                    }
                    if r.lost_focus() {
                        ui.memory_mut(|m| m.data.remove::<String>(id));
                        if value != property.value {
                            self.mutate("Alterar propriedade", |d| d.set(n, &property.key, &value));
                        }
                    }
                }
            });
        ui.separator();
        ui.label("Adicionar / definir propriedade");
        ui.text_edit_singleline(&mut self.property_key);
        ui.text_edit_singleline(&mut self.property_value);
        if ui.button("Aplicar propriedade").clicked() {
            let k = self.property_key.clone();
            let v = self.property_value.clone();
            self.mutate("Definir propriedade", |d| d.set(n, &k, &v));
        }
        ui.separator();
        ui.label("Anchors — vínculo ao pai");
        ui.horizontal_wrapped(|ui| {
            for (label, key, value) in [
                ("Esquerda", "anchors.left", "parent.left"),
                ("Topo", "anchors.top", "parent.top"),
                ("Direita", "anchors.right", "parent.right"),
                ("Base", "anchors.bottom", "parent.bottom"),
                ("Centro", "anchors.centerIn", "parent"),
                ("Preencher", "anchors.fill", "parent"),
            ] {
                if ui.button(label).clicked() {
                    self.mutate("Definir anchor", |d| d.set(n, key, value));
                }
            }
        });
        if ui.button("Limpar anchors").clicked() {
            self.mutate("Limpar anchors", |d| {
                let keys: Vec<String> = d.nodes[n]
                    .properties
                    .iter()
                    .filter(|p| p.key.starts_with("anchors."))
                    .map(|p| p.key.clone())
                    .collect();
                for k in keys {
                    d.remove_property(n, &k)?;
                }
                Ok(())
            });
        }
        ui.label("Margens e tamanho: use margin-left/top/right/bottom e size.");
        if ui.button("Excluir elemento").clicked() {
            self.mutate("Excluir elemento", |d| d.remove(n));
        }
    }
}

fn visual_state(name: &str, hovered: bool, pressed: bool) -> VisualState {
    let mut state = VisualState {
        hovered,
        pressed,
        ..Default::default()
    };
    match name {
        "Normal" => state = VisualState::default(),
        "Hover" => state.hovered = true,
        "Pressionado" => state.pressed = true,
        "Desabilitado" => state.disabled = true,
        "Marcado" => state.checked = true,
        "Ligado" => state.on = true,
        "Foco" => state.focused = true,
        _ => {}
    }
    state
}

fn parse_color(value: &str) -> Option<Color32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("transparent") || value.eq_ignore_ascii_case("alpha") {
        return Some(Color32::TRANSPARENT);
    }
    let hex = value.strip_prefix('#')?;
    let number = u32::from_str_radix(hex, 16).ok()?;
    match hex.len() {
        6 => Some(Color32::from_rgb(
            (number >> 16) as u8,
            (number >> 8) as u8,
            number as u8,
        )),
        8 => Some(Color32::from_rgba_unmultiplied(
            (number >> 24) as u8,
            (number >> 16) as u8,
            (number >> 8) as u8,
            number as u8,
        )),
        _ => None,
    }
}

fn pair(value: &str) -> Option<Vec2> {
    let values: Vec<f32> = value
        .split_whitespace()
        .filter_map(|part| part.parse().ok())
        .collect();
    (values.len() == 2).then(|| Vec2::new(values[0], values[1]))
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn paint_otui_image(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    target: Rect,
    style: &ResolvedStyle,
    opacity: f32,
) {
    let size = texture.size_vec2();
    let clip: Vec<f32> = style
        .get("image-clip")
        .split_whitespace()
        .filter_map(|part| part.parse().ok())
        .collect();
    let mut source = if clip.len() == 4 {
        Rect::from_min_size(Pos2::new(clip[0], clip[1]), Vec2::new(clip[2], clip[3]))
    } else {
        Rect::from_min_size(Pos2::ZERO, size)
    };
    if style.get("image-fixed-ratio").eq_ignore_ascii_case("true") && target.is_positive() {
        let source_aspect = source.width() / source.height();
        let target_aspect = target.width() / target.height();
        if source_aspect > target_aspect {
            let width = source.height() * target_aspect;
            source = Rect::from_center_size(source.center(), Vec2::new(width, source.height()));
        } else {
            let height = source.width() / target_aspect;
            source = Rect::from_center_size(source.center(), Vec2::new(source.width(), height));
        }
    }
    let tint = parse_color(style.get("image-color"))
        .unwrap_or(Color32::WHITE)
        .gamma_multiply(opacity);
    let border = style
        .get("image-border")
        .parse::<f32>()
        .unwrap_or(0.0)
        .max(0.0);
    if border <= 0.0 || source.width() <= border * 2.0 || source.height() <= border * 2.0 {
        painter.image(texture.id(), target, pixel_uv(source, size), tint);
        return;
    }
    let left = style
        .get("image-border-left")
        .parse::<f32>()
        .unwrap_or(border)
        .min(source.width() / 2.0);
    let right = style
        .get("image-border-right")
        .parse::<f32>()
        .unwrap_or(border)
        .min(source.width() / 2.0);
    let top = style
        .get("image-border-top")
        .parse::<f32>()
        .unwrap_or(border)
        .min(source.height() / 2.0);
    let bottom = style
        .get("image-border-bottom")
        .parse::<f32>()
        .unwrap_or(border)
        .min(source.height() / 2.0);
    let dx = [
        target.left(),
        (target.left() + left).min(target.center().x),
        (target.right() - right).max(target.center().x),
        target.right(),
    ];
    let dy = [
        target.top(),
        (target.top() + top).min(target.center().y),
        (target.bottom() - bottom).max(target.center().y),
        target.bottom(),
    ];
    let sx = [
        source.left(),
        source.left() + left,
        source.right() - right,
        source.right(),
    ];
    let sy = [
        source.top(),
        source.top() + top,
        source.bottom() - bottom,
        source.bottom(),
    ];
    for row in 0..3 {
        for column in 0..3 {
            let destination = Rect::from_min_max(
                Pos2::new(dx[column], dy[row]),
                Pos2::new(dx[column + 1], dy[row + 1]),
            );
            let source_part = Rect::from_min_max(
                Pos2::new(sx[column], sy[row]),
                Pos2::new(sx[column + 1], sy[row + 1]),
            );
            if destination.is_positive() && source_part.is_positive() {
                painter.image(texture.id(), destination, pixel_uv(source_part, size), tint);
            }
        }
    }
}

fn pixel_uv(rect: Rect, texture_size: Vec2) -> Rect {
    Rect::from_min_max(
        Pos2::new(rect.left() / texture_size.x, rect.top() / texture_size.y),
        Pos2::new(
            rect.right() / texture_size.x,
            rect.bottom() / texture_size.y,
        ),
    )
}

impl eframe::App for Studio {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.frames += 1;
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing && !self.smoke {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            let mut allowed = true;
            for i in 0..self.pages.len() {
                self.active = i;
                if !self.confirm_page() {
                    allowed = false;
                    break;
                }
            }
            if allowed {
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if let Some(image) = self.preview.poll() {
            self.image(&ctx, &image, true);
        }
        if self.preview.running() {
            ctx.request_repaint_after(Duration::from_millis(350));
        }
        if self
            .pending
            .is_some_and(|t| t.elapsed() > Duration::from_millis(400))
        {
            if self.live && self.preview.running() {
                self.send_preview();
            }
            self.pending = None;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.start_preview();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.save(false);
        }
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                self.undo(false);
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
                self.undo(true);
            }
        }
        egui::Panel::top("application_chrome")
            .exact_size(36.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(6.0))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("◆  NextGen Studio").strong());
                    ui.separator();
                    ui.menu_button("Arquivo", |ui| {
                        if ui.button("Novo").clicked() {
                            self.new_document();
                            ui.close();
                        }
                        if ui.button("Abrir…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Módulos", &["otui", "lua", "otmod", "html", "css"])
                                .pick_file()
                            {
                                self.open(&path);
                            }
                            ui.close();
                        }
                        if ui.button("Salvar").clicked() {
                            self.save(false);
                            ui.close();
                        }
                        if ui.button("Salvar como…").clicked() {
                            self.save(true);
                            ui.close();
                        }
                    });
                    ui.menu_button("Editar", |ui| {
                        if ui.button("Desfazer   Ctrl+Z").clicked() {
                            self.undo(false);
                            ui.close();
                        }
                        if ui.button("Refazer     Ctrl+Y").clicked() {
                            self.undo(true);
                            ui.close();
                        }
                        if ui.button("Adicionar elemento").clicked() {
                            self.add_dialog = true;
                            ui.close();
                        }
                    });
                    ui.menu_button("Exibir", |ui| {
                        ui.checkbox(&mut self.grid, "Grid");
                        ui.checkbox(&mut self.snap, "Snapping");
                        ui.checkbox(&mut self.live, "Atualização automática");
                    });
                    ui.menu_button("Projeto", |ui| {
                        if ui.button("Abrir pasta do projeto…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.load_project(path);
                            }
                            ui.close();
                        }
                        if ui.button("Atualizar arquivos").clicked() {
                            self.files.clear();
                            ui.close();
                        }
                    });
                    ui.menu_button("Ferramentas", |ui| {
                        if ui.button("Validar no motor NextGen   F5").clicked() {
                            self.start_preview();
                            ui.close();
                        }
                        if ui.button("Parar validação nativa").clicked() {
                            self.preview.stop();
                            ui.close();
                        }
                    });
                    ui.menu_button("Ajuda", |ui| {
                        ui.label(concat!(
                            "NextGen Studio ",
                            env!("CARGO_PKG_VERSION"),
                            " • Rust"
                        ));
                        ui.label("Editor desktop independente");
                    });
                });
            });
        egui::Panel::top("workspace_navigation")
            .exact_size(46.0)
            .frame(egui::Frame::new().fill(PANEL_RAISED).inner_margin(8.0))
            .show(root, |ui| {
            ui.horizontal(|ui| {
                for (workspace, icon, label) in [
                    (Workspace::Interface, "▦", "Interface"),
                    (Workspace::Behaviors, "⌘", "Comportamentos"),
                    (Workspace::Resources, "▦", "Recursos"),
                    (Workspace::Test, "▶", "Teste"),
                ] {
                    let active = self.workspace == workspace;
                    if ui
                        .selectable_label(active, egui::RichText::new(format!("{icon}  {label}")).size(14.0))
                        .clicked() { self.workspace = workspace; }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("▶  Validar no NextGen").fill(Color32::from_rgb(18, 105, 116))).on_hover_text("Opcional: abre um processo separado do client para conferir fidelidade").clicked() {
                        self.start_preview();
                    }
                    if ui.button("Salvar").clicked() { self.save(false); }
                });
            });
        });
        egui::Panel::bottom("status")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(6.0))
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                    ui.separator();
                    ui.label(if cfg!(debug_assertions) {
                        "DEBUG • console ativo"
                    } else {
                        "RELEASE"
                    });
                });
            });
        egui::Panel::bottom("workspace_bottom")
            .resizable(true)
            .default_size(112.0)
            .min_size(72.0)
            .max_size(240.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(8.0))
            .show(root, |ui| {
                self.bottom_content(ui);
            });
        egui::Panel::left("project")
            .resizable(true)
            .default_size(240.0)
            .show(root, |ui| {
                ui.strong("Projeto");
                ui.horizontal(|ui| {
                    if ui.button("Abrir projeto…").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.load_project(p);
                        }
                    }
                    if ui.small_button("↻").clicked() {
                        self.files.clear();
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.project_filter)
                        .hint_text("Buscar no projeto…"),
                );
                let project = self.settings.root.clone();
                egui::ScrollArea::vertical()
                    .id_salt("files")
                    .max_height(ui.available_height() * 0.42)
                    .show(ui, |ui| {
                        if project.is_dir() {
                            self.tree(ui, &project, 0);
                        } else {
                            ui.label("Selecione qualquer pasta de projeto.");
                        }
                    });
                ui.separator();
                ui.strong("Hierarquia");
                ui.add(
                    egui::TextEdit::singleline(&mut self.hierarchy_filter)
                        .hint_text("Buscar na hierarquia…"),
                );
                if let Some(p) = self.pages.get_mut(self.active) {
                    egui::ScrollArea::vertical().id_salt("tree").show(ui, |ui| {
                        for (i, n) in p.doc.nodes.iter().take(10000).enumerate() {
                            let caption = format!("{}  {}", n.name, p.doc.value(i, "id"));
                            if !self.hierarchy_filter.is_empty()
                                && !caption
                                    .to_lowercase()
                                    .contains(&self.hierarchy_filter.to_lowercase())
                            {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                ui.add_space((n.indent.min(24) * 5) as f32);
                                if ui.selectable_label(p.selected == i, caption).clicked() {
                                    p.selected = i;
                                }
                            });
                        }
                    });
                }
            });
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(280.0)
            .show(root, |ui| {
                if matches!(self.workspace, Workspace::Interface | Workspace::Behaviors) {
                    self.properties(ui);
                } else if self.workspace == Workspace::Test {
                    ui.heading("Cenário de teste");
                    ui.checkbox(&mut self.interact, "Interação ativada");
                    ui.label("Passe o mouse e clique nos widgets. Os eventos são simulados localmente e nunca executam Lua arbitrário.");
                    if ui.button("Limpar eventos").clicked() { self.simulation_log.clear(); }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for event in self.simulation_log.iter().rev() { ui.label(event); }
                    });
                } else {
                    ui.heading("Detalhes do recurso");
                    ui.label("Selecione um item para editar suas opções.");
                }
                ui.separator();
                if let Some(t) = &self.asset {
                    ui.add(
                        egui::Image::new(t)
                            .fit_to_exact_size(Vec2::new(240.0, 160.0))
                            .maintain_aspect_ratio(true),
                    );
                }
            });
        egui::CentralPanel::default().show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, p) in self.pages.iter().enumerate() {
                    let name = p
                        .doc
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or("Novo documento".into());
                    if ui
                        .selectable_label(
                            self.active == i,
                            format!("{name}{}", if p.doc.dirty() || p.draft { " *" } else { "" }),
                        )
                        .clicked()
                    {
                        self.active = i;
                    }
                }
                if ui.small_button("Fechar aba").clicked() {
                    self.close_page();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(!self.code && !self.show_native, "Prévia offline")
                    .on_hover_text("Funciona sem abrir ou instalar o NextGen")
                    .clicked()
                {
                    self.code = false;
                    self.show_native = false;
                }
                if ui.selectable_label(self.code, "Código").clicked() {
                    self.code = true;
                    self.show_native = false;
                }
                if ui
                    .selectable_label(self.show_native, "Validação no motor")
                    .clicked()
                {
                    self.show_native = true;
                    self.code = false;
                }
                ui.separator();
                ui.label("Desktop  1280 × 720");
            });
            if self.show_native {
                ui.label("Validação opcional renderizada por um processo NextGen separado.");
                ui.label("Não é vídeo contínuo: a captura é renovada após cada revisão.");
                if let Some(t) = &self.texture {
                    ui.add(
                        egui::Image::new(t)
                            .max_size(ui.available_size())
                            .maintain_aspect_ratio(true),
                    );
                } else {
                    ui.label("Inicie a prévia nativa com F5 para receber a captura.");
                }
            } else if self.code || self.pages.get(self.active).is_some_and(|p| !p.doc.is_otui) {
                if let Some(p) = self.pages.get_mut(self.active) {
                    ui.label(
                        "Código OTUI / Lua / OTMOD • aplicar registra uma única ação no histórico.",
                    );
                    let apply = ui.button("Aplicar código").clicked();
                    egui::ScrollArea::both().id_salt("source").show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut p.source)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(32),
                            )
                            .changed()
                        {
                            p.draft = true;
                        }
                    });
                    if apply {
                        let text = p.source.clone();
                        self.mutate("Editar código", |d| d.replace_text(&text));
                    }
                }
            } else {
                match self.workspace {
                    Workspace::Interface | Workspace::Test => self.canvas(ui),
                    Workspace::Behaviors => self.behavior_canvas(ui),
                    Workspace::Resources => self.resource_workspace(ui),
                }
            }
        });
        if self.add_dialog {
            let mut open = true;
            egui::Window::new("Adicionar elemento")
                .open(&mut open)
                .show(&ctx, |ui| {
                    ui.label("Tipo de widget");
                    ui.text_edit_singleline(&mut self.new_type);
                    ui.label("ID");
                    ui.text_edit_singleline(&mut self.new_id);
                    if ui.button("Adicionar ao selecionado").clicked() {
                        let kind = self.new_type.clone();
                        let id = self.new_id.clone();
                        let n = self.pages.get(self.active).map(|p| p.selected);
                        self.mutate("Adicionar elemento", |d| d.add(n, &kind, &id));
                        self.add_dialog = false;
                    }
                });
            self.add_dialog &= open;
        }
        if self.smoke {
            ctx.request_repaint_after(Duration::from_millis(30));
            if self.frames > 5 {
                log::info!("Desktop smoke passed");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.preview.stop();
        if !self.smoke {
            if let Ok(bytes) = serde_json::to_vec_pretty(&self.settings) {
                let _ = crate::document::atomic_write(&self.settings_path, &bytes);
            }
        }
        log::info!("Encerramento normal");
        log::logger().flush();
    }
}
