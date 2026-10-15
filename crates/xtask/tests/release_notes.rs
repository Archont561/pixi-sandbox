//! TASK-74 AC#10: the main release notes and the starter README cross-link, so a user can tell
//! which pixi-sandbox release generated the scaffold they cloned.
//!
//! The starter's half is rendered by the released binary and held by
//! `the_readme_names_the_release_and_the_commit` in `tests/starter.rs`. This is the other
//! direction, which lives in `release.yml` — a file no unit test would otherwise read, and the
//! half that was still missing when v0.7.0 shipped: its notes named the pull requests and never
//! the starter.
//!
//! The notes are created by the GitHub API, and `generate_release_notes: true` keeps the
//! changelog. A `body` is *prepended* to those generated notes, not substituted for them, so the
//! cross-link can be added without losing the release's own history.

/// The step that creates the GitHub Release, read as text: its `with:` block is the only place
/// the notes' content is decided.
const CREATE_RELEASE_STEP: &str = "- name: Create GitHub Release";

const RELEASE_WORKFLOW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.github/workflows/release.yml"
));

const STARTER_REPO: &str = "https://github.com/Archont561/pixi-sandbox-starter";

/// The `Create GitHub Release` step, from its name to the next step in the job.
fn create_release_step() -> &'static str {
    let start = RELEASE_WORKFLOW
        .find(CREATE_RELEASE_STEP)
        .expect("the release-creating step");
    let rest = &RELEASE_WORKFLOW[start..];
    let end = rest
        .find("\n      - ")
        .expect("the step after the release-creating one");
    &rest[..end]
}

#[test]
fn the_release_notes_cross_link_the_starter_without_replacing_the_changelog() {
    let step = create_release_step();
    assert!(
        step.contains("generate_release_notes: true"),
        "the generated changelog must survive the cross-link: {step}"
    );
    assert!(
        step.contains(STARTER_REPO),
        "the notes must link the starter repository this release publishes: {step}"
    );
    assert!(
        step.contains("body:"),
        "the cross-link reaches the notes through a body the API prepends: {step}"
    );
}

/// The link has to survive the next release. A branch link (`/tree/main`) or a bare repository
/// link would answer with whatever is newest, which is the one thing a consumer must not have to
/// reason about, so the notes name the release's own immutable evidence tag instead.
#[test]
fn the_notes_name_the_immutable_evidence_tag_rather_than_a_floating_branch() {
    let step = create_release_step();
    for floating in ["/tree/main", "/blob/main"] {
        assert!(
            !step.contains(floating),
            "the notes point at {floating}, which will not describe this release later: {step}"
        );
    }
    assert!(
        step.contains("pixi-sandbox-${{ inputs.version || github.ref_name }}"),
        "the notes must name the evidence tag the starter lane seals for this release: {step}"
    );
}
