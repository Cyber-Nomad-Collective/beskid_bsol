//! Pipeline observer hooks (mirrors Beskid pipeline UX patterns).

use std::fmt;

/// Receives phase start/end notifications during analysis.
pub trait PipelineObserver: Send {
    fn on_phase_start(&mut self, phase: &str);
    fn on_phase_end(&mut self, phase: &str, result: PhaseResult);
}

/// Outcome of a pipeline phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseResult {
    Ok,
    Err,
}

/// No-op observer for tests and library callers.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullObserver;

impl PipelineObserver for NullObserver {
    fn on_phase_start(&mut self, _phase: &str) {}
    fn on_phase_end(&mut self, _phase: &str, _result: PhaseResult) {}
}

/// Forward phase lifecycle to a closure.
pub struct FnObserver<F>
where
    F: FnMut(&str, bool) + Send,
{
    callback: F,
}

impl<F> FnObserver<F>
where
    F: FnMut(&str, bool) + Send,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> PipelineObserver for FnObserver<F>
where
    F: FnMut(&str, bool) + Send,
{
    fn on_phase_start(&mut self, phase: &str) {
        (self.callback)(phase, true);
    }

    fn on_phase_end(&mut self, phase: &str, result: PhaseResult) {
        let _ = (phase, result);
    }
}

impl fmt::Debug for dyn PipelineObserver + Send {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PipelineObserver")
    }
}

pub fn observe_phase<T, O: PipelineObserver + ?Sized>(
    observer: &mut O,
    phase: &str,
    run: impl FnOnce() -> Result<T, bsol_schema::BsolError>,
) -> Result<T, bsol_schema::BsolError> {
    observer.on_phase_start(phase);
    let result = run();
    observer.on_phase_end(
        phase,
        if result.is_ok() {
            PhaseResult::Ok
        } else {
            PhaseResult::Err
        },
    );
    result
}
