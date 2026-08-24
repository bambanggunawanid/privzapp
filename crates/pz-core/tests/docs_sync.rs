//! Continuous-documentation enforcement: user-facing docs must track the
//! tool registry. These tests embed the docs at compile time (no I/O at
//! runtime, so the crate stays pure) and fail the build when they drift.

use pz_core::TOOLS;

const README: &str = include_str!("../../../README.md");
const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

#[test]
fn readme_lists_every_tool() {
    let missing: Vec<&str> = TOOLS
        .iter()
        .map(|t| t.name)
        .filter(|name| !README.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "README.md tools table is out of date — add: {missing:?}"
    );
}

#[test]
fn changelog_has_unreleased_or_current_section() {
    assert!(
        CHANGELOG.contains("## [Unreleased]") || CHANGELOG.contains(env!("CARGO_PKG_VERSION")),
        "CHANGELOG.md needs an [Unreleased] section or an entry for the current version"
    );
}

#[test]
fn readme_keeps_the_privacy_promise() {
    // The privacy promise is load-bearing product copy; make sure nobody
    // "fixes" the README into dropping it.
    assert!(README.contains("Zero uploads"));
    assert!(README.contains("no processing server"));
}
