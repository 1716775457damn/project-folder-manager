use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use chrono::{DateTime, TimeZone, Utc};
use walkdir::WalkDir;

use crate::models::{AutoScanProgress, CategoryStats, FileCategory, FileEntry, ProjectInfo, ScanResult};

/// 跳过隐藏目录和构建产物目录（scan_directory 和 search_files 共用）
fn skip_hidden_and_artifact_dirs(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    !name.starts_with('.')
        && name != "node_modules"
        && name != "target"
        && name != "__pycache__"
        && name != ".git"
}

pub fn scan_directory(root_path: &Path) -> Result<ScanResult, String> {
    if !root_path.exists() {
        return Err(format!("路径不存在: {:?}", root_path));
    }
    if !root_path.is_dir() {
        return Err(format!("路径不是目录: {:?}", root_path));
    }

    let root_name = root_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "root".to_string());

    let mut root_entry = FileEntry {
        name: root_name,
        path: root_path.to_path_buf(),
        is_dir: true,
        size: 0,
        size_recursive: 0,
        category: FileCategory::Other,
        children: Vec::new(),
        last_modified: get_modified_time(root_path),
    };

    let mut category_sizes: HashMap<FileCategory, (u64, usize)> = HashMap::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .max_depth(20)
        .into_iter()
        .filter_entry(skip_hidden_and_artifact_dirs)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.path() == root_path {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let name = entry.file_name().to_string_lossy().to_string();
        let modified = get_modified_time(entry.path());

        let category = if is_dir {
            FileCategory::Other
        } else {
            FileCategory::from_extension(
                &entry
                    .path()
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
            )
        };

        if !is_dir {
            let ed = category_sizes.entry(category).or_insert((0, 0));
            ed.0 += size;
            ed.1 += 1;
        }

        let file_entry = FileEntry {
            name,
            path: entry.path().to_path_buf(),
            is_dir,
            size,
            size_recursive: 0,
            category,
            children: Vec::new(),
            last_modified: modified,
        };

        insert_into_tree(&mut root_entry, file_entry);
    }

    let mut category_stats: Vec<CategoryStats> = FileCategory::all()
        .iter()
        .filter_map(|cat| {
            category_sizes.get(cat).map(|&(size, count)| CategoryStats {
                category: *cat,
                size,
                count,
            })
        })
        .collect();
    category_stats.sort_by_key(|s| -(s.size as i64));

    let total_size: u64 = category_stats.iter().map(|s| s.size).sum();
    let file_count: usize = category_stats.iter().map(|s| s.count).sum();
    let dir_count = count_dirs(&root_entry) - 1;

    // 预计算每个目录的递归大小，避免 UI 排序时反复递归
    compute_recursive_sizes(&mut root_entry);

    Ok(ScanResult {
        total_size,
        file_count,
        dir_count,
        category_stats,
        root: root_entry,
    })
}

fn insert_into_tree(root: &mut FileEntry, entry: FileEntry) {
    let relative = match entry.path.strip_prefix(&root.path) {
        Ok(r) => r.to_path_buf(),
        Err(_) => return,
    };

    let components: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components.is_empty() {
        return;
    }

    insert_recursive(root, &components, 0, &entry);
}

/// 安全递归：沿路径逐层找到或创建中间节点，最后插入叶子
fn insert_recursive(node: &mut FileEntry, components: &[String], idx: usize, entry: &FileEntry) {
    if idx >= components.len() {
        return;
    }

    let name = &components[idx];
    let is_last = idx == components.len() - 1;

    if is_last {
        node.children.push(entry.clone());
        return;
    }

    // 查找已存在的中间目录
    if let Some(pos) = node.children.iter().position(|c| c.is_dir && c.name == *name) {
        insert_recursive(&mut node.children[pos], components, idx + 1, entry);
    } else {
        let mid_path = node.path.join(name);
        node.children.push(FileEntry {
            name: name.clone(),
            path: mid_path,
            is_dir: true,
            size: 0,
            size_recursive: 0,
            category: FileCategory::Other,
            children: Vec::new(),
            last_modified: None,
        });
        let last = node.children.len() - 1;
        insert_recursive(&mut node.children[last], components, idx + 1, entry);
    }
}

