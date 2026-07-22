use buildline::adapters::{cargo::Cargo, Adapter};

mod common;

/// Golden-file contract for the cargo adapter. Fixture is a real
/// `cargo build --timings` HTML report (probed against cargo 1.96.1 — stable
/// cargo has no `--timings=json`, so the adapter extracts the `UNIT_DATA`
/// JS array embedded in the HTML instead). Includes a real build-script unit,
/// which is what exercises the `Category::Other("run-custom-build")` path.
#[test]
fn cargo_simple_matches_golden() {
    let input = include_bytes!("fixtures/cargo/simple.cargo-timing.html");
    let spans = Cargo
        .parse(input)
        .expect("cargo adapter should parse the fixture");

    let got = common::normalize(&serde_json::to_string_pretty(&spans).unwrap());
    let expected = common::normalize(include_str!("fixtures/cargo/simple.expected.json"));

    assert_eq!(
        got.trim(),
        expected.trim(),
        "\n--- got ---\n{got}\n--- expected ---\n{expected}\n"
    );
}
