use state::State;

heimdall::init_watchable!(state: State, watchable: Plugin);

pub struct Plugin;

impl heimdall::Watchable<State> for Plugin {
    fn init() -> State {
        println!("A init has occurred.");

        State { counter: 0 }
    }

    fn update(mut state: State) -> State {
        state.counter += 11;
        println!("An update has occurred. State: {:?}", state);
        state
    }
}
