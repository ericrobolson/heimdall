A crate for 'hot loading' Rust libraries. This is to enable on the fly editing of code.

Inspired by:
* https://nullprogram.com/blog/2014/12/23/
* https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html

Usage:
* By default uses static linking, but with the feature `hot-reload` dynamic loading will be performed.
* Typically involves a 'host' executable that watches some 'plugins' (dynamic + static libraries) and a shared 'state' crate.
* The 'host' is the main program that runs `Watcher`s. These are containers that handle loading of `Watchable`s, crates that are both compiled statically + dynamically.
* In some form of main loop, the `watcher.watch()` method is called to check if the dynamic library has been initialized. If it has been updated, it is swapped out.
* While the host is running with `hot-reload` enabled, recompiling the libraries containing `Watchable`s will reload them at run time. 

Implementation:
* It is suggested to have three crates, a 'state' crate, a 'host' crate and a 'plugin' crate. 
* 'host'
* * The main executable
* 'state'
* * Contains the shared state between the host and plugin. This is a static library.
* 'plugin'
* * Library that has `crate-type = ["cdylib", "rlib"]` set. This enables the static and dynamic linking.
* * At the top of the `lib.rs` file, `heimdall::init_watchable!(state: State, watchable: Plugin);` is called. This creates the linking for the dynamic library.
* * Next is the implementation of `heimdall::Watchable<State>`. This enables the `init_watchable!` macro to hook into your functionality.

Example:
* See `examples/test_host` for an example of how this is used. 

Warnings:
* Only tested on Windows. This may work on other platforms, but it is not guaranteed.
* Not intended for production usage.

