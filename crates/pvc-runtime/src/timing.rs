//! Cheap per-stage frame timings. `std::time::Instant` panics on wasm32-unknown-unknown, so a
//! host there installs a time source with [`set_time_source`] (e.g. `performance.now`).

use std::cell::Cell;

thread_local! {
    static SOURCE: Cell<Option<fn() -> f64>> = const { Cell::new(None) };
}

/// Install a monotonic millisecond clock for this thread. Required on wasm32, optional natively.
pub fn set_time_source(f: fn() -> f64) {
    SOURCE.with(|c| c.set(Some(f)));
}

#[cfg(not(target_arch = "wasm32"))]
fn native_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

#[cfg(target_arch = "wasm32")]
fn native_ms() -> f64 {
    0.0
}

/// Monotonic milliseconds from the installed source, else the native clock (0.0 on wasm32
/// without a source).
pub fn now_ms() -> f64 {
    match SOURCE.with(|c| c.get()) {
        Some(f) => f(),
        None => native_ms(),
    }
}

/// Per-stage timings of the last frame, milliseconds (host clock; definitions in
/// `/workspace/dev_work/dev-measure/METHOD.md`). `render_ms` is filled by whoever calls the
/// renderer (host or `render_*` helpers) through `Runtime::record_render_ms`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTimings {
    pub update_ms: f64,
    pub layout_ms: f64,
    pub scene_ms: f64,
    pub a11y_ms: f64,
    pub render_ms: f64,
    pub total_ms: f64,
}