fn count_dirs(entry: &FileEntry) -> usize {
    if !entry.is_dir {
        return 0;
    }
    1 + entry.children.iter().map(count_dirs).sum::<usize>()
}

/// 自底向上填充每个目录的 size_recursive
fn compute_recursive_sizes(entry: &mut FileEntry) -> u64 {
    if !entry.is_dir {
        return entry.size;
    }
    let total: u64 = entry.children.iter_mut().map(compute_recursive_sizes).sum();
    entry.size_recursive = total;
    total
}

fn get_modified_time(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| Utc.timestamp_opt(d.as_secs() as i64, 0).single())
}

pub fn search_files(root_path: &Path, query: &str) -> Vec<PathBuf> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .max_depth(20)
        .into_iter()
        .filter_entry(skip_hidden_and_artifact_dirs)
    {
        if let Ok(entry) = entry {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(&query_lower) {
                results.push(entry.path().to_path_buf());
            }
        }
    }

    results
}

// ============================================================
// 自动磁盘扫描 - 发现项目文件夹
// ============================================================

/// 项目标志文件（精确匹配）
const PROJECT_MARKER_FILES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "CMakeLists.txt",
    "Makefile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "setup.py",
    "setup.cfg",
    "pyproject.toml",
    "go.mod",
    "composer.json",
    "Gemfile",
    "Dockerfile",
    "docker-compose.yml",
    "meson.build",
    "BUILD",
    "BUILD.bazel",
    ".project",
    "pubspec.yaml",
    "mix.exs",
    "rebar.config",
    "stack.yaml",
];

/// 项目标志目录
const PROJECT_MARKER_DIRS: &[&str] = &[".git"];

/// 项目标志通配模式（按后缀）
const PROJECT_MARKER_GLOBS: &[&str] = &[".sln", ".csproj", ".fsproj", ".cabal"];

/// 自动扫描时每个目录层级最多检查的子目录数
const MAX_DIRS_PER_LEVEL: usize = 80;

/// 扫描时排除的系统目录
const SYSTEM_DIRS: &[&str] = &[
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "$Recycle.Bin",
    "System Volume Information",
    "Recovery",
    "PerfLogs",
];

/// 获取当前系统可用磁盘列表
fn get_available_drives() -> Vec<String> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if Path::new(&drive).exists() {
            drives.push(drive);
        }
    }
    drives
}

/// 从已读取的目录条目名称中检测项目标志
/// 传入 read_dir 结果中所有条目的文件名（含扩展名），避免重复打开目录
fn is_project_dir_from_entries(entry_names: &[String]) -> bool {
    // 检查标志目录
    for name in entry_names {
        if PROJECT_MARKER_DIRS.contains(&name.as_str()) {
            return true;
        }
    }

    // 检查标志文件（精确匹配）
    for name in entry_names {
        if PROJECT_MARKER_FILES.contains(&name.as_str()) {
            return true;
        }
    }

    // 检查通配模式
    for name in entry_names {
        let lower = name.to_lowercase();
        for glob in PROJECT_MARKER_GLOBS {
            if lower.ends_with(glob) {
                return true;
            }
        }
    }

    false
}
    /// 判断目录是否为项目文件夹（公开接口，供顶层 L1 检测用）
pub fn is_project_dir(path: &Path) -> bool {
    // 检查标志目录
    for marker_dir in PROJECT_MARKER_DIRS {
        let marker_path = path.join(marker_dir);
        if marker_path.exists() && marker_path.is_dir() {
            return true;
        }
    }

    // 检查标志文件（精确匹配）
    for marker_file in PROJECT_MARKER_FILES {
        let marker_path = path.join(marker_file);
        if marker_path.exists() && marker_path.is_file() {
            return true;
        }
    }

    // 检查通配模式
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            for glob in PROJECT_MARKER_GLOBS {
                if name.ends_with(glob) {
                    return true;
                }
            }
        }
    }

    false
}

