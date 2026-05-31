#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod models;
mod preview;
mod scanner;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Project Folder Manager",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            load_chinese_font(&cc.egui_ctx);

            let app = app::App::new(cc);
            Ok(Box::new(app))
        }),
    )
}

/// 从 Windows 系统字体目录加载中文字体以支持中文显示
fn load_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];

    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            let font_name = "ChineseFont".to_owned();
            fonts
                .font_data
                .insert(font_name.clone(), egui::FontData::from_owned(data));
            for family in fonts.families.values_mut() {
                family.insert(0, font_name.clone());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);
}
