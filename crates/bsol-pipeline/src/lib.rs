//! BSOL analysis pipeline — phase identifiers and session hooks.

mod observer;
pub mod phases;

pub use observer::{
    FnObserver, NullObserver, PhaseResult, PipelineObserver, observe_phase,
};
