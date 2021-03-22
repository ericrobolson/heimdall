use state::State;

heimdall::init_watchable!(state: State, watchable: Plugin);

pub struct Plugin;

impl heimdall::Watchable<State> for Plugin {
    fn init() -> State {
        println!("A init has occurred.");

        State { counter: 0 }
    }

    fn reload(state: &mut State) {
        state.counter += 1;
        println!("A reload has occurred. State: {:?}", state);
    }

    fn unload(state: &mut State) {
        state.counter -= 1;
        println!("An unload has occurred. State: {:?}", state);
    }

    fn update(state: &mut State) {
        state.counter += 11;
        println!("An update has occurred. State: {:?}", state);
    }

    fn finalize(state: &mut State) {
        state.counter = 0;
        println!("A finalize has occurred. State: {:?}", state);
    }
}
