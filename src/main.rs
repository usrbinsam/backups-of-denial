use backups_of_denial::{Watcher, WatcherBackupHandler};

fn main() {
    let save_game_path = r"C:\Users\minic\OneDrive\Documents\NBGI\DARK SOULS REMASTERED"
        .parse()
        .unwrap();

    let backup_dir = r"\\10.10.10.254\sam\BackupsOfDenial".parse().unwrap();
    let watcher_handler = WatcherBackupHandler::new(backup_dir);
    let mut watcher = Watcher::new(save_game_path, watcher_handler);

    loop {
        watcher.watch()
    }
}
