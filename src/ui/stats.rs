use egui::{Color32, RichText, Ui};

use crate::models::{CategoryStats, FileCategory, ScanResult};

/// 渲染统计面板
pub fn render(scan_result: &ScanResult, ui: &mut Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(RichText::new("资源占用分析").size(15.0).strong());
        ui.add_space(8.0);

        // 总体信息
        ui.horizontal(|ui| {
            ui.label(RichText::new("总大小:").strong());
            ui.label(
                RichText::new(humansize::format_size(
                    scan_result.total_size,
                    humansize::BINARY,
                ))
                .color(Color32::from_rgb(255, 200, 100)),
            );
            ui.separator();
            ui.label(RichText::new("文件数:").strong());
            ui.label(
                RichText::new(format!("{}", scan_result.file_count))
                    .color(Color32::from_rgb(100, 200, 255)),
            );
            ui.separator();
            ui.label(RichText::new("目录数:").strong());
            ui.label(
                RichText::new(format!("{}", scan_result.dir_count))
                    .color(Color32::from_rgb(150, 255, 150)),
            );
        });

        ui.add_space(10.0);

        // 分类统计 — 进度条风格
        let total = scan_result.total_size.max(1); // 避免除零
        let bar_height = 18.0;

        for stats in &scan_result.category_stats {
            let ratio = stats.size as f32 / total as f32;
            let percentage = ratio * 100.0;

            ui.horizontal(|ui| {
                // 分类标签
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        stats.category.icon(),
                        stats.category.display_name()
                    ))
                    .size(12.0),
                );

                // 进度条
                let desired_width = 200.0;
                let (rect, _) = ui.allocate_at_least(
                    egui::vec2(desired_width, bar_height),
                    egui::Sense::hover(),
                );

                if ui.is_rect_visible(rect) {
                    let painter = ui.painter_at(rect);

                    // 背景
                    painter.rect_filled(rect, 2.0, Color32::from_gray(35));

                    // 进度
                    if ratio > 0.001 {
                        let fill_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(rect.width() * ratio, rect.height()),
                        );
                        painter.rect_filled(fill_rect, 2.0, stats.category.color());
                    }

                    // 百分比文本
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1}%", percentage),
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                }

                // 大小和文件数
                ui.label(
                    RichText::new(format!(
                        "{} ({} 文件)",
                        humansize::format_size(stats.size, humansize::BINARY),
                        stats.count
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
                );
            });

            ui.add_space(3.0);
        }

        // 饼图风格的占比文字总结
        if let Some(largest) = scan_result.category_stats.first() {
            ui.add_space(6.0);
            let pct = largest.size as f32 / total as f32 * 100.0;
            ui.label(
                RichText::new(format!(
                    "占比最大: {} ({:.1}%)",
                    largest.category.display_name(),
                    pct
                ))
                .size(11.0)
                .color(largest.category.color()),
            );
        }
    });
}

/// 计算分类统计（用于独立渲染）
pub fn calculate_category_stats(
    entries: &[crate::models::FileEntry],
) -> Vec<CategoryStats> {
    let mut map: std::collections::HashMap<FileCategory, (u64, usize)> =
        std::collections::HashMap::new();

    fn collect(entry: &crate::models::FileEntry, map: &mut std::collections::HashMap<FileCategory, (u64, usize)>) {
        if !entry.is_dir {
            let data = map.entry(entry.category).or_insert((0, 0));
            data.0 += entry.size;
            data.1 += 1;
        }
        for child in &entry.children {
            collect(child, map);
        }
    }

    for entry in entries {
        collect(entry, &mut map);
    }

    let mut stats: Vec<CategoryStats> = FileCategory::all()
        .iter()
        .filter_map(|cat| {
            map.get(cat).map(|&(size, count)| CategoryStats {
                category: *cat,
                size,
                count,
            })
        })
        .collect();
    stats.sort_by_key(|s| -(s.size as i64));
    stats
}
