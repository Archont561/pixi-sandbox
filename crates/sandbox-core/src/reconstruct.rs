use crate::error::{Result, SandboxError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    Cache = 1,
    FileChannel = 2,
    Unpack = 3,
    EnvYml = 4,
    Tar = 5,
}

impl std::fmt::Display for Rung {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackFormat {
    Auto,
    Pack,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Cache,
    FileChannel,
    Unpack,
    EnvYml,
    Tar,
}

pub struct ReconstructArgs {
    pub from: PathBuf,
    pub envs: Vec<String>,
    pub workspace: PathBuf,
    pub with_vendor: bool,
    pub mode: Mode,
    pub pack_format: PackFormat,
    pub print_rung: bool,
    pub promote_path: bool,
    pub self_test: bool,
}

fn driver_path(bin_dir: &Path, name: &str) -> Option<PathBuf> {
    // Try arch-suffixed first: pixi-$(uname -m)-unknown-linux-musl
    let arch = std::process::Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "x86_64".to_string())
        .trim()
        .to_string();

    let suffixed = bin_dir.join(format!("{name}-{arch}-unknown-linux-musl"));
    if suffixed.exists() {
        return Some(suffixed);
    }
    let bare = bin_dir.join(name);
    if bare.exists() {
        return Some(bare);
    }
    None
}

fn find_driver(bin_dir: &Path, name: &str) -> Option<PathBuf> {
    if let Some(p) = driver_path(bin_dir, name) {
        return Some(p);
    }
    // Fallback to PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(crate) fn pack_shape(pack_path: &Path) -> PackFormat {
    // List tar contents, check for pixi-pack.json or channel/
    if let Ok(file) = std::fs::File::open(pack_path) {
        // Try to handle both plain tar and gzipped tar
        // We sniff: try gz decoder first? For simplicity, try plain tar, then gz
        let mut found_pack = false;
        // Try plain tar
        if let Ok(mut archive) = tar::Archive::new(file).entries() {
            while let Some(Ok(entry)) = archive.next() {
                if let Ok(path) = entry.path() {
                    let path_str = path.to_string_lossy();
                    if path_str.contains("pixi-pack.json") || path_str.contains("channel/") {
                        found_pack = true;
                        break;
                    }
                }
            }
            if found_pack {
                return PackFormat::Pack;
            }
        }
        // Try gzipped
        if let Ok(file3) = std::fs::File::open(pack_path) {
            let gz = flate2::read::GzDecoder::new(file3);
            if let Ok(mut entries) = tar::Archive::new(gz).entries() {
                while let Some(Ok(entry)) = entries.next() {
                    if let Ok(path) = entry.path() {
                        let path_str = path.to_string_lossy();
                        if path_str.contains("pixi-pack.json") || path_str.contains("channel/") {
                            return PackFormat::Pack;
                        }
                    }
                }
            }
        }
    }
    PackFormat::Raw
}

fn install_drivers(
    from: &Path,
    bin_dir: &Path,
    workspace: &Path,
) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    std::fs::create_dir_all(bin_dir).map_err(|e| SandboxError::Other(e.into()))?;

    // Never inherit another kit's driver if bin_dir is workspace/.pixi/bin
    let is_workspace_bin = bin_dir == workspace.join(".pixi").join("bin");
    if is_workspace_bin {
        for b in ["pixi", "pixi-unpack", "pixi-sandbox"] {
            let _ = std::fs::remove_file(bin_dir.join(b));
        }
    } else {
        for b in ["pixi", "pixi-unpack", "pixi-sandbox"] {
            if bin_dir.join(b).exists() {
                println!(
                    "  note: left existing {}/{} in place (PIXISB_BIN_DIR is yours)",
                    bin_dir.display(),
                    b
                );
            }
        }
    }

    let from_bin = from.join("bin");
    let mut installed = Vec::new();
    for name in ["pixi", "pixi-unpack", "pixi-sandbox"] {
        if let Some(src) = driver_path(&from_bin, name) {
            let dest = bin_dir.join(name);
            std::fs::copy(&src, &dest).map_err(|e| SandboxError::Other(e.into()))?;
            // Preserve executable bit
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest)
                    .map_err(|e| SandboxError::Other(e.into()))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest, perms)
                    .map_err(|e| SandboxError::Other(e.into()))?;
            }
            println!("  driver: {name} installed");
            installed.push(name);
        }
    }

    // Record which kit wrote bin dir
    let kit_marker = bin_dir.join(".pixi-sandbox-kit");
    let _ = std::fs::write(&kit_marker, from.to_string_lossy().as_bytes());

    let pixi = find_driver(bin_dir, "pixi");
    let unpack = find_driver(bin_dir, "pixi-unpack");

    if pixi.is_none() {
        return Err(SandboxError::Unavailable(
            "no pixi binary in the kit and none on PATH".to_string(),
        ));
    }

    if let Some(ref p) = pixi
        && p != &bin_dir.join("pixi")
    {
        println!(
            "  warning[W-PROVENANCE]: pixi resolved to {}, outside the verified kit",
            p.display()
        );
    }

    if unpack.is_none() {
        println!(
            "  rung 3 unavailable (no kit pixi-unpack) — auto mode will land on the tar floor"
        );
    }

    Ok((pixi, unpack))
}

