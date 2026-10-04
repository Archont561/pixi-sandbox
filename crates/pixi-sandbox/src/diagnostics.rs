//! Durable, credential-safe diagnostics for the connected-side transport pipeline.
//!
//! GitHub's Actions log CDN is not reachable from every airlocked deployment. These events are
//! therefore deliberately plain text, flushed after every line, and useful without a tracing
//! collector. Callers pass reviewed fields instead of raw argv/environment dumps so credentials
//! never become diagnostic data by accident.

use crate::cli::DiagnosticsArgs;
use anyhow::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct State {
    command: &'static str,
    verbose: u8,
    file: Option<BufWriter<File>>,
    write_failure: Option<String>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

pub struct Session {
    enabled: bool,
}

impl Session {
    pub fn start(command: &'static str, options: &DiagnosticsArgs, context: &str) -> Result<Self> {
        let file = match &options.log_file {
            Some(path) => {
                if path.exists() {
                    bail!(
                        "refusing to overwrite diagnostic log {} — choose a new --log-file so existing evidence is preserved",
                        path.display()
                    );
                }
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .with_context(|| format!("creating diagnostic log {}", path.display()))?;
                Some(BufWriter::new(file))
            }
            None => None,
        };
        let enabled = options.verbose > 0 || file.is_some();
        if !enabled {
            return Ok(Self { enabled: false });
        }

        {
            let mut current = state().lock().expect("diagnostic state lock");
            current.command = command;
            current.verbose = options.verbose;
            current.file = file;
            current.write_failure = None;
        }
        write_event("event=start", Some(context))?;
        Ok(Self { enabled: true })
    }

    pub fn finish_success(self) -> Result<()> {
        if self.enabled {
            write_event("result=success", None)?;
            ensure_no_write_failure()?;
        }
        Ok(())
    }

    pub fn finish_failure(self, error: &anyhow::Error) -> Result<()> {
        if self.enabled {
            write_event("result=failure", None)?;
            write_event("error=begin", Some(&format!("{error:?}")))?;
            write_event("error=end", None)?;
            ensure_no_write_failure()?;
        }
        Ok(())
    }
}

/// Record a stable phase boundary. `detail` is reserved for reviewed, non-secret values such as
/// local paths and helper names; never pass argv, environment dumps, or remote URLs.
pub fn phase(name: &str, detail: impl AsRef<str>) {
    if let Err(error) = write_event(&format!("phase={name}"), Some(detail.as_ref())) {
        state().lock().expect("diagnostic state lock").write_failure = Some(format!("{error:#}"));
    }
}

fn ensure_no_write_failure() -> Result<()> {
    if let Some(error) = state()
        .lock()
        .expect("diagnostic state lock")
        .write_failure
        .take()
    {
        bail!("the diagnostic log became unwritable: {error}");
    }
    Ok(())
}

fn write_event(event: &str, detail: Option<&str>) -> Result<()> {
    let mut current = state().lock().expect("diagnostic state lock");
    let detail = detail.map(redact);
    let mut line = format!("command={} {event}", current.command);
    if let Some(detail) = &detail {
        if !detail.is_empty() {
            line.push(' ');
            line.push_str(detail);
        }
    }

    if let Some(file) = &mut current.file {
        writeln!(file, "{line}").context("writing diagnostic log")?;
        file.flush().context("flushing diagnostic log")?;
    }
    if current.verbose > 0 {
        if current.verbose > 1 {
            eprintln!("[diagnostic] {line}");
        } else {
            eprintln!("[diagnostic] command={} {event}", current.command);
        }
    }
    Ok(())
}

/// Remove credentials from URLs and common secret-valued query fields if a nested process error
/// includes them despite the reviewed event API. This is a final safety net, not permission to
/// log raw command lines.
fn redact(input: &str) -> String {
    let mut text = input.to_string();
    let mut search_from = 0;
    while let Some(relative) = text[search_from..].find("://") {
        let scheme_end = search_from + relative + 3;
        let authority_end = text[scheme_end..]
            .find(|character: char| character == '/' || character.is_whitespace())
            .map_or(text.len(), |end| scheme_end + end);
        if let Some(at) = text[scheme_end..authority_end].rfind('@') {
            let at = scheme_end + at;
            text.replace_range(scheme_end..at, "<redacted>");
            search_from = scheme_end + "<redacted>@".len();
        } else {
            search_from = authority_end;
        }
    }

    for key in ["token=", "access_token=", "password=", "authorization="] {
        let mut start = 0;
        while let Some(relative) = text[start..].to_ascii_lowercase().find(key) {
            let value_start = start + relative + key.len();
            let value_end = text[value_start..]
                .find(|character: char| character == '&' || character.is_whitespace())
                .map_or(text.len(), |end| value_start + end);
            text.replace_range(value_start..value_end, "<redacted>");
            start = value_start + "<redacted>".len();
        }
    }
    text
}
