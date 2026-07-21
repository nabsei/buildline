use super::Adapter;
use crate::lane::pack_lanes;
use crate::span::{Category, Span, Status};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Reads cargo's `--timings` report.
///
/// Verified empirically against cargo 1.96.1: on stable, `--timings` accepts no
/// `=json` variant (`error: unexpected value 'json'`) — it only writes
/// `target/cargo-timings/cargo-timing.html`. That HTML is self-contained and
/// embeds the real per-unit data as a JS array literal, `const UNIT_DATA = [...]`,
/// which is syntactically valid JSON. This adapter extracts and parses that
/// array directly — no unstable flags, works on stable cargo.
pub struct Cargo;

#[derive(Debug, Deserialize)]
struct CargoUnit {
    name: String,
    version: String,
    mode: String,
    target: String,
    start: f64,
    duration: f64,
}

impl Adapter for Cargo {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn parse(&self, input: &[u8]) -> anyhow::Result<Vec<Span>> {
        let html = std::str::from_utf8(input)?;
        let marker = "const UNIT_DATA = ";
        let after_marker = html
            .find(marker)
            .map(|i| &html[i + marker.len()..])
            .ok_or_else(|| {
                anyhow!("no UNIT_DATA found — is this a cargo --timings HTML report?")
            })?;
        let json_slice = extract_balanced_array(after_marker)
            .ok_or_else(|| anyhow!("UNIT_DATA array in cargo timing report is not well-formed"))?;

        let units: Vec<CargoUnit> =
            serde_json::from_str(json_slice).context("parsing cargo timing UNIT_DATA as JSON")?;

        let mut spans: Vec<Span> = units
            .into_iter()
            .map(|u| {
                let suffix = u.target.trim();
                let name = if suffix.is_empty() {
                    format!("{} v{}", u.name, u.version)
                } else {
                    format!("{} v{} {}", u.name, u.version, suffix)
                };
                let mut args = BTreeMap::new();
                args.insert("version".to_string(), u.version);

                Span {
                    name,
                    category: classify(&u.mode),
                    status: Status::Success, // report is written after a successful build
                    track: "cargo".to_string(),
                    lane: 0, // set by pack_lanes below
                    start_us: (u.start * 1_000_000.0).round() as i64,
                    dur_us: (u.duration * 1_000_000.0).round() as i64,
                    args,
                }
            })
            .collect();

        pack_lanes(&mut spans);
        Ok(spans)
    }
}

/// `mode` is cargo's own internal unit-kind label. `"todo"` covers actual
/// compilation (of a crate *or* of its build script — both are real rustc
/// invocations). `"run-custom-build"` is *executing* an already-compiled build
/// script — running arbitrary code, not compiling — so it does NOT collapse
/// into `Compile`. This is the enum's escape hatch doing its job: a build
/// script run is real cargo-specific behavior with no shared-vocabulary
/// equivalent, so it stays visibly `Other` rather than being mislabeled.
fn classify(mode: &str) -> Category {
    match mode {
        "todo" => Category::Compile,
        other => Category::Other(other.to_string()),
    }
}

/// Scans forward from the first `[` to its matching `]`, respecting JSON string
/// literals (so a `[` or `]` inside a quoted value doesn't unbalance the count).
/// Returns the matched slice including both brackets.
fn extract_balanced_array(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, ch) in s.char_indices() {
        if i < start {
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}
