extern crate libloading;
use libloading::{Library, Symbol};
use std::{env, error::Error, path::PathBuf};
use std::{fs, time::SystemTime};
use std::{path::Path, time::Duration};

type AddFunc = unsafe fn(isize, isize) -> isize;

fn main() {
    let library_path = "./test_print/target/debug/test_print.dll";

    let mut reloadable = Reloadable::new(library_path.into());

    let mut should_run = true;

    loop {
        if !should_run {
            match reloadable.watch() {
                WatchResult::Updated => {
                    should_run = true;
                }
                WatchResult::NoChange => {}
                WatchResult::Err(e) => {
                    println!("{:?}", e);
                }
            }
        }

        if should_run {
            should_run = false;
            match reloadable.lib() {
                Some(lib) => unsafe {
                    let func: Symbol<AddFunc> = lib.get(b"add").unwrap();
                    let answer = func(1, 2);
                    println!("1 + 2 = {}", answer);
                },
                None => {}
            }
        }
    }
}

pub fn test_load(s: &'static str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::prelude::*;

    let file = File::open(s)?;
    println!("{:?}", file.metadata().unwrap().created());
    Ok(())
}

struct Reloadable {
    fname: PathBuf,
    last_updated: std::time::SystemTime,
    lib: Option<libloading::Library>,
}

pub enum WatchResult {
    NoChange,
    Updated,
    Err(Box<dyn Error>),
}

impl Reloadable {
    pub fn new(fname: PathBuf) -> Self {
        let (lib, last_updated) = Self::load_lib(&fname, true).unwrap();

        Self {
            fname,
            last_updated,
            lib: Some(lib),
        }
    }

    /// Clones the original lib, then returns a handle to the clone.
    fn load_lib(
        original_path: &PathBuf,
        throw_errors: bool,
    ) -> Result<(libloading::Library, SystemTime), Box<dyn Error>> {
        use std::fs::File;

        // Clone the DLL to enable watching
        let cloned_name = Self::make_cloned_name(original_path);
        std::fs::copy(original_path, cloned_name.clone())?;

        // Get the last updated
        let file = File::open(original_path)?;

        let last_updated = file.metadata()?.modified()?;

        // Load the lib
        let lib = unsafe { Library::new(cloned_name.clone().as_os_str())? };

        Ok((lib, last_updated))
    }

    pub fn watch(&mut self) -> WatchResult {
        use std::fs::File;

        let file = match File::open(self.fname.clone()) {
            Ok(f) => f,
            Err(e) => {
                return WatchResult::Err(Box::new(e));
            }
        };

        let last_updated = file.metadata().unwrap().modified().unwrap();

        if last_updated > self.last_updated {
            self.lib = None;

            let (lib, last_updated) = match Self::load_lib(&self.fname, false) {
                Ok(result) => result,
                Err(e) => {
                    return WatchResult::Err(e);
                }
            };

            self.last_updated = last_updated;
            self.lib = Some(lib);

            WatchResult::Updated
        } else {
            WatchResult::NoChange
        }
    }

    /// Creates the 'cloned' dll name
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

    pub fn lib(&self) -> Option<&libloading::Library> {
        match &self.lib {
            Some(lib) => Some(lib),
            None => None,
        }
    }
}
