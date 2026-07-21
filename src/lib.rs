//! buildline — normalize each build tool's native profiling output into one
//! shared span schema, then merge every tool onto a single wall-clock timeline.
//!
//! Module map: `span` is the normalized representation every adapter emits,
//! and the shared `Category` vocabulary that makes "one timeline" actually
//! one. `adapters` holds one parser per build tool (native format -> relative
//! spans). `lane` does swim-lane packing so parallel work is readable.
//! `session` is the wrapper's T0 envelope; the merger reads it to offset.
//! `chrome_trace` turns spans into Chrome Trace Event JSON (opens in
//! Perfetto).
//!
//! Adapters emit spans **relative to the tool's own start** (T0 = 0). Offsetting
//! them onto the shared wall-clock axis is `Session`'s job — so adapter tests
//! stay deterministic and no wall-clock ever leaks into a fixture.

pub mod adapters;
pub mod appender;
pub mod chrome_trace;
pub mod lane;
pub mod session;
pub mod span;
