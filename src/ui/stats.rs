use egui::{Color32, RichText, Ui};

use crate::models::ScanResult;

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

        // 分类统计 — 使用 egui ProgressBar
        let total = scan_result.total_size.max(1);

        for stats in &scan_result.category_stats {
            let ratio = stats.size as f32 / total as f32;
            let percentage = ratio * 100.0;

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        stats.category.icon(),
                        stats.category.display_name()
                    ))
                    .size(12.0),
                );

                // egui 内置进度条
                let bar = egui::ProgressBar::new(ratio)
                    .desired_width(200.0)
                    .fill(stats.category.color())
                    .text(format!("{:.1}%", percentage));
                ui.add(bar);

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


