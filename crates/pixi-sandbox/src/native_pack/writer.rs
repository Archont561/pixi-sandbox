//! Assemble the format-1 directory from a locked environment; never solve or execute code.
use anyhow::{Context, Result, bail};
use rattler_conda_types::{PackageRecord, package::CondaArchiveIdentifier};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io, path::Path};
use url::Url;

/// Every locked-artifact spelling the writer accepts, in the wording every refusal repeats.
const ACCEPTED_LOCATIONS: &str =
    "an https:// or http:// URL, a file:// URL, or an absolute local path";

/// Resolve one locked `conda:`/`pypi:` location to a URL the rest of the writer can use.
///
/// Lock v7 does not carry a URL for every artifact. Pixi writes a **bare absolute path** when
/// the channel is a local directory or a `file://` URL (issue #157's neighbouring shape, reported
/// as TASK-132), so a scheme test alone refuses a lock pixi itself produced. A location is a URL
/// only when it starts with one of the schemes below; anything else must be an absolute path,
/// which also keeps a Windows drive letter (`C:\…`) out of URL parsing, where `c` would look
/// like a scheme.
///
/// # Errors
/// Returns an error naming the offending entry for an unsupported scheme, a relative path, or a
/// path that cannot be expressed as a file URL.
fn artifact_location(location: &str) -> Result<Url> {
    let scheme = location
        .split_once("://")
        .map(|(scheme, _)| scheme.to_ascii_lowercase());
    match scheme.as_deref() {
        Some("https" | "http" | "file") => Url::parse(location)
            .with_context(|| format!("locked artifact {location:?} is not a usable URL")),
        Some(other) => bail!(
            "unsupported locked artifact scheme {other:?} in {location:?}; expected {ACCEPTED_LOCATIONS}"
        ),
        None => {
            let path = Path::new(location);
            if !path.is_absolute() {
                bail!(
                    "locked artifact {location:?} is neither a URL nor an absolute path; expected {ACCEPTED_LOCATIONS}"
                );
            }
            Url::from_file_path(path).map_err(|()| {
                anyhow::anyhow!("locked artifact {location:?} is not a usable local file path")
            })
        }
    }
}

struct Artifact {
    location: Url,
    filename: String,
    digest: String,
    record: Value,
    subdir: Option<String>,
}

/// Validate lock selections and artifact identities before the caller creates output.
///
/// # Errors
/// Returns an error for unsupported locks, missing selections, hashes or unsafe identities.
pub fn preflight(project: &Path, environment: &str, platform: &str) -> Result<()> {
    let lock: Value = serde_yaml::from_slice(&fs::read(project.join("pixi.lock"))?)
        .context("reading pixi.lock")?;
    select(&lock, environment, platform).map(|_| ())
}

/// Assemble selected locked packages into a fresh local channel and wheel directory.
///
/// # Errors
/// Refuses unsupported locks, missing selections, non-wheel artifacts, unsafe identities,
/// missing hashes, existing output, download failures, or digest mismatches.
pub fn write(project: &Path, environment: &str, platform: &str, target: &Path) -> Result<()> {
    let lock: Value = serde_yaml::from_slice(&fs::read(project.join("pixi.lock"))?)
        .context("reading pixi.lock")?;
    let artifacts = select(&lock, environment, platform)?;
    if target.exists() {
        bail!("native pack output already exists: {}", target.display());
    }
    fs::create_dir_all(target)?;
    let mut channels: BTreeMap<String, Value> = BTreeMap::new();
    let mut dependencies = Vec::new();
    for artifact in artifacts {
        let directory = if let Some(subdir) = &artifact.subdir {
            target.join("channel").join(subdir)
        } else {
            target.join("pypi")
        };
        fs::create_dir_all(&directory)?;
        let destination = directory.join(&artifact.filename);
        acquire(&artifact.location, &destination)?;
        if pixi_sandbox_core::shard::sha256_file(&destination)? != artifact.digest {
            fs::remove_file(&destination)?;
            bail!("SHA256 mismatch for locked artifact {}", artifact.filename);
        }
        if let Some(subdir) = artifact.subdir {
            let section = if Path::new(&artifact.filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("conda"))
            {
                "packages.conda"
            } else {
                "packages"
            };
            let data = channels.entry(subdir.clone()).or_insert_with(|| json!({
                "info": {"subdir": subdir}, "packages": {}, "packages.conda": {}, "repodata_version": 2,
            }));
            let record: PackageRecord = serde_json::from_value(artifact.record)?;
            dependencies.push(format!(
                "{}={}={}",
                record.name.as_normalized(),
                record.version,
                record.build
            ));
            data[section][&artifact.filename] = serde_json::to_value(record)?;
        }
    }
    // An empty environment still has a readable local channel.
    channels.entry("noarch".into()).or_insert_with(|| json!({
        "info": {"subdir": "noarch"}, "packages": {}, "packages.conda": {}, "repodata_version": 2,
    }));
    for (subdir, data) in channels {
        let directory = target.join("channel").join(subdir);
        fs::create_dir_all(&directory)?;
        fs::write(
            directory.join("repodata.json"),
            serde_json::to_vec_pretty(&data)?,
        )?;
    }
    fs::write(
        target.join("pixi-pack.json"),
        serde_json::to_vec_pretty(&json!({"version": "1", "platform": platform}))?,
    )?;
    fs::write(
        target.join("environment.yml"),
        serde_yaml::to_string(&json!({
            "channels": ["./channel", "nodefaults"], "dependencies": dependencies,
        }))?,
    )?;
    Ok(())
}

