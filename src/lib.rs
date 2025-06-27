pub mod config;
pub mod watcher;

use jiff::Zoned;
use std::os::windows::prelude::OpenOptionsExt;
use std::path::PathBuf;
use std::{fs, io};
use windows::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};

pub use config::*;
pub use watcher::*;
pub struct WatcherBackupHandler {
    pub backup_dir: PathBuf,
}

impl WatcherBackupHandler {
    pub fn new(backup_dir: PathBuf) -> Self {
        if !backup_dir.exists() {
            println!("Creating backup dir: {:?}", backup_dir);
            fs::create_dir(&backup_dir).unwrap();
        }

        Self { backup_dir }
    }
    fn generate_backup_path(&self, path: &PathBuf) -> Option<PathBuf> {
        let file_stem = path.file_stem()?.to_string_lossy();
        let ext = path.extension()?.to_string_lossy();
        let ts: String = Zoned::now().strftime("%Y-%m-%d at %I_%M_%S %p").to_string();
        let backup_filename = format!("{file_stem}_{ts}.{ext}");

        Some(self.backup_dir.join(&backup_filename))
    }

    pub fn backup(&self, path: &PathBuf) -> Option<PathBuf> {
        if !path.exists() || path.is_dir() {
            return None;
        }

        let backup_path = self.generate_backup_path(path)?;
        if backup_path.exists() {
            panic!("the backup target already exists!")
        }
        // fs::copy() attempts exclusive access to the file
        let mut src = fs::OpenOptions::new()
            .read(true)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .open(path)
            .expect("unable to open file for backup");

        let mut dst = fs::File::create(&backup_path).expect("unable to create backup file");
        io::copy(&mut src, &mut dst).expect("failed to copy file contents");
        Some(backup_path)
    }
}

impl WatcherCallback for WatcherBackupHandler {
    fn handle(&mut self, events: &Vec<WatchEvent>) {
        for event in events {
            println!("Received event: {:?}", event);
            match event {
                WatchEvent::Modified(path) => {
                    if path
                        .to_str()
                        .unwrap()
                        .starts_with(self.backup_dir.to_str().unwrap())
                    {
                        // ignore events in the backup dir
                        continue;
                    }

                    self.backup(&path);
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::WatcherBackupHandler;
    use std::env;
    use std::fs;
    #[test]
    fn test_watcher_generate_backup_path() {
        let tmp = env::temp_dir();
        let test_path = &tmp.join("DRAKS00005.sl2");
        let parent = tmp.to_str().unwrap().to_string();
        let child = test_path.to_str().unwrap().to_string();

        let watcher = WatcherBackupHandler::new(tmp);

        let v = watcher.generate_backup_path(test_path).unwrap();
        assert!(!v.exists());
        assert!(child.starts_with(&parent))
    }

    #[test]
    fn test_backup() {
        let tmp = env::temp_dir();
        let target = "DRAKS00005.sl2";
        let test_path = &tmp.join(target);
        let target_contents = "If I could only be so grossly incandescent!";
        fs::write(test_path, target_contents).expect("failed to write to test file");

        let watcher = WatcherBackupHandler::new(tmp);
        watcher.backup(test_path);

        let found = fs::read_dir(&watcher.backup_dir.canonicalize().unwrap())
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| p.to_string_lossy().contains(target))
            .next()
            .expect("backup file doesn't exist in the expected location");

        let contents = fs::read_to_string(found).unwrap();
        assert_eq!(contents, target_contents);
    }
}
