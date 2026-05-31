use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppConfig;

/// 默认配置文件路径（与可执行文件同目录）
fn default_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    exe_dir.join("config.json")
}

/// 加载配置，文件不存在则返回默认配置
pub fn load_config(path: Option<&Path>) -> AppConfig {
    let config_path = path.map(|p| p.to_path_buf()).unwrap_or_else(default_config_path);

    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(config) => {
                    eprintln!("配置已从 {:?} 加载", config_path);
                    return config;
                }
                Err(e) => {
                    eprintln!("配置解析失败 ({}), 使用默认配置", e);
                }
            },
            Err(e) => {
                eprintln!("无法读取配置文件 ({}), 使用默认配置", e);
            }
        }
    }

    AppConfig::default()
}

/// 保存配置到文件
pub fn save_config(config: &AppConfig, path: Option<&Path>) -> Result<(), String> {
    let config_path = path.map(|p| p.to_path_buf()).unwrap_or_else(default_config_path);

    let json = serde_json::to_string_pretty(config).map_err(|e| format!("序列化失败: {}", e))?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    fs::write(&config_path, json).map_err(|e| format!("写入失败: {}", e))?;

    eprintln!("配置已保存到 {:?}", config_path);
    Ok(())
}
