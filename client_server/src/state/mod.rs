//! State machine module

mod machine;
mod transitions;
mod shared;

pub use machine::StateMachine;
pub use shared::{AlarmState, SharedState, ActuatorState, ConnectivityState, CloudStatus, AppState, new_app_state};
pub use transitions::StateTransition;
