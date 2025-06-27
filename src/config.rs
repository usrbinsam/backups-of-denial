use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use toml;

#[derive(Deserialize)]
pub struct Config {
    pub backup_dir: String,
    pub save_game_dir: String,
}

impl Config {
    pub fn from_file(path: &PathBuf) -> Self {
        let config_str = fs::read_to_string(path).expect("failed to read config file");
        toml::from_str(&config_str).unwrap()
    }
}