fn write_assemble_env(bin_dir: &Path, workspace: &Path) -> Result<PathBuf> {
    let envrc = workspace.join(".pixi").join("assemble.env");
    if let Some(parent) = envrc.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SandboxError::Other(e.into()))?;
    }
    let content = format!("export PATH=\"{}:$PATH\"\n", bin_dir.display());
    std::fs::write(&envrc, content).map_err(|e| SandboxError::Other(e.into()))?;

    if let Ok(github_path) = std::env::var("GITHUB_PATH")
        && !github_path.is_empty()
    {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .open(&github_path)
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", bin_dir.display())
            });
        println!("  path: {} appended to $GITHUB_PATH", bin_dir.display());
    }

    println!(
        "  path: source \"{}\"  —  or re-run with --promote-path to edit your rc files",
        envrc.display()
    );
    Ok(envrc)
}

fn promote_path(envrc: &Path) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    for rc_name in [".profile", ".bashrc"] {
        let rc_path = Path::new(&home).join(rc_name);
        if !rc_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&rc_path).unwrap_or_default();
        if content.contains("# pixi-sandbox assemble.sh") {
            continue;
        }
        let addition = format!("\n# pixi-sandbox assemble.sh\n. \"{}\"\n", envrc.display());
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .open(&rc_path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(addition.as_bytes())
            });
    }
    println!("  path: rc files point at {} (idempotent)", envrc.display());
    Ok(())
}

