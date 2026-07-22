/// Normalizes CRLF to LF before a golden comparison.
///
/// Without this, the same fixture compares differently depending on how git
/// checked it out: Windows runners default to converting committed LF text
/// files to CRLF on checkout, while `serde_json` always emits LF. The
/// `.gitattributes` `eol=lf` rule fixes this at the source for fresh clones,
/// but normalizing here too means a golden test can never fail on line
/// endings alone, regardless of how a contributor's local git is configured.
pub fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}
