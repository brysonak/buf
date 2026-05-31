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


use egui::{Color32, FontData, FontDefinitions, FontFamily, Stroke, Style, Visuals};

pub const BG: Color32         = Color32::from_rgb(14, 14, 16);
pub const SURFACE: Color32    = Color32::from_rgb(22, 22, 26);
pub const SURFACE2: Color32   = Color32::from_rgb(30, 30, 36);
pub const BORDER: Color32     = Color32::from_rgb(48, 48, 58);
pub const ACCENT: Color32     = Color32::from_rgb(82, 196, 130);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(46, 110, 72);
pub const TEXT: Color32       = Color32::from_rgb(220, 220, 225);
pub const TEXT_DIM: Color32   = Color32::from_rgb(110, 110, 120);
pub const DANGER: Color32     = Color32::from_rgb(220, 80, 70);
pub const WARNING: Color32    = Color32::from_rgb(210, 160, 60);

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // jetbrains mono for device paths and size labels
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());

    ctx.set_fonts(fonts);

    let mut style = Style::default();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(20);

    let mut vis = Visuals::dark();
    vis.window_fill = BG;
    vis.panel_fill = BG;
    vis.faint_bg_color = SURFACE;
    vis.extreme_bg_color = SURFACE2;
    vis.window_stroke = Stroke::new(1.0, BORDER);

    let cr6 = egui::CornerRadius::same(6);

    vis.widgets.noninteractive.bg_fill = SURFACE;
    vis.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    vis.widgets.noninteractive.corner_radius = cr6;

    vis.widgets.inactive.bg_fill = SURFACE2;
    vis.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    vis.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    vis.widgets.inactive.corner_radius = cr6;

    vis.widgets.hovered.bg_fill = ACCENT_DIM;
    vis.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    vis.widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
    vis.widgets.hovered.corner_radius = cr6;

    vis.widgets.active.bg_fill = ACCENT;
    vis.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    vis.widgets.active.fg_stroke = Stroke::new(1.5, BG);
    vis.widgets.active.corner_radius = cr6;

    vis.widgets.open.bg_fill = SURFACE2;
    vis.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    vis.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
    vis.widgets.open.corner_radius = cr6;

    vis.selection.bg_fill = ACCENT_DIM;
    vis.selection.stroke = Stroke::new(1.0, ACCENT);
    vis.override_text_color = Some(TEXT);

    style.visuals = vis;
    ctx.set_global_style(style);
}
