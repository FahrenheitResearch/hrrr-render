/// HRRR Interactive Weather Map Viewer
///
/// Pan/zoom map of HRRR weather model data overlaid on rustmaps dark-theme
/// Natural Earth base maps with Lambert Conformal Conic projection.
/// Features: categorized field selector, animation loop, GIF export, frame caching.

use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hrrr_render::fields::{self, FIELDS, field_groups, fields_in_group};
use hrrr_render::render::projection::LambertProjection;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("HRRR Weather Viewer")
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "hrrr-gui",
        options,
        Box::new(|cc| Ok(Box::new(HrrrApp::new(cc)))),
    )
}

/// A rendered frame that can be cached.
#[derive(Clone)]
struct CachedFrame {
    pixels: Vec<u8>,    // flat RGBA
    width: u32,
    height: u32,
    values: Vec<f64>,   // for cursor readout
    nx: usize,
    ny: usize,
}

/// Data arriving from background thread.
struct IncomingFrame {
    frame: CachedFrame,
    forecast_hour: u8,
}

/// Shared state between UI and background fetch thread.
struct FetchState {
    incoming: Option<IncomingFrame>,
    status: String,
    fetching: bool,
}

/// GIF export progress.
struct GifProgress {
    current: u8,
    total: u8,
    done: bool,
    error: Option<String>,
    path: Option<String>,
}

/// Generate available model run options (last 24 hours of cycles).
fn available_runs() -> Vec<(String, String)> {
    use chrono::{Utc, Duration, Timelike};
    let now = Utc::now();
    let mut runs = vec![("latest".to_string(), "Latest".to_string())];
    for hours_ago in 2..26 {
        let t = now - Duration::hours(hours_ago);
        let code = t.format("%Y%m%d%H").to_string();
        let label = format!("{} {:02}z", t.format("%m/%d"), t.hour());
        runs.push((code, label));
    }
    runs
}

struct HrrrApp {
    fetch_state: Arc<Mutex<FetchState>>,
    texture: Option<egui::TextureHandle>,
    tex_size: [u32; 2],

    // UI state
    selected_field: usize,
    run_mode: String,
    run_options: Vec<(String, String)>,
    forecast_hour: u8,
    render_width: u32,
    render_height: u32,

    // Pan/zoom
    pan: egui::Vec2,
    zoom: f32,

    // Projection
    proj: LambertProjection,

    // Current cursor readout data
    cached_values: Option<Vec<f64>>,
    cached_nx: usize,
    cached_ny: usize,
    cached_field_idx: usize,
    cached_img_width: u32,

    // Frame cache: keyed by forecast_hour, valid for current field+run
    frame_cache: HashMap<u8, CachedFrame>,
    cache_field_idx: usize,
    cache_run_mode: String,

    // Animation
    animating: bool,
    anim_start: u8,
    anim_end: u8,
    anim_speed_ms: u64,
    last_anim_advance: Instant,

    // GIF export
    gif_progress: Arc<Mutex<GifProgress>>,
    exporting_gif: bool,

    first_frame: bool,
}

