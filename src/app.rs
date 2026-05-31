use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Local;
use egui::{Color32, RichText, ScrollArea, TextureHandle};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;

use crate::config;
use crate::models::*;
use crate::preview;
use crate::scanner;
use crate::ui;

/// 主应用状态
pub struct App {
    // 配置
    pub config: AppConfig,
    config_path: PathBuf,

    // 项目状态
    pub selected_project_index: Option<usize>,
    pub scan_result: Option<ScanResult>,
    pub is_scanning: bool,
    scan_rx: Option<std::sync::mpsc::Receiver<ScanResult>>,

    // 文件树
    pub expanded_dirs: HashSet<PathBuf>,
    pub sort_by: SortBy,
    pub sort_descending: bool,

    // 预览
    pub preview_path: Option<PathBuf>,
    preview_content: PreviewContent,
    texture_cache: HashMap<String, TextureHandle>,

    // 搜索
    pub search_query: String,
    pub search_results: Vec<PathBuf>,

    // 添加项目对话框
    pub show_add_dialog: bool,
    pub new_project_path: String,
    pub new_project_name: String,
    pub new_project_desc: String,
    pub new_project_tags: String,

    // 编辑项目
    pub editing_project: Option<usize>,

    // 语法高亮
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,

    // 状态消息
    status_message: String,

    // 自动磁盘扫描
    pub auto_scan_rx: Option<std::sync::mpsc::Receiver<AutoScanProgress>>,
    pub auto_discovered: Vec<ProjectInfo>,
    pub auto_scanning_drive: String,
    pub is_auto_scanning: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.json")))
            .unwrap_or_else(|| PathBuf::from("config.json"));

        let config = config::load_config(Some(&config_path));

