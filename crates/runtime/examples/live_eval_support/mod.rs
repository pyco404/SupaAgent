//! Diagnostics contain only application-owned codes, never raw I/O, SQL or HTTP errors.
pub mod environment;
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use supa_domain::Error;
use uuid::Uuid;

#[derive(Debug)]
pub struct Failure {
    pub stage: &'static str,
    pub code: String,
    io_kind: Option<std::io::ErrorKind>,
}
pub type EvalResult<T> = std::result::Result<T, Failure>;
impl Failure {
    fn io(error: std::io::Error) -> Self {
        Self {
            stage: "report_initialization",
            code: if error.kind() == std::io::ErrorKind::AlreadyExists {
                "report_already_exists"
            } else {
                "report_io_failed"
            }
            .into(),
            io_kind: Some(error.kind()),
        }
    }
    pub fn json(&self) -> Value {
        json!({"error":self.code,"stage":self.stage,
            "io_error_kind":self.io_kind.map(|kind| format!("{kind:?}")),
            "remediation":match self.code.as_str() {
                "provider_http_401" => Some("Check the API credential, project/organization access and IP restrictions privately. Exported credentials override the local environment file. Do not reset the budget."),
                "report_already_exists" => Some("Keep the existing report and budget. Unset SUPA_EVAL_OUTPUT for an automatic attempt filename, or select a new report path."),
                _ => None,
            },"live_evaluation_success":false})
    }
}
pub fn at<T>(stage: &'static str, result: supa_domain::Result<T>) -> EvalResult<T> {
    result.map_err(|error| Failure {
        stage: match error {
            Error::ProviderHttp(_) => "openai_response",
            Error::ProviderTransport => "openai_transport",
            Error::ProviderRequest => "request_construction",
            // Keep local setup stages; generic provider errors during cycles denote response validation.
            Error::Provider if stage == "autonomous_cycle" || stage == "tweet_cycle" => {
                "response_validation"
            }
            _ => stage,
        },
        code: error.to_string(),
        io_kind: None,
    })
}

pub struct Report {
    pub path: PathBuf,
    file: File,
}
impl Report {
    /// An attempt filename is independent of the persisted spending budget identity.
    pub fn create(directory: &Path, budget: Uuid, explicit: Option<&Path>) -> EvalResult<Self> {
        let mut path = explicit
            .map(Path::to_path_buf)
            .unwrap_or_else(|| directory.join(format!("{budget}.json")));
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(Failure::io)?;
        }
        for _ in 0..8 {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(file) => return Ok(Self { path, file }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && explicit.is_none() => {
                    path = directory.join(format!("{budget}.attempt-{}.json", Uuid::new_v4()));
                }
                Err(e) => return Err(Failure::io(e)),
            }
        }
        Err(Failure::io(std::io::ErrorKind::AlreadyExists.into()))
    }
    pub fn write(&mut self, value: &Value) -> EvalResult<()> {
        let bytes = at(
            "report_write",
            serde_json::to_vec_pretty(value).map_err(|_| Error::Invalid),
        )?;
        // Retain the exclusive file handle: never reopen a path that could have been replaced.
        (|| {
            self.file.seek(SeekFrom::Start(0))?;
            self.file.write_all(&bytes)?;
            self.file.set_len(bytes.len() as u64)?;
            self.file.sync_all()
        })()
        .map_err(|e| Failure {
            stage: "report_write",
            ..Failure::io(e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_keeps_previous_evidence_and_same_budget() {
        let scratch = std::env::temp_dir().join(format!("supa-report-test-{}", Uuid::new_v4()));
        let budget = Uuid::new_v4();
        let original = scratch.join(format!("{budget}.json"));
        let mut first = Report::create(&scratch, budget, None).unwrap();
        first
            .write(&json!({"budget_id":budget,"failure":"provider_http_401"}))
            .unwrap();
        let before = std::fs::read(&original).unwrap();
        // This is the exact old failure: create_new on the stable budget filename.
        assert_eq!(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&original)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::AlreadyExists
        );
        let mut retry = Report::create(&scratch, budget, None).unwrap();
        assert_ne!(retry.path, original);
        assert!(
            retry
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(&format!("{budget}.attempt-"))
        );
        retry.write(&json!({"budget_id":budget})).unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), before);
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(&retry.path).unwrap()).unwrap()["budget_id"],
            budget.to_string()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&retry.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let explicit = Report::create(&scratch, budget, Some(&original))
            .err()
            .unwrap();
        assert_eq!(explicit.stage, "report_initialization");
        assert_eq!(explicit.code, "report_already_exists");
        assert_eq!(std::fs::read(&original).unwrap(), before);
        std::fs::remove_dir_all(scratch).unwrap();
    }
    #[test]
    fn diagnostics_identify_stages_without_private_values() {
        let cases = [
            (
                "database_connection",
                Error::Database,
                "database_connection",
            ),
            ("database_migration", Error::Database, "database_migration"),
            ("agent_setup", Error::Invalid, "agent_setup"),
            (
                "autonomous_cycle",
                Error::ProviderRequest,
                "request_construction",
            ),
            (
                "autonomous_cycle",
                Error::ProviderHttp(401),
                "openai_response",
            ),
            ("tweet_cycle", Error::Provider, "response_validation"),
        ];
        for (stage, error, expected) in cases {
            let failure = at::<()>(stage, Err(error)).unwrap_err().json();
            assert_eq!(failure["stage"], expected);
            assert_eq!(failure["live_evaluation_success"], false);
        }
        let raw = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            concat!(
                "postgresql://",
                "secret-user",
                ":",
                "private-password",
                "@host sk-private prompt"
            ),
        );
        let serialized = Failure::io(raw).json().to_string();
        for private in ["secret-user", "private-password", "sk-private", "prompt"] {
            assert!(!serialized.contains(private));
        }
    }
}
