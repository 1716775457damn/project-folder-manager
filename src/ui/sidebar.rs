use egui::{RichText, ScrollArea, Ui};

use crate::app::App;

pub fn render(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::left("project_sidebar")
        .resizable(true)
        .default_width(260.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            render_header(app, ui);
            ui.separator();
            render_auto_scan(app, ui);
            render_project_list(app, ui);
            ui.separator();
            render_add_dialog(app, ui);
        });
}

fn render_header(app: &mut App, ui: &mut Ui) {
    let manual_count = app.config.projects.iter().filter(|p| !p.auto_discovered).count();
    let auto_count = app.config.projects.iter().filter(|p| p.auto_discovered).count();

    ui.vertical_centered(|ui| {
        ui.heading(RichText::new("项目管理器").size(18.0));
        if auto_count > 0 {
            ui.label(
                RichText::new(format!(
                    "共 {} 个项目 (手动: {}, 自动: {})",
                    app.config.projects.len(),
                    manual_count,
                    auto_count
                ))
                .size(12.0)
                .color(egui::Color32::GRAY),
            );
        } else {
            ui.label(
                RichText::new(format!("共 {} 个项目", app.config.projects.len()))
                    .size(12.0)
                    .color(egui::Color32::GRAY),
            );
        }
    });
    ui.add_space(8.0);
}

fn render_auto_scan(app: &mut App, ui: &mut Ui) {
    // 自动扫描按钮
    if app.is_auto_scanning {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                RichText::new(format!("正在扫描 {}...", app.auto_scanning_drive))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(255, 200, 100)),
            );
        });
    } else {
        if ui
            .button(RichText::new("自动扫描磁盘").size(13.0))
            .clicked()
        {
            app.start_auto_scan();
        }
    }

    // 已发现项目区域
    if !app.auto_discovered.is_empty() {
        ui.add_space(6.0);
        ui.separator();
        ui.label(
            RichText::new(format!(
                "已发现 {} 个候选项目",
                app.auto_discovered.len()
            ))
            .size(13.0)
            .strong(),
        );
        ui.add_space(4.0);

        ScrollArea::vertical()
            .max_height(200.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let indices: Vec<usize> = (0..app.auto_discovered.len()).collect();
                for &idx in &indices {
                    let project = app.auto_discovered[idx].clone();
                    let type_color = project_type_color(&project.project_type);

                    let resp = ui.horizontal(|ui| {
                        // 类型标签
                        ui.label(
                            RichText::new(format!("[{}]", project.project_type))
                                .size(10.0)
                                .color(type_color),
                        );

                        // 名称
                        ui.label(
                            RichText::new(&project.name)
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(RichText::new("+").size(14.0).color(egui::Color32::from_rgb(100, 255, 100)))
                                .clicked()
                            {
                                app.add_discovered_project(idx);
                            }
                        });
                    });

                    // 路径提示
                    if resp.response.hovered() {
                        let path_display = if project.path.len() > 50 {
                            format!("{}...", &project.path[..50])
                        } else {
                            project.path.clone()
                        };
                        resp.response.on_hover_text(path_display);
                    }
                }
            });

        // "一键全添加" 按钮
        if app.auto_discovered.len() > 1 {
            ui.add_space(4.0);
            if ui
                .button(
                    RichText::new(format!(
                        "一键全添加 ({})",
                        app.auto_discovered.len()
                    ))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(100, 255, 150)),
                )
                .clicked()
            {
                app.add_all_discovered();
            }
        }
        ui.separator();
    }
}

fn project_type_color(project_type: &str) -> egui::Color32 {
    match project_type {
        "Rust" => egui::Color32::from_rgb(222, 165, 132),
        "Node.js" => egui::Color32::from_rgb(247, 223, 30),
        "Go" => egui::Color32::from_rgb(0, 173, 216),
        "Python" => egui::Color32::from_rgb(55, 118, 171),
        "Flutter" => egui::Color32::from_rgb(2, 180, 250),
        "CMake" => egui::Color32::from_rgb(200, 200, 200),
        "Maven" => egui::Color32::from_rgb(204, 52, 45),
        "Gradle" => egui::Color32::from_rgb(2, 48, 74),
        "VS Solution" => egui::Color32::from_rgb(150, 100, 200),
        "Elixir" => egui::Color32::from_rgb(75, 42, 92),
        "PHP" => egui::Color32::from_rgb(79, 93, 149),
        "Ruby" => egui::Color32::from_rgb(204, 52, 45),
        "Make" => egui::Color32::from_rgb(200, 200, 200),
        "Docker" => egui::Color32::from_rgb(36, 150, 237),
        "Haskell" => egui::Color32::from_rgb(69, 58, 82),
        "Erlang" => egui::Color32::from_rgb(162, 0, 51),
        _ => egui::Color32::from_rgb(140, 140, 140),
    }
}

fn render_project_list(app: &mut App, ui: &mut Ui) {
    // 分组：手动添加 vs 自动发现
    let manual: Vec<usize> = (0..app.config.projects.len())
        .filter(|&i| !app.config.projects[i].auto_discovered)
        .collect();
    let auto: Vec<usize> = (0..app.config.projects.len())
        .filter(|&i| app.config.projects[i].auto_discovered)
        .collect();

    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            // 手动添加分组
            if !manual.is_empty() {
                ui.label(
                    RichText::new("手动添加")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 140, 180)),
                );
                for &idx in &manual {
                    render_project_item(app, ui, idx);
                }
                if !auto.is_empty() {
                    ui.add_space(4.0);
                }
            }

            // 自动发现分组
            if !auto.is_empty() {
                ui.label(
                    RichText::new("自动发现")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(100, 200, 150)),
                );
                for &idx in &auto {
                    render_project_item(app, ui, idx);
                }
            }
        });
}

