//! 递归目录扫描。
//!
//! # 关于目录环（Linux 上尤其要小心）
//!
//! `dir_entry.metadata()` 在 Unix 上是 `lstat` 语义，Windows 上 Rust 也不会
//! 解析 reparse point，所以符号链接根本不会被递归进去——`ln -s . loop`
//! 这种环天然安全。
//!
//! 但有三类问题 stat 拦不住：
//!
//! 1. `mount --bind /a /a/b` 造出来的环在 stat 里就是普通目录；
//! 2. 某些 FUSE / 网络文件系统可能给出自成一体的目录图；
//! 3. 递归本身没有深度上限，异常深的树会直接把栈吃穿。
//!
//! 所以这里额外维护一条祖先链（Unix 上用 `(dev, ino)`），绕回祖先的目录
//! 只留一个空占位、不再展开；同时给递归加 [`MAX_DEPTH`] 兜底。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use rust_i18n::t;

/// 递归深度上限。正常目录远达不到，超过基本就是文件系统在骗人，
/// 顺便也挡住"栈溢出"这条路。
const MAX_DEPTH: usize = 256;

/// 目录身份：`(dev, ino)`。用于识别绕回祖先的目录环。
pub type DirId = (u64, u64);

#[cfg(unix)]
fn dir_id(meta: &fs::Metadata) -> Option<DirId> {
    use std::os::unix::fs::MetadataExt;

    let ino = meta.ino();
    // 部分伪文件系统（/proc、某些 FUSE）会把 inode 报成 0 或重复值，
    // 那样整棵子树都会被误判成环，这种情况宁可不做环检测。
    (ino != 0).then(|| (meta.dev(), ino))
}

#[cfg(not(unix))]
fn dir_id(_meta: &fs::Metadata) -> Option<DirId> {
    // Windows 上目录联接（junction）不会被跟随，也没有 bind mount，
    // 拿不到稳定身份的平台上就不做这项检测。
    None
}

/// 这个目录是否已经在自己祖先链上——绕回来了。
fn is_ancestor(ancestors: &[DirId], id: Option<DirId>) -> bool {
    match id {
        Some(id) => ancestors.contains(&id),
        None => false,
    }
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<FileEntry>,
    pub is_dir: bool,
    /// 符号链接 / 目录联接：不跟随，大小取链接自身（`lstat`）。
    pub is_symlink: bool,
}

pub struct ScanProgress {
    pub phase: ScanPhase,
    pub result: Option<Arc<FileEntry>>,
    pub partial_root: Option<FileEntry>,
    pub files_processed: u64,
    pub dirs_processed: u64,
    pub skipped_dirs: u64,
    pub total_bytes: u64,
    pub current_path: String,
    pub cancelled: Arc<AtomicBool>,
}

#[derive(Clone, PartialEq)]
pub enum ScanPhase {
    Idle,
    Scanning,
    Done,
    Error(String),
}

/// Starts a recursive scan in a background thread.
/// Uses `Arc<FileEntry>` for the final result to avoid a deep clone
/// when transferring ownership from the scanner thread to the UI thread.
/// The `cancelled` flag is also passed by reference to the recursive
/// function so it can be read without locking `ScanProgress`.
pub fn start_scan(root_path: PathBuf, progress: Arc<Mutex<ScanProgress>>) {
    let cancelled = {
        let p = progress.lock().unwrap();
        p.cancelled.clone()
    };

    std::thread::spawn(move || {
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Scanning;
            p.result = None;
            p.partial_root = None;
            p.files_processed = 0;
            p.dirs_processed = 0;
            p.skipped_dirs = 0;
            p.total_bytes = 0;
            p.current_path = root_path.display().to_string();
        }

        let last_update = Arc::new(Mutex::new(Instant::now()));
        let progress_clone = progress.clone();

        let on_progress = move |snapshot: &FileEntry| {
            let mut last = last_update.lock().unwrap();
            let now = Instant::now();
            if now.duration_since(*last).as_millis() < 250 {
                return;
            }
            *last = now;
            if let Ok(mut p) = progress_clone.lock() {
                p.partial_root = Some(snapshot.clone());
            }
        };

        // 根目录的祖先链从它自己开始（这里跟随链接：用户是显式指定这条路径的）。
        let mut ancestors = Vec::new();
        if let Ok(meta) = fs::metadata(&root_path) {
            if let Some(id) = dir_id(&meta) {
                ancestors.push(id);
            }
        }

        match scan_recursive(
            &root_path,
            &progress,
            &cancelled,
            &on_progress,
            &mut ancestors,
        ) {
            Ok(root) => {
                let mut p = progress.lock().unwrap();
                p.phase = ScanPhase::Done;
                p.result = Some(Arc::new(root));
                p.partial_root = None;
            }
            Err(e) => {
                let mut p = progress.lock().unwrap();
                if !cancelled.load(Ordering::Relaxed) {
                    p.phase = ScanPhase::Error(e.to_string());
                } else {
                    p.phase = ScanPhase::Error(t!("scan_cancelled_msg").as_ref().to_string());
                }
            }
        }
    });
}

