# Category vocabulary

This is the shared vocabulary every adapter maps its tool's native output onto.
It's the actual contract that makes a merged trace *one* timeline instead of
several timelines that happen to render next to each other: a golden test
only checks an adapter against itself, so nothing stops one adapter emitting
`"compile"` and another `"Compile"` — each passing its own test while the
"unified" view is silently incoherent underneath. Every span's `category`
must be one of the terms below, or `Other`.

This document describes what each term *means*, independent of any specific
tool, so that:
- an adapter author has a clear bar for "does my tool's step X map to
  `Compile` or does it need `Other`", and
- a reader looking at a merged trace can trust that `Compile` means the same
  thing whether the span came from `cargo` or `ninja`.

## Reserved terms

### `Compile`
Turning source into an intermediate or final artifact for a single
compilation unit: a `.o` file from a `.c`/`.cpp`, a `.rlib` from a Rust crate,
a `.class` from Java source. Scoped to one unit — an operation over the
whole crate/target graph (dependency resolution, linking) is not `Compile`
even if the tool's own terminology calls it "building".

### `Link`
Combining already-compiled units into a final binary or library. Distinct
from `Compile` because it has different scaling behavior (link time often
grows with the number of units, not their individual size) and is frequently
the actual bottleneck a `Compile`-only view would hide.

### `Configure`
Build-graph setup that happens before any compilation can start: CMake's
configure step, Cargo's own manifest resolution/planning phase, Gradle's
configuration phase. If it produces the plan for what to build rather than
building anything itself, it's `Configure`.

### `Resolve`
Determining which versions of dependencies to use — the solver step, as
distinct from actually fetching them (`Download`) or configuring the build
graph around them (`Configure`). Tools that fetch and resolve as one
inseparable step should pick whichever dominates in practice, or use
`Other` if genuinely ambiguous, rather than guessing.

### `Download`
Fetching bytes over the network: crate/package downloads, registry index
updates, toolchain component fetches. This is deliberately the category most
likely to explain a wall-clock gap that no tool's own profiler records (see
the README) — a slow or flaky network shows up here, not as "the compiler is
slow".

### `Test`
Running tests as part of the build (e.g. `cargo test` compiling and then
executing test binaries). Not for a separate, independently-invoked test
suite — only for test execution that's genuinely part of the profiled build
invocation.

### `Other(String)`
The escape hatch for a step that is real, tool-specific, and doesn't honestly
fit any category above. **`Other` is not a failure to classify — using it
correctly is part of the contract.** Folding something ambiguous into the
nearest reserved category to avoid `Other` is the actual violation, because it
quietly corrupts what that category means across every adapter.

Concrete example already in the codebase: cargo's `run-custom-build` step
*runs* an already-compiled build script (`build.rs`) — it's not compiling
anything, so it stays `Other("run-custom-build")` rather than being folded
into `Compile`.

`Other` values are plain strings and are never diffed against another
adapter's `Other` values — there is no expectation that two tools' `Other`
categories line up. If a tool-specific concept turns out to be common enough
across multiple adapters to deserve its own reserved term, that's a signal to
propose adding one here, not to reuse an existing term loosely.

## Adding a new adapter

If your tool's native output has a concept that doesn't cleanly map to one of
the terms above, prefer `Other("your-tools-own-name")` over stretching an
existing category. A slightly-wrong `Other` is honest and visible; a
comfortable-but-wrong reserved category corrupts the one thing this project
actually promises.