fn render_project_item(app: &mut App, ui: &mut Ui, idx: usize) {
    let project = app.config.projects[idx].clone();
    let is_selected = app.selected_project_index == Some(idx);

    let resp_opt = ui
        .selectable_label(is_selected, "")
        .context_menu(|ui| {
            if ui.button("编辑").clicked() {
                app.start_editing_project(idx);
                ui.close_menu();
            }
            if ui
                .button(RichText::new("删除").color(egui::Color32::RED))
                .clicked()
            {
                app.remove_project(idx);
                ui.close_menu();
            }
        });

    if let Some(ref inner) = resp_opt {
        let rect = inner.response.rect;
        let painter = ui.painter_at(rect);

        if is_selected {
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_premultiplied(60, 100, 200, 40),
            );
        }

        // 项目类型标签（自动发现的显示）
        let mut x_offset = 8.0;
        if project.auto_discovered && !project.project_type.is_empty() {
            let type_color = project_type_color(&project.project_type);
            let type_pos = egui::pos2(rect.left() + x_offset, rect.top() + 4.0);
            painter.text(
                type_pos,
                egui::Align2::LEFT_TOP,
                format!("[{}]", project.project_type),
                egui::FontId::proportional(10.0),
                type_color,
            );
            x_offset += painter.layout_no_wrap(
                format!("[{}]  ", project.project_type),
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            ).rect.width();
        }

        let name_pos = egui::pos2(rect.left() + x_offset, rect.top() + 4.0);
        painter.text(
            name_pos,
            egui::Align2::LEFT_TOP,
            &project.name,
            egui::FontId::proportional(14.0),
            if is_selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(200, 200, 200)
            },
        );

        let path_pos = egui::pos2(rect.left() + 8.0, rect.top() + 22.0);
        painter.text(
            path_pos,
            egui::Align2::LEFT_TOP,
            shorten_path(&project.path, 35),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(140, 140, 140),
        );

        if is_selected {
            if let Some(ref scan) = app.scan_result {
                let size_text = format!(
                    "{} | {} 文件",
                    humansize::format_size(scan.total_size, humansize::BINARY),
                    scan.file_count
                );
                let sp = egui::pos2(rect.left() + 8.0, rect.top() + 36.0);
                painter.text(
                    sp,
                    egui::Align2::LEFT_TOP,
                    size_text,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(160, 200, 255),
                );
            } else if app.is_scanning {
                let sp = egui::pos2(rect.left() + 8.0, rect.top() + 36.0);
                painter.text(
                    sp,
                    egui::Align2::LEFT_TOP,
                    "扫描中...",
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(255, 200, 100),
                );
            }
        }

        if !project.description.is_empty() && is_selected {
            let dp = egui::pos2(rect.left() + 8.0, rect.top() + 50.0);
            let desc = if project.description.len() > 40 {
                format!("{}...", &project.description[..40])
            } else {
                project.description.clone()
            };
            painter.text(
                dp,
                egui::Align2::LEFT_TOP,
                desc,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(180, 180, 180),
            );
        }

        let line_height = if !project.description.is_empty() && is_selected {
            66.0
        } else if is_selected {
            52.0
        } else {
            38.0
        };
        ui.add_space(line_height - 18.0);

        if inner.response.clicked() {
            app.select_project(idx);
        }
    }
    ui.add_space(2.0);
}

fn render_add_dialog(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);

    if ui
        .button(RichText::new("+ 添加项目").size(13.0))
        .clicked()
    {
        app.show_add_dialog = true;
        app.new_project_path.clear();
        app.new_project_name.clear();
        app.new_project_desc.clear();
        app.new_project_tags.clear();
    }

    if app.show_add_dialog {
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label("添加新项目");
            ui.add_space(4.0);

            ui.label("文件夹路径:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut app.new_project_path);
                if ui.button("浏览...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.new_project_path = folder.to_string_lossy().to_string();
                        if app.new_project_name.is_empty() {
                            app.new_project_name = folder
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                        }
                    }
                }
            });

            ui.label("项目名称:");
            ui.text_edit_singleline(&mut app.new_project_name);

            ui.label("用途描述:");
            ui.text_edit_singleline(&mut app.new_project_desc);

            ui.label("标签 (逗号分隔):");
            ui.text_edit_singleline(&mut app.new_project_tags);

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new("确认添加")
                            .color(egui::Color32::from_rgb(100, 255, 100)),
                    )
                    .clicked()
                {
                    app.add_project();
                }
                if ui.button("取消").clicked() {
                    app.show_add_dialog = false;
                }
            });
        });
    }

    if let Some(edit_idx) = app.editing_project {
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label("编辑项目");
            ui.add_space(4.0);

            let project = &app.config.projects[edit_idx].clone();
            let mut name = project.name.clone();
            let mut desc = project.description.clone();
            let mut tags = project.tags.join(", ");
            let mut path = project.path.clone();

            ui.label("文件夹路径:");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut path);
                if ui.button("浏览...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        path = folder.to_string_lossy().to_string();
                    }
                }
            });

            ui.label("项目名称:");
            ui.text_edit_singleline(&mut name);
            ui.label("用途描述:");
            ui.text_edit_singleline(&mut desc);
            ui.label("标签 (逗号分隔):");
            ui.text_edit_singleline(&mut tags);
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("保存").clicked() {
                    app.save_edited_project(edit_idx, name, desc, tags, path);
                }
                if ui.button("取消").clicked() {
                    app.editing_project = None;
                }
            });
        });
    }
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let head = &path[..max_len / 2 - 2];
    let tail = &path[path.len() - max_len / 2 + 2..];
    format!("{}...{}", head, tail)
}