impl HrrrApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let default_field = FIELDS.iter().position(|f| f.name == "ref").unwrap_or(0);

        Self {
            fetch_state: Arc::new(Mutex::new(FetchState {
                incoming: None,
                status: "Ready".to_string(),
                fetching: false,
            })),
            texture: None,
            tex_size: [0, 0],
            selected_field: default_field,
            run_mode: "latest".to_string(),
            run_options: available_runs(),
            forecast_hour: 0,
            render_width: 1799,
            render_height: 1059,
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            proj: LambertProjection::hrrr_default(),
            cached_values: None,
            cached_nx: 1799,
            cached_ny: 1059,
            cached_field_idx: default_field,
            cached_img_width: 1799 + 60,
            frame_cache: HashMap::new(),
            cache_field_idx: default_field,
            cache_run_mode: "latest".to_string(),
            animating: false,
            anim_start: 0,
            anim_end: 18,
            anim_speed_ms: 250,
            last_anim_advance: Instant::now(),
            gif_progress: Arc::new(Mutex::new(GifProgress {
                current: 0, total: 0, done: false, error: None, path: None,
            })),
            exporting_gif: false,
            first_frame: true,
        }
    }

    /// Invalidate cache if field or run changed.
    fn check_cache_validity(&mut self) {
        if self.cache_field_idx != self.selected_field || self.cache_run_mode != self.run_mode {
            self.frame_cache.clear();
            self.cache_field_idx = self.selected_field;
            self.cache_run_mode = self.run_mode.clone();
        }
    }

    /// Try to load a frame from cache. Returns true if found.
    fn load_from_cache(&mut self, fhour: u8, ctx: &egui::Context) -> bool {
        self.check_cache_validity();
        if let Some(cached) = self.frame_cache.get(&fhour) {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [cached.width as usize, cached.height as usize],
                &cached.pixels,
            );
            self.texture = Some(ctx.load_texture(
                "hrrr_map", color_image, egui::TextureOptions::LINEAR,
            ));
            self.tex_size = [cached.width, cached.height];
            self.cached_values = Some(cached.values.clone());
            self.cached_nx = cached.nx;
            self.cached_ny = cached.ny;
            self.cached_field_idx = self.selected_field;
            self.cached_img_width = cached.width;
            self.forecast_hour = fhour;

            let field = &FIELDS[self.selected_field];
            let mut s = self.fetch_state.lock().unwrap();
            s.status = format!("{} f{:02} (cached)", field.label, fhour);
            true
        } else {
            false
        }
    }

    fn start_fetch(&mut self, ctx: &egui::Context) {
        self.check_cache_validity();

        // Try cache first
        if self.load_from_cache(self.forecast_hour, ctx) {
            return;
        }

        let state = Arc::clone(&self.fetch_state);
        {
            let mut s = state.lock().unwrap();
            if s.fetching { return; }
            s.fetching = true;
            s.status = "Fetching...".to_string();
        }

        let field = FIELDS[self.selected_field].clone();
        let run = self.run_mode.clone();
        let fhour = self.forecast_hour;
        let width = self.render_width;
        let height = self.render_height;
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let result = fetch_and_render(&field, &run, fhour, width, height, &state, &ctx);
            let mut s = state.lock().unwrap();
            match result {
                Ok((frame, date, run_hour, ms)) => {
                    s.incoming = Some(IncomingFrame { frame, forecast_hour: fhour });
                    s.status = format!("{} | {} {:02}z f{:02} | {:.0}ms",
                        field.label, date, run_hour, fhour, ms);
                }
                Err(msg) => {
                    s.status = msg;
                }
            }
            s.fetching = false;
            ctx.request_repaint();
        });
    }

    fn start_fetch_hour(&mut self, fhour: u8, ctx: &egui::Context) {
        self.forecast_hour = fhour;
        self.start_fetch(ctx);
    }

    fn export_gif(&mut self, ctx: &egui::Context) {
        if self.exporting_gif { return; }
        self.exporting_gif = true;

        let progress = Arc::clone(&self.gif_progress);
        {
            let mut p = progress.lock().unwrap();
            p.current = 0;
            p.total = self.anim_end - self.anim_start + 1;
            p.done = false;
            p.error = None;
            p.path = None;
        }

        let field = FIELDS[self.selected_field].clone();
        let run = self.run_mode.clone();
        let start = self.anim_start;
        let end = self.anim_end;
        let width = self.render_width;
        let height = self.render_height;
        let speed_ms = self.anim_speed_ms;
        // Clone cached frames we already have
        let existing_cache: HashMap<u8, CachedFrame> = self.frame_cache.clone();
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let result = do_gif_export(
                &field, &run, start, end, width, height, speed_ms,
                &existing_cache, &progress, &ctx,
            );
            let mut p = progress.lock().unwrap();
            match result {
                Ok(path) => {
                    p.path = Some(path);
                    p.done = true;
                }
                Err(e) => {
                    p.error = Some(e);
                    p.done = true;
                }
            }
            ctx.request_repaint();
        });
    }
}

