//! Declarative publish planning for `.pixi-sandbox.toml`.
//!
//! A project must opt into the environments it publishes. Inferring "all environments" from
//! `pixi.toml` is unsafe: projects commonly keep experimental, CI-only, or platform-specific
//! environments there. This small schema turns a reviewed bundle list into a stable GitHub
//! Actions matrix without coupling the transport format to a CI provider.

use crate::error::{Error, Result};
use crate::platform::Platform;
use crate::tools_lock::ToolsLock;
use crate::transport_budget::{
    DEFAULT_MAX_BLOB_MIB, DEFAULT_MAX_REPOSITORY_PUSH_MIB, DEFAULT_MAX_RESTORE_REQUIRED_MIB,
    DEFAULT_MAX_TRANSPORT_MIB, TransportBudgets, mib_to_bytes,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::str::FromStr;

/// Bumped only for incompatible changes; readers refuse anything newer but keep accepting every
/// older schema they understand for a deprecation window (task-47 AC#5 / decision D16), mirroring
/// `manifest::SCHEMA_VERSION`'s policy exactly. Config is reviewed data and is never rewritten
/// automatically to a newer schema — unlike a transport manifest, which this build itself wrote,
/// a sandbox config is maintained by the consumer, so bumping it is their edit to make. No
/// `config migrate` command exists while this constant is still `1`: there is only one schema
/// shape to migrate *from*, so there is nothing for it to do yet.
pub const CONFIG_SCHEMA: u32 = 1;

/// True when this build can act on a config's schema (see [`CONFIG_SCHEMA`]).
#[must_use]
pub fn schema_supported(schema: u32) -> bool {
    (1..=CONFIG_SCHEMA).contains(&schema)
}

/// Read only the `schema` field, tolerating any other shape the rest of this file might have.
///
/// `SandboxConfig::load` enforces `deny_unknown_fields` against *today's* field set, which is
/// correct for every caller that needs the config's contents but wrong for a diagnostic that
/// must still say something useful about a schema this build cannot otherwise parse at all —
/// `init --check` uses this to turn a schema mismatch into a named finding instead of an opaque
/// parse failure (task-47 AC#5).
///
/// # Errors
/// Returns an error if the file cannot be read or contains no parseable `schema` key, which
/// means the file is not a sandbox config at all rather than merely an old or new one.
pub fn peek_schema(path: &Path) -> Result<u32> {
    #[derive(Deserialize)]
    struct SchemaOnly {
        schema: u32,
    }
    let text = std::fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let peek: SchemaOnly = toml::from_str(&text).map_err(|error| Error::InvalidManifest {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    Ok(peek.schema)
}

/// Default project-level declaration consumed by `pixi-sandbox plan`.
pub const DEFAULT_FILE: &str = ".pixi-sandbox.toml";

/// A checked-in description of sandbox bundles and their native runners.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    pub schema: u32,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
    #[serde(default = "default_cargo_vendor")]
    pub cargo_vendor: bool,
    /// Optional platform → single GitHub runner label overrides. The defaults cover the four
    /// platforms GitHub hosts for every plan; a platform with no safe hosted default
    /// (`linux-aarch64`, for one) must be named explicitly, so a typo cannot dispatch a job to a
    /// runner that does not exist.
    #[serde(default)]
    pub runners: BTreeMap<String, String>,
    #[serde(default, rename = "bundle")]
    pub bundles: Vec<Bundle>,
    /// Reviewed size thresholds for the generated publisher's pre-publish doctor gate.
    #[serde(default)]
    pub budgets: BudgetPolicy,
    /// Optional generated-publisher CI policy (issue #79, task-53). See [`WorkflowPolicy`].
    #[serde(default)]
    pub workflow: Option<WorkflowPolicy>,
}

/// One environment set to publish for one or more native platforms.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// Stable branch-name component, e.g. `developer`.
    pub name: String,
    /// Explicit Pixi environments. Their order is retained in the pack invocation and branch
    /// identity, so review changes carefully.
    pub environments: Vec<String>,
    /// Pixi platform names, e.g. `linux-64`, `osx-arm64`, `win-64`.
    pub platforms: Vec<String>,
    /// Override the top-level cargo-vendor policy for this bundle.
    #[serde(default)]
    pub cargo_vendor: Option<bool>,
}

/// `[budgets]`: hard size thresholds enforced by the generated publisher before it pushes a
/// transport. Missing fields keep the reviewed defaults; consumers may lower them for tighter
/// repositories, but `max_blob_mib` cannot be raised above the packer's GitHub-safe shard limit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetPolicy {
    #[serde(default = "default_max_blob_mib")]
    pub max_blob_mib: f64,
    #[serde(default = "default_max_transport_mib")]
    pub max_transport_mib: f64,
    #[serde(default = "default_max_repository_push_mib")]
    pub max_repository_push_mib: f64,
    #[serde(default = "default_max_restore_required_mib")]
    pub max_restore_required_mib: f64,
}