fn scan_recursive(
    path: &Path,
    progress: &Arc<Mutex<ScanProgress>>,
    cancelled: &Arc<AtomicBool>,
    on_progress: &dyn Fn(&FileEntry),
    ancestors: &mut Vec<DirId>,
) -> std::io::Result<FileEntry> {
    scan_recursive_inner(path, progress, cancelled, on_progress, 0, ancestors)
}

fn scan_recursive_inner(
    path: &Path,
    progress: &Arc<Mutex<ScanProgress>>,
    cancelled: &Arc<AtomicBool>,
    on_progress: &dyn Fn(&FileEntry),
    depth: usize,
    ancestors: &mut Vec<DirId>,
) -> std::io::Result<FileEntry> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Scan cancelled",
        ));
    }

    let name = entry_name(path);

    let mut entry = FileEntry {
        name,
        path: path.to_path_buf(),
        size: 0,
        children: Vec::new(),
        is_dir: true,
        is_symlink: false,
    };

    let mut children = Vec::new();
    let mut child_dirs_to_scan: Vec<(PathBuf, Option<DirId>)> = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for dir_entry in entries {
                let dir_entry = match dir_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let child_path = dir_entry.path();
                let file_type = match dir_entry.file_type() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let meta = match dir_entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                {
                    let mut p = progress.lock().unwrap();
                    p.files_processed += 1;
                    p.current_path = child_path.display().to_string();
                }

                let child_name = entry_name(&child_path);

                // 符号链接 / 目录联接：不跟随。
                // 跟随会把目标子树整个重复计一遍，也容易踩进环里；
                // 这里按 `lstat` 的大小（链接自身，通常就是目标路径长度）记，
                // 和 `ls -l` 一致，用户也能在 treemap 里看到它的位置。
                if file_type.is_symlink() {
                    let size = meta.len();
                    entry.size += size;
                    children.push(FileEntry {
                        name: child_name,
                        path: child_path,
                        size,
                        children: Vec::new(),
                        is_dir: false,
                        is_symlink: true,
                    });
                    continue;
                }

                if meta.is_dir() {
                    child_dirs_to_scan.push((child_path, dir_id(&meta)));
                } else {
                    let size = meta.len();
                    entry.size += size;
                    {
                        let mut p = progress.lock().unwrap();
                        p.total_bytes += size;
                    }
                    children.push(FileEntry {
                        name: child_name,
                        path: child_path,
                        size,
                        children: Vec::new(),
                        is_dir: false,
                        is_symlink: false,
                    });
                }
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let mut p = progress.lock().unwrap();
                p.skipped_dirs += 1;
                return Ok(entry);
            }
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(entry);
            }
            return Err(e);
        }
    }

    if depth == 0 {
        let snapshot = snapshot_entry(&entry, &children);
        on_progress(&snapshot);
    }

    for (child_path, child_id) in child_dirs_to_scan {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        // 目录环（bind mount 等）或超过深度上限：留个空目录占位，不展开。
        if depth + 1 > MAX_DEPTH || is_ancestor(ancestors, child_id) {
            let mut p = progress.lock().unwrap();
            p.skipped_dirs += 1;
            drop(p);

            children.push(FileEntry {
                name: entry_name(&child_path),
                path: child_path,
                size: 0,
                children: Vec::new(),
                is_dir: true,
                is_symlink: false,
            });
            continue;
        }

        let pushed = child_id.is_some();
        if let Some(id) = child_id {
            ancestors.push(id);
        }
        let result = scan_recursive_inner(
            &child_path,
            progress,
            cancelled,
            on_progress,
            depth + 1,
            ancestors,
        );
        if pushed {
            ancestors.pop();
        }

        match result {
            Ok(child) => {
                entry.size += child.size;
                children.push(child);
            }
            Err(_) => {
                children.push(FileEntry {
                    name: entry_name(&child_path),
                    path: child_path,
                    size: 0,
                    children: Vec::new(),
                    is_dir: true,
                    is_symlink: false,
                });
            }
        }

        if depth == 0 {
            let snapshot = snapshot_entry(&entry, &children);
            on_progress(&snapshot);
        }
    }

    children.sort_by_key(|child| std::cmp::Reverse(child.size));

    {
        let mut p = progress.lock().unwrap();
        p.dirs_processed += 1;
    }

    entry.children = children;
    Ok(entry)
}

