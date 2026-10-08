//! `smoke.rs` — the conda-package probe (moved from the module's inline `#[cfg(test)]`,
//! TASK-83: names kept, bodies verbatim).

use std::fs;
use xtask::smoke::conda_packages;

#[test]
fn only_top_level_conda_files_count_and_they_come_back_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("b.conda"), b"x").expect("b");
    fs::write(dir.path().join("a.conda"), b"x").expect("a");
    fs::write(dir.path().join("notes.txt"), b"x").expect("txt");
    fs::create_dir(dir.path().join("sub")).expect("sub");
    fs::write(dir.path().join("sub/c.conda"), b"x").expect("nested");
    let names: Vec<_> = conda_packages(dir.path())
        .expect("list")
        .into_iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["a.conda", "b.conda"]);
}

#[test]
fn an_absent_out_dir_is_just_an_empty_list_so_the_caller_owns_the_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(
        conda_packages(&dir.path().join("missing"))
            .expect("list")
            .is_empty()
    );
}
