use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

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
    pub result: Option<FileEntry>,
    pub files_processed: u64,
    pub dirs_processed: u64,
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

pub fn start_scan(root_path: PathBuf, progress: Arc<Mutex<ScanProgress>>) {
    std::thread::spawn(move || {
        {
            let mut p = progress.lock().unwrap();
            p.phase = ScanPhase::Scanning;
            p.result = None;
            p.files_processed = 0;
            p.dirs_processed = 0;
            p.total_bytes = 0;
            p.current_path = root_path.display().to_string();
        }

        match scan_recursive(&root_path, &progress) {
            Ok(root) => {
                let mut p = progress.lock().unwrap();
                p.phase = ScanPhase::Done;
                p.result = Some(root);
            }
            Err(e) => {
                let mut p = progress.lock().unwrap();
                if !p.cancelled.load(Ordering::Relaxed) {
                    p.phase = ScanPhase::Error(e.to_string());
                } else {
                    p.phase = ScanPhase::Error("Scan cancelled.".to_string());
                }
            }
        }
    });
}

fn scan_recursive(path: &Path, progress: &Arc<Mutex<ScanProgress>>) -> std::io::Result<FileEntry> {
    if progress.lock().unwrap().cancelled.load(Ordering::Relaxed) {
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
            if e.kind() == std::io::ErrorKind::PermissionDenied
                || e.kind() == std::io::ErrorKind::NotFound
            {
                return Ok(entry);
            }
            return Err(e);
        }
    }

    for child_path in child_dirs_to_scan {
        match scan_recursive(&child_path, progress) {
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

        if progress.lock().unwrap().cancelled.load(Ordering::Relaxed) {
            break;
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
