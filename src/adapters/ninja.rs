use super::Adapter;
use crate::lane::pack_lanes;
use crate::span::{Category, Span, Status};

/// Reads ninja's `.ninja_log` (format v5/v6). Every ninja build writes this file
/// automatically, so no `ninjatracing` wrapper is required.
///
/// Line format, tab-separated: `start_ms  end_ms  restat_mtime  output  cmd_hash`.
/// `start_ms` / `end_ms` are milliseconds since ninja started.
pub struct Ninja;

impl Adapter for Ninja {
    fn name(&self) -> &'static str {
        "ninja"
    }

    fn parse(&self, input: &[u8]) -> anyhow::Result<Vec<Span>> {
        let text = std::str::from_utf8(input)?;
        let mut spans = Vec::new();

        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue; // header / comment
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 5 {
                continue; // malformed line — skip it, don't fail the whole build
            }
            let start_ms: i64 = f[0].parse()?;
            let end_ms: i64 = f[1].parse()?;
            let output = f[3].to_string();

            spans.push(Span {
                category: classify(&output),
                status: Status::Success, // .ninja_log only records completed steps
                name: output,
                track: "ninja".to_string(),
                lane: 0, // set by pack_lanes below
                start_us: start_ms * 1000,
                dur_us: (end_ms - start_ms) * 1000,
                args: Default::default(),
            });
        }

        pack_lanes(&mut spans);
        Ok(spans)
    }
}

/// Heuristic classifier. Object files are compiles; anything else ninja records
/// as a produced output is treated as a link for now. Refined per-generator later.
fn classify(output: &str) -> Category {
    let base = output.rsplit('/').next().unwrap_or(output);
    match base.rsplit('.').next() {
        Some("o") | Some("obj") => Category::Compile,
        _ => Category::Link,
    }
}
