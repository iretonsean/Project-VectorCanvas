//! Per-frame CSV (PVC_TRACE) + meta.json (PVC_META) + rects.json (PVC_RECTS) + state dump (PVC_STATE),
//! schema fixed by /workspace/dev_work/dev-measure/METHOD.md §3, §11.
use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub const HEADER: &str = "frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms,n_events";

pub fn realtime_ns() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0)
}

pub struct Row {
    pub frame_idx: u64,
    pub t_ns: u64,
    pub event: &'static str,
    pub update_ms: f64,
    pub layout_ms: f64,
    pub scene_ms: f64,
    pub a11y_ms: f64,
    pub render_ms: f64,
    pub total_ms: f64,
    pub completion_ms: f64,
    pub n_events: u32,
}

pub struct Trace {
    file: Option<File>,
    scenario: String,
}

impl Trace {
    pub fn from_env() -> Self {
        let scenario = std::env::var("PVC_SCENARIO").unwrap_or_default();
        let file = std::env::var("PVC_TRACE").ok().filter(|p| !p.is_empty()).map(|p| {
            let mut f = File::create(&p).unwrap_or_else(|e| panic!("PVC_TRACE {p}: {e}"));
            writeln!(f, "{HEADER}").unwrap();
            f
        });
        Self { file, scenario }
    }
    pub fn enabled(&self) -> bool {
        self.file.is_some()
    }
    /// Called after completion_ms was sampled (never inside a timed span).
    pub fn write(&mut self, r: &Row) {
        if let Some(f) = self.file.as_mut() {
            // mirror_ms empty on native (METHOD §3)
            let _ = writeln!(
                f,
                "{},{},{},{},{:.3},{:.3},{:.3},{:.3},,{:.3},{:.3},{:.3},{}",
                r.frame_idx, r.t_ns, self.scenario, r.event, r.update_ms, r.layout_ms, r.scene_ms, r.a11y_ms, r.render_ms, r.total_ms, r.completion_ms, r.n_events
            );
        }
    }
}

pub fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// `VmHWM:` etc from /proc/self/status, in kB.
pub fn proc_status_kb(key: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines().find(|l| l.starts_with(key))?.split_whitespace().nth(1)?.parse().ok()
}

pub fn write_file(env: &str, body: &str) {
    if let Ok(p) = std::env::var(env) {
        if !p.is_empty() {
            let tmp = format!("{p}.tmp");
            if std::fs::write(&tmp, body).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }
}

/// SIGTERM/SIGINT -> flag (libc `signal` declared directly; no extra crate).
pub mod sig {
    use std::sync::atomic::{AtomicBool, Ordering};
    static TERM: AtomicBool = AtomicBool::new(false);
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    extern "C" fn on_term(_: i32) {
        TERM.store(true, Ordering::SeqCst);
    }
    pub fn install() {
        unsafe {
            signal(15, on_term as *const () as usize);
            signal(2, on_term as *const () as usize);
        }
    }
    pub fn requested() -> bool {
        TERM.load(Ordering::SeqCst)
    }
}
