//! The committed `tools.lock.json` catalogue and its naming rules (moved from the module's
//! inline `#[cfg(test)]`, TASK-83 — names kept, bodies verbatim).

use pixi_sandbox_core::platform::executable_filename;

#[test]
fn transport_tool_names_keep_a_windows_executable_suffix() {
    assert_eq!(executable_filename("pixi", "win-64"), "pixi.exe");
    assert_eq!(
        executable_filename("pixi-unpack.exe", "win-64"),
        "pixi-unpack.exe"
    );
    assert_eq!(executable_filename("pixi", "linux-64"), "pixi");
}