fn select(lock: &Value, environment: &str, platform: &str) -> Result<Vec<Artifact>> {
    if lock["version"].as_u64() != Some(7) {
        bail!("unsupported pixi.lock version; native packing currently requires version 7");
    }
    let selected = lock["environments"][environment]["packages"][platform]
        .as_array()
        .with_context(|| {
            format!("pixi.lock has no environment {environment} for platform {platform}")
        })?;
    let records = lock["packages"]
        .as_array()
        .context("pixi.lock has no package records")?;
    let mut artifacts = Vec::new();
    let mut identities = std::collections::BTreeSet::new();
    for entry in selected {
        let (kind, location) = if let Some(location) = entry["conda"].as_str() {
            ("conda", location)
        } else {
            (
                "pypi",
                entry["pypi"]
                    .as_str()
                    .context("unknown locked package kind")?,
            )
        };
        let mut record = records
            .iter()
            .find(|record| record[kind].as_str() == Some(location))
            .with_context(|| format!("missing locked package record for {location}"))?
            .clone();
        let url = artifact_location(location)?;
        let filename = url
            .path_segments()
            .and_then(Iterator::last)
            .context("artifact URL has no filename")?
            .to_string();
        if filename.is_empty() || filename.starts_with('.') || filename.contains(['\\', ':']) {
            bail!("unsafe locked artifact filename: {filename}");
        }
        let digest = record["sha256"]
            .as_str()
            .context("locked artifact requires SHA256")?
            .to_string();
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("invalid locked artifact SHA256");
        }
        let subdir = if kind == "conda" {
            let package = conda_record(&mut record, &url, &filename)?;
            let subdir = package.subdir.clone();
            if subdir != "noarch" && subdir != platform {
                bail!(
                    "locked artifact {location:?} sits in a directory named {subdir:?}, which is neither \"noarch\" nor the requested platform {platform:?}; a locked archive must live under <channel>/<subdir>/{filename}"
                );
            }
            if !identities.insert(format!("conda:{}", package.name.as_normalized())) {
                bail!("duplicate locked Conda package");
            }
            Some(subdir)
        } else {
            let wheel: uv_distribution_filename::WheelFilename = filename
                .parse()
                .context("locked PyPI artifact must be a wheel")?;
            if !identities.insert(format!("pypi:{}", wheel.name)) {
                bail!("duplicate locked wheel package");
            }
            None
        };
        artifacts.push(Artifact {
            location: url,
            filename,
            digest,
            record: record.clone(),
            subdir,
        });
    }
    Ok(artifacts)
}

fn conda_record(record: &mut Value, url: &Url, filename: &str) -> Result<PackageRecord> {
    let identifier: CondaArchiveIdentifier = filename.parse().map_err(anyhow::Error::msg)?;
    let derived = &identifier.identifier;
    let url_subdir = url
        .path_segments()
        .and_then(|mut parts| parts.nth_back(1))
        .with_context(|| {
            format!(
                "locked Conda artifact {filename:?} has no channel subdir segment; expected <channel>/<subdir>/{filename}"
            )
        })?;
    let fields = record
        .as_object_mut()
        .context("invalid locked package record")?;
    for (key, value) in [
        ("name", &*derived.name),
        ("version", &*derived.version),
        ("build", &*derived.build_string),
        ("subdir", url_subdir),
    ] {
        fields.entry(key).or_insert_with(|| value.into());
    }
    // Lock v7 omits a build number when it equals the trailing decimal digits
    // of the build string (or zero if there are none). Preserve explicit values.
    // This is rattler_lock's derive_build_number_from_build rule, not an
    // assumption that all compact records have build number zero.
    let build = fields["build"]
        .as_str()
        .context("invalid locked Conda build")?;
    let trailing_digits = build
        .rsplit_once(|c: char| !c.is_ascii_digit())
        .map_or(build, |(_, digits)| digits);
    let build_number = trailing_digits.parse::<u64>().unwrap_or(0);
    fields
        .entry("build_number")
        .or_insert_with(|| build_number.into());
    // For noarch builds, v7 infers the kind from whether the build contains "py".
    if url_subdir == "noarch" {
        let noarch = if derived.build_string.contains("py") {
            "python" // stale-ref-allowed: Conda runtime noarch kind.
        } else {
            "generic"
        };
        fields.entry("noarch").or_insert_with(|| noarch.into());
    }
    let package: PackageRecord =
        serde_json::from_value(record.clone()).context("invalid locked Conda record")?;
    if identifier.identifier.to_string()
        != format!(
            "{}-{}-{}",
            package.name.as_normalized(),
            package.version,
            package.build
        )
    {
        bail!("locked package identity disagrees with archive filename");
    }
    Ok(package)
}

fn acquire(url: &Url, destination: &Path) -> Result<()> {
    if url.scheme() == "file" {
        let path = url
            .to_file_path()
            .map_err(|()| anyhow::anyhow!("invalid local artifact URL"))?;
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            bail!("local artifact must be a regular file");
        }
        fs::copy(path, destination)?;
    } else {
        let mut input = ureq::get(url.as_str()).call()?.into_body().into_reader();
        let mut output = fs::File::create(destination)?;
        io::copy(&mut input, &mut output)?;
    }
    Ok(())
}