impl Default for BudgetPolicy {
    fn default() -> Self {
        Self {
            max_blob_mib: DEFAULT_MAX_BLOB_MIB,
            max_transport_mib: DEFAULT_MAX_TRANSPORT_MIB,
            max_repository_push_mib: DEFAULT_MAX_REPOSITORY_PUSH_MIB,
            max_restore_required_mib: DEFAULT_MAX_RESTORE_REQUIRED_MIB,
        }
    }
}

impl BudgetPolicy {
    /// Convert reviewed MiB thresholds to byte ceilings used by `doctor`.
    pub fn to_transport_budgets(&self) -> Result<TransportBudgets> {
        validate_positive_mib("budgets.max_blob_mib", self.max_blob_mib)?;
        if self.max_blob_mib > DEFAULT_MAX_BLOB_MIB {
            return Err(Error::Invalid(format!(
                "budgets.max_blob_mib must be at most {DEFAULT_MAX_BLOB_MIB} MiB, got {}; \
                 larger Git blobs are rejected upstream, so split before publish instead",
                self.max_blob_mib
            )));
        }
        validate_positive_mib("budgets.max_transport_mib", self.max_transport_mib)?;
        validate_positive_mib(
            "budgets.max_repository_push_mib",
            self.max_repository_push_mib,
        )?;
        validate_positive_mib(
            "budgets.max_restore_required_mib",
            self.max_restore_required_mib,
        )?;
        Ok(TransportBudgets {
            max_blob_bytes: mib_to_bytes(self.max_blob_mib),
            max_transport_bytes: mib_to_bytes(self.max_transport_mib),
            max_repository_push_bytes: mib_to_bytes(self.max_repository_push_mib),
            max_restore_required_bytes: mib_to_bytes(self.max_restore_required_mib),
        })
    }
}

/// Optional consumer-owned CI policy for the generated publisher (issue #79, task-53).
///
/// The whole table is absent by default; a config with no `[workflow]` section at all renders
/// byte-identically to the pre-task-53 template (task-53 AC#1) — that property is the migration
/// path for every existing consumer. Once the table is *present* (even empty), two fields whose
/// current hardcoded behaviour the issue treats as a correctness or cost hazard rather than a
/// mere preference — `push_paths` (an unfiltered trigger repacks and force-pushes on every push,
/// including a README typo) and `setup_pixi_cache` (a cache keyed on the consumer manifest can
/// restore a solve pixi-sandbox's own native `pixi install` will not reuse) — move to their
/// safer default unless given an explicit value (decision D18). The remaining three fields
/// (`permissions`, `concurrency`, `timeouts`) carry no such defect in the status quo, so they
/// stay fully per-field opt-in with no default change from the table's mere presence.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPolicy {
    /// Explicit `on.push.paths:` allowlist. Unset, the publisher derives one from the known
    /// transport inputs instead (task-53 AC#3; see [`derive_push_paths`]).
    #[serde(default)]
    pub push_paths: Option<Vec<String>>,
    /// Scope `permissions:` to least privilege: `contents: read` top-level, `contents: write`
    /// only on the `publish` job (default `false`, today's unconditional top-level
    /// `contents: write`, unchanged by the table's mere presence).
    #[serde(default)]
    pub permissions: bool,
    /// `concurrency:` group and cancel policy for the whole workflow.
    #[serde(default)]
    pub concurrency: Option<ConcurrencyPolicy>,
    /// `timeout-minutes:` for the `plan` and `publish` jobs.
    #[serde(default)]
    pub timeouts: Option<TimeoutsPolicy>,
    /// Pin `prefix-dev/setup-pixi`'s own `pixi-version:` input. Unset keeps that step's
    /// existing behaviour (no pin, the action's own resolution).
    #[serde(default)]
    pub pixi_version: Option<String>,
    /// `setup-pixi`'s `cache:` input. Defaults to `false` once `[workflow]` is present at all
    /// (decision D18); a consumer who wants the old best-effort caching back sets
    /// `setup_pixi_cache = true` explicitly.
    #[serde(default)]
    pub setup_pixi_cache: Option<bool>,
}

