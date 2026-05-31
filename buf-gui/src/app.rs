/*
    buf - Tool for flashing USB drives across platforms
    Copyright (C) 2026 Bryson Kelly
 
    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.
 
    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.
 
    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/


use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use egui::{Align, Color32, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2};
use libbuf::{UsbDevice, WriteParams};

use crate::flash::{self, FlashHandle, Progress};
use crate::style;

// Parse "32MiB", "4096", "1G" etc the same way the CLI does.
fn parse_size(s: &str) -> Result<usize, String> {
    let s = s.trim();
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(split);
    if num_str.is_empty() {
        return Err(format!("No numeric value in '{s}'"));
    }
    let num: u64 = num_str.parse().map_err(|_| format!("Cannot parse '{num_str}'"))?;
    let mul: u64 = match suffix.to_ascii_uppercase().as_str() {
        "" | "B"            => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        other               => return Err(format!("Unknown suffix '{other}'")),
    };
    let bytes = num.checked_mul(mul).ok_or_else(|| format!("Size overflows: {s}"))?;
    if bytes > usize::MAX as u64 {
        return Err(format!("Too large for this platform: {s}"));
    }
    Ok(bytes as usize)
}

fn human_bytes(bytes: u64) -> String {
    libbuf::list::human_bytes(bytes)
}

#[derive(Default, PartialEq)]
enum Phase {
    #[default]
    Idle,
    Confirming,
    Flashing,
    Done,
    Failed,
}

pub struct BufApp {
    source:   Option<PathBuf>,
    devices:  Vec<UsbDevice>,
    selected: Option<usize>,

    block_size: String,
    offset:     String,

    phase:        Phase,
    flash_handle: Option<FlashHandle>,
    // 0.0..=1.0, driven by Progress events from the flash thread
    progress:     f32,
    result_msg:   String,
    error_msg:    String,
    status_msg:   String,

    device_load_err: String,
}

impl Default for BufApp {
    fn default() -> Self {
        Self {
            source: None,
            devices: Vec::new(),
            selected: None,
            block_size: "32MiB".to_owned(),
            offset: "0".to_owned(),
            phase: Phase::Idle,
            flash_handle: None,
            progress: 0.0,
            result_msg: String::new(),
            error_msg: String::new(),
            status_msg: String::new(),
            device_load_err: String::new(),
        }
    }
}

impl BufApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        style::apply(&cc.egui_ctx);
        let mut app = Self::default();
        app.refresh_devices();
        app
    }

    fn refresh_devices(&mut self) {
        match libbuf::list_drives() {
            Ok(devs) => {
                self.devices = devs;
                self.selected = None;
                self.device_load_err.clear();
            }
            Err(e) => {
                self.device_load_err = format!("{:#}", e);
                self.devices.clear();
            }
        }
    }

    fn source_name(&self) -> Option<String> {
        self.source
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
    }

    fn selected_device(&self) -> Option<&UsbDevice> {
        self.selected.and_then(|i| self.devices.get(i))
    }

    fn can_flash(&self) -> bool {
        self.source.is_some()
            && self.selected.is_some()
            && self.phase == Phase::Idle
            && parse_size(&self.block_size).is_ok()
            && self.offset.trim().parse::<u64>().is_ok()
    }

    fn start_confirm(&mut self) {
        if self.can_flash() {
            self.phase = Phase::Confirming;
        }
    }

    fn do_flash(&mut self, ctx: &egui::Context) {
        let source_path = match &self.source {
            Some(p) => p.clone(),
            None => return,
        };
        let target_path = match self.selected_device() {
            Some(d) => d.path.clone(),
            None => return,
        };

        let block_size = match parse_size(&self.block_size) {
            Ok(b) => b,
            Err(e) => {
                self.error_msg = e;
                self.phase = Phase::Failed;
                return;
            }
        };
        let offset = match self.offset.trim().parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                self.error_msg = "Offset must be a plain integer in bytes".to_owned();
                self.phase = Phase::Failed;
                return;
            }
        };

        let params = WriteParams {
            source: source_path.to_string_lossy().to_string(),
            target: target_path,
            block_size,
            offset,
        };

        self.status_msg = "Validating...".to_owned();
        self.phase = Phase::Flashing;
        self.progress = 0.0;

        let (source_size, target_file) = match libbuf::validate(&params) {
            Ok(r) => r,
            Err(e) => {
                self.error_msg = format!("{:#}", e);
                self.phase = Phase::Failed;
                return;
            }
        };

        ctx.request_repaint();

        let handle = flash::start(params, source_size, target_file);
        self.flash_handle = Some(handle);
        self.status_msg = "Writing...".to_owned();
    }

    fn poll_flash(&mut self, ctx: &egui::Context) {
        if self.phase != Phase::Flashing {
            return;
        }
        let handle = match &self.flash_handle {
            Some(h) => h,
            None => return,
        };

        loop {
            match handle.rx.try_recv() {
                Ok(Progress::Update(written, total)) => {
                    if total > 0 {
                        self.progress = written as f32 / total as f32;
                    }
                    ctx.request_repaint();
                }
                Ok(Progress::Done { elapsed_secs, bytes }) => {
                    let speed = bytes as f64 / elapsed_secs.max(0.001);
                    self.result_msg = format!(
                        "{} written in {:.1}s  ({}/s)",
                        human_bytes(bytes),
                        elapsed_secs,
                        human_bytes(speed as u64),
                    );
                    self.progress = 1.0;
                    self.phase = Phase::Done;
                    self.flash_handle = None;
                    ctx.request_repaint();
                    break;
                }
                Ok(Progress::Error(e)) => {
                    self.error_msg = e;
                    self.phase = Phase::Failed;
                    self.flash_handle = None;
                    ctx.request_repaint();
                    break;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint();
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.error_msg = "Flash thread disconnected unexpectedly".to_owned();
                    self.phase = Phase::Failed;
                    self.flash_handle = None;
                    break;
                }
            }
        }
    }
}

impl eframe::App for BufApp {
    // logic() runs before ui() each frame; good place for non-drawing work.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_flash(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // ui() receives a bare Ui with no margin or background, so we wrap
        // everything in a Frame to get our surface colour and padding.
        Frame::new()
            .fill(style::BG)
            .inner_margin(Margin::same(24))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());

                ui.horizontal(|ui| {
                    ui.label(RichText::new("buf").size(26.0).color(style::ACCENT).strong());
                    ui.label(
                        RichText::new("bootable usb flasher")
                            .size(13.0)
                            .color(style::TEXT_DIM),
                    );
                });
                ui.add_space(18.0);

                let ctx = ui.ctx().clone();
                match self.phase {
                    Phase::Idle | Phase::Confirming => self.draw_main(ui, &ctx),
                    Phase::Flashing                 => self.draw_flashing(ui),
                    Phase::Done                     => self.draw_done(ui),
                    Phase::Failed                   => self.draw_failed(ui),
                }
            });
    }
}

impl BufApp {
    fn draw_main(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        section(ui, "Source image", |ui| {
            ui.horizontal(|ui| {
                let label = self.source_name().unwrap_or_else(|| "No file selected".to_owned());
                let color = if self.source.is_some() { style::TEXT } else { style::TEXT_DIM };
                ui.add(egui::Label::new(RichText::new(&label).color(color).monospace()).truncate());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Disk images", &["iso", "img"])
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            self.source = Some(path);
                        }
                    }
                });
            });
        });

        ui.add_space(10.0);

        section_fixed(ui, "Target device", 130.0, |ui| {
            if !self.device_load_err.is_empty() {
                ui.label(
                    RichText::new(&self.device_load_err.clone())
                        .color(style::DANGER)
                        .small(),
                );
            }
            if self.devices.is_empty() && self.device_load_err.is_empty() {
                ui.label(RichText::new("No storage devices found").color(style::TEXT_DIM));
            }

            egui::ScrollArea::vertical()
                .id_salt("device_list")
                .show(ui, |ui| {
                    for i in 0..self.devices.len() {
                        let dev = &self.devices[i];
                        let selected = self.selected == Some(i);
                        let model = if dev.model.is_empty() { "Unknown" } else { dev.model.as_str() };
                        let label = format!("{}   {}   {}", dev.path, dev.size_human, model);

                        if ui
                            .selectable_label(selected, RichText::new(&label).monospace().size(12.5))
                            .clicked()
                        {
                            self.selected = Some(i);
                        }
                    }
                });

            ui.add_space(4.0);
            if ui.small_button("Refresh").clicked() {
                self.refresh_devices();
            }
        });

        ui.add_space(10.0);

        egui::CollapsingHeader::new(
            RichText::new("Advanced options").color(style::TEXT_DIM).size(13.0),
        )
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("adv_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Block size");
                    let bs_valid = parse_size(&self.block_size).is_ok();
                    let bs_color = if bs_valid { style::TEXT } else { style::DANGER };
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.block_size)
                            .desired_width(120.0)
                            .text_color(bs_color)
                            .hint_text("e.g. 32MiB"),
                    );
                    resp.on_hover_text("Accepted suffixes: B, K/KB/KiB, M/MB/MiB, G/GB/GiB");
                    ui.end_row();

                    ui.label("Offset (bytes)");
                    let off_valid = self.offset.trim().parse::<u64>().is_ok();
                    let off_color = if off_valid { style::TEXT } else { style::DANGER };
                    ui.add(
                        egui::TextEdit::singleline(&mut self.offset)
                            .desired_width(120.0)
                            .text_color(off_color)
                            .hint_text("0"),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        if self.phase == Phase::Confirming {
            if let (Some(src), Some(dev)) = (self.source_name(), self.selected_device()) {
                let dev_path = dev.path.clone();
                let dev_size = dev.size_human.clone();

                warn_box(ui, |ui| {
                    ui.label(
                        RichText::new("This will permanently erase all data on the target.")
                            .color(style::WARNING),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("  Source : {src}")).monospace().size(12.0));
                    ui.label(
                        RichText::new(format!("  Target : {dev_path}  ({dev_size})"))
                            .monospace()
                            .size(12.0),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if accent_button(ui, "Flash it").clicked() {
                            self.do_flash(ctx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.phase = Phase::Idle;
                        }
                    });
                });
            }
        } else {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_enabled_ui(self.can_flash(), |ui| {
                    if accent_button(ui, "Flash").clicked() {
                        self.start_confirm();
                    }
                });
            });
        }

        #[cfg(not(target_os = "windows"))]
        if !libbuf::is_privileged() {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Run buf-gui with sudo to write to block devices")
                    .color(style::WARNING)
                    .small(),
            );
        }
    }

    fn draw_flashing(&mut self, ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new(&self.status_msg).color(style::TEXT_DIM).size(13.0));
                ui.add_space(16.0);

                let desired = egui::vec2(ui.available_width().min(380.0), 18.0);
                let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
                let p = ui.painter();

                p.rect_filled(rect, 9.0, style::SURFACE2);
                if self.progress > 0.0 {
                    let filled = egui::Rect::from_min_size(
                        rect.min,
                        Vec2::new(rect.width() * self.progress.clamp(0.0, 1.0), rect.height()),
                    );
                    p.rect_filled(filled, 9.0, style::ACCENT);
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("{:.0}%", self.progress * 100.0))
                        .color(style::ACCENT)
                        .monospace(),
                );
            });
        });
    }

    fn draw_done(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("Done").color(style::ACCENT).size(22.0).strong());
            ui.add_space(10.0);
            ui.label(RichText::new(&self.result_msg.clone()).monospace().size(13.0));
            ui.add_space(20.0);
            if ui.button("Flash another").clicked() {
                self.phase = Phase::Idle;
                self.result_msg.clear();
                self.progress = 0.0;
                self.refresh_devices();
            }
        });
    }

    fn draw_failed(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("Error").color(style::DANGER).size(22.0).strong());
            ui.add_space(10.0);
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                ui.label(
                    RichText::new(&self.error_msg.clone())
                        .color(style::DANGER)
                        .monospace()
                        .size(12.0),
                );
            });
            ui.add_space(20.0);
            if ui.button("Back").clicked() {
                self.phase = Phase::Idle;
                self.error_msg.clear();
            }
        });
    }
}

fn section(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(title).color(style::TEXT_DIM).size(11.5));
    ui.add_space(4.0);
    Frame::new()
        .fill(style::SURFACE)
        .stroke(Stroke::new(1.0, style::BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::same(12))
        .show(ui, content);
}

// Like section(), but constrains the inner height so ScrollArea children
// actually have a bounded viewport to scroll within.
fn section_fixed(ui: &mut Ui, title: &str, height: f32, content: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(title).color(style::TEXT_DIM).size(11.5));
    ui.add_space(4.0);
    Frame::new()
        .fill(style::SURFACE)
        .stroke(Stroke::new(1.0, style::BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_max_height(height);
            content(ui);
        });
}

fn warn_box(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(Color32::from_rgba_unmultiplied(50, 30, 10, 200))
        .stroke(Stroke::new(1.0, style::WARNING))
        .corner_radius(8.0)
        .inner_margin(Margin::same(12))
        .show(ui, content);
}

fn accent_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(style::BG).strong())
            .fill(style::ACCENT)
            .stroke(Stroke::new(1.0, style::ACCENT))
            .corner_radius(6.0),
    )
}
