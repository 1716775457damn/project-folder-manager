use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::models::FileCategory;
use image::GenericImageView;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::models::{FileCategory, PreviewContent};

/// 加载文件预览内容
pub fn load_preview(file_path: &Path) -> PreviewContent {
    if !file_path.exists() || !file_path.is_file() {
        return PreviewContent::Empty;
    }

    let extension = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // 图片预览
    if FileCategory::from_extension(&extension) == FileCategory::Image {
        return load_image_preview(file_path);
    }

    // 文本文件（包括代码和 Markdown）
    let cat = FileCategory::from_extension(&extension);
    if cat == FileCategory::Code || cat == FileCategory::Document {
        return load_text_preview(file_path, &extension);
    }

    // 无法预览
    let size = fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    PreviewContent::Unsupported(format!(
        "不支持预览此文件类型 (.{})\n文件大小: {}",
        extension,
        humansize::format_size(size, humansize::BINARY)
    ))
}

/// 加载图片预览（限制最大尺寸，沙箱化防止损坏图片 panic）
fn load_image_preview(path: &Path) -> PreviewContent {
    let path_buf = path.to_path_buf();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| image::open(&path_buf)));

    let img = match result {
        Ok(Ok(img)) => img,
        Ok(Err(e)) => return PreviewContent::Unsupported(format!("图片加载失败: {}", e)),
        Err(_) => return PreviewContent::Unsupported("此图片文件已损坏，无法预览".to_string()),
    };

    const MAX_DIM: u32 = 1920;
    let (w, h) = img.dimensions();
    let img = if w > MAX_DIM || h > MAX_DIM {
        let ratio = MAX_DIM as f64 / w.max(h) as f64;
        img.resize(
            (w as f64 * ratio) as u32,
            (h as f64 * ratio) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };
    let (width, height) = img.dimensions();
    let rgba = img.into_rgba8();
    let raw = rgba.into_raw();
    PreviewContent::Image(raw, [width as usize, height as usize])
}

/// 加载文本/代码预览
fn load_text_preview(path: &Path, extension: &str) -> PreviewContent {
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

    // 先检查文件大小，防止 OOM
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() > MAX_FILE_SIZE {
            return PreviewContent::Unsupported(format!(
                "文件过大 ({}), 无法预览文本",
                humansize::format_size(meta.len(), humansize::BINARY)
            ));
        }
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return PreviewContent::Unsupported(format!("文件读取失败: {}", e)),
    };

    // 限制预览长度（避免大文件卡顿）
    let preview_text = if content.len() > 100_000 {
        let truncated: String = content.chars().take(100_000).collect();
        format!("{}\n\n... (文件过大，仅显示前 100KB)", truncated)
    } else {
        content
    };

    if extension == "md" || extension == "markdown" {
        return PreviewContent::Markdown(preview_text);
    }

    if FileCategory::from_extension(extension) == FileCategory::Code {
        return PreviewContent::Code {
            text: preview_text,
            language: extension.to_string(),
        };
    }

    PreviewContent::Text(preview_text)
}

/// 对代码文本进行语法高亮，返回 (颜色, 文本) 列表
pub fn highlight_code(
    code: &str,
    extension: &str,
    syntax_set: &SyntaxSet,
    theme_set: &ThemeSet,
) -> Vec<(egui::Color32, String)> {
    let syntax = syntax_set
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let theme = theme_set
        .themes
        .get("base16-ocean.dark")
        .unwrap_or_else(|| {
            // fallback: 任意可用主题
            theme_set.themes.values().next().unwrap()
        });
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut spans = Vec::new();
    for line in LinesWithEndings::from(code) {
        if let Ok(highlighted) = highlighter.highlight_line(line, syntax_set) {
            for (style, text) in highlighted {
                let color = egui::Color32::from_rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                );
                spans.push((color, text.to_string()));
            }
        }
    }

    spans
}

/// 在 egui 中创建图片纹理
pub fn get_or_create_texture(
    ctx: &egui::Context,
    path: &Path,
    cache: &mut HashMap<String, egui::TextureHandle>,
    raw_data: &[u8],
    size: [usize; 2],
) -> egui::TextureHandle {
    let key = path.to_string_lossy().to_string();

    if let Some(handle) = cache.get(&key) {
        return handle.clone();
    }

    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([size[0], size[1]], raw_data);

    let options = egui::TextureOptions {
        magnification: egui::TextureFilter::Linear,
        minification: egui::TextureFilter::Linear,
        ..Default::default()
    };

    let handle = ctx.load_texture(key.clone(), color_image, options);
    cache.insert(key, handle.clone());
    handle
}
