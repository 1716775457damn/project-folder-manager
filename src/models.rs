use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub description: String,
    pub tags: Vec<String>,
    pub added_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub projects: Vec<ProjectInfo>,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            window_width: 1200.0,
            window_height: 800.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileCategory {
    Code,
    Image,
    Video,
    Document,
    Archive,
    Other,
}

impl FileCategory {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" | "py" | "js" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "kts" | "scala" | "r" | "lua"
            | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "sql" | "html" | "css"
            | "scss" | "sass" | "less" | "xml" | "json" | "yaml" | "yml" | "toml" | "ini"
            | "cfg" | "conf" | "vue" | "svelte" | "dart" | "ex" | "exs" | "erl" | "hrl"
            | "hs" | "elm" | "clj" | "cljs" | "edn" | "ml" | "mli" | "nim" | "zig" | "v"
            | "proto" | "cmake" | "gradle" | "lock" | "sln" | "csproj" | "fsproj" => {
                FileCategory::Code
            }
            "ts" => FileCategory::Code, // TypeScript
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff"
            | "tif" | "heic" | "heif" | "avif" => FileCategory::Image,
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg"
            | "mpeg" | "3gp" | "ogv" | "m2ts" => FileCategory::Video,
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods"
            | "odp" | "rtf" | "txt" | "md" | "rst" | "tex" | "csv" | "tsv" | "log"
            | "epub" | "mobi" => FileCategory::Document,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "lz" | "lz4" | "zst"
            | "cab" | "iso" | "dmg" | "pkg" | "deb" | "rpm" | "apk" | "jar" | "war"
            | "tgz" | "tbz2" | "txz" => FileCategory::Archive,
            _ => FileCategory::Other,
        }
    }

    pub fn all() -> Vec<FileCategory> {
        vec![
            FileCategory::Code,
            FileCategory::Image,
            FileCategory::Video,
            FileCategory::Document,
            FileCategory::Archive,
            FileCategory::Other,
        ]
    }

    pub fn display_name(&self) -> &str {
        match self {
            FileCategory::Code => "代码",
            FileCategory::Image => "图片",
            FileCategory::Video => "视频",
            FileCategory::Document => "文档",
            FileCategory::Archive => "压缩包",
            FileCategory::Other => "其他",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            FileCategory::Code => egui::Color32::from_rgb(100, 180, 255),
            FileCategory::Image => egui::Color32::from_rgb(255, 150, 100),
            FileCategory::Video => egui::Color32::from_rgb(200, 100, 255),
            FileCategory::Document => egui::Color32::from_rgb(100, 255, 150),
            FileCategory::Archive => egui::Color32::from_rgb(255, 200, 100),
            FileCategory::Other => egui::Color32::from_rgb(180, 180, 180),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            FileCategory::Code => "{}",
            FileCategory::Image => "[i]",
            FileCategory::Video => "[v]",
            FileCategory::Document => "[d]",
            FileCategory::Archive => "[z]",
            FileCategory::Other => "[o]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub category: FileCategory,
    pub children: Vec<FileEntry>,
    pub last_modified: Option<DateTime<Utc>>,
}

impl FileEntry {
    pub fn extension(&self) -> String {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }

    pub fn total_size(&self) -> u64 {
        if !self.is_dir {
            return self.size;
        }
        self.children.iter().map(|c| c.total_size()).sum()
    }

    pub fn file_count(&self) -> usize {
        if !self.is_dir {
            return 1;
        }
        self.children.iter().map(|c| c.file_count()).sum()
    }

    pub fn dir_count(&self) -> usize {
        if !self.is_dir {
            return 0;
        }
        1 + self.children.iter().map(|c| c.dir_count()).sum::<usize>()
    }
}

#[derive(Debug, Clone)]
pub struct CategoryStats {
    pub category: FileCategory,
    pub size: u64,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub total_size: u64,
    pub file_count: usize,
    pub dir_count: usize,
    pub category_stats: Vec<CategoryStats>,
    pub root: FileEntry,
}

#[derive(Clone)]
pub enum PreviewContent {
    Image(Vec<u8>, [usize; 2]),
    Text(String),
    Code { text: String, language: String },
    Markdown(String),
    Unsupported(String),
    Loading,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Modified,
}