/// `[workflow.concurrency]`: see [`WorkflowPolicy::concurrency`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcurrencyPolicy {
    pub group: String,
    #[serde(default)]
    pub cancel_in_progress: bool,
}

/// `[workflow.timeouts]`: see [`WorkflowPolicy::timeouts`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutsPolicy {
    #[serde(default)]
    pub plan: Option<u32>,
    #[serde(default)]
    pub publish: Option<u32>,
}

/// JSON shape emitted by `pixi-sandbox plan --json`.
///
/// Consumers must build their matrix from `.include` alone, never from this
/// object wholesale. Actions reads every top-level key other than
/// `include`/`exclude` as a matrix dimension and requires an array, so the
/// scalar `schema` makes `matrix: ${{ fromJSON(plan) }}` expand to zero jobs.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PublishPlan {
    pub schema: u32,
    pub include: Vec<PublishTarget>,
    /// The resolved `on.push.paths:` allowlist (explicit override or derivation; empty when no
    /// `[workflow]` table is configured at all) — surfaced so a consumer can diff what `init`
    /// would emit against what they expect (task-53 AC#3). `fromJSON(plan).include` is the only
    /// key the generated workflow ever reads from this object; an added array key is therefore
    /// safe the same way `schema` already is (see this struct's own doc comment above).
    #[serde(default)]
    pub push_paths: Vec<String>,
}

/// One native job: it creates exactly one orphan branch.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PublishTarget {
    pub bundle: String,
    /// Comma-separated specifically so the output can flow straight into `--envs`.
    pub environments: String,
    pub platform: String,
    pub runner: String,
    pub branch: String,
    pub cargo_vendor: bool,
}

/// Bundle name used for a one-target override plan, matching the historical workflow label.
pub const OVERRIDE_BUNDLE: &str = "custom";

/// A one-target plan for a manual dispatch, built from the *same* runner map and the same
/// validation as the reviewed config path.
///
/// This exists so the platform → runner table lives in exactly one place. The workflow used to
/// repeat it as an inline `case` statement in each of its two matrix steps, which meant a
/// platform could be mapped one way in `.pixi-sandbox.toml` planning and another way the moment
/// someone dispatched by hand — the precise drift D11 exists to prevent. Composing a synthetic
/// config and running the normal [`SandboxConfig::plan`] means an override cannot skip the
/// runner-label check or the embedded helper-coverage check.
pub fn plan_override(
    bundle: &str,
    environments: &[String],
    platform: &str,
    branch_prefix: &str,
    cargo_vendor: bool,
) -> Result<PublishPlan> {
    SandboxConfig {
        schema: CONFIG_SCHEMA,
        branch_prefix: branch_prefix.to_string(),
        cargo_vendor,
        runners: BTreeMap::new(),
        bundles: vec![Bundle {
            name: bundle.to_string(),
            environments: environments.to_vec(),
            platforms: vec![platform.to_string()],
            cargo_vendor: None,
        }],
        budgets: BudgetPolicy::default(),
        workflow: None,
    }
    .plan()
}