/// 检测项目类型，按优先级返回类型名称
pub fn detect_project_type(path: &Path) -> String {
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("go.mod", "Go"),
        ("package.json", "Node.js"),
        ("CMakeLists.txt", "CMake"),
        ("pom.xml", "Maven"),
        ("build.gradle", "Gradle"),
        ("build.gradle.kts", "Gradle"),
        ("setup.py", "Python"),
        ("pyproject.toml", "Python"),
        ("pubspec.yaml", "Flutter"),
        ("mix.exs", "Elixir"),
        ("composer.json", "PHP"),
        ("Gemfile", "Ruby"),
        ("Makefile", "Make"),
        ("Dockerfile", "Docker"),
    ];

    for (marker, type_name) in checks {
        if path.join(marker).exists() {
            return type_name.to_string();
        }
    }

    // 检查通配模式
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".sln") {
                return "VS Solution".to_string();
            }
            if name.ends_with(".cabal") || name == "stack.yaml" {
                return "Haskell".to_string();
            }
            if name == "rebar.config" {
                return "Erlang".to_string();
            }
        }
    }

    "Other".to_string()
}

/// 从路径创建 ProjectInfo
fn create_project_info(path: &Path) -> ProjectInfo {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let project_type = detect_project_type(path);

    ProjectInfo {
        name,
        path: path.to_string_lossy().to_string(),
        description: String::new(),
        tags: vec![project_type.clone()],
        added_date: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        auto_discovered: true,
        project_type,
    }
}

/// 递归扫描目录发现项目，最大深度从盘符算
fn scan_dir_recursive(
    path: &Path,
    depth: u32,
    max_depth: u32,
    tx: &std::sync::mpsc::Sender<AutoScanProgress>,
    cancel: &Arc<AtomicBool>,
    all_found: &mut Vec<ProjectInfo>,
    dirs_scanned: &mut u32,
) {
    if depth > max_depth {
        return;
    }
    if cancel.load(Ordering::Relaxed) {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[scan] read_dir failed for {}: {}", path.display(), e);
            return;
        }
    };

    // 一次遍历收集子目录、记录文件名用于项目检测
    let mut sub_dirs: Vec<PathBuf> = Vec::new();
    let mut entry_names: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();
        let sub_path = entry.path();

        entry_names.push(name.clone());

        if !sub_path.is_dir() {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }
        sub_dirs.push(sub_path);
    }

    // 用已收集的文件名检测当前目录是否为项目
    if is_project_dir_from_entries(&entry_names) {
        *dirs_scanned += 1;
        let project = create_project_info(path);
        eprintln!("[scan] FOUND project: {}", path.display());
        let _ = tx.send(AutoScanProgress::FoundProject(project.clone()));
        all_found.push(project);
        return; // 项目目录不再递归深入
    }

    // 递归进入子目录
    for sub_path in &sub_dirs {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        scan_dir_recursive(sub_path, depth + 1, max_depth, tx, cancel, all_found, dirs_scanned);
    }
}

/// 已知的项目容器目录名（小写），这些目录优先扫描且享有更深递归深度
const PROJECT_CONTAINER_NAMES: &[&str] = &[
    "projects", "project", "dev", "code", "src", "source",
    "workspace", "workspaces", "repo", "repos", "git", "work",
];

/// 检查目录名是否是已知的项目容器
fn is_project_container(name: &str) -> bool {
    PROJECT_CONTAINER_NAMES.contains(&name.to_lowercase().as_str())
}

/// 对目录列表排序：项目容器目录排在前面
fn sort_dirs_by_priority(dirs: &mut Vec<PathBuf>) {
    dirs.sort_by(|a, b| {
        let a_name = a.file_name().map(|n| n.to_string_lossy().to_string().to_lowercase()).unwrap_or_default();
        let b_name = b.file_name().map(|n| n.to_string_lossy().to_string().to_lowercase()).unwrap_or_default();
        let a_prio = is_project_container(&a_name);
        let b_prio = is_project_container(&b_name);
        b_prio.cmp(&a_prio) // true > false，所以项目容器排在前面
    });
}