/// Fetch and render a single frame (runs in background thread).
fn fetch_and_render(
    field: &hrrr_render::fields::FieldDef,
    run: &str,
    fhour: u8,
    width: u32,
    height: u32,
    state: &Arc<Mutex<FetchState>>,
    ctx: &egui::Context,
) -> Result<(CachedFrame, String, u8, f64), String> {
    let t0 = Instant::now();

    let (date, run_hour) = hrrr_render::fetch::parse_run(run)
        .map_err(|e| format!("Error: {}", e))?;

    {
        let mut s = state.lock().unwrap();
        s.status = format!("Fetching {} f{:02}...", field.label, fhour);
    }
    ctx.request_repaint();

    let grib_data = hrrr_render::fetch::fetch_field(
        &date, run_hour, fhour, field.idx_name, field.level
    ).map_err(|e| format!("Fetch error: {}", e))?;

    {
        let mut s = state.lock().unwrap();
        s.status = format!("Parsing {:.0} KB...", grib_data.len() as f64 / 1024.0);
    }
    ctx.request_repaint();

    let (mut values, nx, ny) = hrrr_render::parse_grib2_field(&grib_data)
        .map_err(|e| format!("Parse error: {}", e))?;

    fields::convert_values(field, &mut values);

    {
        let mut s = state.lock().unwrap();
        s.status = format!("Rendering f{:02}...", fhour);
    }
    ctx.request_repaint();

    let proj = LambertProjection::new(
        38.5, 38.5, -97.5, 21.138, -122.72,
        3000.0, 3000.0, nx as u32, ny as u32,
    );

    let (pixel_buf, img_width, img_height) =
        hrrr_render::render::render_to_pixels(&values, field, &proj, width, height);

    let render_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let flat: Vec<u8> = pixel_buf.iter().flat_map(|c| c.iter().copied()).collect();

    Ok((CachedFrame {
        pixels: flat,
        width: img_width,
        height: img_height,
        values,
        nx,
        ny,
    }, date, run_hour, render_ms))
}

