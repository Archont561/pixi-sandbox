//! Who owns the binary we are about to replace.
//!
//! Decision-4 scopes self-update to *standalone* binaries: a Pixi-managed executable belongs to
//! a package manager that records its digest, and overwriting it in place leaves the prefix
//! lying about its own contents. So this is a refusal ladder, not a heuristic — classification
//! is on path provenance, which is a fact about the installation, rather than on file contents,
//! which an attacker or a coincidence can shape.
//!
//! The order matters: the first match wins, and the four refusals each carry the remedy that
//! actually works for that owner. There is deliberately no `--force`: an escape hatch here
//! would quietly reintroduce exactly the corruption the ladder exists to prevent.

use std::path::{Path, PathBuf};

/// Who owns the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ownership {
    /// A pixi-sandbox-managed launcher script (`user_tools`), not the binary itself.
    ManagedLauncher,
    /// A `pixi global install` trampoline, recognised by its sibling config (D4).
    GlobalTrampoline,
    /// Inside a conda/Pixi prefix, recognised by an ancestor `conda-meta/`.
    CondaPrefix { prefix: PathBuf },
    /// The manifest-verified tool copy a restore materialised under `.pixi/tools/<platform>/`.
    TransportTool,
    /// A plain standalone binary — the only thing this command will replace.
    Standalone,
}

impl Ownership {
    pub fn is_standalone(&self) -> bool {
        matches!(self, Ownership::Standalone)
    }

    /// The refusal message, or `None` when the path may be replaced.
    pub fn refusal(&self, path: &Path) -> Option<String> {
        let path = path.display();
        match self {
            Ownership::Standalone => None,
            Ownership::ManagedLauncher => Some(format!(
                "{path} is a pixi-sandbox-managed launcher, not the binary itself\n\
                 it execs the manifest-verified tool under .pixi/tools/<platform>/; updating it \
                 here would point your PATH at bytes the manifest does not describe\n\
                 remedy: restore a newer transport, or pass --dest with a standalone path"
            )),
            Ownership::GlobalTrampoline => Some(format!(
                "{path} is a `pixi global install` trampoline, owned by pixi\n\
                 remedy: pixi global update pixi-sandbox"
            )),
            Ownership::CondaPrefix { prefix } => Some(format!(
                "{path} lives inside the conda/Pixi prefix {}\n\
                 the prefix records this file's digest, so replacing it in place would leave the \
                 environment describing bytes that are no longer there\n\
                 remedy: pixi update pixi-sandbox (or pixi global update pixi-sandbox)",
                prefix.display()
            )),
            Ownership::TransportTool => Some(format!(
                "{path} is a restored transport tool, owned by the sandbox manifest\n\
                 every byte there was verified against manifest.json at restore time, and \
                 nothing may be written into it afterwards\n\
                 remedy: repack and republish the transport with the new release"
            )),
        }
    }
}

/// Marker of a conda/Pixi environment prefix. Both `pixi install` prefixes
/// (`.pixi/envs/<name>/`) and `pixi global` prefixes carry one.
pub const CONDA_META: &str = "conda-meta";

/// Classify `path`. First match wins; see the module docs for why the order is the contract.
pub fn classify(path: &Path) -> Ownership {
    if is_managed_launcher(path) {
        return Ownership::ManagedLauncher;
    }
    if is_global_trampoline(path) {
        return Ownership::GlobalTrampoline;
    }
    if let Some(prefix) = conda_prefix_of(path) {
        return Ownership::CondaPrefix { prefix };
    }
    if is_transport_tool(path) {
        return Ownership::TransportTool;
    }
    Ownership::Standalone
}

/// A launcher is a small text file carrying the managed marker. Reading it as UTF-8 is also
/// what rules out a real ELF/PE binary cheaply: those are not valid UTF-8 in practice, and a
/// read failure simply means "not a launcher".
fn is_managed_launcher(path: &Path) -> bool {
    // Launchers are a few hundred bytes; refuse to slurp a 4 MiB binary to look for a marker.
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() <= 64 * 1024 => {}
        _ => return false,
    }
    std::fs::read_to_string(path)
        .map(|text| text.contains(crate::user_tools::MANAGED_MARKER))
        .unwrap_or(false)
}

/// `pixi global install` exposes commands as trampolines with a sibling
/// `trampoline_configuration/<name>.json` — the same signal `xtask airlock` follows to find the
/// real package binary.
fn is_global_trampoline(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    path.parent()
        .unwrap_or_else(|| Path::new(""))
        .join("trampoline_configuration")
        .join(format!("{name}.json"))
        .is_file()
}

/// The nearest ancestor holding `conda-meta/`. Walking up rather than matching a fixed depth
/// covers `<prefix>/bin/pixi-sandbox` and `<prefix>/Scripts/pixi-sandbox.exe` alike.
fn conda_prefix_of(path: &Path) -> Option<PathBuf> {
    let mut directory = path.parent();
    while let Some(candidate) = directory {
        if candidate.join(CONDA_META).is_dir() {
            return Some(candidate.to_path_buf());
        }
        directory = candidate.parent();
    }
    None
}

/// `<project>/.pixi/tools/<platform>/pixi-sandbox[.exe]` — what `restore` materialises.
fn is_transport_tool(path: &Path) -> bool {
    let mut parts = path.ancestors().skip(1);
    // <platform>, then `tools`, then `.pixi`.
    let Some(_platform) = parts.next() else {
        return false;
    };
    let Some(tools) = parts.next() else {
        return false;
    };
    let Some(dot_pixi) = parts.next() else {
        return false;
    };
    tools.file_name() == Some(std::ffi::OsStr::new("tools"))
        && dot_pixi.file_name() == Some(std::ffi::OsStr::new(".pixi"))
}
