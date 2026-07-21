use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// The shared semantic vocabulary — the whole point of the project.
///
/// A golden test only diffs an adapter against *itself*, so a free-form string
/// would let ninja emit `"compile"`, cargo `"Compile"`, Bazel `"compilation"`,
/// each passing its own test while the "unified" timeline is silently
/// incoherent. The enum is the contract that makes one timeline actually one:
/// every adapter must map onto the same terms, and `Other` is the explicit
/// escape hatch for the genuinely tool-specific (still visible, never faked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    Compile,
    Link,
    Configure,
    Resolve,
    Download,
    Test,
    Other(String),
}

impl Category {
    pub fn as_str(&self) -> &str {
        match self {
            Category::Compile => "compile",
            Category::Link => "link",
            Category::Configure => "configure",
            Category::Resolve => "resolve",
            Category::Download => "download",
            Category::Test => "test",
            Category::Other(s) => s,
        }
    }
}

impl From<&str> for Category {
    fn from(s: &str) -> Self {
        match s {
            "compile" => Category::Compile,
            "link" => Category::Link,
            "configure" => Category::Configure,
            "resolve" => Category::Resolve,
            "download" => Category::Download,
            "test" => Category::Test,
            other => Category::Other(other.to_string()),
        }
    }
}

// Serialize as a flat string so `category` maps 1:1 onto the Chrome-trace `cat`
// field and stays human-readable in goldens. The reserved terms normalize
// (`Other("compile")` round-trips to `Compile`), which enforces the vocabulary.
impl Serialize for Category {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Category {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Category::from(String::deserialize(d)?.as_str()))
    }
}

/// Outcome of a step — carries half the product's thesis. A profiler that only
/// shows green durations misses "at which phase did it break or hang". Reserved
/// now (ninja emits only `Success` for the moment) so adding failure/hang colour
/// later never churns existing goldens.
///
/// `Incomplete` is the started-never-finished span of a killed or hung build —
/// literally the multi-hour toolchain hang. Perfetto colours on it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Success,
    Failed,
    Skipped,
    Incomplete,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Success => "success",
            Status::Failed => "failed",
            Status::Skipped => "skipped",
            Status::Incomplete => "incomplete",
        }
    }
}

/// One unit of work on the build timeline.
///
/// `start_us` is relative to the span's reference zero: for adapter output that
/// zero is the tool's own start; after `Session::place` offsets a batch it
/// becomes the session's wall-clock axis. The field never changes shape — only
/// what it is measured from.
///
/// Maps cleanly to a Chrome-trace "Complete" event (`ph: "X"`): `track` -> pid,
/// `lane` -> tid, `start_us` -> ts, `dur_us` -> dur, `category` -> cat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// The unit only — e.g. `obj/parser.o`, `serde v1.0`. No tool prefix:
    /// `track` already carries the tool and Perfetto groups rows by it.
    pub name: String,
    /// Shared kind (see `Category`).
    pub category: Category,
    /// Outcome (see `Status`).
    pub status: Status,
    /// Logical tool lane — e.g. `ninja`, `cargo`. Groups rows in Perfetto (pid).
    pub track: String,
    /// Sub-lane within a track, for parallel work (tid). Set by `lane::pack_lanes`.
    pub lane: u32,
    /// Microseconds from the reference zero (see struct docs).
    pub start_us: i64,
    /// Duration in microseconds.
    pub dur_us: i64,
    /// Optional key/value detail for drill-down (Perfetto's args panel).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, String>,
}
