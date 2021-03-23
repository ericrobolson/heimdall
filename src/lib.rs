use std::{error::Error, marker::PhantomData, path::PathBuf};

/// Macro for enabling a watchable library.
/// Ensure that
/// ```
/// [lib]
/// crate-type = ["cdylib", "rlib"]
/// ```
/// Is added to the crate.
/// This was intended that the plugin will live at the top level of the module.
/// `state` is the state that will be utilized by the library. Typically this lives in the 'host' crate.
/// `watchable` is an implementation of the `Watchable` trait. That ensures that the proper functionality is provided
/// and can be utilized by the macro.
#[macro_export]
macro_rules! init_watchable {
    (
        state: $state:ty,
        watchable: $watchable:ty
    ) => {
        use heimdall::Watchable;

        //TODO: try out by hidning no mangle + Extern C behind features

        /// Watchable init function
        #[cfg(feature = "hot-reload")]
        #[no_mangle]
        pub extern "C" fn heimdall_init() -> $state {
            <$watchable>::init()
        }

        /*
        /// Watchable reload function
        #[cfg(feature = "hot-reload")]
        #[no_mangle]
        pub extern "C" fn heimdall_reload(state: &mut $state) -> $state {
            <$watchable>::reload(state)
        }

        /// Watchable unload function
        #[cfg(feature = "hot-reload")]
        #[no_mangle]
        pub extern "C" fn heimdall_unload(state: &mut $state) -> $state {
            <$watchable>::unload(state)
        }
        */
        /// Watchable update function
        #[cfg(feature = "hot-reload")]
        #[no_mangle]
        pub extern "C" fn heimdall_update(state: $state) -> $state {
            <$watchable>::update(state)
        }
    };
}

/// Implementation required for a watchable/reloadable library
pub trait Watchable<State> {
    /// Called upon initial loading of the program
    fn init() -> State;

    /*
    /// Called when the module is reloaded
    fn reload(state: State) -> State;
    /// Called when the module is unloaded
    fn unload(state: State) -> State;
    */
    /// Called when the program requires an update of the state
    fn update(state: State) -> State;
}

pub enum WatchResult {
    NoChange,
    Updated,
    Err(Box<dyn Error>),
}

pub struct Watcher<State, Plugin>
where
    Plugin: Watchable<State>,
{
    #[cfg(feature = "hot-reload")]
    file_path: PathBuf,
    #[cfg(feature = "hot-reload")]
    last_updated: std::time::SystemTime,
    #[cfg(feature = "hot-reload")]
    lib: Option<libloading::Library>,

    phantom: PhantomData<(Plugin, State)>,
}

impl<State, Plugin> Watcher<State, Plugin>
where
    Plugin: Watchable<State>,
{
    /// Creates a new Watcher and executor for the plugin.
    /// `file_path` is a link to the dynamic library.
    pub fn new(file_path: PathBuf) -> (Self, State) {
        #[cfg(not(feature = "hot-reload"))]
        {
            let state = Plugin::init();

            (
                Self {
                    phantom: PhantomData,
                },
                state,
            )
        }

        #[cfg(feature = "hot-reload")]
        {
            let (lib, last_updated) = Self::load_lib(&file_path).unwrap();
            let state = Self::heimdall_init(&lib);

            (
                Self {
                    file_path,
                    last_updated,
                    lib: Some(lib),
                    phantom: PhantomData,
                },
                state,
            )
        }
    }

    /// Watches the file, reloading the dynamic library if it was modified. No-op when the feature is not enabled.
    pub fn watch(&mut self) -> WatchResult {
        #[cfg(not(feature = "hot-reload"))]
        {
            return WatchResult::NoChange;
        }

        #[cfg(feature = "hot-reload")]
        {
            use std::fs::File;

            let file = match File::open(self.file_path.clone()) {
                Ok(f) => f,
                Err(e) => {
                    return WatchResult::Err(Box::new(e));
                }
            };

            let last_updated = file.metadata().unwrap().modified().unwrap();

            if last_updated > self.last_updated {
                // Do unload
                // Self::heimdall_unload(self.lib(), state);

                // Get and load the library
                {
                    self.lib = None;

                    let (lib, last_updated) = match Self::load_lib(&self.file_path) {
                        Ok(result) => result,
                        Err(e) => {
                            return WatchResult::Err(e);
                        }
                    };

                    self.last_updated = last_updated;
                    self.lib = Some(lib);
                }

                // Reload
                //  Self::heimdall_reload(self.lib(), state);

                WatchResult::Updated
            } else {
                WatchResult::NoChange
            }
        }
    }

    /// Calls the 'update' state for the plugin.
    pub fn update(&self, state: State) -> State {
        #[cfg(not(feature = "hot-reload"))]
        {
            Plugin::update(state)
        }
        #[cfg(feature = "hot-reload")]
        {
            Self::heimdall_update(self.lib(), state)
        }
    }

    /// Returns a handle to the loaded library. Will panic if it has not been initialized.
    #[cfg(feature = "hot-reload")]
    fn lib(&self) -> &libloading::Library {
        match &self.lib {
            Some(lib) => lib,
            None => panic!("Dynamic plugin has not been loaded!"),
        }
    }

    /// Clones the original lib, then returns a handle to the clone.
    #[cfg(feature = "hot-reload")]
    fn load_lib(
        original_path: &PathBuf,
    ) -> Result<(libloading::Library, std::time::SystemTime), Box<dyn Error>> {
        use std::fs::File;

        // Clone the DLL to enable watching of the original. While expensive, it bypasses a lot of issues
        // that may occur when another process is modifying the original.
        let cloned_name = Self::make_cloned_name(original_path);
        std::fs::copy(original_path, cloned_name.clone())?;

        // Get the last updated
        let file = File::open(original_path)?;

        let last_updated = file.metadata()?.modified()?;

        // Load the lib
        let lib = unsafe { libloading::Library::new(cloned_name.clone().as_os_str())? };

        Ok((lib, last_updated))
    }

    /// Creates the 'cloned' dll name
    #[cfg(feature = "hot-reload")]
    fn make_cloned_name(path: &PathBuf) -> PathBuf {
        let file_name = path.file_name().unwrap();
        let extension = path.extension().unwrap().to_str().unwrap();
        let file_name = String::from(file_name.to_str().unwrap());

        let mut file_name = file_name.replace(extension, "");
        file_name.pop();
        file_name.push_str("_updated");
        file_name.push('.');
        file_name.push_str(extension);

        let mut path = path.clone();
        path.set_file_name(file_name);

        path
    }

    /// Implementation for the `init()` functionality.
    #[cfg(feature = "hot-reload")]
    fn heimdall_init(lib: &libloading::Library) -> State {
        let func: libloading::Symbol<unsafe fn() -> State> =
            unsafe { lib.get(b"heimdall_init").unwrap() };
        let state = unsafe { func() };

        state
    }

    /// Implementation for the `update()` functionality.
    #[cfg(feature = "hot-reload")]
    fn heimdall_update(lib: &libloading::Library, state: State) -> State {
        let func: libloading::Symbol<unsafe fn(State) -> State> =
            unsafe { lib.get(b"heimdall_update").unwrap() };

        unsafe { func(state) }
    }

    /// Implementation for the `unload()` functionality.
    #[cfg(feature = "hot-reload")]
    fn heimdall_unload(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_unload").unwrap() };

        unsafe {
            func(state);
        };
    }

    /// Implementation for the `reload()` functionality.
    #[cfg(feature = "hot-reload")]
    fn heimdall_reload(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_reload").unwrap() };

        unsafe {
            func(state);
        };
    }
}
