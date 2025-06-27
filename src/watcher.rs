use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::{ptr, slice};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_ACTION_MODIFIED, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

// for Dark Souls, we only care about the Modified event,
// but this could be expanded to deal with the other events.
// https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-file_notify_information
#[derive(Debug, Eq, PartialEq, PartialOrd, Clone)]
pub enum WatchEvent {
    Modified(PathBuf),
}

pub trait WatcherCallback {
    fn handle(&mut self, paths: &Vec<WatchEvent>);
}

pub struct Watcher<C: WatcherCallback> {
    pub base_dir: PathBuf,
    pub callback: C,
}
impl<C: WatcherCallback> Watcher<C> {
    pub fn new(base_dir: PathBuf, callback: C) -> Self {
        Self { base_dir, callback }
    }

    pub fn watch(&mut self) {
        let encoded_path: Vec<u16> = self
            .base_dir
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut buffer: [u8; 4096] = [0; 4096];
        let mut bytes_received = 0u32;
        let mut changes: Vec<WatchEvent> = vec![];

        unsafe {
            let handle = CreateFileW(
                PCWSTR::from_raw(encoded_path.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_DELETE | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
            .expect("CreateFileW() failed");

            ReadDirectoryChangesW(
                handle,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                true,
                FILE_NOTIFY_CHANGE_LAST_WRITE,
                Some(&mut bytes_received),
                None,
                None,
            )
            .expect("ReadDirectoryChangesW() failed");
        };

        let mut cur_offset: *const u8 = buffer.as_ptr();
        let mut cur_entry =
            unsafe { ptr::read_unaligned(cur_offset as *const FILE_NOTIFY_INFORMATION) };

        loop {
            let file_name_len = cur_entry.FileNameLength as usize / 2;
            let encoded_path: &[u16] = unsafe {
                slice::from_raw_parts(
                    cur_offset
                        .offset(std::mem::offset_of!(FILE_NOTIFY_INFORMATION, FileName) as isize)
                        as _,
                    file_name_len,
                )
            };

            let changed_path = self.base_dir.join(PathBuf::from(
                String::from_utf16_lossy(encoded_path).to_string(),
            ));

            let action = match cur_entry.Action {
                FILE_ACTION_MODIFIED => WatchEvent::Modified(changed_path),
                _ => {
                    continue; // don't care about this action
                }
            };

            changes.push(action);

            if cur_entry.NextEntryOffset == 0 {
                break;
            }

            unsafe {
                cur_offset = cur_offset.offset(cur_entry.NextEntryOffset as isize);
                cur_entry = ptr::read_unaligned(cur_offset as *const FILE_NOTIFY_INFORMATION);
            }
        }

        if !changes.is_empty() {
            self.callback.handle(&changes);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    struct TestHandler {
        changes: Arc<Mutex<Vec<WatchEvent>>>,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                changes: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl WatcherCallback for TestHandler {
        fn handle(&mut self, paths: &Vec<WatchEvent>) {
            let mut changes = self.changes.lock().expect("couldn't lock Mutex");
            *changes = paths.to_owned();
        }
    }
    #[test]
    fn test_win32_watch() {
        let tmp = std::env::temp_dir().join("BackupsOfDenial");
        let test_file = tmp.join("foo.txt");
        let test_content = "if i could only be so grossly incandescent ...\n";
        fs::write(&test_file, &test_content).expect("unable to write test file");

        let test_handler = TestHandler::new();
        let changes = test_handler.changes.clone();
        let mut watcher = Watcher::new(tmp, test_handler);

        let thr = thread::spawn(move || watcher.watch());
        thread::sleep(Duration::from_millis(100));

        let mut f = fs::File::options()
            .append(true)
            .open(test_file)
            .expect("unable to re-open test file");

        f.write_all("the flow of time is convoluted in Lordran\n".as_bytes())
            .expect("unable to write to file");

        drop(f);

        thr.join().unwrap();

        let events = changes.lock().unwrap();
        let event = events.first().unwrap();

        match event {
            WatchEvent::Modified(path) => {
                assert_eq!(
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .contains("foo.txt"),
                    true
                );
            }
        }
    }
}