/// Export animation frames to GIF.
fn do_gif_export(
    field: &hrrr_render::fields::FieldDef,
    run: &str,
    start: u8,
    end: u8,
    width: u32,
    height: u32,
    speed_ms: u64,
    existing_cache: &HashMap<u8, CachedFrame>,
    progress: &Arc<Mutex<GifProgress>>,
    ctx: &egui::Context,
) -> Result<String, String> {
    use std::fs::File;

    let path = format!("hrrr_{}_{}.gif", field.name,
        chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    // Collect all frames
    let mut frames: Vec<CachedFrame> = Vec::new();
    let dummy_state = Arc::new(Mutex::new(FetchState {
        incoming: None,
        status: String::new(),
        fetching: false,
    }));

    for fhour in start..=end {
        {
            let mut p = progress.lock().unwrap();
            p.current = fhour - start;
        }
        ctx.request_repaint();

        let frame = if let Some(cached) = existing_cache.get(&fhour) {
            cached.clone()
        } else {
            let (frame, _, _, _) = fetch_and_render(
                field, run, fhour, width, height, &dummy_state, ctx,
            )?;
            frame
        };
        frames.push(frame);
    }

    // Encode GIF
    if frames.is_empty() {
        return Err("No frames to export".to_string());
    }

    let first = &frames[0];
    let file = File::create(&path).map_err(|e| format!("File create error: {}", e))?;

    let mut encoder = gif::Encoder::new(
        file, first.width as u16, first.height as u16, &[]
    ).map_err(|e| format!("GIF encoder error: {}", e))?;

    encoder.set_repeat(gif::Repeat::Infinite)
        .map_err(|e| format!("GIF repeat error: {}", e))?;

    let delay = (speed_ms / 10) as u16; // centiseconds

    for (i, frame) in frames.iter().enumerate() {
        {
            let mut p = progress.lock().unwrap();
            p.current = (end - start + 1) + i as u8; // second pass indicator
        }
        ctx.request_repaint();

        let mut rgba = frame.pixels.clone();
        let mut gif_frame = gif::Frame::from_rgba_speed(
            frame.width as u16, frame.height as u16, &mut rgba, 10
        );
        gif_frame.delay = delay;

        encoder.write_frame(&gif_frame)
            .map_err(|e| format!("GIF write error: {}", e))?;
    }

    Ok(path)
}

impl eframe::App for HrrrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let accent = egui::Color32::from_rgb(0x00, 0xE5, 0xFF);
        let bg_panel = egui::Color32::from_rgb(0x25, 0x25, 0x35);
        let text_dim = egui::Color32::from_rgb(0x80, 0x80, 0x90);
        let border = egui::Color32::from_rgb(0x35, 0x35, 0x45);

        if self.first_frame {
            self.first_frame = false;
            self.start_fetch(ctx);
        }

        // Receive rendered frames and cache them
        {
            let mut s = self.fetch_state.lock().unwrap();
            if let Some(incoming) = s.incoming.take() {
                let fhour = incoming.forecast_hour;
                let frame = incoming.frame;

                // Update texture
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.pixels,
                );
                self.texture = Some(ctx.load_texture(
                    "hrrr_map", color_image, egui::TextureOptions::LINEAR,
                ));
                self.tex_size = [frame.width, frame.height];
                self.cached_values = Some(frame.values.clone());
                self.cached_nx = frame.nx;
                self.cached_ny = frame.ny;
                self.cached_field_idx = self.selected_field;
                self.cached_img_width = frame.width;

                // Store in cache
                self.frame_cache.insert(fhour, frame);
            }
        }

        // Check GIF export progress
        {
            let p = self.gif_progress.lock().unwrap();
            if p.done && self.exporting_gif {
                self.exporting_gif = false;
                if let Some(ref err) = p.error {
                    self.fetch_state.lock().unwrap().status = format!("GIF error: {}", err);
                } else if let Some(ref path) = p.path {
                    self.fetch_state.lock().unwrap().status = format!("GIF saved: {}", path);
                }
            }
        }

        // Animation logic
        if self.animating {
            let now = Instant::now();
            let fetching = self.fetch_state.lock().unwrap().fetching;
            if now.duration_since(self.last_anim_advance).as_millis() >= self.anim_speed_ms as u128
                && !fetching
            {
                let mut next = self.forecast_hour + 1;
                if next > self.anim_end { next = self.anim_start; }

                if !self.load_from_cache(next, ctx) {
                    self.start_fetch_hour(next, ctx);
                }
                self.last_anim_advance = now;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let fetching = self.fetch_state.lock().unwrap().fetching;

        // ── Left sidebar: Field selector ─────────────────────────────
        egui::SidePanel::left("fields_panel")
            .default_width(200.0)
            .max_width(280.0)
            .frame(egui::Frame::new()
                .fill(bg_panel)
                .inner_margin(egui::Margin::symmetric(8, 8))
                .stroke(egui::Stroke::new(1.0, border)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Fields").color(accent).strong().size(14.0));
                ui.add_space(4.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for group in field_groups() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(group)
                            .color(egui::Color32::from_rgb(0xA0, 0xA0, 0xB0))
                            .strong().size(11.0));
                        ui.add_space(2.0);

                        for field in fields_in_group(group) {
                            let idx = FIELDS.iter().position(|f| f.name == field.name).unwrap();
                            let is_active = self.selected_field == idx;
                            let text = if is_active {
                                egui::RichText::new(field.label)
                                    .color(egui::Color32::BLACK).strong().size(11.0)
                            } else {
                                egui::RichText::new(field.label)
                                    .color(egui::Color32::from_rgb(0xC0, 0xC0, 0xD0)).size(11.0)
                            };

                            let btn = egui::Button::new(text)
                                .corner_radius(egui::CornerRadius::same(3))
                                .min_size(egui::vec2(ui.available_width(), 20.0));
                            let btn = if is_active { btn.fill(accent) }
                                else { btn.fill(egui::Color32::TRANSPARENT) };

                            let response = ui.add(btn);
                            if response.clicked() && !fetching {
                                self.animating = false;
                                self.selected_field = idx;
                                self.start_fetch(ctx);
                            }
                            if response.hovered() && !is_active {
                                response.on_hover_text(format!("{} ({})", field.label, field.unit));
                            }
                        }
                    }
                });
            });

        // ── Top toolbar ──────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .exact_height(38.0)
            .frame(egui::Frame::new()
                .fill(bg_panel)
                .inner_margin(egui::Margin::symmetric(10, 4))
                .stroke(egui::Stroke::new(1.0, border)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    ui.label(egui::RichText::new("HRRR Viewer")
                        .color(accent).strong().size(14.0));
                    ui.separator();

                    // ── Run picker ───────────────────────────────
                    ui.label(egui::RichText::new("Run:").color(text_dim).size(11.0));
                    let current_label = self.run_options.iter()
                        .find(|(code, _)| code == &self.run_mode)
                        .map(|(_, label)| label.as_str())
                        .unwrap_or(&self.run_mode);

                    egui::ComboBox::from_id_salt("run_sel")
                        .selected_text(egui::RichText::new(current_label)
                            .color(accent).size(12.0).monospace())
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for (code, label) in &self.run_options {
                                if ui.selectable_label(self.run_mode == *code,
                                    egui::RichText::new(label).monospace().size(11.0)
                                ).clicked() {
                                    self.run_mode = code.clone();
                                    self.animating = false;
                                }
                            }
                        });
                    ui.separator();

                    // ── Forecast hour ────────────────────────────
                    ui.label(egui::RichText::new("Fhr:").color(text_dim).size(11.0));

                    let can_step = !fetching && !self.animating;
                    if ui.add_enabled(can_step && self.forecast_hour > 0,
                        egui::Button::new(egui::RichText::new("<").size(12.0).monospace())
                            .min_size(egui::vec2(20.0, 22.0))
                    ).clicked() {
                        self.forecast_hour -= 1;
                        self.start_fetch(ctx);
                    }

                    ui.label(egui::RichText::new(format!("f{:02}", self.forecast_hour))
                        .color(accent).size(13.0).monospace().strong());

                    if ui.add_enabled(can_step && self.forecast_hour < 48,
                        egui::Button::new(egui::RichText::new(">").size(12.0).monospace())
                            .min_size(egui::vec2(20.0, 22.0))
                    ).clicked() {
                        self.forecast_hour += 1;
                        self.start_fetch(ctx);
                    }
                    ui.separator();

                    // ── Play/Pause ───────────────────────────────
                    let play_text = if self.animating { "\u{23F8}" } else { "\u{25B6}" };
                    if ui.button(egui::RichText::new(play_text).size(14.0)).clicked() {
                        self.animating = !self.animating;
                        if self.animating {
                            self.last_anim_advance = Instant::now();
                        }
                    }

                    // Animation range
                    ui.label(egui::RichText::new("f").color(text_dim).size(10.0));
                    let mut start = self.anim_start as i32;
                    if ui.add(egui::DragValue::new(&mut start).range(0..=47).speed(0.2)
                        .custom_formatter(|n, _| format!("{:02}", n as i32))
                    ).changed() {
                        self.anim_start = start.clamp(0, 47) as u8;
                    }
                    ui.label(egui::RichText::new("-").color(text_dim).size(10.0));
                    let mut end = self.anim_end as i32;
                    if ui.add(egui::DragValue::new(&mut end).range(1..=48).speed(0.2)
                        .custom_formatter(|n, _| format!("{:02}", n as i32))
                    ).changed() {
                        self.anim_end = end.clamp(1, 48) as u8;
                    }

                    // Speed
                    ui.label(egui::RichText::new("ms:").color(text_dim).size(10.0));
                    let mut spd = self.anim_speed_ms as i32;
                    if ui.add(egui::DragValue::new(&mut spd).range(50..=2000).speed(5.0))
                        .changed()
                    {
                        self.anim_speed_ms = spd.clamp(50, 2000) as u64;
                    }

                    ui.separator();

                    // ── GIF Export ────────────────────────────────
                    if self.exporting_gif {
                        let p = self.gif_progress.lock().unwrap();
                        ui.label(egui::RichText::new(
                            format!("Exporting {}/{}...", p.current, p.total)
                        ).color(text_dim).size(11.0));
                    } else {
                        if ui.button(egui::RichText::new("GIF").size(11.0).strong())
                            .on_hover_text("Export loop as animated GIF").clicked()
                        {
                            self.export_gif(ctx);
                        }
                    }

                    ui.separator();

                    // ── Fetch button ─────────────────────────────
                    let btn_text = if fetching { "Loading..." } else { "Fetch" };
                    let btn = ui.add_enabled(!fetching,
                        egui::Button::new(
                            egui::RichText::new(btn_text).strong().size(12.0)
                                .color(if fetching { text_dim } else { egui::Color32::BLACK })
                        ).fill(if fetching { egui::Color32::from_rgb(0x40, 0x40, 0x50) } else { accent })
                         .min_size(egui::vec2(70.0, 24.0)),
                    );
                    if btn.clicked() {
                        self.start_fetch(ctx);
                    }

                    ui.separator();

                    // ── Zoom ─────────────────────────────────────
                    if ui.button(egui::RichText::new("-").size(14.0).monospace()).clicked() {
                        self.zoom = (self.zoom / 1.25).max(0.25);
                    }
                    ui.label(egui::RichText::new(format!("{:.0}%", self.zoom * 100.0))
                        .color(text_dim).size(11.0).monospace());
                    if ui.button(egui::RichText::new("+").size(14.0).monospace()).clicked() {
                        self.zoom = (self.zoom * 1.25).min(4.0);
                    }
                    if ui.button(egui::RichText::new("Fit").size(11.0)).clicked() {
                        self.zoom = 1.0;
                        self.pan = egui::Vec2::ZERO;
                    }

                    // Cache indicator
                    let cached_count = self.frame_cache.len();
                    if cached_count > 0 {
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{} cached", cached_count))
                            .color(text_dim).size(10.0));
                    }
                });
            });

        // ── Bottom status bar ────────────────────────────────────────
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .frame(egui::Frame::new()
                .fill(bg_panel)
                .inner_margin(egui::Margin::symmetric(10, 2))
                .stroke(egui::Stroke::new(1.0, border)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let status = self.fetch_state.lock().unwrap().status.clone();
                    ui.label(egui::RichText::new(&status).color(text_dim).size(11.0));
                });
            });

        // ── Central map ──────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(13, 17, 23)))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.dragged() {
                    self.pan += response.drag_delta();
                }

                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                    if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                        let center = rect.center();
                        let before = egui::vec2(
                            pointer.x - center.x - self.pan.x,
                            pointer.y - center.y - self.pan.y,
                        );
                        self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
                        let after = before * factor;
                        self.pan += before - after;
                    } else {
                        self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
                    }
                }

                if let Some(ref texture) = self.texture {
                    let img_w = self.tex_size[0] as f32;
                    let img_h = self.tex_size[1] as f32;
                    let fit_zoom = (rect.width() / img_w).min(rect.height() / img_h);
                    let display_w = img_w * self.zoom * fit_zoom;
                    let display_h = img_h * self.zoom * fit_zoom;

                    let center = rect.center();
                    let img_rect = egui::Rect::from_center_size(
                        egui::pos2(center.x + self.pan.x, center.y + self.pan.y),
                        egui::vec2(display_w, display_h),
                    );

                    ui.painter().image(
                        texture.id(), img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );

                    // Cursor readout
                    if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                        if rect.contains(pointer) && img_rect.contains(pointer) {
                            let img_x = (pointer.x - img_rect.min.x) / display_w * img_w;
                            let img_y = (pointer.y - img_rect.min.y) / display_h * img_h;
                            let data_w = self.cached_img_width.saturating_sub(60);

                            if img_x >= 0.0 && img_x < data_w as f32
                                && img_y >= 0.0 && img_y < img_h
                            {
                                let scale_x = self.cached_nx as f64 / data_w as f64;
                                let scale_y = self.cached_ny as f64 / img_h as f64;
                                let gi = img_x as f64 * scale_x;
                                let gj = (img_h as f64 - 1.0 - img_y as f64) * scale_y;
                                let (lat, lon) = self.proj.grid_to_latlon(gi, gj);

                                let val_text = if let Some(ref vals) = self.cached_values {
                                    let i = gi.round() as usize;
                                    let j = gj.round() as usize;
                                    if i < self.cached_nx && j < self.cached_ny {
                                        let idx = j * self.cached_nx + i;
                                        if idx < vals.len() && !vals[idx].is_nan() {
                                            let field = &FIELDS[self.cached_field_idx];
                                            format!("{:.1} {}", vals[idx], field.unit)
                                        } else { "N/A".into() }
                                    } else { String::new() }
                                } else { String::new() };

                                let lat_dir = if lat >= 0.0 { "N" } else { "S" };
                                let lon_dir = if lon >= 0.0 { "E" } else { "W" };
                                let readout = format!("{:.3}\u{00B0}{} {:.3}\u{00B0}{}  {}",
                                    lat.abs(), lat_dir, lon.abs(), lon_dir, val_text);

                                let tp = egui::pos2(pointer.x + 15.0, pointer.y - 20.0);
                                let font = egui::FontId::monospace(12.0);
                                let galley = ui.painter().layout_no_wrap(
                                    readout.clone(), font.clone(), egui::Color32::WHITE);
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(tp.x - 4.0, tp.y - 2.0),
                                        galley.size() + egui::vec2(8.0, 4.0)),
                                    4.0,
                                    egui::Color32::from_rgba_unmultiplied(20, 20, 30, 230));
                                ui.painter().text(tp, egui::Align2::LEFT_TOP, readout, font,
                                    egui::Color32::from_rgb(0xE0, 0xE0, 0xE0));

                                let stroke = egui::Stroke::new(1.0,
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60));
                                ui.painter().line_segment(
                                    [egui::pos2(pointer.x, rect.top()),
                                     egui::pos2(pointer.x, rect.bottom())], stroke);
                                ui.painter().line_segment(
                                    [egui::pos2(rect.left(), pointer.y),
                                     egui::pos2(rect.right(), pointer.y)], stroke);
                            }
                        }
                    }
                } else {
                    let status = self.fetch_state.lock().unwrap().status.clone();
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new(&status).color(text_dim).size(18.0));
                    });
                }
            });
    }
}
