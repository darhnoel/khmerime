use dioxus::prelude::Signal;

use crate::engine_registry::EngineReadiness;

#[derive(Clone, Copy)]
pub(crate) struct StartupSignals {
    pub(crate) engine_readiness: Signal<EngineReadiness>,
    pub(crate) engine_ready: Signal<bool>,
    pub(crate) engine_progress: Signal<u8>,
}
