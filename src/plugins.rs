use crate::config;

use std::fs;
use std::io;
use std::path::{PathBuf};
use serde::{Deserialize, Serialize};
use serde_json;
use semver::Version;
use url::Url;
use libloading::{Library, Symbol, Error};
use meiti_common::Plugin;

#[derive(Serialize, Deserialize)]
pub struct PluginManifest {
    name: String,
    version: Version,
    scope: String,
    summary: String,
    description: String,
    homepage: Url,
    license: String,
    source: Url,
    entry: Option<String>
}

pub struct LoadedPlugin {
    name: String,
    library: Option<Box<dyn Plugin>>
}

pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
    loaded_libraries: Vec<Library>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Vec::new(),
            loaded_libraries: Vec::new(),
        }
    }

    fn get_plugin_directories(&mut self, config: &config::Config) -> Result<Vec<PathBuf>, io::Error> {
        return Ok(fs::read_dir(&config.plugins_file_path)?
                    .into_iter()
                    .map(|r| r.unwrap().path())
                    .filter(|r| r.is_dir())
                    .collect());
    }

    pub unsafe fn load_all_plugins(&mut self, config: &config::Config) -> Result<(), Error>  {
        let plugin_directories = self.get_plugin_directories(&config).expect("Failed to get plugin directories");

        for path in plugin_directories {
            info!("Loading plugin from {:?}", path);
            self.load_plugin(path)?;
        }

        return Ok(());
    }

    pub unsafe fn load_plugin(&mut self, filename: PathBuf) -> Result<(), Error> {
        // Try to get the manifest

        let mut manifest_filename = filename.clone();
        manifest_filename.push("manifest.json");

        let manifest_file = fs::File::open(manifest_filename.as_path()).expect("Failed to open plugin manifest");
        let reader = io::BufReader::new(manifest_file);

        let manifest: PluginManifest = serde_json::from_reader(reader).expect("Invalid manifest format");

        let plugin: Option<Box<dyn Plugin>> = None;

        if manifest.entry.is_some() {
            // Load the library itself
            type PluginCreate = unsafe fn() -> *mut dyn Plugin;

            let mut library_filename = filename.clone();
            library_filename.push(manifest.entry.unwrap());

            let lib = Library::new(library_filename)?;

            // We need to keep the library around otherwise our plugin's vtable will
            // point to garbage. We do this little dance to make sure the library
            // doesn't end up getting moved.
            self.loaded_libraries.push(lib);

            let lib = self.loaded_libraries.last().unwrap();

            let constructor: Symbol<PluginCreate> = lib.get(b"_plugin_create")?;
            let boxed_raw = constructor();

            let plugin = Box::from_raw(boxed_raw);

            plugin.on_plugin_load();
        }

        let loaded_plugin_info = LoadedPlugin {
            name: manifest.name,
            library: plugin
        };

        log::info!("Loaded {} plugin", loaded_plugin_info.name);

        self.plugins.push(loaded_plugin_info);

        Ok(())
    }

    pub fn unload(&mut self) {
        log::info!("Unloading plugins");

        for plugin in self.plugins.drain(..) {
            log::info!("Running unload hooks for plugin {}", plugin.name);
            if plugin.library.is_some() {
                plugin.library.unwrap().on_plugin_unload();
            }
        }

        for lib in self.loaded_libraries.drain(..) {
            drop(lib);
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        if !self.plugins.is_empty() || !self.loaded_libraries.is_empty() {
            self.unload();
        }
    }
}