/// Derive an `on.push.paths:` allowlist from the transport inputs a publish actually reads
/// (task-53 AC#3): the sandbox config itself, pixi's own manifest and lock (always present —
/// every bundle is a Pixi environment), and the vendor-specific manifests a bundle's
/// environments might add, included only when `repo_root` actually has one so an unrelated
/// bundle's publisher does not gain an irrelevant trigger path.
#[must_use]
pub fn derive_push_paths(repo_root: &Path, config_display: &str) -> Vec<String> {
    let mut paths = vec![
        config_display.to_string(),
        "pixi.toml".to_string(),
        "pixi.lock".to_string(),
    ];
    for candidate in ["package.json", "bun.lock", "Cargo.toml", "Cargo.lock"] {
        if repo_root.join(candidate).is_file() {
            paths.push(candidate.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

impl SandboxConfig {
    /// Read and validate a project declaration.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
        let config: SandboxConfig =
            toml::from_str(&text).map_err(|error| Error::InvalidManifest {
                path: path.display().to_string(),
                reason: error.to_string(),
            })?;
        config.validate()?;
        Ok(config)
    }

    /// Produce a stable one-entry-per-(bundle × platform) publish plan.
    pub fn plan(&self) -> Result<PublishPlan> {
        self.validate()?;
        // The release-driven publisher has no project-local tool lock input: it deliberately
        // fetches only the reviewed catalogue compiled into its verified binary. Fail before a
        // matrix job starts if a configured platform would later fail to fetch pixi, pack, or
        // unpack helpers.
        let embedded_tools = ToolsLock::embedded()?;
        let mut include = Vec::new();
        for bundle in &self.bundles {
            let environments = bundle.environments.join(",");
            for platform in &bundle.platforms {
                ensure_embedded_tools_cover(&embedded_tools, platform)?;
                let runner = self.runner_for(platform)?;
                include.push(PublishTarget {
                    bundle: bundle.name.clone(),
                    environments: environments.clone(),
                    platform: platform.clone(),
                    runner,
                    branch: format!("{}/{}-{}", self.branch_prefix, bundle.name, platform),
                    cargo_vendor: bundle.cargo_vendor.unwrap_or(self.cargo_vendor),
                });
            }
        }
        include.sort_by(|left, right| left.branch.cmp(&right.branch));
        Ok(PublishPlan {
            schema: CONFIG_SCHEMA,
            include,
            // Resolving push_paths needs a repo root to check which optional vendor manifests
            // exist, which this method deliberately does not take (every other call site stays
            // unaffected by task-53): callers that care call `resolved_push_paths` themselves
            // and assign it in, the same way `commands::plan` does for `plan --json`.
            push_paths: Vec::new(),
        })
    }

    /// Resolve the `on.push.paths:` allowlist: an explicit `[workflow] push_paths` always wins;
    /// otherwise, once `[workflow]` is present at all, derive one from the transport inputs
    /// `plan` already depends on (task-53 AC#3); absent no `[workflow]` table, no filter at all
    /// (empty), matching the unconditional pre-task-53 trigger exactly.
    #[must_use]
    pub fn resolved_push_paths(&self, repo_root: &Path, config_display: &str) -> Vec<String> {
        match &self.workflow {
            None => Vec::new(),
            Some(policy) => policy
                .push_paths
                .clone()
                .unwrap_or_else(|| derive_push_paths(repo_root, config_display)),
        }
    }

    fn validate(&self) -> Result<()> {
        // A reviewed config outlives the CLI version that last touched it, the same way a
        // published transport manifest outlives the binary that packed it: an older schema is
        // still accepted (there is only ever one to accept today), and only a newer one — which
        // could mean anything this build does not understand — is refused.
        if !schema_supported(self.schema) {
            return Err(Error::Invalid(format!(
                "sandbox config schema {} is not supported by this build (understands 1..={CONFIG_SCHEMA})",
                self.schema
            )));
        }
        validate_branch_prefix(&self.branch_prefix)?;
        for (platform, runner) in &self.runners {
            validate_platform(platform)?;
            validate_runner_label(platform, runner)?;
        }
        self.budgets.to_transport_budgets()?;
        if let Some(workflow) = &self.workflow {
            validate_workflow_policy(workflow)?;
        }
        if self.bundles.is_empty() {
            return Err(Error::Invalid(
                "sandbox config needs at least one [[bundle]]".to_string(),
            ));
        }

        let mut names = BTreeSet::new();
        let mut branches = BTreeSet::new();
        let mut requested_platforms = BTreeSet::<&str>::new();
        for bundle in &self.bundles {
            validate_slug("bundle name", &bundle.name)?;
            if !names.insert(&bundle.name) {
                return Err(Error::Invalid(format!(
                    "bundle {:?} is declared more than once",
                    bundle.name
                )));
            }
            if bundle.environments.is_empty() {
                return Err(Error::Invalid(format!(
                    "bundle {:?} has no environments",
                    bundle.name
                )));
            }
            if bundle.platforms.is_empty() {
                return Err(Error::Invalid(format!(
                    "bundle {:?} has no platforms",
                    bundle.name
                )));
            }

            let mut environments = BTreeSet::new();
            for environment in &bundle.environments {
                validate_slug("environment", environment)?;
                if !environments.insert(environment) {
                    return Err(Error::Invalid(format!(
                        "bundle {:?} requests environment {:?} more than once",
                        bundle.name, environment
                    )));
                }
            }

            let mut platforms = BTreeSet::new();
            for platform in &bundle.platforms {
                validate_platform(platform)?;
                if !platforms.insert(platform) {
                    return Err(Error::Invalid(format!(
                        "bundle {:?} requests platform {:?} more than once",
                        bundle.name, platform
                    )));
                }
                requested_platforms.insert(platform.as_str());
                let branch = format!("{}/{}-{platform}", self.branch_prefix, bundle.name);
                if !branches.insert(branch.clone()) {
                    return Err(Error::Invalid(format!(
                        "multiple bundles would publish the same branch {branch:?}"
                    )));
                }
                let _ = self.runner_for(platform)?;
            }
        }
        for platform in self.runners.keys() {
            if !requested_platforms.contains(platform.as_str()) {
                return Err(Error::Invalid(format!(
                    "runner override for {platform:?} is unused; remove it or declare that platform in a bundle"
                )));
            }
        }
        Ok(())
    }

    fn runner_for(&self, platform: &str) -> Result<String> {
        if let Some(runner) = self.runners.get(platform) {
            if runner.trim().is_empty() {
                return Err(Error::Invalid(format!(
                    "runner override for {platform:?} is empty"
                )));
            }
            return Ok(runner.clone());
        }
        // Delegates to `Platform::gh_runner` (task-55/task-60) instead of its own match; a
        // platform string this project does not recognise at all falls through the same
        // "no safe default" error as one it recognises but has no hosted runner for
        // (`linux-aarch64` today).
        let default = Platform::from_str(platform)
            .ok()
            .and_then(Platform::gh_runner);
        default.map(str::to_string).ok_or_else(|| {
            Error::Invalid(format!(
                "platform {platform:?} needs runners.{platform:?}; no safe default runner is known"
            ))
        })
    }
}

/// A planned release job always uses compiled pins: allowing a target whose helper releases are
/// absent would turn a reviewed plan into a slow, avoidable failure on a native runner.
fn ensure_embedded_tools_cover(lock: &ToolsLock, platform: &str) -> Result<()> {
    let missing = ["pixi", "pixi-pack", "pixi-unpack"]
        .into_iter()
        .filter(|tool| lock.pin(tool, platform).is_none())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "platform {platform:?} has no complete embedded helper-tool pins (missing {}); \
         add reviewed pins and release a new pixi-sandbox binary before publishing it",
        missing.join(", ")
    )))
}

