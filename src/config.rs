extern crate directories;

use std::path::{Path, PathBuf};
use directories::{ProjectDirs};
use serde::{Deserialize};

#[derive(Deserialize, serde::Serialize)]
pub struct Config {
    pub port: u16,
    pub log_file_path: PathBuf
}

impl Default for Config {
    fn default() -> Self {
        let mut log_file_path = [".", "logs"].iter().collect();

        if let Some(proj_dirs) = ProjectDirs::from("tv", "Meiti",  "Meiti Server") {
            log_file_path = PathBuf::from(proj_dirs.data_dir());
            log_file_path.push("logs");
        }

        Self {
            port: 23400,
            log_file_path: log_file_path
        }
    }
}
