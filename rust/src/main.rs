//! C02 Naval — version Windows native (Rust).
//! Bataille navale à deux : en ligne (compatible avec la version web), contre un bot,
//! comptes locaux avec XP et statistiques.

#![windows_subsystem = "windows"]

mod accounts;
mod app;
mod binarypack;
mod game;
mod i18n;
mod icon;
mod net;
mod sound;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon = egui::IconData { rgba: icon::icon_rgba(64), width: 64, height: 64 };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("C02 Naval")
            .with_app_id("c02-naval")
            .with_inner_size([1100.0, 820.0])
            .with_min_inner_size([420.0, 560.0])
            .with_icon(icon),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native("C02 Naval", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
