use crate::span::{Span, Status};
use serde_json::{json, Value};

/// Serialize normalized spans to the Chrome Trace Event Format that
/// `ui.perfetto.dev` and `chrome://tracing` open natively — so buildline ships
/// no UI of its own. Each span becomes a "Complete" event; each track becomes a
/// named process row-group; `status` drives a colour so failed / hung phases
/// stand out.
///
/// One-shot form: assigns pids by first-appearance order within `spans`. Used
/// for standalone renders (the merge example, golden tests). The wrapper uses
/// `process_name_event` + `span_event` directly instead, because it needs pids
/// that stay stable across separate process invocations — see `Session::pid_for`.
pub fn to_chrome_trace(spans: &[Span]) -> Value {
    let mut tracks: Vec<&str> = Vec::new();
    for s in spans {
        if !tracks.iter().any(|&t| t == s.track) {
            tracks.push(s.track.as_str());
        }
    }
    let pid_of = |track: &str| tracks.iter().position(|&x| x == track).unwrap() as i64;

    let mut events: Vec<Value> = Vec::new();
    for (i, t) in tracks.iter().enumerate() {
        events.push(process_name_event(t, i as i64));
    }
    for s in spans {
        events.push(span_event(s, pid_of(&s.track)));
    }

    json!({ "traceEvents": events, "displayTimeUnit": "ms" })
}

/// Metadata event that labels a pid with its track name in Perfetto's UI.
pub fn process_name_event(track: &str, pid: i64) -> Value {
    json!({ "ph": "M", "name": "process_name", "pid": pid, "tid": 0, "args": { "name": track } })
}

/// One span as a Chrome-trace "Complete" (`ph: "X"`) event, on the given pid.
pub fn span_event(s: &Span, pid: i64) -> Value {
    let mut ev = json!({
        "ph": "X",
        "name": s.name,
        "cat": s.category.as_str(),
        "ts": s.start_us,
        "dur": s.dur_us,
        "pid": pid,
        "tid": s.lane,
        "args": { "status": s.status.as_str() },
    });
    if let Some(colour) = colour_of(&s.status) {
        ev["cname"] = json!(colour);
    }
    if !s.args.is_empty() {
        let a = ev["args"].as_object_mut().unwrap();
        for (k, v) in &s.args {
            a.insert(k.clone(), json!(v));
        }
    }
    ev
}

/// Reserved Chrome-trace colour names. `Success` keeps the default hashed-by-name
/// colour so successful work still reads as distinct blocks.
fn colour_of(status: &Status) -> Option<&'static str> {
    match status {
        Status::Success => None,
        Status::Failed => Some("terrible"), // red
        Status::Incomplete => Some("bad"),  // orange — the hung / killed span
        Status::Skipped => Some("grey"),
    }
}
