Layout:
* `src` contains the 'host' functionality, which loads the library and executes it.
* `plugin` is the reloadable library and implementation.
* `state` contains the static, shared state for the `src` and `plugin` areas.

Execution:
* `cargo run` will execute the static libraries.
* `cargo run --features hot-reload` will run the dynamically linked libraries. Compiling `plugin` in a separate terminal while `test_host` is running will reload and apply the plugin.

NOTE: 
* This was only tested on Windows.