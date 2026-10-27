//! Load the same private environment for evaluation and generation-free authentication diagnostics.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use supa_domain::{Error, Result};
use supa_runtime::evaluation::ENVIRONMENT;

fn diagnostics(shell: Option<&str>, file: Option<&str>, loaded: bool, default: bool) -> Value {
    json!({
        "environment_file_loaded":loaded,"default_environment_file_selected":default,
        "shell_key_present":shell.is_some(),"file_key_present":file.is_some(),
        "shell_key_overrides_file":shell.is_some() && file.is_some(),
        "shell_key_matches_file":shell.zip(file).map(|(a,b)| a==b),
        "key_source":if shell.is_some(){"shell"}else if file.is_some(){"environment_file"}else{"missing"},
        "key_format":supa_providers::key_diagnostics(shell.or(file)),
        "organization_project_selection":"credential_scope; no explicit headers",
        "ignored_connection_environment_present":(["OPENAI_BASE_URL","OPENAI_API_BASE","OPENAI_ORG_ID","OPENAI_ORGANIZATION","OPENAI_PROJECT","OPENAI_PROJECT_ID"].into_iter().filter(|k|std::env::var_os(k).is_some()).collect::<Vec<_>>())
    })
}
/// Must run before Tokio threads exist. Never print or return secret configuration values.
pub fn load_local_environment() -> Result<Value> {
    let shell = std::env::var("OPENAI_API_KEY").ok();
    let override_path = std::env::var_os("SUPA_EVAL_ENV_FILE");
    if override_path.as_ref().is_some_and(|p| p.is_empty()) {
        return Ok(diagnostics(shell.as_deref(), None, false, false));
    }
    let explicit = override_path.is_some();
    let default_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env.eval");
    let path = override_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| default_path.clone());
    let default = path == default_path;
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(e) if !explicit && e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(diagnostics(shell.as_deref(), None, false, default));
        }
        Err(_) => return Err(Error::Invalid),
    };
    if !metadata.file_type().is_file() || metadata.len() > 64 * 1024 {
        return Err(Error::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::Denied);
        }
    }
    let contents = std::fs::read(path).map_err(|_| Error::Invalid)?;
    let mut keys = BTreeSet::new();
    let mut parsed = BTreeMap::new();
    for entry in dotenvy::from_read_iter(std::io::Cursor::new(&contents)) {
        let (key, value) = entry.map_err(|_| Error::Invalid)?;
        if (!ENVIRONMENT.contains(&key.as_str()) && key != "SUPA_EVAL_OUTPUT")
            || !keys.insert(key.clone())
        {
            return Err(Error::Invalid);
        }
        parsed.insert(key, value);
    }
    let report = diagnostics(
        shell.as_deref(),
        parsed.get("OPENAI_API_KEY").map(String::as_str),
        true,
        default,
    );
    dotenvy::from_read(std::io::Cursor::new(contents)).map_err(|_| Error::Invalid)?;
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credential_precedence_is_visible_without_values() {
        for (shell, file, source, overrides, matches) in [
            (None, Some("sk-file-fake"), "environment_file", false, None),
            (
                Some("sk-shell-fake"),
                Some("sk-file-fake"),
                "shell",
                true,
                Some(false),
            ),
            (
                Some("sk-file-fake"),
                Some("sk-file-fake"),
                "shell",
                true,
                Some(true),
            ),
            (Some(""), Some("sk-file-fake"), "shell", true, Some(false)),
            (None, None, "missing", false, None),
        ] {
            let report = diagnostics(shell, file, true, true);
            assert_eq!(report["key_source"], source);
            assert_eq!(report["shell_key_overrides_file"], overrides);
            assert_eq!(report["shell_key_matches_file"], json!(matches));
            assert!(!report.to_string().contains("sk-"));
        }
    }
}
