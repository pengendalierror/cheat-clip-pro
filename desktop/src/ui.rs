use crate::engine::{self, analyze, export_clips, load_cues, resolve_source};
use crate::model::{fmt_time, Aspect, Clip, Settings, Source};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

enum Work {
    Open(String),
    Analyze,
    Export,
}

enum Event {
    Opened(Source),
    Planned { summary: String, clips: Vec<Clip> },
    Exported(Vec<PathBuf>),
    Failed(String),
}

pub struct CheatClip {
    settings: Settings,
    source_input: String,
    subtitle_path: String,
    source: Option<Source>,
    summary: String,
    clips: Vec<Clip>,
    selected: Option<u64>,
    status: String,
    busy: bool,
    inbox: Option<Receiver<Event>>,
    has_ffmpeg: bool,
    has_ytdlp: bool,
}

impl CheatClip {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<String>) -> Self {
        apply_theme(&cc.egui_ctx);
        let settings = Settings::load();
        let mut app = Self {
            source_input: String::new(),
            subtitle_path: String::new(),
            source: None,
            summary: String::new(),
            clips: Vec::new(),
            selected: None,
            status: "Siap.".into(),
            busy: false,
            inbox: None,
            has_ffmpeg: engine::tool_exists("ffmpeg") && engine::tool_exists("ffprobe"),
            has_ytdlp: engine::tool_exists("yt-dlp"),
            settings,
        };
        if let Some(initial) = initial {
            app.source_input = initial.clone();
            app.start(Work::Open(initial));
        }
        app
    }

    fn start(&mut self, work: Work) {
        if self.busy {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.inbox = Some(rx);
        self.busy = true;
        self.settings.save();
        match work {
            Work::Open(input) => {
                self.status = "Membuka video…".into();
                thread::spawn(move || {
                    let event = match resolve_source(&input) {
                        Ok(source) => Event::Opened(source),
                        Err(err) => Event::Failed(err),
                    };
                    let _ = tx.send(event);
                });
            }
            Work::Analyze => {
                let Some(source) = self.source.clone() else {
                    self.busy = false;
                    self.status = "Buka video dulu.".into();
                    return;
                };
                self.status = "Menyusun klip…".into();
                let subtitle = self.subtitle_path.clone();
                let settings = self.settings.clone();
                thread::spawn(move || {
                    let cues = load_cues(&subtitle, &source.path);
                    let event = match analyze(
                        &source,
                        &cues,
                        &settings.api_key,
                        "gemini-2.5-flash",
                        settings.duration_secs as f64,
                        settings.clip_count.max(1) as usize,
                        &settings.focus,
                        &settings.language,
                    ) {
                        Ok((summary, clips)) => Event::Planned { summary, clips },
                        Err(err) => Event::Failed(err),
                    };
                    let _ = tx.send(event);
                });
            }
            Work::Export => {
                let Some(source) = self.source.clone() else {
                    self.busy = false;
                    self.status = "Tidak ada video.".into();
                    return;
                };
                let clips = self.clips.clone();
                let output = PathBuf::from(&self.settings.output_dir);
                let aspect = self.settings.aspect();
                let burn = self.settings.burn_title;
                self.status = "Merender…".into();
                thread::spawn(move || {
                    let event = match export_clips(&source, &clips, &output, aspect, burn) {
                        Ok(files) => Event::Exported(files),
                        Err(err) => Event::Failed(err),
                    };
                    let _ = tx.send(event);
                });
            }
        }
    }

    fn poll(&mut self) {
        let Some(rx) = &self.inbox else { return };
        while let Ok(event) = rx.try_recv() {
            self.busy = false;
            match event {
                Event::Opened(source) => {
                    self.status = format!("{} · {}", source.title, fmt_time(source.duration));
                    self.source_input = source.path.clone();
                    self.summary.clear();
                    self.clips.clear();
                    self.selected = None;
                    self.source = Some(source);
                }
                Event::Planned { summary, clips } => {
                    self.selected = clips.first().map(|clip| clip.id);
                    self.status = format!("{} klip", clips.len());
                    self.summary = summary;
                    self.clips = clips;
                }
                Event::Exported(files) => {
                    self.status = format!("{} file tersimpan", files.len());
                }
                Event::Failed(err) => {
                    self.status = err;
                }
            }
        }
    }
}

