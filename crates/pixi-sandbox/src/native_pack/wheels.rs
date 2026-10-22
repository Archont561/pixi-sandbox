// stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
//! Offline wheel linking; no Python interpreter, resolver or HTTP client is run.
use anyhow::{Context, Result, bail};
// stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
use rattler::install::PythonInfo;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use uv_distribution_filename::WheelFilename;
use uv_install_wheel::{InstallState, Layout, LinkMode};
use uv_pypi_types::{DirectUrl, Scheme};

pub(super) struct Wheel {
    path: PathBuf,
    filename: WheelFilename,
}

// stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
pub(super) fn inspect(pack: &Path, interpreter: Option<&PythonInfo>) -> Result<Vec<Wheel>> {
    let directory = pack.join("pypi");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
    files.sort_by_key(std::fs::DirEntry::file_name);
    let mut wheels = Vec::new();
    for entry in files {
        if !entry.file_type()?.is_file() {
            bail!(
                "PyPI artifacts must be regular wheel files: {}",
                entry.path().display()
            );
        }
        let filename = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("wheel filename is not UTF-8"))?;
        let filename: WheelFilename = filename
            .parse()
            .context("unsupported PyPI artifact: expected a wheel")?;
        let mut archive = zip::ZipArchive::new(File::open(entry.path())?)?;
        for index in 0..archive.len() {
            let file = archive.by_index(index)?;
            let name = file.name()?;
            if file.enclosed_name().is_none()
                || name.is_empty()
                || name.contains(['\\', ':'])
                || Path::new(name.as_ref())
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || file
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170_000 == 0o120_000)
            {
                bail!("unsafe wheel entry: {}", file.name()?);
            }
        }
        wheels.push(Wheel {
            path: entry.path(),
            filename,
        });
    }
    if !wheels.is_empty() {
        // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
        let interpreter =
            interpreter.context("a wheel pack requires a locked Conda interpreter package")?;
        if interpreter
            .site_packages_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
            bail!("unsafe Python site-packages path in the package record");
        }
    }
    Ok(wheels)
}

pub(super) fn install(
    wheels: Vec<Wheel>,
    // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
    interpreter: Option<&PythonInfo>,
    stage: &Path,
    destination: &Path,
    cache: &Path,
) -> Result<()> {
    if wheels.is_empty() {
        return Ok(());
    }
    // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
    let interpreter =
        interpreter.context("a wheel pack requires a locked Conda interpreter package")?;
    let windows = interpreter.platform.is_windows();
    let (major, minor) = interpreter.short_version;
    let site = stage.join(&interpreter.site_packages_path);
    let layout = Layout {
        sys_executable: destination.join(interpreter.path()),
        // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
        python_version: (u8::try_from(major)?, u8::try_from(minor)?),
        os_name: if windows { "nt" } else { "posix" }.into(),
        scheme: Scheme {
            purelib: site.clone(),
            platlib: site,
            scripts: stage.join(&interpreter.bin_dir),
            data: stage.to_path_buf(),
            include: stage
                .join(if windows { "Include" } else { "include" })
                .join(format!("interpreter{major}.{minor}")),
        },
    };
    let install_state = InstallState::default();
    for (index, wheel) in wheels.into_iter().enumerate() {
        let extracted = cache.join(format!("wheel-{index}"));
        let mut archive = zip::ZipArchive::new(File::open(&wheel.path)?)?;
        archive.extract(&extracted)?;
        let direct_url = DirectUrl::ArchiveUrl {
            url: url::Url::from_file_path(&wheel.path)
                .map_err(|()| anyhow::anyhow!("invalid local wheel path"))?
                .to_string(),
            archive_info: serde_json::from_str("{}")?,
            subdirectory: None,
        };
        uv_install_wheel::install_wheel::<serde_json::Value, serde_json::Value>(
            &layout,
            false,
            &extracted,
            &wheel.filename,
            Some(&direct_url),
            None,
            None,
            Some("uv"),
            true,
            LinkMode::Copy,
            &install_state,
        )
        .with_context(|| format!("installing local wheel {}", wheel.path.display()))?;
    }
    Ok(())
}
