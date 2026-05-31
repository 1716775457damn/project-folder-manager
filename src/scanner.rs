use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, TimeZone, Utc};
use walkdir::WalkDir;

use crate::models::{CategoryStats, FileCategory, FileEntry, ScanResult};

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