        // 加载语法高亮配置
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        Self {
            config,
            config_path,
            selected_project_index: None,
            scan_result: None,
            is_scanning: false,
            scan_rx: None,
            expanded_dirs: HashSet::new(),
            sort_by: SortBy::Name,
            sort_descending: false,
            preview_path: None,
            preview_content: PreviewContent::Empty,
            texture_cache: HashMap::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            show_add_dialog: false,
            new_project_path: String::new(),
            new_project_name: String::new(),
            new_project_desc: String::new(),
            new_project_tags: String::new(),
            editing_project: None,
            syntax_set,
            theme_set,
            status_message: String::new(),
            auto_scan_rx: None,
            auto_discovered: Vec::new(),
            auto_scanning_drive: String::new(),
            is_auto_scanning: false,
        }
    }

    /// 选择项目并触发扫描
    pub fn select_project(&mut self, index: usize) {
        if self.selected_project_index == Some(index) {
            return;
        }

        self.selected_project_index = Some(index);
        self.preview_content = PreviewContent::Empty;
        self.preview_path = None;
        self.search_results.clear();
        self.search_query.clear();
        self.expanded_dirs.clear();
        self.scan_result = None;

        self.start_scan(index);
    }

    /// 开始后台扫描
    fn start_scan(&mut self, index: usize) {
        if index >= self.config.projects.len() {
            return;
        }

        let path = PathBuf::from(&self.config.projects[index].path);
        self.is_scanning = true;

        let (tx, rx) = std::sync::mpsc::channel();
        self.scan_rx = Some(rx);

        std::thread::spawn(move || {
            let result = scanner::scan_directory(&path);
            match result {
                Ok(scan) => {
                    tx.send(scan).ok();
                }
                Err(e) => {
                    eprintln!("扫描失败: {}", e);
                }
            }
        });
    }

    /// 刷新当前项目扫描
    pub fn refresh_scan(&mut self) {
        self.scan_result = None;
        self.preview_content = PreviewContent::Empty;
        self.preview_path = None;

        if let Some(idx) = self.selected_project_index {
            self.start_scan(idx);
        }
    }

    /// 检查后台扫描是否完成
    fn check_scan_result(&mut self) {
        if let Some(rx) = &self.scan_rx {
            if let Ok(result) = rx.try_recv() {
                // 自动展开第一层目录
                for child in &result.root.children {
                    if child.is_dir {
                        self.expanded_dirs.insert(child.path.clone());
                    }
                }

                self.scan_result = Some(result);
                self.scan_rx = None;
                self.is_scanning = false;
                self.status_message = "扫描完成".to_string();
            }
        }
    }

    /// 添加项目
    pub fn add_project(&mut self) {
        let path = self.new_project_path.trim().to_string();
        let name = self.new_project_name.trim().to_string();

        if path.is_empty() || name.is_empty() {
            self.status_message = "路径和名称不能为空".to_string();
            return;
        }

        let project = ProjectInfo {
            name,
            path,
            description: self.new_project_desc.trim().to_string(),
            tags: self
                .new_project_tags
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
            added_date: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            auto_discovered: false,
            project_type: String::new(),
        };

        self.config.projects.push(project);
        self.save_config();
        self.show_add_dialog = false;
        self.status_message = "项目已添加".to_string();
    }

    /// 删除项目
    pub fn remove_project(&mut self, index: usize) {
        if index < self.config.projects.len() {
            self.config.projects.remove(index);

            if self.selected_project_index == Some(index) {
                self.selected_project_index = None;
                self.scan_result = None;
                self.preview_content = PreviewContent::Empty;
                self.preview_path = None;
            } else if let Some(ref mut sel) = self.selected_project_index {
                if *sel > index {
                    *sel -= 1;
                }
            }

            self.save_config();
            self.status_message = "项目已删除".to_string();
        }
    }

    /// 开始编辑项目
    pub fn start_editing_project(&mut self, index: usize) {
        self.editing_project = Some(index);
    }

    /// 保存编辑的项目
    pub fn save_edited_project(
        &mut self,
        index: usize,
        name: String,
        desc: String,
        tags: String,
        path: String,
    ) {
        if index < self.config.projects.len() {
            let project = &mut self.config.projects[index];
            project.name = name.trim().to_string();
            project.description = desc.trim().to_string();
            project.path = path.trim().to_string();
            project.tags = tags
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            self.save_config();
            self.status_message = "项目已更新".to_string();

            // 如果路径变了，重新扫描
            if self.selected_project_index == Some(index) {
                self.refresh_scan();
            }
        }
        self.editing_project = None;
    }

    /// 预览文件
    pub fn preview_file(&mut self, path: &Path) {
        if self.preview_path.as_ref().map(|p| p == path).unwrap_or(false) {
            return; // 已经在预览同一文件
        }

        self.preview_path = Some(path.to_path_buf());
        self.preview_content = preview::load_preview(path);
    }

    /// 执行搜索
    pub fn perform_search(&mut self) {
        self.search_results.clear();

        if self.search_query.is_empty() {
            return;
        }

        if let Some(idx) = self.selected_project_index {
            let root = PathBuf::from(&self.config.projects[idx].path);
            self.search_results = scanner::search_files(&root, &self.search_query);
        }
    }

    /// 启动自动磁盘扫描
    pub fn start_auto_scan(&mut self) {
        if self.is_auto_scanning {
            return;
        }
        self.is_auto_scanning = true;
        self.auto_discovered.clear();
        self.auto_scanning_drive.clear();

        let (tx, rx) = std::sync::mpsc::channel();
        self.auto_scan_rx = Some(rx);

        scanner::discover_projects(tx);
    }

    /// 每帧检查自动扫描进度
    fn check_auto_scan(&mut self) {
        if let Some(rx) = &self.auto_scan_rx {
            loop {
                match rx.try_recv() {
                    Ok(AutoScanProgress::ScanningDrive(drive)) => {
                        self.auto_scanning_drive = drive;
                    }
                    Ok(AutoScanProgress::FoundProject(project)) => {
                        // 去重：跳过已在列表中的项目
                        let already_exists = self.auto_discovered
                            .iter()
                            .any(|p| p.path == project.path)
                            || self.config.projects
                                .iter()
                                .any(|p| p.path == project.path);
                        if !already_exists {
                            self.auto_discovered.push(project);
                        }
                    }
                    Ok(AutoScanProgress::Finished(all)) => {
                        self.auto_discovered = all
                            .into_iter()
                            .filter(|p| {
                                !self.auto_discovered.iter().any(|existing| existing.path == p.path)
                                    && !self.config.projects.iter().any(|existing| existing.path == p.path)
                            })
                            .collect();
                        self.auto_scan_rx = None;
                        self.is_auto_scanning = false;
                        self.auto_scanning_drive.clear();
                        self.status_message = format!(
                            "磁盘扫描完成，发现 {} 个候选项目",
                            self.auto_discovered.len()
                        );
                        return;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.auto_scan_rx = None;
                        self.is_auto_scanning = false;
                        return;
                    }
                }
            }
        }
    }

    /// 将自动发现的项目添加到正式列表
    pub fn add_discovered_project(&mut self, index: usize) {
        if index < self.auto_discovered.len() {
            let project = self.auto_discovered.remove(index);
            self.config.projects.push(project);
            self.save_config();
            self.status_message = "项目已添加".to_string();
        }
    }

    /// 一键添加所有自动发现的项目
    pub fn add_all_discovered(&mut self) {
        let count = self.auto_discovered.len();
        let projects: Vec<ProjectInfo> = self.auto_discovered.drain(..).collect();
        self.config.projects.extend(projects);
        self.save_config();
        self.status_message = format!("已添加 {} 个项目", count);
    }

    /// 保存配置
    fn save_config(&self) {
        if let Err(e) = config::save_config(&self.config, Some(&self.config_path)) {
            eprintln!("保存配置失败: {}", e);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检查后台扫描
        self.check_scan_result();
        self.check_auto_scan();

        // 渲染侧边栏
        ui::sidebar::render(self, ctx);

        // 渲染主区域
        egui::CentralPanel::default().show(ctx, |ui| {
            render_main_area(self, ui, ctx);
        });

        // 持续刷新以处理后台任务
        if self.is_scanning || self.is_auto_scanning {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_config();
    }
}

/// 渲染主区域布局
fn render_main_area(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    // 顶部工具栏
    render_toolbar(app, ui);

    // 状态消息
    if !app.status_message.is_empty() {
        ui.label(
            RichText::new(&app.status_message)
                .size(12.0)
                .color(Color32::from_rgb(100, 255, 150)),
        );
    }

    ui.separator();

    if app.selected_project_index.is_none() {
        // 无项目选中时的欢迎页面
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(RichText::new("Project Folder Manager").size(24.0));
            ui.add_space(16.0);
            ui.label(
                RichText::new("本地项目文件夹可视化管理系统")
                    .size(14.0)
                    .color(Color32::GRAY),
            );
            ui.add_space(24.0);
            ui.label("请点击左侧面板的「添加项目」按钮添加要监控的文件夹");
            ui.label("或选择已有项目查看详情");
        });
        return;
    }

    // 上半部分：统计面板
    if let Some(ref scan) = app.scan_result {
        ui::stats::render(scan, ui);
        ui.add_space(8.0);
    }

    // 下半部分：文件树 + 预览，使用左右分栏
    let available = ui.available_size();

    egui::CentralPanel::default().show_inside(ui, |ui| {
        // 搜索栏（在文件树上方）
        ui::search::render(app, ui);
        ui.separator();

        let half_width = available.x / 2.0 - 4.0;
        let total_height = available.y - 160.0;

        ui.horizontal(|ui| {
            // 左：文件树
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(half_width);
                ui.set_height(total_height);
                ui::file_tree::render(app, ui);
            });

            ui.add_space(4.0);

            // 右：预览面板
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(half_width);
                ui.set_height(total_height);
                render_preview_panel(app, ui, ctx);
            });
        });
    });
}

