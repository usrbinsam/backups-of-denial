use backups_of_denial::Config;
use backups_of_denial::{Watcher, WatcherBackupHandler};
use std::env;

fn main() {
    let home = env::home_dir().expect("your %USERPROFILE% envvar is not set");
    let config_path = home.join(".backups-of-denial.toml");
    if !config_path.exists() {
        eprintln!("config file not found: {}", config_path.display());
        return;
    }
    let config = Config::from_file(&config_path);
    let watcher_handler = WatcherBackupHandler::new(
        config.backup_dir.parse().unwrap(),
        config.encryption_key,
        config.verify_bnd4,
    );
    let mut watcher = Watcher::new(config.save_game_dir.parse().unwrap(), watcher_handler);
    println!("watching {} for changes", config.save_game_dir);
    loop {
        watcher.watch()
    }
}
