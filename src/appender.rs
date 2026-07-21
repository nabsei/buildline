use anyhow::Context;
use serde_json::Value;
use std::io::Write;
use std::path::Path;

/// Appends events to a growing, deliberately never-closed Chrome-trace JSON
/// array: `[\n  {...},\n  {...},\n` with no final `]`. This is intentional and
/// spec-legal — the Chrome Trace Event format explicitly tolerates an
/// unterminated array with a trailing comma, exactly so a trace can be
/// streamed to incrementally without a "finalize" step. Perfetto and
/// `chrome://tracing` both parse it leniently. That's what makes
/// `BUILDLINE_SESSION=./build.trace buildline -- <tool>`, run once per tool,
/// produce a file that's openable in Perfetto after every single invocation —
/// no extra command required.
pub fn append_events(trace_path: &Path, events: &[Value]) -> anyhow::Result<()> {
    let is_new = !trace_path.exists();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(trace_path)
        .with_context(|| format!("opening trace file {}", trace_path.display()))?;

    if is_new {
        writeln!(f, "[")?;
    }
    for ev in events {
        writeln!(f, "  {},", serde_json::to_string(ev)?)?;
    }
    Ok(())
}
