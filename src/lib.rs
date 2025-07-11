pub mod config;
pub mod watcher;

use bnd4::{BND4Error, BND4File, CipherMode};
use glob::Pattern;
use jiff::Zoned;
use log::{debug, error, info};
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
    retention_duration: Option<Duration>,
    min_backup_count: Option<usize>,
}

#[derive(Debug)]
pub enum VerifyError {
    Io(io::Error),
    BND4(BND4Error),
}
impl WatcherBackupHandler {
    pub fn new(backup_dir: PathBuf, encryption_key: Option<String>, verify_bnd4: bool) -> Self {
        if !backup_dir.exists() {
            info!("creating backup dir: {:?}", backup_dir);
            fs::create_dir(&backup_dir).unwrap();
        }

        info!("backups will be saved in: {:?}", backup_dir);
        let mut decoded = None;
        if encryption_key.is_some() {
            decoded = Some(hex::decode(&encryption_key.unwrap()).expect("Invalid decryption key"));
        }

        Self {
            backup_dir,
            encryption_key: decoded,
            verify_bnd4,
            backup_mask: None,
            retention_duration: None,
            min_backup_count: None,
        }
    }
    pub fn with_mask(mut self, mask: &str) -> Self {
        self.backup_mask = Some(Pattern::new(mask).expect("Invalid mask"));
        info!("mask supplied, only backing up files that match: {}", mask);
        self
    }

    pub fn with_retention_options(mut self, duration: Duration, min_backups: usize) -> Self {
        self.retention_duration = Some(duration);
        self.min_backup_count = Some(min_backups);
        info!(
            "backups will be pruned after {:?} once at least {} backup(s) exist",
            duration, min_backups
        );
        self
    }

    fn generate_backup_path(&self, path: &PathBuf) -> Option<PathBuf> {
        let filename = path.file_name().unwrap().to_string_lossy();
        let ts: String = Zoned::now().strftime("%Y-%m-%d %I.%M.%S %p").to_string();
        let backup_filename = format!("{ts} {filename}");

        Some(self.backup_dir.join(&backup_filename))
    }

    pub fn backup(&self, path: &PathBuf) -> Option<PathBuf> {
        if !path.exists() || path.is_dir() {
            return None;
        }

        // let game finish writing
        sleep(Duration::from_millis(200));

        if let Some(pattern) = &self.backup_mask {
            debug!("Checking mask: {:?}", pattern);
            if !pattern.matches(&path.to_string_lossy()) {
                debug!("Path does not match mask, skipping");
                return None;
            }
        }

        let backup_path = self.generate_backup_path(path)?;
        if backup_path.exists() {
            debug!("the backup target already exists!");
            return None;
        }
        // fs::copy() attempts exclusive access to the file
        let mut retry = 0;
        loop {
            let mut dst = File::create(&backup_path).expect("unable to create backup file");
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
            io::copy(&mut src, &mut dst).expect("failed to copy file contents");
            drop(src);
            break;
        }

        if self.verify_bnd4 {
            let result = self.verify(&backup_path);
            if result.is_err() {
                error!(
                    "integrity check failed, this backup file is corrupt: {:?}",
                    backup_path
                );
                panic!("exiting due to integrity check failure");
            }
        }

        info!("backup created: {:?}", backup_path);
        self.prune();
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

        debug!("integrity check passed: {:?}", path);
        Ok(())
    }

    pub fn prune(&self) {
        let duration = match &self.retention_duration {
            Some(d) => d,
            None => {
                debug!("not pruning, retention_duration is not set");
                return;
            }
        };

        let files: Vec<PathBuf> = fs::read_dir(&self.backup_dir)
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| p.is_file())
            .filter(|p| match &self.backup_mask {
                Some(pattern) => pattern.matches_path(&p),
                None => true,
            })
            .collect();

        let min_backups = self.min_backup_count.unwrap_or(0);
        debug!("there are {} backups", files.len());
        if min_backups > files.len() {
            debug!("there are not enough backups to start pruning");
            return;
        }

        let mut remaining = files.len();
        for file in files {
            let age = file
                .metadata()
                .unwrap()
                .modified()
                .unwrap()
                .elapsed()
                .unwrap()
                .as_secs();

            if age > duration.as_secs() && remaining > min_backups {
                info!("pruning old backup file: {:?}", file);
                fs::remove_file(file).unwrap();
                remaining -= 1;
            }
        }
    }
}

impl WatcherCallback for WatcherBackupHandler {
    fn handle(&mut self, events: &Vec<WatchEvent>) {
        let mut seen = vec![];
        for event in events {
            debug!("Received event: {:?}", event);
            match event {
                WatchEvent::Modified(path) => {
                    let path_str = path.to_str().unwrap();
                    if seen.contains(&path_str) {
                        debug!("skipping duplicate event: {:?}", path);
                        continue;
                    }

                    if path
                        .to_str()
                        .unwrap()
                        .starts_with(self.backup_dir.to_str().unwrap())
                    {
                        // ignore events in the backup dir
                        continue;
                    }

                    self.backup(&path);
                    seen.push(path_str);
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