impl eframe::App for CheatClip {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        let dropped: Vec<String> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.as_ref().map(|path| path.display().to_string()))
                .collect()
        });
        if let Some(path) = dropped.into_iter().next() {
            if !self.busy {
                self.source_input = path.clone();
                self.start(Work::Open(path));
            }
        }
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        egui::TopBottomPanel::top("bar").exact_height(46.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("Cheat Clip").size(18.0).strong());
                ui.label(RichText::new("desktop").size(12.0).color(Color32::from_rgb(154, 152, 145)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tools = format!(
                        "ffmpeg {}   yt-dlp {}",
                        if self.has_ffmpeg { "ada" } else { "tidak ada" },
                        if self.has_ytdlp { "ada" } else { "tidak ada" }
                    );
                    ui.label(RichText::new(tools).size(12.0).color(Color32::from_rgb(154, 152, 145)));
                });
            });
        });

        egui::TopBottomPanel::bottom("status").exact_height(32.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if self.busy {
                    ui.spinner();
                }
                ui.label(&self.status);
            });
        });

        egui::SidePanel::left("source").exact_width(320.0).resizable(false).show(ctx, |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Sumber").strong());
            ui.add_space(6.0);
            ui.label("File atau tautan");
            ui.add(egui::TextEdit::singleline(&mut self.source_input).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.busy, brass_button("Buka")).clicked() {
                    match pick_video() {
                        Pick::Chosen(path) => self.source_input = path,
                        Pick::Cancelled => {}
                        Pick::Unavailable => {
                            self.status = "Dialog file tidak ada. Tempel path video, lalu Muat.".into();
                        }
                    }
                }
                if ui.add_enabled(!self.busy, egui::Button::new("Muat")).clicked() {
                    let input = self.source_input.clone();
                    self.start(Work::Open(input));
                }
            });
            ui.add_space(10.0);
            ui.label("Subtitle .srt, kalau ada");
            ui.add(egui::TextEdit::singleline(&mut self.subtitle_path).desired_width(f32::INFINITY));
            ui.add_space(10.0);
            ui.label("Panjang klip");
            ui.horizontal(|ui| {
                for (label, secs) in [("15 dtk", 15u32), ("30 dtk", 30), ("60 dtk", 60)] {
                    let on = self.settings.duration_secs == secs;
                    if ui.selectable_label(on, label).clicked() {
                        self.settings.duration_secs = secs;
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(format!("Jumlah klip  {}", self.settings.clip_count));
            ui.add(egui::Slider::new(&mut self.settings.clip_count, 1..=12).show_value(false));
            ui.add_space(6.0);
            ui.label("Fokus, opsional");
            ui.add(egui::TextEdit::singleline(&mut self.settings.focus).desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                if ui.selectable_label(self.settings.language == "id", "Indonesia").clicked() {
                    self.settings.language = "id".into();
                }
                if ui.selectable_label(self.settings.language == "en", "English").clicked() {
                    self.settings.language = "en".into();
                }
            });
            ui.add_space(6.0);
            ui.label("Kunci Gemini");
            ui.add(egui::TextEdit::singleline(&mut self.settings.api_key).password(true).desired_width(f32::INFINITY));
            ui.label(RichText::new("Kosongkan untuk bagi klip secara lokal.").size(12.0).color(Color32::from_rgb(154, 152, 145)));
            ui.add_space(8.0);
            if ui.add_enabled(!self.busy && self.source.is_some(), brass_button("Susun klip")).clicked() {
                self.start(Work::Analyze);
            }
        });

        egui::SidePanel::right("export").exact_width(280.0).resizable(false).show(ctx, |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Ekspor").strong());
            ui.add_space(6.0);
            ui.label("Bingkai");
            ui.horizontal(|ui| {
                for aspect in [Aspect::Portrait, Aspect::Square, Aspect::Wide] {
                    let label = aspect.label();
                    if ui.selectable_label(self.settings.aspect == label, label).clicked() {
                        self.settings.aspect = label.into();
                    }
                }
            });
            ui.checkbox(&mut self.settings.burn_title, "Tulis judul di gambar");
            ui.add_space(6.0);
            ui.label("Folder keluaran");
            ui.add(egui::TextEdit::singleline(&mut self.settings.output_dir).desired_width(f32::INFINITY));
            ui.add_space(8.0);
            let marked = self.clips.iter().filter(|clip| clip.selected).count();
            if ui.add_enabled(!self.busy && marked > 0 && self.has_ffmpeg, brass_button(&format!("Render {marked} klip"))).clicked() {
                self.start(Work::Export);
            }
            ui.add_space(14.0);
            if let Some(clip) = self.clips.iter().find(|clip| Some(clip.id) == self.selected) {
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new(&clip.title).strong());
                ui.label(format!("{}  –  {}   skor {}", fmt_time(clip.start), fmt_time(clip.end), clip.score));
                if let Some(source) = &self.source {
                    draw_range(ui, source.duration, clip.start, clip.end);
                }
                if !clip.quote.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(&clip.quote).size(13.0));
                }
                if !clip.caption.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(&clip.caption).size(12.0).color(Color32::from_rgb(196, 190, 178)));
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(source) = &self.source {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&source.title).size(18.0).strong());
                    ui.label(RichText::new(fmt_time(source.duration)).monospace().color(Color32::from_rgb(184, 138, 74)));
                });
                if source.origin != source.path {
                    ui.label(RichText::new(&source.origin).size(12.0).color(Color32::from_rgb(154, 152, 145)));
                }
                if !self.summary.is_empty() {
                    ui.label(RichText::new(&self.summary).color(Color32::from_rgb(196, 190, 178)));
                }
                ui.add_space(8.0);
            } else {
                ui.add_space(28.0);
                ui.label(RichText::new("Belum ada video.").size(20.0).strong());
                ui.label("Tempel path file, jatuhkan file ke jendela, atau tempel tautan. Lalu Muat.");
                return;
            }

            if self.clips.is_empty() {
                ui.label("Belum ada klip. Tekan Susun klip.");
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for clip in &mut self.clips {
                    let selected = self.selected == Some(clip.id);
                    let row = egui::Frame::none()
                        .fill(if selected { Color32::from_rgb(48, 42, 32) } else { Color32::from_rgb(32, 33, 36) })
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut clip.selected, "");
                                ui.label(RichText::new(format!("{:02}", clip.score)).monospace().color(Color32::from_rgb(184, 138, 74)));
                                ui.monospace(format!("{}–{}", fmt_time(clip.start), fmt_time(clip.end)));
                                ui.label(&clip.title);
                            });
                        });
                    if row.response.interact(egui::Sense::click()).clicked() {
                        self.selected = Some(clip.id);
                    }
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

