#[cfg(test)]
mod tests {
    use crate::reconstruct::{Mode, PackFormat, ReconstructArgs, pack_shape, run};
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[allow(dead_code)]
    fn create_fake_bin(dir: &Path, name: &str, script: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
        }
        path
    }

    #[test]
    fn test_pack_shape_detection() {
        let tmp = TempDir::new().unwrap();
        let pack_path = tmp.path().join("test.tar");

        // Create a plain tar with pixi-pack.json
        {
            let file = std::fs::File::create(&pack_path).unwrap();
            let mut builder = tar::Builder::new(file);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_cksum();
            builder
                .append_data(&mut header, "pixi-pack.json", b"test".as_ref())
                .unwrap();
            builder.finish().unwrap();
        }

        let shape = pack_shape(&pack_path);
        assert_eq!(shape, PackFormat::Pack);

        // Create raw tar without pixi-pack.json
        let raw_path = tmp.path().join("raw.tar");
        {
            let file = std::fs::File::create(&raw_path).unwrap();
            let mut builder = tar::Builder::new(file);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_cksum();
            builder
                .append_data(&mut header, "bin/foo", b"test".as_ref())
                .unwrap();
            builder.finish().unwrap();
        }

        let shape = pack_shape(&raw_path);
        assert_eq!(shape, PackFormat::Raw);
    }

    #[test]
    fn test_sha256_verification() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let hash = crate::hash::sha256_file(&file_path).unwrap();
        // echo -n "hello world" | sha256sum = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Create SHA256SUMS
        let sums_path = tmp.path().join("SHA256SUMS");
        std::fs::write(&sums_path, format!("{hash}  test.txt\n")).unwrap();

        // Should verify ok
        crate::hash::verify_sha256sums(tmp.path()).unwrap();

        // Perturb file
        std::fs::write(&file_path, b"hello world!").unwrap();
        let result = crate::hash::verify_sha256sums(tmp.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SandboxError::Integrity(msg) => {
                assert!(msg.contains("mismatch"));
            }
            _ => panic!("expected integrity error"),
        }
    }

    #[test]
    fn test_reconstruct_missing_kit() {
        let args = ReconstructArgs {
            from: PathBuf::from("/nonexistent/kit"),
            envs: vec!["default".to_string()],
            workspace: TempDir::new().unwrap().path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };
        let result = run(args);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SandboxError::Unavailable(msg) => {
                assert!(msg.contains("no kit"));
            }
            _ => panic!("expected unavailable"),
        }
    }

    #[test]
    fn test_reconstruct_integrity_failure() {
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(&kit_dir).unwrap();
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();

        // Create a file and SHA256SUMS with wrong hash
        let payload = kit_dir.join("payload.tar");
        std::fs::write(&payload, b"original content").unwrap();
        let hash = crate::hash::sha256_file(&payload).unwrap();

        // Write SHA256SUMS with correct hash
        std::fs::write(kit_dir.join("SHA256SUMS"), format!("{hash}  payload.tar\n")).unwrap();

        // Now perturb payload
        std::fs::write(&payload, b"perturbed content").unwrap();

        // Create fake pixi binary in kit
        let pixi_bin = kit_dir.join("bin").join("pixi");
        std::fs::write(&pixi_bin, b"#!/bin/sh\necho pixi 0.81.0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&pixi_bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&pixi_bin, perms).unwrap();
        }

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec![],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SandboxError::Integrity(_) => {} // Expected
            e => panic!("expected integrity error, got {e:?}"),
        }
    }

    #[test]
    fn test_reconstruct_no_pixi() {
        let _guard = env_lock();
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::write(kit_dir.join("SHA256SUMS"), "").unwrap();

        let workspace = TempDir::new().unwrap();
        // Ensure no pixi on PATH by using empty PATH
        let original_path = std::env::var("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var("PATH", "");
        }

        let args = ReconstructArgs {
            from: kit_dir.clone(),
            envs: vec![],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: true, // self_test allows missing SHA256SUMS
        };

        let result = run(args);
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::SandboxError::Unavailable(msg) => {
                assert!(msg.contains("no pixi"));
            }
            e => panic!("expected unavailable, got {e:?}"),
        }
    }

    #[test]
    fn test_reconstruct_raw_tar_floor() {
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();

        // Create fake pixi binary
        let pixi_bin = kit_dir.join("bin").join("pixi");
        std::fs::write(&pixi_bin, b"#!/bin/sh\necho pixi 0.81.0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&pixi_bin).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&pixi_bin, perms).unwrap();
        }

        // Create raw tar pack
        let packs_dir = kit_dir.join("packs");
        std::fs::create_dir_all(&packs_dir).unwrap();
        let pack_path = packs_dir.join("demo-linux-64.tar");
        {
            let file = std::fs::File::create(&pack_path).unwrap();
            let mut builder = tar::Builder::new(file);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_cksum();
            builder
                .append_data(&mut header, "bin/foo", b"test".as_ref())
                .unwrap();
            builder.finish().unwrap();
        }

        // SHA256SUMS
        let hash = crate::hash::sha256_file(&pack_path).unwrap();
        let pixi_hash = crate::hash::sha256_file(&pixi_bin).unwrap();
        std::fs::write(
            kit_dir.join("SHA256SUMS"),
            format!("{hash}  packs/demo-linux-64.tar\n{pixi_hash}  bin/pixi\n"),
        )
        .unwrap();

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Raw, // Force raw to test floor rung
            print_rung: true,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_ok(), "raw tar floor should succeed: {:?}", result);

        // Check that env was unpacked
        assert!(
            workspace
                .path()
                .join(".pixi")
                .join("envs")
                .join("demo")
                .join("bin")
                .join("foo")
                .exists()
        );
    }

    // === Oracle vectors T1-T9 ===

    fn write_executable(path: &Path, content: &str) {
        std::fs::write(path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    fn make_tar_with_entries(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut builder = tar::Builder::new(file);
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_cksum();
            builder.append_data(&mut header, *name, *data).unwrap();
        }
        builder.finish().unwrap();
    }

    #[test]
    fn test_t1_full_kit_rung3() {
        // T1: full kit, pixi-unpack + cargo stubs → rung3
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();

        // Fake pixi
        write_executable(
            &kit_dir.join("bin").join("pixi"),
            "#!/bin/sh\necho pixi 0.81.0\n",
        );
        // Fake pixi-unpack that extracts and creates conda-meta/history
        write_executable(
            &kit_dir.join("bin").join("pixi-unpack"),
            r#"#!/bin/sh
# Parse args: -o <out> -e <env> <pack>
OUT=""
ENV=""
PACK=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o) OUT="$2"; shift 2;;
    -e) ENV="$2"; shift 2;;
    *) PACK="$1"; shift;;
  esac
