//! Mechanism demo of the money-shot: two build tools on ONE timeline, with a
//! Download phase no per-tool profiler surfaces.
//!
//! The ninja spans are real (parsed from the checked-in fixture). The cargo
//! batch is SYNTHETIC — a stand-in until the cargo adapter lands. It exists only
//! to prove the merge + offset + colour path visually in Perfetto. Run:
//!
//!   cargo run --example merge_demo > demo.trace.json
//!
//! then drag demo.trace.json into https://ui.perfetto.dev

use buildline::adapters::{ninja::Ninja, Adapter};
use buildline::chrome_trace::to_chrome_trace;
use buildline::session::Session;
use buildline::span::{Category, Span, Status};
use std::collections::BTreeMap;

fn main() {
    // Real ninja spans (relative to ninja's own T0).
    let mut ninja = Ninja
        .parse(include_bytes!("../tests/fixtures/ninja/simple.ninja_log"))
        .expect("parse ninja fixture");

    // Synthetic cargo batch (relative to cargo's own T0). The two Download spans
    // at the front are the time cargo spends before any compile — the part that
    // `cargo --timings` folds away and nobody sees on the CI wall-clock.
    let mut cargo = vec![
        cargo_span("fetch crates.io index", Category::Download, 0, 900_000),
        cargo_span("download serde v1.0", Category::Download, 900_000, 400_000),
        cargo_span(
            "compile serde v1.0",
            Category::Compile,
            1_300_000,
            1_100_000,
        ),
        cargo_span("compile app", Category::Compile, 2_400_000, 600_000),
    ];

    // Session envelope: ninja launched at wall-clock 0; cargo launched 3.0s
    // later. `place` lifts each batch onto the shared axis.
    let mut sess = Session::default();
    sess.track_t0_us.insert("ninja".into(), 0);
    sess.track_t0_us.insert("cargo".into(), 3_000_000);
    sess.place(&mut ninja);
    sess.place(&mut cargo);

    let mut all = ninja;
    all.extend(cargo);

    let trace = to_chrome_trace(&all);
    println!("{}", serde_json::to_string_pretty(&trace).unwrap());
}

fn cargo_span(name: &str, category: Category, start_us: i64, dur_us: i64) -> Span {
    Span {
        name: name.into(),
        category,
        status: Status::Success,
        track: "cargo".into(),
        lane: 0,
        start_us,
        dur_us,
        args: BTreeMap::new(),
    }
}