/// 扫描单个根目录，收集其子目录并按优先级排序后递归扫描
fn scan_root(
    root: &Path,
    tx: &std::sync::mpsc::Sender<AutoScanProgress>,
    cancel: &Arc<AtomicBool>,
    all_found: &mut Vec<ProjectInfo>,
    dirs_scanned: &mut u32,
) {
    const NORMAL_MAX_DEPTH: u32 = 4;
    const CONTAINER_MAX_DEPTH: u32 = 6;

    if cancel.load(Ordering::Relaxed) {
        return;
    }

    eprintln!("[scan] Scanning root: {}", root.display());
    let _ = tx.send(AutoScanProgress::ScanningDrive(root.display().to_string()));

    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[scan] Cannot read root {}: {}", root.display(), e);
            return;
        }
    };

    // 收集顶层目录：先收入所有目录，再排序
    let mut top_dirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if SYSTEM_DIRS.contains(&name.as_str()) {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }
        top_dirs.push(path);
    }

    // 排序：项目容器目录优先，确保它们在 MAX_DIRS_PER_LEVEL 截断时不被丢弃
    sort_dirs_by_priority(&mut top_dirs);

    let total = top_dirs.len();
    eprintln!("[scan] {} has {} top-level dirs, scanning up to {}", root.display(), total, MAX_DIRS_PER_LEVEL);

    for (i, dir) in top_dirs.iter().enumerate() {
        if i >= MAX_DIRS_PER_LEVEL {
            eprintln!("[scan] Reached MAX_DIRS_PER_LEVEL ({MAX_DIRS_PER_LEVEL}), skipping remaining {}/{} dirs", total - i, total);
            break;
        }
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let name = dir.file_name().map(|n| n.to_string_lossy().to_string().to_lowercase()).unwrap_or_default();
        let is_container = is_project_container(&name);

        if is_project_dir(dir) {
            *dirs_scanned += 1;
            let project = create_project_info(dir);
            eprintln!("[scan] FOUND project (L1): {}", dir.display());
            let _ = tx.send(AutoScanProgress::FoundProject(project.clone()));
            all_found.push(project);
        } else {
            // 项目容器目录用更深的最大深度
            let max_depth = if is_container { CONTAINER_MAX_DEPTH } else { NORMAL_MAX_DEPTH };
            if is_container {
                eprintln!("[scan] Container dir: {}, using max_depth={}", dir.display(), max_depth);
            }
            scan_dir_recursive(dir, 2, max_depth, tx, cancel, all_found, dirs_scanned);
        }
    }
}

/// 启动磁盘扫描线程，通过 channel 回传进度。
/// `cancel` 为取消标志，UI 设置为 true 后扫描会尽快退出。
pub fn discover_projects(tx: std::sync::mpsc::Sender<AutoScanProgress>, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let drives = get_available_drives();
        eprintln!("[scan] Available drives: {:?}", drives);
        let mut all_found: Vec<ProjectInfo> = Vec::new();
        let mut dirs_scanned = 0u32;

        // 收集所有扫描根目录
        let mut roots: Vec<PathBuf> = drives.iter().map(PathBuf::from).collect();

        // 将 USERPROFILE 加入扫描根（如果不在已有磁盘列表中）
        if let Ok(home) = std::env::var("USERPROFILE") {
            let home_path = PathBuf::from(&home);
            // 检查 USERPROFILE 本身是不是项目（如 dotfiles 仓库）
            if is_project_dir(&home_path) {
                dirs_scanned += 1;
                let project = create_project_info(&home_path);
                eprintln!("[scan] FOUND project (home itself): {}", home_path.display());
                let _ = tx.send(AutoScanProgress::FoundProject(project.clone()));
                all_found.push(project);
            }
            if !roots.contains(&home_path) {
                eprintln!("[scan] Added USERPROFILE as scan root: {}", home_path.display());
                roots.push(home_path);
            }
        }

        for root in &roots {
            if cancel.load(Ordering::Relaxed) {
                eprintln!("[scan] Cancelled before root {}", root.display());
                break;
            }
            scan_root(root, &tx, &cancel, &mut all_found, &mut dirs_scanned);
        }

        eprintln!("[scan] Done. {} dirs checked, {} projects found", dirs_scanned, all_found.len());
        let _ = tx.send(AutoScanProgress::Finished);
    });
}
