use state::State;

use heimdall::{Watchable, Watcher};
use plugin::Plugin;

fn main() {
    let (mut watcher, mut state): (Watcher<State, Plugin>, State) =
        Watcher::new("./plugin/target/debug/plugin.dll".into());

    loop {
        watcher.watch(&mut state);
        watcher.update(&mut state);
    }
}