fn entry_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn snapshot_entry(entry: &FileEntry, children: &[FileEntry]) -> FileEntry {
    let mut snap = entry.clone();
    let mut sorted: Vec<FileEntry> = children.to_vec();
    sorted.sort_by_key(|child| std::cmp::Reverse(child.size));
    snap.children = sorted;
    snap
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("spacesniffer-scan-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn new_progress() -> Arc<Mutex<ScanProgress>> {
        Arc::new(Mutex::new(ScanProgress {
            phase: ScanPhase::Idle,
            result: None,
            partial_root: None,
            files_processed: 0,
            dirs_processed: 0,
            skipped_dirs: 0,
            total_bytes: 0,
            current_path: String::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }))
    }

    /// 从 `root` 开始扫，ancestors 按真实根目录播种，模拟 start_scan。
    fn scan(root: &Path) -> (FileEntry, Arc<Mutex<ScanProgress>>) {
        let progress = new_progress();
        let cancelled = Arc::new(AtomicBool::new(false));
        let noop = |_: &FileEntry| {};

        let mut ancestors = Vec::new();
        if let Ok(meta) = fs::metadata(root) {
            if let Some(id) = dir_id(&meta) {
                ancestors.push(id);
            }
        }

        let tree =
            scan_recursive_inner(root, &progress, &cancelled, &noop, 0, &mut ancestors).unwrap();
        (tree, progress)
    }

    #[test]
    fn sums_nested_file_sizes() {
        let root = temp_dir("sums");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.bin"), vec![0u8; 1000]).unwrap();
        fs::write(root.join("sub").join("b.bin"), vec![0u8; 500]).unwrap();

        let (tree, _) = scan(&root);
        assert_eq!(tree.size, 1500);

        let sub = tree.children.iter().find(|c| c.name == "sub").unwrap();
        assert!(sub.is_dir);
        assert_eq!(sub.size, 500);

        let a = tree.children.iter().find(|c| c.name == "a.bin").unwrap();
        assert!(!a.is_dir && !a.is_symlink);
        assert_eq!(a.size, 1000);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ancestor_check_detects_cycles() {
        let ancestors = vec![(1, 2), (3, 4)];
        assert!(is_ancestor(&ancestors, Some((3, 4))));
        assert!(!is_ancestor(&ancestors, Some((5, 6))));
        // 拿不到身份的平台上不做拦截，避免误杀。
        assert!(!is_ancestor(&ancestors, None));
    }

    /// 把 depth 直接设在最深处，验证不再往下递归。
    #[test]
    fn depth_limit_stops_descent() {
        let root = temp_dir("depth");
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("a").join("f.bin"), vec![0u8; 4096]).unwrap();

        let progress = new_progress();
        let cancelled = Arc::new(AtomicBool::new(false));
        let noop = |_: &FileEntry| {};
        let mut ancestors = Vec::new();

        let tree = scan_recursive_inner(
            &root,
            &progress,
            &cancelled,
            &noop,
            MAX_DEPTH,
            &mut ancestors,
        )
        .unwrap();

        let a = tree.children.iter().find(|c| c.name == "a").unwrap();
        assert!(a.is_dir);
        assert!(a.children.is_empty(), "超过深度上限的目录不应展开");
        assert_eq!(progress.lock().unwrap().skipped_dirs, 1);

        fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycles_are_not_followed() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("f.bin"), vec![0u8; 1024]).unwrap();
        // sub/loop -> 根目录，真跟随就是无限递归
        symlink(&root, root.join("sub").join("loop")).unwrap();

        let (tree, _) = scan(&root);

        let sub = tree.children.iter().find(|c| c.name == "sub").unwrap();
        let link = sub
            .children
            .iter()
            .find(|c| c.name == "loop")
            .expect("链接本身应该出现在树里");
        assert!(link.is_symlink);
        assert!(link.children.is_empty(), "链接目标不能被展开");
        // 目标只算一次：1024 字节的文件 + 链接自身
        assert_eq!(tree.size, 1024 + link.size);
        assert!(
            link.size > 0,
            "链接按 lstat 大小记录，方便在 treemap 里定位"
        );

        fs::remove_dir_all(&root).ok();
    }

    /// Windows 上目录联接（junction）同样不能被跟随。
    #[cfg(windows)]
    #[test]
    fn junctions_are_not_followed() {
        let root = temp_dir("junction");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("f.bin"), vec![0u8; 1024]).unwrap();

        let link = root.join("sub").join("loop");
        // 显式用 System32 下的 cmd，避免 PATH 上的同名壳程序干扰。
        let cmd = std::env::var("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32").join("cmd.exe"))
            .unwrap_or_else(|_| PathBuf::from("cmd"));
        let made = std::process::Command::new(cmd)
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if !made {
            // 某些环境不允许创建联接，跳过而不是失败。
            eprintln!("跳过：当前环境无法创建目录联接，未覆盖 junction 分支");
            fs::remove_dir_all(&root).ok();
            return;
        }

        let (tree, _) = scan(&root);

        let sub = tree.children.iter().find(|c| c.name == "sub").unwrap();
        let junction = sub
            .children
            .iter()
            .find(|c| c.name == "loop")
            .expect("联接本身应该出现在树里");
        assert!(junction.children.is_empty(), "联接不应被展开");
        assert_eq!(tree.size, 1024 + junction.size);

        fs::remove_dir_all(&root).ok();
    }
}
