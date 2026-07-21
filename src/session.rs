use crate::span::Span;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Session envelope — the twin of the span stream, kept explicit so offsetting
/// never scatters into ad-hoc code.
///
/// The wrapper writes one entry per `buildline -- <tool>` invocation: the track
/// it ran and the wall-clock instant it launched. The merger reads these T0s to
/// lift each adapter's *relative* spans onto the shared absolute axis. Storing
/// T0 here — rather than baking it into spans at parse time — is what lets the
/// post-hoc `merge` mode re-place artifacts it only collected after the fact.
///
/// The wrapper persists this as a JSON sidecar (`<trace>.session`) so that
/// separate process invocations (`buildline -- cargo build`, then later
/// `buildline -- ninja`) share one growing session instead of each starting
/// from a fresh clock.
///
/// v1 assumes one invocation per track (cargo once, ninja once), so both maps
/// are keyed by track name. Repeated invocations of the same track overwrite
/// its T0 with the latest run; a per-invocation id generalizes this later.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Absolute anchor (µs since Unix epoch) for the session's zero.
    pub epoch_unix_us: i64,
    /// track name -> wall-clock T0 (µs since `epoch_unix_us`).
    pub track_t0_us: BTreeMap<String, i64>,
    /// track name -> stable pid for the Chrome-trace output. Assigned once per
    /// track, on first use, and never reused — this is what lets the trace
    /// file grow across separate wrapper invocations without two different
    /// processes racing to pick pid 0.
    pub track_pid: BTreeMap<String, i64>,
    /// Next pid to hand out. Kept explicit (not `track_pid.len()`) so it never
    /// goes backwards if a track were ever removed in a future version.
    #[serde(default)]
    next_pid: i64,
}

impl Session {
    /// Load the sidecar next to `trace_path`, or start a fresh session anchored
    /// at the current wall-clock instant if none exists yet.
    pub fn load_or_new(trace_path: &Path) -> anyhow::Result<Self> {
        let sidecar = sidecar_path(trace_path);
        if sidecar.exists() {
            let bytes = std::fs::read(&sidecar)
                .with_context(|| format!("reading session sidecar {}", sidecar.display()))?;
            let session: Session = serde_json::from_slice(&bytes)
                .with_context(|| format!("parsing session sidecar {}", sidecar.display()))?;
            Ok(session)
        } else {
            let epoch_unix_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before Unix epoch")
                .as_micros() as i64;
            Ok(Session {
                epoch_unix_us,
                ..Default::default()
            })
        }
    }

    pub fn save(&self, trace_path: &Path) -> anyhow::Result<()> {
        let sidecar = sidecar_path(trace_path);
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(&sidecar, bytes)
            .with_context(|| format!("writing session sidecar {}", sidecar.display()))
    }

    /// Record that `track` launched at the given wall-clock instant (µs since
    /// Unix epoch), storing it relative to the session's epoch.
    pub fn record_start(&mut self, track: &str, wall_clock_unix_us: i64) {
        self.track_t0_us
            .insert(track.to_string(), wall_clock_unix_us - self.epoch_unix_us);
    }

    /// Stable pid for `track`, allocating a new one on first use.
    pub fn pid_for(&mut self, track: &str) -> i64 {
        if let Some(&pid) = self.track_pid.get(track) {
            return pid;
        }
        let pid = self.next_pid;
        self.next_pid += 1;
        self.track_pid.insert(track.to_string(), pid);
        pid
    }

    /// True the first time `track` is seen (i.e. `pid_for` would allocate).
    /// Used to decide whether a `process_name` metadata event still needs to
    /// be written for this track.
    pub fn is_new_track(&self, track: &str) -> bool {
        !self.track_pid.contains_key(track)
    }

    /// Lift one track's relative spans onto the shared axis, in place. Spans
    /// whose track has no recorded T0 are left untouched (0 offset).
    pub fn place(&self, spans: &mut [Span]) {
        for s in spans {
            if let Some(&t0) = self.track_t0_us.get(&s.track) {
                s.start_us += t0;
            }
        }
    }
}

fn sidecar_path(trace_path: &Path) -> std::path::PathBuf {
    let mut s = trace_path.as_os_str().to_os_string();
    s.push(".session");
    std::path::PathBuf::from(s)
}
