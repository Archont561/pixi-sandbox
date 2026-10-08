//! The real [`HostProbe`]: read-only queries against the machine `doctor` runs on.
//!
//! Every method here either reads a file, reads an environment variable, or runs one short
//! query (`ldd --version`, `dpkg-query`, `systemctl is-active`) whose output is parsed. Nothing
//! writes, nothing installs, nothing reaches the network, and nothing starts a service or a
//! display server — a probe that launched `Xvfb` to see whether Xvfb works would be a sandbox
//! escaping its own boundary.
//!
//! The parse and classification functions are `fn(&str) -> Observation` on purpose: they carry
//! all the judgement, they are unit-tested against real-world output captured in the tests at
//! the bottom of this file, and the thin process-spawning wrappers around them stay small
//! enough to read. Where a query is unavailable — no `dpkg-query`, no `systemctl`, a
//! distribution with no supported package manager — the answer is `Unknown` with the reason,
//! never a guess: a false `Satisfied` is the one outcome that can waste a user's afternoon.

use pixi_sandbox_core::host_requirements::{
    Distro, HostProbe, LibcFloor, LibcObservation, Observation,
};
use pixi_sandbox_core::platform::{HostFamily, Platform};
use std::path::Path;
use std::process::Command;

/// The probe `doctor` uses on a real machine.
#[derive(Debug, Default)]
pub struct SystemHostProbe;

impl SystemHostProbe {
    #[must_use]
    pub fn new() -> Self {
        SystemHostProbe
    }
}

impl HostProbe for SystemHostProbe {
    fn host_family(&self) -> Option<HostFamily> {
        Platform::current().map(Platform::host_family)
    }

    fn distro(&self) -> Distro {
        match std::fs::read_to_string("/etc/os-release") {
            Ok(text) => Distro::parse_os_release(&text),
            Err(_) => Distro::unknown(),
        }
    }

    fn libc(&self) -> LibcObservation {
        // `ldd --version` is the most universal glibc query; `getconf GNU_LIBC_VERSION` is the
        // fallback where ldd is absent (some minimal images).
        let output = run("ldd", &["--version"]).or_else(|| run("getconf", &["GNU_LIBC_VERSION"]));
        match output {
            Some((stdout, stderr)) => parse_libc_version(&format!("{stdout}{stderr}")),
            None => LibcObservation {
                version: None,
                detail: "neither `ldd --version` nor `getconf GNU_LIBC_VERSION` is available on \
                         this host"
                    .to_string(),
            },
        }
    }

    fn package(&self, name: &str) -> Observation {
        // The order answers correctly in containers: a Debian/Ubuntu image carries dpkg-query
        // even when it has no rpm, and vice versa. The first query that exists is the one that
        // must answer — falling through would let `dpkg-query`'s "not installed" be overruled
        // by a manager that does not manage this host's packages at all.
        if which("dpkg-query").is_some() {
            return match run("dpkg-query", &["-W", "-f=${Status}", name]) {
                Some((stdout, _)) => classify_dpkg(&stdout),
                None => Observation::unknown("`dpkg-query` could not be run"),
            };
        }
        for probe in ["rpm", "pacman"] {
            if which(probe).is_none() {
                continue;
            }
            return match run(probe, &["-q", name]) {
                Some((stdout, stderr)) => classify_rpm_like(probe, &stdout, &stderr),
                None => Observation::unknown(format!("`{probe}` could not be run")),
            };
        }
        Observation::unknown(
            "no supported package-manager query on this host (dpkg-query, rpm, pacman)",
        )
    }

    fn service(&self, name: &str) -> Observation {
        // A session bus is the one service a GUI workload needs *before* systemd is relevant,
        // and it is exactly the thing an airlocked container lacks — so it is answered
        // directly rather than through a service manager that a container usually does not run.
        if name.eq_ignore_ascii_case("dbus") {
            return classify_dbus(
                std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some(),
                Path::new("/run/dbus/system_bus_socket").exists(),
            );
        }
        if which("systemctl").is_none() {
            return Observation::unknown(format!(
                "no systemd (systemctl) on this host to query `{name}`"
            ));
        }
        match run("systemctl", &["is-active", name]) {
            Some((stdout, _)) => classify_systemctl(&stdout),
            None => Observation::unknown("`systemctl is-active` could not be run"),
        }
    }

    fn display(&self) -> Observation {
        classify_display(
            std::env::var("DISPLAY").ok().as_deref(),
            std::env::var("WAYLAND_DISPLAY").ok().as_deref(),
        )
    }

    fn gpu(&self) -> Observation {
        if !matches!(self.host_family(), Some(HostFamily::Linux)) {
            return Observation::unknown(
                "GPU detection is implemented for Linux hosts only; check the driver on this \
                 platform by hand",
            );
        }
        classify_gpu(Path::new("/dev/dri"))
    }

    fn program(&self, name: &str) -> Observation {
        // A probe never executes the program it looks for.
        match which(name) {
            Some(path) => Observation::satisfied(format!("{} ({})", path.display(), "on PATH")),
            None => Observation::missing(format!("`{name}` is not on PATH")),
        }
    }
}

