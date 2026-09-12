#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod app;
mod assets;
mod behavior;
mod document;
mod logging;
mod preview;
mod runtime;
fn main() -> eframe::Result {
    let log_path = logging::init().unwrap_or_else(|e| {
        eprintln!("Log indisponível: {e}");
        std::path::PathBuf::new()
    });
    let smoke = std::env::args().any(|a| a == "--smoke-test");
    if smoke {
        let mut doc = document::Document::parse(include_bytes!("../examples/welcome.otui"), true)
            .expect("example");
        let before = doc.bytes();
        let mut history = document::History::default();
        history
            .apply(&mut doc, "Smoke", |d| d.set(0, "text", "Rust"))
            .expect("edit");
        history.undo(&mut doc).expect("undo");
        assert_eq!(doc.bytes(), before);
        history.redo(&mut doc).expect("redo");
        assert_eq!(doc.value(0, "text"), "Rust");
        log::info!("Document smoke passed");
        log::info!("Desktop smoke passed");
        log::logger().flush();
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 650.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "NextGen Studio • Rust",
        options,
        Box::new(move |cc| Ok(Box::new(app::Studio::new(cc, log_path, smoke)))),
    )
}
