//! The `buildline` wrapper binary.
//!
//! Usage, one invocation per build tool, sharing a session via the trace path:
//!
//!   BUILDLINE_SESSION=./build.trace buildline -- cargo build
//!   BUILDLINE_SESSION=./build.trace buildline -- ninja
//!
//! Each invocation: stamps its own real wall-clock start (this is what
//! recovers the dead time *between* tools — toolchain downloads, dependency
//! resolution — that no single tool's own profiler sees, because it happens
//! outside that tool); runs the tool transparently (inherited stdio, exact
//! exit code passed through — a CI step wrapped in `buildline --` behaves
//! exactly like the unwrapped command); then locates that tool's native
//! profiling artifact, normalizes it, offsets it onto the shared session
//! clock, and appends it to the trace file. No finalize step — the trace file
//! is openable in Perfetto after every single invocation.
//!
//! v1 wraps one tool invocation per process: no child-process interception,
//! so multi-tool orchestrators (`make all` spawning both cargo and ninja)
//! aren't captured — wrap each real tool invocation directly instead.

use anyhow::{anyhow, Context};
use buildline::adapters::{cargo::Cargo, ninja::Ninja, Adapter};
use buildline::appender::append_events;
use buildline::chrome_trace::{process_name_event, span_event};
use buildline::session::Session;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(e) = run() {
        eprintln!("buildline: {e:#}");
        std::process::exit(2);
    }
}

fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dash_dash = args.iter().position(|a| a == "--").ok_or_else(|| {
        anyhow!("usage: buildline -- <tool> [args...] (BUILDLINE_SESSION must be set)")
    })?;
    let mut tool_argv: Vec<String> = args[dash_dash + 1..].to_vec();
    if tool_argv.is_empty() {
        return Err(anyhow!("no command given after --"));
    }

    let session_env = std::env::var("BUILDLINE_SESSION")
        .context("BUILDLINE_SESSION must be set to a trace file path, e.g. ./build.trace")?;
    let trace_path = PathBuf::from(session_env);

    let track = Path::new(&tool_argv[0])
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("could not determine tool name from '{}'", tool_argv[0]))?
        .to_string();

    if let Some(extra) = maybe_inject_timings(&track, &tool_argv[1..]) {
        tool_argv.extend(extra);
    }

    // Load (or create) the session BEFORE spawning, not after. For a session
    // that already exists (a later invocation in the same trace) this is just
    // an ordering choice — the epoch is already fixed on disk. But for the
    // *first* invocation, which creates the session, `Session::load_or_new`
    // anchors `epoch_unix_us` to "now" — loading it after the tool has
    // already run would anchor the epoch at build-finish instead of
    // build-start, making every relative timestamp in that first batch
    // negative (offset = start_time - epoch, with epoch > start_time).
    let mut session = Session::load_or_new(&trace_path)?;

    let wall_clock_start_unix_us = now_unix_us();
    let status = Command::new(&tool_argv[0])
        .args(&tool_argv[1..])
        .status()
        .with_context(|| format!("launching '{}'", tool_argv[0]))?;

    match capture(&track, &trace_path, &mut session, wall_clock_start_unix_us) {
        Ok(n) => eprintln!(
            "buildline: appended {n} span(s) for '{track}' to {}",
            trace_path.display()
        ),
        Err(e) => eprintln!("buildline: warning: could not capture timing for '{track}': {e:#}"),
    }

    std::process::exit(status.code().unwrap_or(1));
}

/// If `tool` is cargo running a subcommand that supports `--timings`, and the
/// caller hasn't already asked for it, return the extra flag to append.
fn maybe_inject_timings(tool: &str, rest_args: &[String]) -> Option<Vec<String>> {
    if tool != "cargo" {
        return None;
    }
    let sub = rest_args.first()?;
    let supports_timings = ["build", "check", "test", "bench"].contains(&sub.as_str());
    let already_requested = rest_args
        .iter()
        .any(|a| a == "--timings" || a.starts_with("--timings="));
    if supports_timings && !already_requested {
        Some(vec!["--timings".to_string()])
    } else {
        None
    }
}

/// Locate this track's native artifact, parse it, offset it onto the session
/// clock, and append it to the trace file. Returns the number of spans written.
/// `session` is the caller's already-loaded session (see the note in `run`
/// about why it must be loaded before the child process is spawned).
fn capture(
    track: &str,
    trace_path: &Path,
    session: &mut Session,
    wall_clock_start_unix_us: i64,
) -> anyhow::Result<usize> {
    let (adapter, artifact_path): (Box<dyn Adapter>, PathBuf) = match track {
        "ninja" => (Box::new(Ninja), PathBuf::from(".ninja_log")),
        "cargo" => (
            Box::new(Cargo),
            PathBuf::from("target/cargo-timings/cargo-timing.html"),
        ),
        other => return Err(anyhow!("no adapter for tool '{other}' yet")),
    };

    let bytes = std::fs::read(&artifact_path).with_context(|| {
        format!(
            "reading {} (did the build actually run {track}?)",
            artifact_path.display()
        )
    })?;
    let mut spans = adapter.parse(&bytes)?;

    session.record_start(track, wall_clock_start_unix_us);
    let is_new_track = session.is_new_track(track);
    let pid = session.pid_for(track);
    session.place(&mut spans);

    let mut events = Vec::with_capacity(spans.len() + 1);
    if is_new_track {
        events.push(process_name_event(track, pid));
    }
    events.extend(spans.iter().map(|s| span_event(s, pid)));

    append_events(trace_path, &events)?;
    session.save(trace_path)?;

    Ok(spans.len())
}

fn now_unix_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_micros() as i64
}
