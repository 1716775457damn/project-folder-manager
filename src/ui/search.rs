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
            app.search_results = Arc::new(Vec::new());
        }

        if app.is_searching {
            ui.spinner();
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

                    let rel_path = if let Some(idx) = app.selected_project_index {
                        let project_path = std::path::Path::new(&app.config.projects[idx].path);
                        path.strip_prefix(project_path)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| path.to_string_lossy().to_string())
                    } else {
                        path.to_string_lossy().to_string()
                    };

                    let is_previewed = app.preview_path.as_ref().map(|p| p == path).unwrap_or(false);
                    let (rect, response) = ui.allocate_at_least(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());

                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter_at(rect);

                        // 卡片高亮与悬停反馈
                        let border_color = if is_previewed {
                            Color32::from_rgb(59, 130, 246)
                        } else if response.hovered() {
                            Color32::from_rgb(75, 85, 99)
                        } else {
                            Color32::from_rgb(45, 55, 72)
                        };

                        let bg_fill = if is_previewed {
                            Color32::from_rgba_unmultiplied(59, 130, 246, 20)
                        } else if response.hovered() {
                            Color32::from_rgb(45, 55, 72)
                        } else {
                            Color32::from_rgb(31, 41, 55)
                        };

                        painter.rect(
                            rect,
                            4.0,
                            bg_fill,
                            egui::Stroke::new(1.0, border_color),
                        );

                        // 分类图标与颜色获取
                        let (icon, color) = if path.is_dir() {
                            ("[dir]", Color32::from_rgb(220, 200, 140))
                        } else {
                            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                            let cat = crate::models::FileCategory::from_extension(&ext);
                            (cat.icon(), cat.color())
                        };

                        // 绘制名称
                        painter.text(
                            egui::pos2(rect.left() + 8.0, rect.top() + 4.0),
                            egui::Align2::LEFT_TOP,
                            format!("{} {}", icon, name),
                            egui::FontId::proportional(11.5),
                            if is_previewed { Color32::from_rgb(255, 220, 100) } else { color },
                        );

                        // 绘制相对路径
                        let short_rel_path = if rel_path.len() > 50 {
                            format!("...{}", &rel_path[rel_path.len() - 47..])
                        } else {
                            rel_path.clone()
                        };
                        painter.text(
                            egui::pos2(rect.left() + 8.0, rect.top() + 20.0),
                            egui::Align2::LEFT_TOP,
                            short_rel_path,
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(156, 163, 175),
                        );
                    }

                    if response.clicked() {
                        app.preview_file(path);

                        // 展开到该文件的父目录
                        let mut current = path.parent().map(|p| p.to_path_buf());
                        while let Some(ref p) = current {
                            app.expanded_dirs.insert(p.clone());
                            current = p.parent().map(|p| p.to_path_buf());
                        }
                    }
                    ui.add_space(4.0);
                }
            });
    } else if app.is_searching {
        ui.label(
            RichText::new("正在搜索中...")
                .size(11.0)
                .color(Color32::from_rgb(255, 200, 100)),
        );
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
