use std::path::{PathBuf};
use libloading::{Library, Symbol, Error};
use meiti_common::Plugin;

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    loaded_libraries: Vec<Library>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Vec::new(),
            loaded_libraries: Vec::new(),
        }
    }

    pub unsafe fn load_plugin(&mut self, filename: PathBuf) -> Result<(), Error> {
        type PluginCreate = unsafe fn() -> *mut dyn Plugin;

        let lib = Library::new(filename)?;

        // We need to keep the library around otherwise our plugin's vtable will
        // point to garbage. We do this little dance to make sure the library
        // doesn't end up getting moved.
        self.loaded_libraries.push(lib);

        let lib = self.loaded_libraries.last().unwrap();

        let constructor: Symbol<PluginCreate> = lib.get(b"_plugin_create")?;
        let boxed_raw = constructor();

        let plugin = Box::from_raw(boxed_raw);
        log::info!("Loaded plugin: {}", plugin.name());
        plugin.on_plugin_load();
        self.plugins.push(plugin);


        Ok(())
    }

    pub fn unload(&mut self) {
        log::info!("Unloading plugins");

        for plugin in self.plugins.drain(..) {
            log::trace!("Firing on_plugin_unload for {:?}", plugin.name());
            plugin.on_plugin_unload();
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
