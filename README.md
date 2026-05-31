# Project Folder Manager

本地项目文件夹可视化管理系统，帮助用户直观查看电脑上各个项目文件夹的用途、占用空间、文件组成，并支持文件预览。

## 功能特性

- **项目管理**：添加/删除/编辑要监控的项目文件夹路径，数据持久化到本地 JSON
- **用途标注**：为每个项目文件夹添加用途描述和标签
- **资源占用分析**：显示文件夹总大小、按类型（代码/图片/视频/文档/压缩包/其他）分类统计、文件数量
- **可视化展示**：进度条风格的占比显示，一目了然
- **文件浏览器**：树形结构展示文件夹内容，支持展开/折叠
- **文件预览**：
  - 图片预览（PNG/JPG/GIF/BMP/WebP）
  - 文本文件预览（带语法高亮）
  - Markdown 渲染预览
- **搜索功能**：在项目文件夹内按文件名搜索
- **排序功能**：按名称、大小排序

## 截���

> *截图占位 — 运行应用后在此处添加截图*

## 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust |
| GUI 框架 | eframe / egui |
| 文件系统扫描 | walkdir |
| 图片处理 | image |
| 语法高亮 | syntect |
| 配置持久化 | serde / serde_json |
| 系统信息 | sysinfo |
| 时间处理 | chrono |
| 文件大小格式化 | humansize |
| 文件对话框 | rfd |

## 安装

### 从源码构建

确保已安装 [Rust](https://www.rust-lang.org/tools/install) 工具链。

```bash
git clone <repository-url>
cd project-folder-manager
cargo build --release
```

编译产物位于 `target/release/project-folder-manager.exe`。

### 直接下载

前往 [Releases](https://github.com/your-username/project-folder-manager/releases) 页面下载最新版本的 exe 文件。

## 使用说明

1. 启动应用后，点击左侧面板的「添加项目」按钮
2. 选择要监控的本地项目文件夹路径
3. 为项目添加名称、用途描述和标签（可选）
4. 点击项目即可查看文件夹统计信息和文件树
5. 在文件树中点击文件可在右侧预览面板查看内容
6. 使用搜索框快速定位文件

## 配置存储

配置文件存储在应用同级目录下的 `config.json`，包含所有项目信息和窗口状态。

## License

MIT
