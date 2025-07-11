pub mod config;
pub mod watcher;

use bnd4::{BND4Error, BND4File, CipherMode};
use glob::Pattern;
use jiff::Zoned;
use log::{debug, info, warn};
use std::fs::File;
use std::os::windows::prelude::OpenOptionsExt;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
use windows::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};

pub use config::*;
pub use watcher::*;
pub struct WatcherBackupHandler {
    pub backup_dir: PathBuf,
    encryption_key: Option<Vec<u8>>,
    verify_bnd4: bool,
    backup_mask: Option<Pattern>,
}

#[derive(Debug)]
pub enum VerifyError {
    Io(io::Error),
    BND4(BND4Error),
}
impl WatcherBackupHandler {
    pub fn new(backup_dir: PathBuf, encryption_key: Option<String>, verify_bnd4: bool) -> Self {
        if !backup_dir.exists() {
            info!("Creating backup dir: {:?}", backup_dir);
            fs::create_dir(&backup_dir).unwrap();
        } else {
            info!("Creating backup files in: {:?}", backup_dir);
        }

        let mut decoded = None;
        if encryption_key.is_some() {
            decoded = Some(hex::decode(&encryption_key.unwrap()).expect("Invalid decryption key"));
        }

        Self {
            backup_dir,
            encryption_key: decoded,
            verify_bnd4,
            backup_mask: None,
        }
    }
    pub fn with_mask(mut self, mask: &str) -> Self {
        self.backup_mask = Some(Pattern::new(mask).expect("Invalid mask"));
        info!("mask supplied, only backing up files that match: {}", mask);
        self
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

        if let Some(pattern) = &self.backup_mask {
            debug!("Checking mask: {:?}", pattern);
            if !pattern.matches(&path.to_string_lossy()) {
                debug!("Path does not match mask, skipping");
                return None;
            }
        }

        let backup_path = self.generate_backup_path(path)?;
        if backup_path.exists() {
            warn!("the backup target already exists!");
            return None;
        }
        // fs::copy() attempts exclusive access to the file
        let mut retry = 0;
        loop {
            let result = fs::OpenOptions::new()
                .read(true)
                .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
                .open(path);

            if result.is_err() {
                if retry > 2000 {
                    panic!("timed out trying to open file: {:?}", result);
                }
                debug!(
                    "couldn't open file, trying again in 100ms: {:?}",
                    result.err().unwrap()
                );
                sleep(Duration::from_millis(100));
                retry += 100;
                continue;
            }

            let mut src = result.unwrap();
            let mut dst = fs::File::create(&backup_path).expect("unable to create backup file");
            io::copy(&mut src, &mut dst).expect("failed to copy file contents");
            break;
        }
        if self.verify_bnd4 {
            self.verify(&backup_path)
                .expect("backup file appears corrupt!");
        }

        info!("backed up file: {:?}", path);
        Some(backup_path)
    }

    pub fn verify(&self, path: &PathBuf) -> Result<(), VerifyError> {
        let mut file = File::open(path).map_err(VerifyError::Io)?;
        let mut bnd4data = BND4File::from_file(&mut file).map_err(VerifyError::BND4)?;

        for entry in bnd4data.entries.iter_mut() {
            if self.encryption_key.is_some() {
                entry
                    .decrypt(CipherMode::CBC, self.encryption_key.as_ref().unwrap())
                    .map_err(VerifyError::BND4)?;
            }
        }

        info!("integrity check passed: {:?}", path);
        Ok(())
    }
}

impl WatcherCallback for WatcherBackupHandler {
    fn handle(&mut self, events: &Vec<WatchEvent>) {
        for event in events {
            debug!("Received event: {:?}", event);
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
    use std::path::PathBuf;

    #[test]
    fn test_watcher_generate_backup_path() {
        let tmp = env::temp_dir();
        let test_path = &tmp.join("DRAKS00005.sl2");
        let parent = tmp.to_str().unwrap().to_string();
        let child = test_path.to_str().unwrap().to_string();

        let watcher = WatcherBackupHandler::new(tmp, None, false);

        let v = watcher.generate_backup_path(test_path).unwrap();
        assert!(!v.exists());
        assert!(child.starts_with(&parent))
    }

    #[test]
    fn test_backup() {
        let i = rand::random::<u8>();
        let tmp = env::temp_dir().join(format!("backups-of-denial-test-{}", i));
        let backup_dir = tmp.join(format!("backups-{}", i));
        if tmp.exists() {
            fs::remove_dir_all(&tmp).unwrap();
        }
        fs::create_dir(&tmp).unwrap();
        let target = "DRAKS00005.sl2";
        let test_path = &tmp.join(target);
        let target_contents = "If I could only be so grossly incandescent!";
        fs::write(test_path, target_contents).expect("failed to write to test file");

        let watcher = WatcherBackupHandler::new(backup_dir.clone(), None, false).with_mask("*.sl2");
        assert!(watcher.backup_mask.is_some());
        watcher.backup(test_path);

        let found = fs::read_dir(&watcher.backup_dir.canonicalize().unwrap())
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| p.to_string_lossy().contains("DRAKS00005"))
            .next()
            .expect(&format!(
                "backup file doesn't exist in backup dir: {}",
                backup_dir.to_string_lossy()
            ));

        let contents = fs::read_to_string(found).unwrap();
        assert_eq!(contents, target_contents);
    }

    #[test]
    fn test_verify() {
        let target = "test/DSR.bnd4";
        let watcher = WatcherBackupHandler::new(
            PathBuf::from(env::temp_dir()),
            Some("0123456789ABCDEFFEDCBA9876543210".into()),
            true,
        );
        watcher
            .verify(&PathBuf::from(target))
            .expect("Test file didn't pass verification");
    }
}
