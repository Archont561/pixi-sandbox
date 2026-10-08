//! The committed `tools.lock.json` catalogue and its naming rules (moved from the module's
//! inline `#[cfg(test)]`, TASK-83 — names kept, bodies verbatim).

use pixi_sandbox_core::tools_lock::{EMBEDDED_JSON, LOCK_SCHEMA, ToolsLock, executable_filename};

#[test]
fn transport_tool_names_keep_a_windows_executable_suffix() {
    assert_eq!(executable_filename("pixi", "win-64"), "pixi.exe");
    assert_eq!(
        executable_filename("pixi-unpack.exe", "win-64"),
        "pixi-unpack.exe"
    );
    assert_eq!(executable_filename("pixi", "linux-64"), "pixi");
}

/// `tools update` re-renders the catalogue it did not change and hands it to a reviewer as a
/// diff. If pretty-printing it did not reproduce the committed bytes, every future pin bump
/// would arrive buried in reformatting, and the one line worth reviewing — the hash — would
/// be the line nobody reads.
#[test]
fn the_catalogue_round_trips_to_its_committed_bytes() {
    let lock = ToolsLock::embedded().expect("embedded tools lock must parse");
    assert_eq!(lock.schema, LOCK_SCHEMA);
    let mut rendered =
        serde_json::to_string_pretty(&lock).expect("serialising the embedded catalogue");
    rendered.push('\n');
    assert_eq!(
        rendered, EMBEDDED_JSON,
        "re-rendering the embedded catalogue must reproduce the asset byte for byte"
    );
}
