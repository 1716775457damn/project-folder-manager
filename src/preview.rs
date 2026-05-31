use std::collections::HashMap;
use std::fs;
use std::path::Path;

use image::GenericImageView;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::models::PreviewContent;

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
    if matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico"
    ) {
        return load_image_preview(file_path);
    }

    // 文本文件（包括代码和 Markdown）
    if is_text_file(&extension) || is_code_file(&extension) {
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

/// 加载图片预览
fn load_image_preview(path: &Path) -> PreviewContent {
    match image::open(path) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let rgba = img.into_rgba8();
            let raw = rgba.into_raw();
            PreviewContent::Image(raw, [width as usize, height as usize])
        }
        Err(e) => PreviewContent::Unsupported(format!("图片加载失败: {}", e)),
    }
}

/// 加载文本/代码预览
fn load_text_preview(path: &Path, extension: &str) -> PreviewContent {
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

    if is_code_file(extension) {
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

    let theme = &theme_set.themes["base16-ocean.dark"];
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

/// 判断是否为文本文件
fn is_text_file(ext: &str) -> bool {
    matches!(
        ext,
        "txt" | "md" | "markdown" | "rst" | "log" | "csv" | "tsv"
    )
}

/// 判断是否为代码文件
fn is_code_file(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp"
            | "cs" | "rb" | "php" | "swift" | "kt" | "kts" | "scala" | "r" | "lua" | "sh"
            | "bash" | "zsh" | "ps1" | "bat" | "sql" | "html" | "css" | "scss" | "less"
            | "xml" | "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "vue"
            | "svelte" | "dart" | "ex" | "exs" | "erl" | "hs" | "elm" | "clj" | "cljs"
            | "ml" | "mli" | "nim" | "zig" | "v" | "proto" | "cmake" | "gradle"
    )
}