fn default_branch_prefix() -> String {
    "sandbox".to_string()
}

const fn default_cargo_vendor() -> bool {
    true
}

const fn default_max_blob_mib() -> f64 {
    DEFAULT_MAX_BLOB_MIB
}

const fn default_max_transport_mib() -> f64 {
    DEFAULT_MAX_TRANSPORT_MIB
}

const fn default_max_repository_push_mib() -> f64 {
    DEFAULT_MAX_REPOSITORY_PUSH_MIB
}

const fn default_max_restore_required_mib() -> f64 {
    DEFAULT_MAX_RESTORE_REQUIRED_MIB
}

fn validate_positive_mib(field: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "{field} must be a positive finite MiB value, got {value}"
        )))
    }
}

fn validate_slug(kind: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Error::Invalid(format!(
            "{kind} {value:?} must use only lowercase ASCII letters, digits, and hyphens"
        )));
    }
    Ok(())
}

fn validate_platform(value: &str) -> Result<()> {
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || value.is_empty()
    {
        return Err(Error::Invalid(format!(
            "platform {value:?} must use only lowercase ASCII letters, digits, and hyphens"
        )));
    }
    Ok(())
}

/// At most a generous sanity bound, not GitHub's own ceiling (which varies by plan): a
/// misconfigured `0` or an implausible multi-day value is far more likely to be a typo than an
/// intentional policy, and failing fast beats a silently useless `timeout-minutes:`.
const MAX_TIMEOUT_MINUTES: u32 = 1440;

