extern crate directories;

use std::path::{PathBuf};
use directories::{ProjectDirs};
use serde::{Deserialize};

#[derive(Deserialize, serde::Serialize)]
pub struct Config {
    pub port: u16,
    pub log_file_path: PathBuf,
    pub plugins_file_path: PathBuf,
    pub database_file_path: PathBuf
}

impl Default for Config {
    fn default() -> Self {
        let mut log_file_path = [".", "logs"].iter().collect();
        let mut plugins_file_path = [".", "plugins"].iter().collect();
        let mut database_file_path = [".", "meiti.sqlite"].iter().collect();

        if let Some(proj_dirs) = ProjectDirs::from("tv", "Meiti",  "Meiti Server") {
            log_file_path = PathBuf::from(proj_dirs.data_dir());
            log_file_path.push("logs");
            plugins_file_path = PathBuf::from(proj_dirs.data_dir());
            plugins_file_path.push("plugins");
            database_file_path = PathBuf::from(proj_dirs.data_dir());
            database_file_path.push("meiti.sqlite");
        }

        Self {
            port: 23400,
            log_file_path: log_file_path,
            plugins_file_path: plugins_file_path,
            database_file_path: database_file_path
        }
    }
}

