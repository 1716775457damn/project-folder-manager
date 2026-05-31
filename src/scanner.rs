use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, TimeZone, Utc};
use walkdir::WalkDir;

use crate::models::{AutoScanProgress, CategoryStats, FileCategory, FileEntry, ProjectInfo, ScanResult};

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
        category: FileCategory::Other,
        children: Vec::new(),
        last_modified: get_modified_time(root_path),
    };

    let mut category_sizes: HashMap<FileCategory, (u64, usize)> = HashMap::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .max_depth(20)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && name != "__pycache__"
                && name != ".git"
        })
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

    let mut current: *mut FileEntry = root;
    let last_idx = components.len() - 1;

    for (i, name) in components.iter().enumerate() {
        let cur = unsafe { &mut *current };
        if i == last_idx {
            cur.children.push(entry.clone());
        } else {
            let found = cur.children.iter().position(|c| c.is_dir && c.name == *name);
            match found {
                Some(idx) => {
                    current = cur.children.as_mut_ptr().wrapping_add(idx);
                }
                None => {
                    let mid_path = cur.path.join(name);
                    cur.children.push(FileEntry {
                        name: name.clone(),
                        path: mid_path,
                        is_dir: true,
                        size: 0,
                        category: FileCategory::Other,
                        children: Vec::new(),
                        last_modified: None,
                    });
                    let new_idx = cur.children.len() - 1;
                    current = cur.children.as_mut_ptr().wrapping_add(new_idx);
                }
            }
        }
    }
}

fn count_dirs(entry: &FileEntry) -> usize {
    if !entry.is_dir {
        return 0;
    }
    1 + entry.children.iter().map(count_dirs).sum::<usize>()
}

fn get_modified_time(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| Utc.timestamp_opt(d.as_secs() as i64, 0).unwrap())
}

pub fn search_files(root_path: &Path, query: &str) -> Vec<PathBuf> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .max_depth(15)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
                && name != "node_modules"
                && name != "target"
                && name != ".git"
        })
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

/// 每层最多扫描目录数
const MAX_DIRS_PER_LEVEL: usize = 200;

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

/// 判断目录是否为项目文件夹
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

/// 启动磁盘扫描线程，通过 channel 回传进度
pub fn discover_projects(tx: std::sync::mpsc::Sender<AutoScanProgress>) {
    std::thread::spawn(move || {
        let drives = get_available_drives();
        let mut all_found: Vec<ProjectInfo> = Vec::new();

        for drive in &drives {
            tx.send(AutoScanProgress::ScanningDrive(drive.clone()))
                .ok();

            let drive_path = Path::new(drive);
            let entries = match fs::read_dir(drive_path) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let mut top_dirs: Vec<PathBuf> = Vec::new();
            for entry in entries.flatten() {
                if top_dirs.len() >= MAX_DIRS_PER_LEVEL {
                    break;
                }
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

            for dir in &top_dirs {
                if is_project_dir(dir) {
                    let project = create_project_info(dir);
                    tx.send(AutoScanProgress::FoundProject(project.clone()))
                        .ok();
                    all_found.push(project);
                } else {
                    // 深入一层检查子目录
                    let sub_entries = match fs::read_dir(dir) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let mut count = 0;
                    for sub_entry in sub_entries.flatten() {
                        if count >= MAX_DIRS_PER_LEVEL {
                            break;
                        }
                        let sub_path = sub_entry.path();
                        if !sub_path.is_dir() {
                            continue;
                        }
                        let sub_name = sub_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        if sub_name.starts_with('.') {
                            continue;
                        }
                        if is_project_dir(&sub_path) {
                            let project = create_project_info(&sub_path);
                            tx.send(AutoScanProgress::FoundProject(project.clone()))
                                .ok();
                            all_found.push(project);
                        }
                        count += 1;
                    }
                }
            }
        }

        tx.send(AutoScanProgress::Finished(all_found)).ok();
    });
}