/// 渲染顶部工具栏
fn render_toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if let Some(idx) = app.selected_project_index {
            if idx < app.config.projects.len() {
                let project = &app.config.projects[idx];
                ui.label(
                    RichText::new(format!("当前项目: {}", project.name))
                        .size(14.0)
                        .strong(),
                );

                // 标签
                if !project.tags.is_empty() {
                    ui.add_space(8.0);
                    for tag in &project.tags {
                        ui.label(
                            RichText::new(format!("#{}", tag))
                                .size(11.0)
                                .color(Color32::from_rgb(100, 200, 255)),
                        );
                    }
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // 排序
            egui::ComboBox::from_label("排序")
                .selected_text(format!("{:?}", app.sort_by))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.sort_by, SortBy::Name, "名称");
                    ui.selectable_value(&mut app.sort_by, SortBy::Size, "大小");
                    ui.selectable_value(&mut app.sort_by, SortBy::Modified, "修改时间");
                });

            if ui.button(if app.sort_descending { "降序" } else { "升序" }).clicked() {
                app.sort_descending = !app.sort_descending;
            }

            // 刷新按钮
            if ui.button("刷新").clicked() {
                app.refresh_scan();
            }
        });
    });
}

/// 渲染预览面板
fn render_preview_panel(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.label(RichText::new("文件预览").size(14.0).strong());
    ui.add_space(4.0);

    // 预览文件路径
    if let Some(ref path) = app.preview_path {
        ui.label(
            RichText::new(format!("文件: {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
                .size(11.0)
                .color(Color32::GRAY),
        );
    }

    ui.separator();

    match &app.preview_content {
        PreviewContent::Image(raw_data, size) => {
            let path = app.preview_path.clone().unwrap_or_default();
            let texture = preview::get_or_create_texture(
                ctx,
                &path,
                &mut app.texture_cache,
                raw_data,
                *size,
            );

            // 计算合适的显示尺寸
            let available = ui.available_size();
            let img_w = size[0] as f32;
            let img_h = size[1] as f32;
            let scale = (available.x / img_w).min(available.y / img_h).min(1.0);
            let display_w = img_w * scale;
            let display_h = img_h * scale;

            ScrollArea::both().show(ui, |ui| {
                ui.image(egui::ImageSource::Texture(
                    egui::load::SizedTexture::new(texture.id(), [display_w, display_h]),
                ));
            });

            ui.label(
                RichText::new(format!("原始尺寸: {} x {} px", size[0], size[1]))
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }

        PreviewContent::Text(text) => {
            ScrollArea::both().show(ui, |ui| {
                ui.label(
                    RichText::new(text.as_str())
                        .size(12.0)
                        .monospace(),
                );
            });
        }

        PreviewContent::Code { text, language } => {
            let spans = preview::highlight_code(
                text,
                language,
                &app.syntax_set,
                &app.theme_set,
            );

            ScrollArea::both().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (color, fragment) in &spans {
                        ui.label(
                            RichText::new(fragment.as_str())
                                .size(12.0)
                                .color(*color)
                                .monospace(),
                        );
                    }
                });
            });
        }

        PreviewContent::Markdown(text) => {
            ScrollArea::both().show(ui, |ui| {
                render_markdown(ui, text);
            });
        }

        PreviewContent::Unsupported(msg) => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new(msg)
                        .size(13.0)
                        .color(Color32::from_rgb(180, 180, 180)),
                );
            });
        }

        PreviewContent::Loading => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.spinner();
                ui.label("加载中...");
            });
        }

        PreviewContent::Empty => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    RichText::new("点击左侧文件树中的文件进行预览")
                        .size(13.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(8.0);
                ui.label("支持的格式:");
                ui.label("- 图片: PNG, JPG, GIF, BMP, WebP");
                ui.label("- 文本: TXT, LOG, CSV");
                ui.label("- 代码: RS, PY, JS, TS, HTML, CSS, JSON 等");
                ui.label("- 文档: MD (Markdown)");
            });
        }
    }
}