pub fn run(args: ReconstructArgs) -> Result<()> {
    let from = args.from;
    let workspace = args.workspace;
    let envs = args.envs;

    if !from.exists() {
        return Err(SandboxError::Unavailable(format!(
            "no kit at {}",
            from.display()
        )));
    }

    // 1. Integrity BEFORE executing anything
    let sums_path = from.join("SHA256SUMS");
    if sums_path.exists() {
        crate::hash::verify_sha256sums(&from)?;
        println!("  integrity: sha256 verified ✅");
    } else {
        println!("  integrity: no SHA256SUMS in kit (refusing to run binaries without it)");
        if !args.self_test {
            return Err(SandboxError::Integrity("missing SHA256SUMS".to_string()));
        }
    }

    // 2. Install drivers
    let bin_dir = if let Ok(dir) = std::env::var("PIXISB_BIN_DIR") {
        PathBuf::from(dir)
    } else {
        workspace.join(".pixi").join("bin")
    };
    std::fs::create_dir_all(&bin_dir).map_err(|e| SandboxError::Other(e.into()))?;

    let (pixi, unpack) = install_drivers(&from, &bin_dir, &workspace)?;

    // 2b. PATH exposure
    let envrc = write_assemble_env(&bin_dir, &workspace)?;
    if args.promote_path {
        promote_path(&envrc)?;
    }

    // 3. Materialise workspace
    std::fs::create_dir_all(workspace.join(".pixi").join("envs"))
        .map_err(|e| SandboxError::Other(e.into()))?;

    for f in ["pixi.toml", "pixi.lock"] {
        let src = from.join("workspace").join(f);
        if src.exists() {
            let dest = workspace.join(f);
            std::fs::copy(&src, &dest).map_err(|e| SandboxError::Other(e.into()))?;
        }
    }

    if !workspace.join("pixi.lock").exists() {
        println!(
            "  warning[W-STALE]: no pixi.lock — pixi install --frozen will refuse, which is correct"
        );
    }

    // Write .pixi/config.toml
    let config_content = r#"offline = true
pinning-strategy = "exact-version"
[cache]
conda-packages = "/var/tmp/pixi-sandbox/pkgs"
"#;
    let config_path = workspace.join(".pixi").join("config.toml");
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SandboxError::Other(e.into()))?;
    }
    std::fs::write(&config_path, config_content).map_err(|e| SandboxError::Other(e.into()))?;

    // 4. Unpack each environment
    let mut rung_used: Option<Rung> = None;

    // Handle chunked archives: reassemble .part_* if needed
    let packs_dir = from.join("packs");
    if packs_dir.exists() {
        // Check for .part_ files and reassemble
        let entries = std::fs::read_dir(&packs_dir).map_err(|e| SandboxError::Other(e.into()))?;
        let mut part_groups: std::collections::HashMap<String, Vec<PathBuf>> =
            std::collections::HashMap::new();
        for entry in entries {
            let entry = entry.map_err(|e| SandboxError::Other(e.into()))?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.contains(".part_")
                && let Some(base) = name.split(".part_").next()
            {
                part_groups.entry(base.to_string()).or_default().push(path);
            }
        }
        for (base, mut parts) in part_groups {
            parts.sort();
            let dest = packs_dir.join(&base);
            if dest.exists() {
                continue;
            }
            println!("  reassembling {} from {} parts", base, parts.len());
            let mut out =
                std::fs::File::create(&dest).map_err(|e| SandboxError::Other(e.into()))?;
            for part in parts {
                let mut input =
                    std::fs::File::open(&part).map_err(|e| SandboxError::Other(e.into()))?;
                std::io::copy(&mut input, &mut out).map_err(|e| SandboxError::Other(e.into()))?;
            }
        }
    }

    for env_name in &envs {
        // Find pack for env
        let mut pack_path: Option<PathBuf> = None;
        if packs_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&packs_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|n| n.to_str())
                    && fname.contains(env_name)
                    && fname.ends_with(".tar")
                {
                    pack_path = Some(path);
                    break;
                }
            }
        }
        // Also check from root
        if pack_path.is_none()
            && let Ok(entries) = std::fs::read_dir(&from)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|n| n.to_str())
                    && fname.contains(env_name)
                    && (fname.ends_with(".tar") || fname.ends_with(".tar.gz"))
                {
                    pack_path = Some(path);
                    break;
                }
            }
        }

        let pack_path = pack_path.ok_or_else(|| {
            let available = if packs_dir.exists() {
                std::fs::read_dir(&packs_dir)
                    .map(|entries| {
                        entries
                            .flatten()
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            } else {
                "".to_string()
            };
            SandboxError::NotDetected(format!(
                "no pack for env '{env_name}' (kit has: {available})"
            ))
        })?;

        let shape = match args.pack_format {
            PackFormat::Auto => pack_shape(&pack_path),
            PackFormat::Pack => PackFormat::Pack,
            PackFormat::Raw => PackFormat::Raw,
        };

        // Helper closures for each rung
        let try_unpack = |unpack_bin: &Option<PathBuf>,
                          env_name: &str,
                          pack_path: &Path,
                          workspace: &Path|
         -> Result<bool> {
            if let Some(bin) = unpack_bin {
                let out_dir = workspace.join(".pixi");
                let env_arg = format!("envs/{env_name}");
                let output = std::process::Command::new(bin)
                    .arg("-o")
                    .arg(&out_dir)
                    .arg("-e")
                    .arg(&env_arg)
                    .arg(pack_path)
                    .output()
                    .map_err(|e| SandboxError::Other(e.into()))?;
                if output.status.success() {
                    println!(
                        "  rung 3: {env_name} unpacked by {} (installer-made prefix)",
                        bin.display()
                    );
                    return Ok(true);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(SandboxError::Reconstruct(format!(
                        "pixi-unpack failed for env {env_name}: {stderr}"
                    )));
                }
            }
            Ok(false)
        };

        let try_file_channel =
            |pixi_bin: &Option<PathBuf>, from: &Path, workspace: &Path| -> Result<bool> {
                let channel_dir = from.join("channel");
                if !channel_dir.exists() {
                    return Ok(false);
                }
                if let Some(bin) = pixi_bin {
                    let _ = std::process::Command::new(bin)
                        .arg("project")
                        .arg("channel")
                        .arg("add")
                        .arg(format!("file://{}", channel_dir.display()))
                        .arg("--no-install")
                        .current_dir(workspace)
                        .output();
                    let install_output = std::process::Command::new(bin)
                        .arg("install")
                        .arg("--frozen")
                        .current_dir(workspace)
                        .output()
                        .map_err(|e| SandboxError::Other(e.into()))?;
                    if install_output.status.success() {
                        println!(
                            "  rung 2: workspace installed offline against file://{}/channel",
                            from.display()
                        );
                        return Ok(true);
                    }
                }
                Ok(false)
            };

        let try_env_yml = |from: &Path, workspace: &Path, env_name: &str| -> Result<bool> {
            let mut found_installer = None;
            for installer in ["micromamba", "conda"] {
                if let Some(path) = which(installer) {
                    found_installer = Some((installer, path));
                    break;
                }
            }
            if let Some((name, bin)) = found_installer {
                let env_path = workspace.join(".pixi").join("envs").join(env_name);
                let yml_path = from.join("channel").join("environment.yml");
                let yml_path = if yml_path.exists() {
                    yml_path
                } else {
                    from.join("environment.yml")
                };
                if yml_path.exists() {
                    let result = match name {
                        "micromamba" => std::process::Command::new(bin)
                            .arg("create")
                            .arg("-y")
                            .arg("-p")
                            .arg(&env_path)
                            .arg("-f")
                            .arg(&yml_path)
                            .output(),
                        _ => std::process::Command::new(bin)
                            .arg("env")
                            .arg("create")
                            .arg("-p")
                            .arg(&env_path)
                            .arg("-f")
                            .arg(&yml_path)
                            .output(),
                    };
                    if let Ok(output) = result
                        && output.status.success()
                    {
                        println!(
                            "  rung 4: {env_name} created by {name} from the pack's environment.yml"
                        );
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        };

        let try_tar_floor = |pack_path: &Path, workspace: &Path, env_name: &str| -> Result<()> {
            let dest = workspace.join(".pixi").join("envs").join(env_name);
            std::fs::create_dir_all(&dest).map_err(|e| SandboxError::Other(e.into()))?;
            let file = std::fs::File::open(pack_path).map_err(|e| SandboxError::Other(e.into()))?;
            let mut archive = tar::Archive::new(file);
            let unpack_result = archive.unpack(&dest);
            if unpack_result.is_err() {
                let file2 =
                    std::fs::File::open(pack_path).map_err(|e| SandboxError::Other(e.into()))?;
                let gz = flate2::read::GzDecoder::new(file2);
                let mut archive2 = tar::Archive::new(gz);
                archive2.unpack(&dest).map_err(|e| {
                    SandboxError::Reconstruct(format!(
                        "tar -xf {} failed: {e}",
                        pack_path.display()
                    ))
                })?;
            }
            println!(
                "  rung 5 (tar): prefix paths are NOT rewritten — fine for relocatable trees, wrong for python entry points"
            );
            Ok(())
        };

        match args.mode {
            Mode::Auto => {
                if shape == PackFormat::Pack {
                    // Rung 3
                    if unpack.is_some() {
                        match try_unpack(&unpack, env_name, &pack_path, &workspace) {
                            Ok(true) => {
                                rung_used = Some(Rung::Unpack);
                                continue;
                            }
                            Ok(false) => {}
                            Err(e) => return Err(e),
                        }
                    }
                    // Rung 2
                    match try_file_channel(&pixi, &from, &workspace) {
                        Ok(true) => {
                            rung_used = Some(Rung::FileChannel);
                            continue;
                        }
                        Ok(false) => {}
                        Err(e) => return Err(e),
                    }
                    // Rung 4
                    match try_env_yml(&from, &workspace, env_name) {
                        Ok(true) => {
                            rung_used = Some(Rung::EnvYml);
                            continue;
                        }
                        Ok(false) => {}
                        Err(e) => return Err(e),
                    }
                    return Err(SandboxError::Unavailable(
                        "no kit pixi-unpack, rung 2 unavailable, no micromamba/conda on PATH — refusing to tar a channel-shaped pack (see /workflows/airlock-clone.md §2.4)".to_string(),
                    ));
                } else {
                    // Raw tar floor
                    try_tar_floor(&pack_path, &workspace, env_name)?;
                    rung_used = Some(Rung::Tar);
                    continue;
                }
            }
            Mode::Unpack => {
                if unpack.is_none() {
                    return Err(SandboxError::Unavailable(
                        "--mode unpack, but the kit ships no pixi-unpack".to_string(),
                    ));
                }
                match try_unpack(&unpack, env_name, &pack_path, &workspace) {
                    Ok(true) => {
                        rung_used = Some(Rung::Unpack);
                        continue;
                    }
                    Ok(false) => {
                        return Err(SandboxError::Unavailable(
                            "--mode unpack, but the kit ships no pixi-unpack".to_string(),
                        ));
                    }
                    Err(e) => return Err(e),
                }
            }
            Mode::FileChannel => match try_file_channel(&pixi, &from, &workspace) {
                Ok(true) => {
                    rung_used = Some(Rung::FileChannel);
                    continue;
                }
                Ok(false) => {
                    return Err(SandboxError::Unavailable(
                        "rung 2 unavailable (no channel or pixi install failed)".to_string(),
                    ));
                }
                Err(e) => return Err(e),
            },
            Mode::EnvYml => match try_env_yml(&from, &workspace, env_name) {
                Ok(true) => {
                    rung_used = Some(Rung::EnvYml);
                    continue;
                }
                Ok(false) => {
                    return Err(SandboxError::Unavailable(
                        "rung 4 unavailable (no micromamba/conda or no environment.yml)"
                            .to_string(),
                    ));
                }
                Err(e) => return Err(e),
            },
            Mode::Tar => {
                if shape == PackFormat::Pack {
                    return Err(SandboxError::Unavailable(
                        "refusing to tar a channel-shaped pack (see /workflows/airlock-clone.md §2.4)".to_string(),
                    ));
                }
                try_tar_floor(&pack_path, &workspace, env_name)?;
                rung_used = Some(Rung::Tar);
                continue;
            }
            Mode::Cache => {
                return Err(SandboxError::Unavailable(
                    "rung 1 (cache) not implemented in M1".to_string(),
                ));
            }
        }
    }

    // 5. Vendor
    if args.with_vendor {
        let vendor_tar = from.join("vendor").join("vendor.tar.gz");
        let vendor_tar = if vendor_tar.exists() {
            vendor_tar
        } else {
            from.join("cargo-vendor.tar.gz")
        };

        if !vendor_tar.exists() {
            return Err(SandboxError::NotDetected(
                "kit has no vendor payload (CI ran with bundle-crates=false)".to_string(),
            ));
        }

        let vendor_dest = workspace.join(".pixi").join("vendor");
        std::fs::create_dir_all(&vendor_dest).map_err(|e| SandboxError::Other(e.into()))?;

        let file = std::fs::File::open(&vendor_tar).map_err(|e| SandboxError::Other(e.into()))?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive
            .unpack(&vendor_dest)
            .map_err(|e| SandboxError::Reconstruct(format!("vendor unpack failed: {e}")))?;

        let cargo_config_dir = workspace.join(".cargo");
        std::fs::create_dir_all(&cargo_config_dir).map_err(|e| SandboxError::Other(e.into()))?;
        let config_content = format!(
            "[source.crates-io]\nreplace-with = \"kit-vendored\"\n\n[source.kit-vendored]\ndirectory = \"{}/vendor/vendor\"\n\n[net]\noffline = true\n",
            vendor_dest.display()
        );
        std::fs::write(cargo_config_dir.join("config.toml"), config_content)
            .map_err(|e| SandboxError::Other(e.into()))?;

        println!(
            "  vendor: directory source wired via .cargo/config.toml (kit-scoped, not your repo's)"
        );

        if which("cargo").is_some() {
            let output = std::process::Command::new("cargo")
                .arg("metadata")
                .arg("--locked")
                .arg("--offline")
                .current_dir(&workspace)
                .output()
                .map_err(|e| SandboxError::Other(e.into()))?;

            if !output.status.success() {
                return Err(SandboxError::VendorIncomplete(
                    "cargo metadata --offline failed".to_string(),
                ));
            }
            println!("  vendor: cargo metadata --locked --offline ✅");
        } else {
            println!(
                "  vendor: cargo not on PATH — compile-on-target needs `rust` in the env (see D17/§7.4)"
            );
        }
    }

    if args.print_rung {
        if let Some(rung) = rung_used {
            println!("rung={rung}");
        } else {
            println!("rung=none");
        }
    }

    println!(
        "  done: pixi run <task> now resolves against {}/.pixi/envs (drivers in {})",
        workspace.display(),
        {
            if let Ok(dir) = std::env::var("PIXISB_BIN_DIR") {
                dir
            } else {
                workspace.join(".pixi").join("bin").display().to_string()
            }
        }
    );

    Ok(())
}

fn which(name: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
