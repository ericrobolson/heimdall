use std::{env, error::Error, marker::PhantomData, path::PathBuf};

/// Macro for enabling a watchable library.
/// Ensure that
/// ```
/// [lib]
/// crate-type = ["cdylib", "rlib"]
/// ```
/// Is added to the crate.
/// `state` is the state that will be utilized by the library. Typically this lives in the 'host' crate.
/// `watchable` is an implementation of the `Watchable` trait. That ensures that the proper functionality is provided.
#[macro_export]
macro_rules! init_watchable {
    (
        state: $state:ty,
        watchable: $watchable:ty
    ) => {
        use heimdall::Watchable;

        /// Watchable init function
        #[no_mangle]
        pub extern "C" fn heimdall_init() -> $state {
            <$watchable>::init()
        }

        /// Watchable reload function
        #[no_mangle]
        pub extern "C" fn heimdall_reload(state: &mut $state) {
            <$watchable>::reload(state);
        }

        /// Watchable unload function
        #[no_mangle]
        pub extern "C" fn heimdall_unload(state: &mut $state) {
            <$watchable>::unload(state);
        }

        /// Watchable update function
        #[no_mangle]
        pub extern "C" fn heimdall_update(state: &mut $state) {
            <$watchable>::update(state);
        }

        /// Watchable finalize function
        #[no_mangle]
        pub extern "C" fn heimdall_finalize(state: &mut $state) {
            <$watchable>::finalize(state);
        }
    };
}

/// Implementation required for a watchable library
pub trait Watchable<State> {
    /// Called upon initial loading of the program
    fn init() -> State;
    /// Called when the module is reloaded
    fn reload(state: &mut State);
    /// Called when the module is unloaded
    fn unload(state: &mut State);
    /// Called when the program requires an update of the state
    fn update(state: &mut State);
    /// Called when the program is about to exit
    fn finalize(state: &mut State);
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

    #[cfg(feature = "hot-reload")]
    fn heimdall_init(lib: &libloading::Library) -> State {
        let func: libloading::Symbol<unsafe fn() -> State> =
            unsafe { lib.get(b"heimdall_init").unwrap() };
        let state = unsafe { func() };

        state
    }

    #[cfg(feature = "hot-reload")]
    fn heimdall_update(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_update").unwrap() };

        unsafe {
            func(state);
        };
    }

    #[cfg(feature = "hot-reload")]
    fn heimdall_unload(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_unload").unwrap() };

        unsafe {
            func(state);
        };
    }

    #[cfg(feature = "hot-reload")]
    fn heimdall_reload(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_reload").unwrap() };

        unsafe {
            func(state);
        };
    }

    #[cfg(feature = "hot-reload")]
    fn heimdall_finalize(lib: &libloading::Library, state: &mut State) {
        let func: libloading::Symbol<unsafe fn(&mut State) -> State> =
            unsafe { lib.get(b"heimdall_finalize").unwrap() };

        unsafe {
            func(state);
        };
    }

    /// Watches the file
    pub fn watch(&mut self, state: &mut State) -> WatchResult {
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
                Self::heimdall_unload(self.lib(), state);

                self.lib = None;

                let (lib, last_updated) = match Self::load_lib(&self.file_path) {
                    Ok(result) => result,
                    Err(e) => {
                        return WatchResult::Err(e);
                    }
                };

                self.last_updated = last_updated;
                self.lib = Some(lib);

                Self::heimdall_reload(self.lib(), state);

                WatchResult::Updated
            } else {
                WatchResult::NoChange
            }
        }
    }

    /// Calls the 'update' state for the plugin.
    pub fn update(&self, state: &mut State) {
        #[cfg(not(feature = "hot-reload"))]
        {
            Plugin::update(state);
        }

        #[cfg(feature = "hot-reload")]
        {
            Self::heimdall_update(self.lib(), state);
        }
    }

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

        // Clone the DLL to enable watching
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
}