done
mkdir -p "$OUT/$ENV/conda-meta"
echo "history" > "$OUT/$ENV/conda-meta/history"
# Extract pack if it exists (simulate)
if [ -f "$PACK" ]; then
  tar -xf "$PACK" -C "$OUT/$ENV" 2>/dev/null || true
fi
exit 0
"#,
        );

        // Pack with pixi-pack.json (pack-shaped)
        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(
            &pack_path,
            &[
                ("pixi-pack.json", br#"{"version":1}"#),
                ("bin/python", b"fake python"),
            ],
        );

        // Workspace files
        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        // SHA256SUMS
        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: true,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_ok(), "T1 should succeed: {:?}", result);
        assert!(
            workspace
                .path()
                .join(".pixi")
                .join("envs")
                .join("demo")
                .join("conda-meta")
                .join("history")
                .exists()
        );
        assert!(workspace.path().join(".pixi").join("assemble.env").exists());
        assert!(
            workspace
                .path()
                .join(".pixi")
                .join("bin")
                .join("pixi")
                .exists()
        );
    }

    fn walkdir(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(walkdir(&path));
                } else {
                    files.push(path);
                }
            }
        }
        files
    }

    #[test]
    fn test_t2_no_unpack_rung2() {
        let _guard = env_lock();
        // T2: kit without pixi-unpack, channel present → rung2
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("channel")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();

        // Fake pixi that handles channel add and install
        write_executable(
            &kit_dir.join("bin").join("pixi"),
            r#"#!/bin/sh
if echo "$@" | grep -q "channel add"; then
  exit 0
fi
if echo "$@" | grep -q "install"; then
  # Simulate pixi install creating env
  # Find workspace via pwd or arg? Use env var or cwd
  # We are called with current_dir = workspace
  mkdir -p .pixi/envs/demo/conda-meta
  echo "history" > .pixi/envs/demo/conda-meta/history
  exit 0
fi
echo pixi 0.81.0
exit 0
"#,
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(
            &pack_path,
            &[
                ("pixi-pack.json", br#"{"version":1}"#),
                ("channel/repodata.json", b"{}"),
            ],
        );

        std::fs::write(kit_dir.join("channel").join("repodata.json"), "{}").unwrap();
        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        // Filter PATH to remove pixi-unpack so T2 really has no unpacker
        let original_path = std::env::var("PATH").unwrap();
        let filtered_path = original_path
            .split(':')
            .filter(|p| !Path::new(p).join("pixi-unpack").exists())
            .collect::<Vec<_>>()
            .join(":");
        unsafe {
            std::env::set_var("PATH", &filtered_path);
        }

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        assert!(result.is_ok(), "T2 should succeed with rung2: {:?}", result);
    }

    #[test]
    fn test_t3_rung2_fail_rung4_micromamba() {
        let _guard = env_lock();
        // T3: rung2 fails, micromamba on PATH → rung4
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        let fake_path_dir = tmp.path().join("fakebin");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("channel")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();
        std::fs::create_dir_all(&fake_path_dir).unwrap();

        // Fake pixi that fails install
        write_executable(
            &kit_dir.join("bin").join("pixi"),
            r#"#!/bin/sh
if echo "$@" | grep -q "channel add"; then
  exit 0
fi
if echo "$@" | grep -q "install"; then
  echo "install failed" >&2
  exit 1
fi
echo pixi 0.81.0
exit 0
"#,
        );

        // Fake micromamba on PATH
        write_executable(
            &fake_path_dir.join("micromamba"),
            r#"#!/bin/sh
# micromamba create -y -p <env_path> -f <yml>
# Parse -p
ENV_PATH=""
while [ $# -gt 0 ]; do
  case "$1" in
    -p) ENV_PATH="$2"; shift 2;;
    *) shift;;
  esac
done
mkdir -p "$ENV_PATH/conda-meta"
echo "history" > "$ENV_PATH/conda-meta/history"
exit 0
"#,
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(&pack_path, &[("pixi-pack.json", br#"{"version":1}"#)]);

        std::fs::write(
            kit_dir.join("channel").join("environment.yml"),
            "name: demo\ndependencies:\n  - python\n",
        )
        .unwrap();
        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        let original_path = std::env::var("PATH").unwrap();
        // Filter pixi-unpack out, but keep fake_path_dir with micromamba
        let filtered_original = original_path
            .split(':')
            .filter(|p| !Path::new(p).join("pixi-unpack").exists())
            .collect::<Vec<_>>()
            .join(":");
        let new_path = format!("{}:{}", fake_path_dir.display(), filtered_original);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        assert!(result.is_ok(), "T3 should succeed with rung4: {:?}", result);
        assert!(
            workspace
                .path()
                .join(".pixi")
                .join("envs")
                .join("demo")
                .join("conda-meta")
                .join("history")
                .exists()
        );
    }

    #[test]
    fn test_t4_channel_shaped_no_driver_refuse() {
        let _guard = env_lock();
        // T4: channel-shaped pack, no unpacker, no channel, no installer → refuse E-UNAVAILABLE
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();

        write_executable(
            &kit_dir.join("bin").join("pixi"),
            "#!/bin/sh\necho pixi 0.81.0\n",
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(&pack_path, &[("pixi-pack.json", br#"{"version":1}"#)]);

        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        // Ensure no micromamba/conda/pixi-unpack on PATH
        let original_path = std::env::var("PATH").unwrap();
        let filtered_path = original_path
            .split(':')
            .filter(|p| {
                !Path::new(p).join("micromamba").exists()
                    && !Path::new(p).join("conda").exists()
                    && !Path::new(p).join("pixi-unpack").exists()
            })
            .collect::<Vec<_>>()
            .join(":");
        unsafe {
            std::env::set_var("PATH", &filtered_path);
        }

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        assert!(result.is_err(), "T4 should fail");
        match result.unwrap_err() {
            crate::error::SandboxError::Unavailable(msg) => {
                assert!(msg.contains("refusing to tar a channel-shaped pack"));
            }
            e => panic!("expected unavailable, got {e:?}"),
        }
    }

    #[test]
    fn test_t6_sha256_mismatch() {
        // T6 already covered by test_reconstruct_integrity_failure, but repeat with exit code check
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();

        let payload = kit_dir.join("payload.tar");
        std::fs::write(&payload, b"original").unwrap();
        let hash = crate::hash::sha256_file(&payload).unwrap();
        std::fs::write(kit_dir.join("SHA256SUMS"), format!("{hash}  payload.tar\n")).unwrap();
        std::fs::write(&payload, b"perturbed").unwrap();

        write_executable(&kit_dir.join("bin").join("pixi"), "#!/bin/sh\necho pixi\n");

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec![],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Auto,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), 4); // E-INTEGRITY exit 4
    }

    #[test]
    fn test_t7_cargo_metadata_fail() {
        let _guard = env_lock();
        // T7: cargo metadata --locked --offline fails → E-VENDOR-INCOMPLETE exit 8
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        let fake_path_dir = tmp.path().join("fakebin");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();
        std::fs::create_dir_all(&fake_path_dir).unwrap();
        std::fs::create_dir_all(kit_dir.join("vendor")).unwrap();

        write_executable(
            &kit_dir.join("bin").join("pixi"),
            "#!/bin/sh\necho pixi 0.81.0\n",
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(&pack_path, &[("bin/foo", b"test")]);

        // Create vendor.tar.gz with dummy vendor dir
        let vendor_tar_path = kit_dir.join("vendor").join("vendor.tar.gz");
        {
            let file = std::fs::File::create(&vendor_tar_path).unwrap();
            let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut builder = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header.set_size(0);
            header.set_entry_type(tar::EntryType::dir());
            header.set_cksum();
            builder
                .append_data(&mut header, "vendor/", std::io::empty())
                .unwrap();
            builder.finish().unwrap();
        }

        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();
        std::fs::write(
            kit_dir.join("workspace").join("Cargo.toml"),
            "[package]\nname=\"test\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("Cargo.lock"), "# lock\n").unwrap();

        // Fake cargo that fails metadata
        write_executable(
            &fake_path_dir.join("cargo"),
            r#"#!/bin/sh
if echo "$@" | grep -q "metadata"; then
  echo "metadata failed" >&2
  exit 101
fi
echo cargo 1.98.0
exit 0
"#,
        );

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        let original_path = std::env::var("PATH").unwrap();
        let new_path = format!("{}:{}", fake_path_dir.display(), original_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }

        let workspace = TempDir::new().unwrap();
        // Copy Cargo.toml/lock to workspace for cargo metadata check? Our reconstruct copies from kit/workspace
        // It already does, but we also need workspace to have those files after copy
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: true,
            mode: Mode::Auto,
            pack_format: PackFormat::Raw,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        assert!(result.is_err(), "T7 should fail vendor incomplete");
        let err = result.unwrap_err();
        assert_eq!(err.code(), 8);
        match err {
            crate::error::SandboxError::VendorIncomplete(_) => {}
            e => panic!("expected vendor incomplete, got {e:?}"),
        }
    }

    #[test]
    fn test_t8_missing_env() {
        // T8: --env gpu when only demo packed → E-NOT-DETECTED
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();

        write_executable(
            &kit_dir.join("bin").join("pixi"),
            "#!/bin/sh\necho pixi 0.81.0\n",
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(&pack_path, &[("bin/foo", b"test")]);

        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        let workspace = TempDir::new().unwrap();
        let args = ReconstructArgs {
            from: kit_dir,
            envs: vec!["gpu".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Raw,
            print_rung: false,
            promote_path: false,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), 7);
        match err {
            crate::error::SandboxError::NotDetected(msg) => {
                assert!(msg.contains("no pack for env 'gpu'"));
            }
            e => panic!("expected not detected, got {e:?}"),
        }
    }

    #[test]
    fn test_t9_github_path_and_promote() {
        let _guard = env_lock();
        // T9: GITHUB_PATH + promote-path idempotent
        let tmp = TempDir::new().unwrap();
        let kit_dir = tmp.path().join("kit");
        let home_dir = tmp.path().join("home");
        std::fs::create_dir_all(kit_dir.join("bin")).unwrap();
        std::fs::create_dir_all(kit_dir.join("packs")).unwrap();
        std::fs::create_dir_all(kit_dir.join("workspace")).unwrap();
        std::fs::create_dir_all(&home_dir).unwrap();

        write_executable(
            &kit_dir.join("bin").join("pixi"),
            "#!/bin/sh\necho pixi 0.81.0\n",
        );

        let pack_path = kit_dir.join("packs").join("demo-linux-64.tar");
        make_tar_with_entries(&pack_path, &[("bin/foo", b"test")]);

        std::fs::write(
            kit_dir.join("workspace").join("pixi.toml"),
            "[workspace]\nname=\"test\"\n",
        )
        .unwrap();
        std::fs::write(kit_dir.join("workspace").join("pixi.lock"), "# lock\n").unwrap();

        let mut sums = String::new();
        for entry in walkdir(&kit_dir) {
            if entry.is_file() && entry.file_name().and_then(|n| n.to_str()) != Some("SHA256SUMS") {
                let rel = entry.strip_prefix(&kit_dir).unwrap();
                let hash = crate::hash::sha256_file(&entry).unwrap();
                sums.push_str(&format!("{hash}  {}\n", rel.display()));
            }
        }
        std::fs::write(kit_dir.join("SHA256SUMS"), sums).unwrap();

        let github_path_file = tmp.path().join("github_path");
        std::fs::write(&github_path_file, "").unwrap();

        let original_github_path = std::env::var("GITHUB_PATH").ok();
        let original_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("GITHUB_PATH", &github_path_file);
            std::env::set_var("HOME", &home_dir);
        }

        // Create fake rc files
        std::fs::write(home_dir.join(".profile"), "# profile\n").unwrap();
        std::fs::write(home_dir.join(".bashrc"), "# bashrc\n").unwrap();

        let workspace = TempDir::new().unwrap();

        // First run with promote-path
        let args = ReconstructArgs {
            from: kit_dir.clone(),
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Raw,
            print_rung: false,
            promote_path: true,
            self_test: false,
        };

        let result = run(args);
        assert!(result.is_ok(), "first run should succeed: {:?}", result);

        // Check GITHUB_PATH
        let github_content = std::fs::read_to_string(&github_path_file).unwrap();
        assert!(github_content.contains(".pixi/bin"));

        // Check assemble.env
        assert!(workspace.path().join(".pixi").join("assemble.env").exists());
        let env_content =
            std::fs::read_to_string(workspace.path().join(".pixi").join("assemble.env")).unwrap();
        assert!(env_content.contains("export PATH="));

        // Check rc files edited
        let profile_content = std::fs::read_to_string(home_dir.join(".profile")).unwrap();
        assert!(profile_content.contains("# pixi-sandbox assemble.sh"));
        let bashrc_content = std::fs::read_to_string(home_dir.join(".bashrc")).unwrap();
        assert!(bashrc_content.contains("# pixi-sandbox assemble.sh"));

        // Second run with promote-path should be idempotent (not duplicate)
        let args2 = ReconstructArgs {
            from: kit_dir,
            envs: vec!["demo".to_string()],
            workspace: workspace.path().to_path_buf(),
            with_vendor: false,
            mode: Mode::Auto,
            pack_format: PackFormat::Raw,
            print_rung: false,
            promote_path: true,
            self_test: false,
        };

        let result2 = run(args2);
        assert!(result2.is_ok(), "second run should succeed: {:?}", result2);

        let profile_content2 = std::fs::read_to_string(home_dir.join(".profile")).unwrap();
        let count = profile_content2
            .matches("# pixi-sandbox assemble.sh")
            .count();
        assert_eq!(
            count, 1,
            "rc file should be edited exactly once, found {count}"
        );

        unsafe {
            if let Some(val) = original_github_path {
                std::env::set_var("GITHUB_PATH", val);
            } else {
                std::env::remove_var("GITHUB_PATH");
            }
            if let Some(val) = original_home {
                std::env::set_var("HOME", val);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}
