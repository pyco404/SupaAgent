//! Strict local Claude credentials. No shell evaluation, global environment changes or Debug output.
use std::{collections::BTreeMap, fs::File, io::Read, os::unix::fs::MetadataExt, path::Path};
use supa_domain::{Error, Result};
use zeroize::Zeroizing;

pub(crate) const KEYS: [&str; 3] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_WORKSPACE_ID",
    "ANTHROPIC_MODEL",
];
pub(crate) fn load(path: &Path) -> Result<BTreeMap<String, Zeroizing<String>>> {
    let before = std::fs::symlink_metadata(path).map_err(|_| Error::Invalid)?;
    let process = std::fs::metadata("/proc/self").map_err(|_| Error::Denied)?;
    if !before.is_file()
        || before.uid() != process.uid()
        || before.mode() & 0o077 != 0
        || before.nlink() != 1
        || before.len() > 8192
    {
        return Err(Error::Denied);
    }
    let file = File::open(path).map_err(|_| Error::Denied)?;
    let opened = file.metadata().map_err(|_| Error::Denied)?;
    if opened.dev() != before.dev() || opened.ino() != before.ino() {
        return Err(Error::Denied);
    }
    let mut text = Zeroizing::new(String::new());
    file.take(8193)
        .read_to_string(&mut text)
        .map_err(|_| Error::Invalid)?;
    if text.len() > 8192 {
        return Err(Error::Invalid);
    }
    let mut result = BTreeMap::new();
    for line in text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    {
        let (key, value) = line.split_once('=').ok_or(Error::Invalid)?;
        // The enrollment utility writes literal ASCII values, never interpolation or shell syntax.
        if !KEYS.contains(&key)
            || value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || result
                .insert(key.to_owned(), Zeroizing::new(value.to_owned()))
                .is_some()
        {
            return Err(Error::Invalid);
        }
    }
    Ok(result)
}
pub(crate) fn local() -> Result<BTreeMap<String, Zeroizing<String>>> {
    let path = match std::env::var_os("ANTHROPIC_ENV_FILE") {
        Some(value) if value.is_empty() => return Ok(BTreeMap::new()),
        Some(value) => std::path::PathBuf::from(value),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env.claude"),
    };
    match std::fs::symlink_metadata(&path) {
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound
                && std::env::var_os("ANTHROPIC_ENV_FILE").is_none() =>
        {
            Ok(BTreeMap::new())
        }
        _ => load(&path),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn strict_private_file_never_accepts_shell_flags_duplicates_or_shared_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env.claude");
        std::fs::write(
            &path,
            "ANTHROPIC_API_KEY=sk-ant-synthetic-only\nANTHROPIC_MODEL=claude-haiku-5-5\n",
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(load(&path).unwrap().len(), 2);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        for text in [
            "SUPA_ALLOW_ANTHROPIC=true\n",
            "OPENAI_API_KEY=preserve-other-provider\n",
            "ANTHROPIC_MODEL=$(not-a-command)\n",
            "ANTHROPIC_MODEL=a\nANTHROPIC_MODEL=b\n",
        ] {
            std::fs::write(&path, text).unwrap();
            assert!(load(&path).is_err());
        }
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(load(&link).is_err());
    }
}
