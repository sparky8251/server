use std::any::Any;
use rocket::Route;

pub trait Plugin: Any + Send + Sync {
    fn name(&self) -> &'static str;
    fn on_plugin_load(&self) {}
    fn on_plugin_unload(&self) {}
    // TODO: There may be a better way to do this than having to import Rocket?
    fn register_routes(&self) -> Vec<Route> {
        vec![]
    }
}

#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut $crate::Plugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<$crate::Plugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}
