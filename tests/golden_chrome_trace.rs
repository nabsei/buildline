use buildline::adapters::{ninja::Ninja, Adapter};
use buildline::chrome_trace::to_chrome_trace;

/// Golden-file lock on the serializer itself, not just the adapters. Spans
/// were already pinned by `golden_ninja.rs`; this fixture takes those same
/// spans one step further — through `to_chrome_trace` — so a change to event
/// shape, pid assignment, or colour mapping shows up as an intentional diff
/// instead of silently drifting.
#[test]
fn chrome_trace_of_ninja_fixture_matches_golden() {
    let input = include_bytes!("fixtures/ninja/simple.ninja_log");
    let spans = Ninja
        .parse(input)
        .expect("ninja adapter should parse the fixture");

    let trace = to_chrome_trace(&spans);
    let got = serde_json::to_string_pretty(&trace).unwrap();
    let expected = include_str!("fixtures/chrome_trace/ninja_simple.expected.json");

    assert_eq!(
        got.trim(),
        expected.trim(),
        "\n--- got ---\n{got}\n--- expected ---\n{expected}\n"
    );
}
