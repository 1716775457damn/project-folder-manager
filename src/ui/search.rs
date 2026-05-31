use std::sync::Arc;

use egui::{Color32, RichText, ScrollArea, Ui};

use crate::app::App;

/// 渲染搜索面板
pub fn render(app: &mut App, ui: &mut Ui) {
    ui.label(RichText::new("搜索文件").size(14.0).strong());
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        let response = ui.text_edit_singleline(&mut app.search_query);

        // 输入即标记待搜索，300ms 内无新输入则触发
        if response.changed() {
            app.search_debounce = Some(std::time::Instant::now());
            app.pending_search = true;
        }

        // 无输入时立即清空结果
        if app.search_query.is_empty() && !app.search_results.is_empty() {
            app.search_results = Arc::new(Vec::new());
            app.pending_search = false;
        }

        if ui.button("清除").clicked() {
            app.search_query.clear();
            app.search_results.clear();
        }
    });

    ui.add_space(4.0);

    // 搜索结果显示
    if !app.search_results.is_empty() {
        ui.label(
            RichText::new(format!("找到 {} 个结果", app.search_results.len()))
                .size(11.0)
                .color(Color32::GRAY),
        );

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(200.0)
            .show(ui, |ui| {
                let results = Arc::clone(&app.search_results);
                for path in results.iter() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.to_string_lossy().to_string());

                    let display = format!(
                        "{}  {}",
                        if path.is_dir() { "[dir]" } else { "[file]" },
                        name
                    );

                    let response = ui.selectable_label(false, RichText::new(&display).size(12.0));

                    if response.clicked() {
                        app.preview_file(path);

                        // 展开到该文件的父目录
                        let mut current = path.parent().map(|p| p.to_path_buf());
                        while let Some(ref p) = current {
                            app.expanded_dirs.insert(p.clone());
                            current = p.parent().map(|p| p.to_path_buf());
                        }
                    }
                }
            });
    } else if app.search_query.is_empty() {
        ui.label(
            RichText::new("输入文件名关键词搜索")
                .size(11.0)
                .color(Color32::GRAY),
        );
    } else {
        ui.label(
            RichText::new("无匹配结果")
                .size(11.0)
                .color(Color32::from_rgb(255, 120, 100)),
        );
    }
}
