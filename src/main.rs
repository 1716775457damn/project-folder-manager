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
            apply_custom_style(&cc.egui_ctx);
            load_chinese_font(&cc.egui_ctx);

            let app = app::App::new(cc);
            Ok(Box::new(app))
        }),
    )
}

/// 应用高级极客蓝/石板灰精致视觉主题
fn apply_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // 1. 设置更现代的全局圆角
    style.visuals.window_rounding = 8.0.into();
    style.visuals.widgets.noninteractive.rounding = 6.0.into();
    style.visuals.widgets.inactive.rounding = 5.0.into();
    style.visuals.widgets.hovered.rounding = 5.0.into();
    style.visuals.widgets.active.rounding = 5.0.into();

    // 2. 高级 Slate Ocean 调色板 (极客深邃蓝灰)
    style.visuals.panel_fill = egui::Color32::from_rgb(17, 24, 39);        // 极暗的主底色
    style.visuals.window_fill = egui::Color32::from_rgb(31, 41, 55);       // 面板/卡片背景色
    style.visuals.faint_bg_color = egui::Color32::from_rgb(55, 65, 81);    // 较暗背景（如进度条空槽）
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(10, 15, 26);  // 预览文本/代码专属超暗黑底

    // 3. 控件多状态交互美化 (Button, Selectable等)
    // 默认空闲状态 (Inactive)
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(31, 41, 55);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 85, 99));
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(229, 231, 235));

    // 鼠标悬停状态 (Hovered)
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 65, 81);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(59, 130, 246)); // 亮蓝悬停线
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    // 点击激活状态 (Active)
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(29, 78, 216); // 经典深海蓝
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(59, 130, 246));
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

    // 4. 调整选区与排版间距
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(31, 41, 55));
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(37, 99, 235); // 蓝高亮选区

    // 5. 间距微调，使排版更显开阔与层次感
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}

/// 从 Windows 系统字体目录加载中文字体以支持中文显示
fn load_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_paths = [
        // Windows
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/Library/Fonts/Songti.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        // Linux
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallback.ttf",
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
