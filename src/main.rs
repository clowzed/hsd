#![allow(non_snake_case)]

mod api;
mod audio;
mod domain;
mod infrastructure;
mod pdf;
mod services;
mod ui;

use eframe::egui;
use ui::App;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Honest Sign Scanner application");

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([700.0, 500.0])
            .with_title("Честный Знак Сканер"),
        ..Default::default()
    };

    eframe::run_native(
        "Honest Sign Scanner",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, runtime)))),
    )
}
