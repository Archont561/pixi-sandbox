//! Human guides are pure renderings of the manifest, not a second transport specification.

mod support;

use pixi_sandbox::branch_docs::render;
use pixi_sandbox_core::manifest::Manifest;
use rstest::rstest;
use std::path::PathBuf;
use support::{isolated_home, pack_reference};

#[rstest]
fn a_guide_without_vendor_or_self_bin_keeps_the_pre_extraction_bytes(pack_reference: PathBuf) {
    let reference = pack_reference.join("linux-no-vendor");
    let manifest = Manifest::load(&reference.join(".pixi-sandbox/manifest.json")).unwrap();
    let docs = render(&manifest, None);
    assert_eq!(
        docs.readme.as_bytes(),
        std::fs::read(reference.join("README.md")).unwrap()
    );
    assert_eq!(
        docs.agents.as_bytes(),
        std::fs::read(reference.join("AGENTS.md")).unwrap()
    );
}

#[rstest]
#[case::linux_with_host_requirements("linux-loose")]
#[case::windows_with_powershell("windows-loose")]
fn self_bootstrap_and_vendor_guides_keep_the_pre_extraction_bytes(
    pack_reference: PathBuf,
    #[case] case: &str,
) {
    let reference = pack_reference.join(case);
    let manifest = Manifest::load(&reference.join(".pixi-sandbox/manifest.json")).unwrap();
    let info = pixi_sandbox::vendor::VendorInfo {
        cargo: "cargo 1.90.0 (fixture)".into(),
        rustc: "rustc 1.90.0 (fixture)".into(),
    };
    let docs = render(&manifest, Some(&info));
    assert_eq!(
        docs.readme.as_bytes(),
        std::fs::read(reference.join("README.md")).unwrap()
    );
    assert_eq!(
        docs.agents.as_bytes(),
        std::fs::read(reference.join("AGENTS.md")).unwrap()
    );
}

#[rstest]
fn absent_provenance_keeps_the_honest_unknown_labels(pack_reference: PathBuf) {
    let mut manifest =
        Manifest::load(&pack_reference.join("linux-loose/.pixi-sandbox/manifest.json")).unwrap();
    manifest.source.commit = None;
    manifest.source.lock_sha256 = None;
    manifest.vendor.as_mut().unwrap().cargo_lock_sha256 = None;
    let docs = render(&manifest, None);
    assert!(docs.readme.contains("from commit `unknown`"));
    assert!(docs.readme.contains("`pixi.lock` sha256 `unknown`"));
    assert!(docs.readme.contains("`Cargo.lock` sha256 `unknown…`"));
    assert!(docs.readme.contains("Built with unknown cargo/rustc."));
}

#[rstest]
fn an_empty_host_set_adds_no_section(pack_reference: PathBuf) {
    let mut manifest =
        Manifest::load(&pack_reference.join("linux-no-vendor/.pixi-sandbox/manifest.json"))
            .unwrap();
    let without = render(&manifest, None);
    manifest.host_requirements =
        Some(pixi_sandbox_core::host_requirements::HostRequirementSet::default());
    assert_eq!(render(&manifest, None), without);
}

#[rstest]
fn environment_rows_and_the_agent_list_are_stably_ordered(pack_reference: PathBuf) {
    let mut manifest =
        Manifest::load(&pack_reference.join("linux-no-vendor/.pixi-sandbox/manifest.json"))
            .unwrap();
    let env = manifest.envs.remove("default").unwrap();
    manifest.envs.insert("zebra".into(), env.clone());
    manifest.envs.insert("alpha".into(), env);
    let docs = render(&manifest, None);
    assert!(docs.readme.find("| `alpha` |").unwrap() < docs.readme.find("| `zebra` |").unwrap());
    assert!(
        docs.agents
            .contains("environments: alpha, zebra (platform linux-64)")
    );
}

#[rstest]
fn writing_the_guides_keeps_both_reference_files(
    isolated_home: tempfile::TempDir,
    pack_reference: PathBuf,
) {
    let reference = pack_reference.join("linux-no-vendor");
    let manifest = Manifest::load(&reference.join(".pixi-sandbox/manifest.json")).unwrap();
    pixi_sandbox::branch_docs::write_branch_docs(isolated_home.path(), &manifest, None).unwrap();
    assert_eq!(
        std::fs::read(isolated_home.path().join("README.md")).unwrap(),
        std::fs::read(reference.join("README.md")).unwrap()
    );
    assert_eq!(
        std::fs::read(isolated_home.path().join("AGENTS.md")).unwrap(),
        std::fs::read(reference.join("AGENTS.md")).unwrap()
    );
}

#[rstest]
fn a_readme_write_failure_names_the_destination_and_creates_nothing(
    isolated_home: tempfile::TempDir,
    pack_reference: PathBuf,
) {
    let manifest =
        Manifest::load(&pack_reference.join("linux-no-vendor/.pixi-sandbox/manifest.json"))
            .unwrap();
    let out = isolated_home.path().join("absent");
    let error = pixi_sandbox::branch_docs::write_branch_docs(&out, &manifest, None).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("writing {}/README.md", out.display())
    );
    assert!(!out.exists());
}

#[rstest]
fn an_agents_write_failure_preserves_the_already_written_readme(
    isolated_home: tempfile::TempDir,
    pack_reference: PathBuf,
) {
    let reference = pack_reference.join("linux-no-vendor");
    let manifest = Manifest::load(&reference.join(".pixi-sandbox/manifest.json")).unwrap();
    std::fs::create_dir(isolated_home.path().join("AGENTS.md")).unwrap();
    let error = pixi_sandbox::branch_docs::write_branch_docs(isolated_home.path(), &manifest, None)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("writing {}/AGENTS.md", isolated_home.path().display())
    );
    assert_eq!(
        std::fs::read(isolated_home.path().join("README.md")).unwrap(),
        std::fs::read(reference.join("README.md")).unwrap()
    );
    assert!(isolated_home.path().join("AGENTS.md").is_dir());
}
