use crate::span::Span;

pub mod cargo;
pub mod ninja;

/// Parses one build tool's native profiling artifact into relative spans.
///
/// Each adapter is a pure function of its input bytes — no clocks, no I/O — so
/// its behavior is pinned by a golden-file test (fixture in, expected JSON out).
/// A new build system = a new file here + a fixture pair. Nothing else moves.
pub trait Adapter {
    /// Short id, e.g. `ninja`. Also the default track name.
    fn name(&self) -> &'static str;

    /// Parse native bytes into spans whose `start_us` is relative to the tool's T0.
    fn parse(&self, input: &[u8]) -> anyhow::Result<Vec<Span>>;
}
