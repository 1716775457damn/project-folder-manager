use egui::{Color32, RichText, ScrollArea, Ui};

use crate::app::App;
use crate::models::FileEntry;

pub fn render(app: &mut App, ui: &mut Ui) {
    ui.label(RichText::new("文件浏览器").size(14.0).strong());
    ui.add_space(4.0);

    if app.scan_result.is_none() {
        if app.is_scanning {
            ui.label(
                RichText::new("正在扫描文件夹...")
                    .color(Color32::from_rgb(255, 200, 100)),
            );
        } else if app.selected_project_index.is_some() {
            ui.label(
                RichText::new("点击刷新按钮开始扫描")
                    .color(Color32::GRAY),
            );
        } else {
            ui.label(
                RichText::new("请先在左侧选择一个项目")
                    .color(Color32::GRAY),
            );
        }
        return;
    }

    let scan = app.scan_result.as_ref();
    if let Some(scan_data) = scan {
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(ui.available_height() - 30.0)
            .show(ui, |ui| {
                render_node(app, ui, &scan_data.root, 0);
            });
    }
}

fn render_node(app: &mut App, ui: &mut Ui, entry: &FileEntry, depth: usize) {
    let indent = depth as f32 * 16.0;
    let is_expanded = app.expanded_dirs.contains(&entry.path);
    let is_previewed = app
        .preview_path
        .as_ref()
        .map(|p| p == &entry.path)
        .unwrap_or(false);

    ui.horizontal(|ui| {
        ui.add_space(indent);

        if entry.is_dir {
            let arrow = if is_expanded { "v" } else { ">" };
            let resp = ui.selectable_label(
                false,
                RichText::new(format!(
                    "{} {} {}",
                    arrow,
                    folder_icon(entry),
                    &entry.name
                ))
                .size(12.5)
                .color(if is_previewed {
                    Color32::from_rgb(255, 220, 100)
                } else {
                    Color32::from_rgb(220, 200, 140)
                }),
            );

            if resp.clicked() {
                if is_expanded {
                    app.expanded_dirs.remove(&entry.path);
                } else {
                    app.expanded_dirs.insert(entry.path.clone());
                }
            }
        } else {
            let file_color = if is_previewed {
                Color32::from_rgb(255, 220, 100)
            } else {
                entry.category.color()
            };

            let size_str = if entry.size > 0 {
                humansize::format_size(entry.size, humansize::BINARY)
            } else {
                String::new()
            };

            let display_name = if size_str.is_empty() {
                entry.name.clone()
            } else {
                format!("{}  ({})", entry.name, size_str)
            };

            let resp = ui.selectable_label(
                is_previewed,
                RichText::new(display_name).size(12.0).color(file_color),
            );

            if resp.clicked() {
                app.preview_file(&entry.path);
            }
        }
    });

    if entry.is_dir && is_expanded {
        let children = &entry.children;
        let mut indices: Vec<usize> = (0..children.len()).collect();
        sort_indices(children, &mut indices, app.sort_by, app.sort_descending);
        for &idx in &indices {
            render_node(app, ui, &children[idx], depth + 1);
        }
    }
}

fn sort_indices(
    entries: &[FileEntry],
    indices: &mut [usize],
    sort_by: crate::models::SortBy,
    descending: bool,
) {
    match sort_by {
        crate::models::SortBy::Name => {
            indices.sort_by(|&a, &b| {
                let ea = &entries[a];
                let eb = &entries[b];
                if ea.is_dir != eb.is_dir {
                    return eb.is_dir.cmp(&ea.is_dir);
                }
                if descending {
                    eb.name.to_lowercase().cmp(&ea.name.to_lowercase())
                } else {
                    ea.name.to_lowercase().cmp(&eb.name.to_lowercase())
                }
            });
        }
        crate::models::SortBy::Size => {
            indices.sort_by(|&a, &b| {
                let sa = entries[a].total_size();
                let sb = entries[b].total_size();
                if descending {
                    sb.cmp(&sa)
                } else {
                    sa.cmp(&sb)
                }
            });
        }
        crate::models::SortBy::Modified => {
            indices.sort_by(|&a, &b| {
                if descending {
                    entries[b].last_modified.cmp(&entries[a].last_modified)
                } else {
                    entries[a].last_modified.cmp(&entries[b].last_modified)
                }
            });
        }
    }
}

fn folder_icon(entry: &FileEntry) -> &str {
    let name = entry.name.to_lowercase();
    if name.contains("src") || name.contains("source") {
        "[src]"
    } else if name.contains("test") || name.contains("spec") {
        "[tst]"
    } else if name.contains("doc") || name.contains("docs") {
        "[doc]"
    } else if name.contains("asset") || name.contains("static") || name.contains("public") {
        "[res]"
    } else if name.contains("build") || name.contains("dist") || name.contains("target") {
        "[bld]"
    } else if name.contains("config") || name.contains(".git") {
        "[cfg]"
    } else if name.contains("node_modules") {
        "[pkg]"
    } else {
        "[dir]"
    }
}
