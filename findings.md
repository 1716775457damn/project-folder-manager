# Findings

## Codebase Structure & Current Gaps — 2026-06-01

**Source**: `src/` files (`scanner.rs`, `preview.rs`, `app.rs`, and `ui/*`)
**Relevance**: Understanding what to optimize for core safety and UX.

### 1. E0382 Compile Error (Fixed)
- **Problem**: `home_path` was used to log information *after* it was moved into the `roots` vector:
  ```rust
  roots.push(home_path);
  eprintln!("[scan] Added USERPROFILE as scan root: {}", home_path.display()); // E0382
  ```
- **Fix**: Reordered to print *before* push. Clean, zero-cost, fully compiles now.

### 2. Search Task Cancellation Gap
- **Problem**: In `scanner.rs`, the `search_files` function runs completely synchronously and doesn't accept a cancel token:
  ```rust
  pub fn search_files(root_path: &Path, query: &str) -> Vec<PathBuf>
  ```
- **Risk**: A user searching within a gigantic project folder could trigger several non-cancellable, resource-heavy filesystem crawls if they type rapidly (though there is debounce, the threads themselves have no check for cancellation once spawned).

### 3. Brute-Force Windows Drive Discovery
- **Problem**: `get_available_drives` attempts to access folders from `A:\` to `Z:\` sequentially:
  ```rust
  for letter in b'A'..=b'Z' {
      let drive = format!("{}:\\", letter as char);
      if Path::new(&drive).exists() { ... }
  }
  ```
- **Risk**: Sequential blocking checks on network mapped drives or virtual optical drives can cause severe delays or freeze up the background thread. We can utilize `sysinfo` or native APIs, or implement parallelized checks.

### 4. Large Text Files Preview OOM or Lag Risk
- **Problem**: In `preview.rs`, `load_text_preview` reads the entire file into a String up to 10MB, and takes the first 100,000 chars for preview. Reading 10MB of text entirely into memory and doing string operations might cause egui performance lags during typing or layout calculations.
