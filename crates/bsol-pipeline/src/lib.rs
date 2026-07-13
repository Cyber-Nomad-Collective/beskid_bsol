//! BSOL analysis pipeline — phase identifiers and session hooks.

mod observer;
pub mod phases;

pub use observer::{observe_phase, FnObserver, NullObserver, PhaseResult, PipelineObserver};