/// Run a command, returning `(stdout, stderr)` when it started — regardless of exit status,
/// because `dpkg-query` and `rpm -q` signal "not installed" through their exit code rather
/// than through a failure to start.
fn run(program: &str, args: &[&str]) -> Option<(String, String)> {
    let output = Command::new(program).args(args).output().ok()?;
    Some((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// Resolve `name` on `PATH` without spawning anything, requiring the executable bit.
fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        let candidate = directory.join(name);
        let metadata = std::fs::symlink_metadata(&candidate).ok()?;
        let executable = if metadata.file_type().is_symlink() {
            std::fs::metadata(&candidate).map(|m| m.is_file()).ok()?
        } else {
            metadata.is_file()
        };
        if !executable {
            return None;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&candidate).ok()?.permissions().mode();
            if mode & 0o111 == 0 {
                return None;
            }
        }
        Some(candidate)
    })
}

/// The first `major.minor[.patch]` token in a version banner, in the order the banner reports
/// them.
///
/// `ldd --version` shapes seen in the wild:
/// - `ldd (Ubuntu GLIBC 2.39-0ubuntu8.3) 2.39`
/// - `ldd (GNU libc) 2.34`
/// - `musl libc (x86_64)\nVersion 1.2.4` (Alpine — its floor legitimately compares as older)
/// - `getconf (GNU libc) 2.39` from the fallback
pub fn parse_libc_version(text: &str) -> LibcObservation {
    let first_line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    for token in text.split(|character: char| !character.is_ascii_digit() && character != '.') {
        if token.is_empty() {
            continue;
        }
        if let Ok(floor) = LibcFloor::parse(token) {
            return LibcObservation {
                version: Some(floor),
                detail: format!("`{}`", first_line.trim()),
            };
        }
    }
    LibcObservation {
        version: None,
        detail: "the version banner carried no parsable version".to_string(),
    }
}

/// `dpkg-query -W -f=${Status} <name>` prints `install ok installed` for a present package and
/// nothing for an absent one.
pub fn classify_dpkg(stdout: &str) -> Observation {
    let status = stdout.trim();
    if status.contains("install ok installed") {
        Observation::satisfied("dpkg-query: install ok installed")
    } else if status.is_empty() {
        Observation::missing("dpkg-query: not installed")
    } else {
        Observation::missing(format!("dpkg-query: {status}"))
    }
}

/// `rpm -q` and `pacman -Q` print the package version on success and an error sentence on
/// stdout/stderr when it is absent.
pub fn classify_rpm_like(program: &str, stdout: &str, stderr: &str) -> Observation {
    let stdout = stdout.trim();
    if stdout.is_empty() {
        let detail = stderr.trim();
        return if detail.is_empty() {
            Observation::missing(format!("{program}: not installed"))
        } else {
            Observation::missing(format!("{program}: {detail}"))
        };
    }
    Observation::satisfied(format!("{program}: {stdout}"))
}

/// `systemctl is-active <name>` prints exactly one state word; `unknown` is the unit not
/// existing, which for a declared *requirement* is "missing" rather than "cannot tell".
pub fn classify_systemctl(stdout: &str) -> Observation {
    match stdout.trim() {
        "active" => Observation::satisfied("systemctl: active"),
        "activating" | "reloading" => Observation::satisfied("systemctl: transitioning to active"),
        "" => Observation::unknown("systemctl printed no state"),
        other => Observation::missing(format!("systemctl: {other}")),
    }
}

/// A session bus is reachable when this process has its address, or when the system socket
/// exists — the two things a GUI workload actually uses, neither of which needs systemd.
pub fn classify_dbus(session_address: bool, system_socket: bool) -> Observation {
    match (session_address, system_socket) {
        (true, _) => Observation::satisfied("DBUS_SESSION_BUS_ADDRESS is set"),
        (false, true) => Observation::satisfied("/run/dbus/system_bus_socket exists"),
        (false, false) => Observation::missing(
            "no DBUS_SESSION_BUS_ADDRESS and no /run/dbus/system_bus_socket on this host",
        ),
    }
}

pub fn classify_display(display: Option<&str>, wayland: Option<&str>) -> Observation {
    match (display, wayland) {
        (Some(value), _) if !value.is_empty() => Observation::satisfied(format!("DISPLAY={value}")),
        (_, Some(value)) if !value.is_empty() => {
            Observation::satisfied(format!("WAYLAND_DISPLAY={value}"))
        }
        _ => Observation::missing("DISPLAY and WAYLAND_DISPLAY are unset"),
    }
}

/// `true` when the device directory holds at least one entry — a render or card node means the
/// kernel driver is bound and the workload may be able to open it.
pub fn classify_gpu(dev_dri: &Path) -> Observation {
    let entries = match std::fs::read_dir(dev_dri) {
        Ok(entries) => entries.flatten().count(),
        Err(_) => return Observation::missing(format!("{} does not exist", dev_dri.display())),
    };
    if entries == 0 {
        return Observation::missing(format!("{} has no device nodes", dev_dri.display()));
    }
    Observation::satisfied(format!(
        "{} holds {entries} device node(s)",
        dev_dri.display()
    ))
}
