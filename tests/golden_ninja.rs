use buildline::adapters::{ninja::Ninja, Adapter};

mod common;

/// Golden-file contract for the ninja adapter: a real `.ninja_log` in, the exact
/// normalized spans out. This is the whole review surface for a new adapter —
/// contributors add a fixture pair, CI diffs it, no judgment call required.
#[test]
fn ninja_simple_matches_golden() {
    let input = include_bytes!("fixtures/ninja/simple.ninja_log");
    let spans = Ninja
        .parse(input)
        .expect("ninja adapter should parse the fixture");

    let got = common::normalize(&serde_json::to_string_pretty(&spans).unwrap());
    let expected = common::normalize(include_str!("fixtures/ninja/simple.expected.json"));

    assert_eq!(
        got.trim(),
        expected.trim(),
        "\n--- got ---\n{got}\n--- expected ---\n{expected}\n"
    );
}
