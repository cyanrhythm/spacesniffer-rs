use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use rust_i18n::t;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub children: Vec<FileEntry>,
    pub is_dir: bool,
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

        match scan_recursive(&root_path, &progress, &cancelled, &on_progress) {
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
) -> std::io::Result<FileEntry> {
    scan_recursive_inner(path, progress, cancelled, on_progress, 0)
}

fn scan_recursive_inner(
    path: &Path,
    progress: &Arc<Mutex<ScanProgress>>,
    cancelled: &Arc<AtomicBool>,
    on_progress: &dyn Fn(&FileEntry),
    depth: usize,
) -> std::io::Result<FileEntry> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Scan cancelled",
        ));
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let mut entry = FileEntry {
        name,
        path: path.to_path_buf(),
        size: 0,
        children: Vec::new(),
        is_dir: true,
    };

    let mut children = Vec::new();
    let mut child_dirs_to_scan = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for dir_entry in entries {
                let dir_entry = match dir_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let child_path = dir_entry.path();
                let meta = match dir_entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                {
                    let mut p = progress.lock().unwrap();
                    p.files_processed += 1;
                    p.current_path = child_path.display().to_string();
                }

                if meta.is_dir() {
                    child_dirs_to_scan.push(child_path);
                } else {
                    let size = meta.len();
                    entry.size += size;
                    {
                        let mut p = progress.lock().unwrap();
                        p.total_bytes += size;
                    }
                    let child_name = dir_entry.file_name().to_string_lossy().to_string();
                    children.push(FileEntry {
                        name: child_name,
                        path: child_path,
                        size,
                        children: Vec::new(),
                        is_dir: false,
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

    for child_path in child_dirs_to_scan {
        match scan_recursive_inner(&child_path, progress, cancelled, on_progress, depth + 1) {
            Ok(child) => {
                entry.size += child.size;
                children.push(child);
            }
            Err(_) => {
                let child_name = child_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| child_path.to_string_lossy().to_string());
                children.push(FileEntry {
                    name: child_name,
                    path: child_path,
                    size: 0,
                    children: Vec::new(),
                    is_dir: true,
                });
            }
        }

        if cancelled.load(Ordering::Relaxed) {
            break;
        }

        if depth == 0 {
            let snapshot = snapshot_entry(&entry, &children);
            on_progress(&snapshot);
        }
    }

    children.sort_by(|a, b| b.size.cmp(&a.size));

    {
        let mut p = progress.lock().unwrap();
        p.dirs_processed += 1;
    }

    entry.children = children;
    Ok(entry)
}

fn snapshot_entry(entry: &FileEntry, children: &[FileEntry]) -> FileEntry {
    let mut snap = entry.clone();
    let mut sorted: Vec<FileEntry> = children.to_vec();
    sorted.sort_by(|a, b| b.size.cmp(&a.size));
    snap.children = sorted;
    snap
}