/// 简易 Markdown 渲染
fn render_markdown(ui: &mut egui::Ui, text: &str) {
    for line in text.lines() {
        if line.starts_with("### ") {
            ui.label(RichText::new(&line[4..]).size(16.0).strong());
        } else if line.starts_with("## ") {
            ui.label(RichText::new(&line[3..]).size(18.0).strong());
        } else if line.starts_with("# ") {
            ui.label(RichText::new(&line[2..]).size(20.0).strong());
        } else if line.starts_with("```") {
            ui.label(
                RichText::new(line)
                    .size(12.0)
                    .color(Color32::from_rgb(150, 150, 150))
                    .monospace(),
            );
        } else if line.starts_with("- ") || line.starts_with("* ") {
            ui.label(
                RichText::new(format!("  > {}", &line[2..]))
                    .size(12.0),
            );
        } else if line.starts_with("> ") {
            ui.label(
                RichText::new(line)
                    .size(12.0)
                    .color(Color32::from_rgb(150, 180, 150)),
            );
        } else if line.is_empty() {
            ui.add_space(4.0);
        } else {
            // 行内代码
            if line.contains('`') {
                ui.horizontal_wrapped(|ui| {
                    let parts: Vec<&str> = line.split('`').collect();
                    for (i, part) in parts.iter().enumerate() {
                        if i % 2 == 0 {
                            ui.label(RichText::new(*part).size(12.0));
                        } else {
                            ui.label(
                                RichText::new(*part)
                                    .size(12.0)
                                    .monospace()
                                    .color(Color32::from_rgb(255, 200, 100)),
                            );
                        }
                    }
                });
            } else {
                ui.label(RichText::new(line).size(12.0));
            }
        }
    }
}