fn validate_workflow_policy(workflow: &WorkflowPolicy) -> Result<()> {
    if let Some(paths) = &workflow.push_paths {
        if paths.is_empty() {
            return Err(Error::Invalid(
                "workflow.push_paths must not be empty when present; omit the key to derive it \
                 or to keep the unfiltered trigger"
                    .to_string(),
            ));
        }
        for path in paths {
            validate_workflow_path(path)?;
        }
    }
    if let Some(concurrency) = &workflow.concurrency {
        if concurrency.group.trim().is_empty()
            || concurrency.group != concurrency.group.trim()
            || concurrency.group.chars().any(char::is_control)
        {
            return Err(Error::Invalid(
                "workflow.concurrency.group must be a non-empty label without leading/trailing \
                 whitespace or control characters"
                    .to_string(),
            ));
        }
    }
    if let Some(timeouts) = &workflow.timeouts {
        for (field, value) in [("plan", timeouts.plan), ("publish", timeouts.publish)] {
            if let Some(minutes) = value {
                if minutes == 0 || minutes > MAX_TIMEOUT_MINUTES {
                    return Err(Error::Invalid(format!(
                        "workflow.timeouts.{field} must be between 1 and {MAX_TIMEOUT_MINUTES} \
                         minutes, got {minutes}"
                    )));
                }
            }
        }
    }
    if let Some(version) = &workflow.pixi_version {
        if version.trim().is_empty()
            || version != version.trim()
            || version.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(Error::Invalid(
                "workflow.pixi_version must be a non-empty version with no whitespace or \
                 control characters"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

/// A `paths:` entry is interpolated straight into the committed workflow's YAML; forbid the
/// characters that would make that interpolation ambiguous or let an entry escape a plain
/// relative path. A leading `!` is allowed — GitHub Actions' own negation syntax for `paths:`.
fn validate_workflow_path(value: &str) -> Result<()> {
    let bare = value.strip_prefix('!').unwrap_or(value);
    if bare.is_empty()
        || value.chars().any(|c| c.is_control())
        || bare.starts_with('/')
        || bare.contains("..")
    {
        return Err(Error::Invalid(format!(
            "workflow.push_paths entry {value:?} must be a non-empty relative path with no \
             control characters, leading '/', or '..' component"
        )));
    }
    Ok(())
}

fn validate_runner_label(platform: &str, runner: &str) -> Result<()> {
    if runner.trim().is_empty() || runner != runner.trim() || runner.chars().any(char::is_control) {
        return Err(Error::Invalid(format!(
            "runner override for {platform:?} must be a non-empty label without leading/trailing whitespace or control characters"
        )));
    }
    Ok(())
}

/// Return whether `value` is safe to interpolate as a complete Git branch name.
///
/// `scripts/restore.sh` owns a Bash spelling of this predicate because the airlock cannot
/// depend on this binary before it has restored it. Keep the two implementations in lockstep:
/// the property test in `tests/restore_script.rs` generates printable branch names and checks
/// their answers agree. This deliberately follows Git's ref-name restrictions plus a leading
/// dash guard, because a branch is passed to command-line plumbing as well as to ref parsing.
pub fn is_safe_git_ref(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains("..")
        || value.contains("@{")
        || value.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte.is_ascii_whitespace()
                || matches!(byte, b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
        })
    {
        return false;
    }

    value.split('/').all(|component| {
        !component.is_empty()
            && component != "."
            && component != ".."
            && !component.starts_with('.')
            && !component.ends_with('.')
            && !component.ends_with(".lock")
    })
}

fn validate_branch_prefix(value: &str) -> Result<()> {
    if is_safe_git_ref(value) {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "branch_prefix {value:?} is not a safe git-ref prefix"
        )))
    }
}