fn brass_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label.to_string()).color(Color32::from_rgb(24, 18, 12)))
        .fill(Color32::from_rgb(196, 149, 74))
        .min_size(Vec2::new(88.0, 28.0))
}

fn draw_range(ui: &mut egui::Ui, duration: f64, start: f64, end: f64) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, Color32::from_rgb(18, 18, 20));
    if duration > 0.0 {
        let x0 = rect.left() + (start / duration) as f32 * rect.width();
        let x1 = rect.left() + (end / duration) as f32 * rect.width();
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1.max(x0 + 2.0), rect.bottom())),
            2.0,
            Color32::from_rgb(196, 149, 74),
        );
    }
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(70, 68, 62)));
}

enum Pick {
    Chosen(String),
    Cancelled,
    Unavailable,
}

fn pick_video() -> Pick {
    if engine::tool_exists("zenity") {
        return command_path(
            "zenity",
            &[
                "--file-selection",
                "--title=Buka video",
                "--file-filter=Video | *.mp4 *.mov *.mkv *.webm *.m4v",
            ],
        );
    }
    if cfg!(target_os = "macos") && engine::tool_exists("osascript") {
        return command_path(
            "osascript",
            &["-e", "POSIX path of (choose file with prompt \"Buka video\" of type {\"mp4\", \"mov\", \"mkv\", \"webm\", \"m4v\", \"public.movie\"})"],
        );
    }
    Pick::Unavailable
}

fn command_path(program: &str, args: &[&str]) -> Pick {
    let Ok(output) = std::process::Command::new(program).args(args).output() else {
        return Pick::Unavailable;
    };
    if !output.status.success() {
        return Pick::Cancelled;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { Pick::Cancelled } else { Pick::Chosen(path) }
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
    ctx.set_style(style);

    let mut visuals = egui::Visuals::dark();
    let text = Color32::from_rgb(232, 230, 223);
    let muted = Color32::from_rgb(154, 152, 145);
    visuals.window_fill = Color32::from_rgb(24, 25, 27);
    visuals.panel_fill = Color32::from_rgb(30, 31, 33);
    visuals.faint_bg_color = Color32::from_rgb(24, 25, 27);
    visuals.extreme_bg_color = Color32::from_rgb(16, 16, 18);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, muted);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(42, 43, 46);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 54, 46);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, text);
    visuals.widgets.active.bg_fill = Color32::from_rgb(196, 149, 74);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(24, 18, 12));
    visuals.selection.bg_fill = Color32::from_rgb(92, 68, 36);
    visuals.override_text_color = Some(text);
    ctx.set_visuals(visuals);
}
