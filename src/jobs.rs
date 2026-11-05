use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::progress::JobSnapshot;
use crate::update::config_path;
use crate::validate::{is_valid_download_url, is_valid_magnet_url};

pub const MISSING_FILES: &str = "files missing from download folder";
pub const HTTP_NOT_RESUMABLE: &str = "HTTP download cannot resume; restart to start over";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Magnet,
    Http,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Magnet => "magnet",
            Self::Http => "http",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "magnet" => Some(Self::Magnet),
            "http" => Some(Self::Http),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedJob {
    pub id: u64,
    pub kind: String,
    pub source: String,
    pub dest: PathBuf,
    #[serde(default)]
    pub seed: bool,
    #[serde(default)]
    pub added: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub downloaded: u64,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct JobListFile {
    #[serde(default)]
    jobs: Vec<PersistedJob>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreAction {
    Start,
    Idle,
    Missing,
}

#[derive(Debug)]
pub enum LoadJobs {
    Loaded(Vec<PersistedJob>),
    Empty,
    Corrupt(String),
}

pub fn jobs_path() -> Result<PathBuf> {
    let config = config_path()?;
    Ok(config.with_file_name("jobs.json"))
}

pub fn load_jobs() -> LoadJobs {
    let Ok(path) = jobs_path() else {
        return LoadJobs::Empty;
    };
    load_jobs_from(&path)
}

pub fn save_jobs(jobs: &[PersistedJob]) -> Result<()> {
    let path = jobs_path()?;
    save_jobs_to(&path, jobs)
}

pub fn load_jobs_from(path: &Path) -> LoadJobs {
    recover_incomplete_save(path);
    if !path.exists() {
        return LoadJobs::Empty;
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            quarantine(path);
            return LoadJobs::Corrupt(error.to_string());
        }
    };
    match serde_json::from_slice::<JobListFile>(&bytes) {
        Ok(file) => LoadJobs::Loaded(
            file.jobs
                .into_iter()
                .filter(PersistedJob::is_valid)
                .collect(),
        ),
        Err(error) => {
            quarantine(path);
            LoadJobs::Corrupt(error.to_string())
        }
    }
}

pub fn save_jobs_to(path: &Path, jobs: &[PersistedJob]) -> Result<()> {
    let file = JobListFile {
        jobs: jobs.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&file).context("failed to serialize jobs")?;
    atomic_write(path, &bytes)
}

impl PersistedJob {
    pub fn kind_id(&self) -> Option<JobKind> {
        JobKind::parse(&self.kind)
    }

    pub fn is_valid(&self) -> bool {
        if self.id == 0 || self.source.trim().is_empty() || self.dest.as_os_str().is_empty() {
            return false;
        }
        match self.kind_id() {
            Some(JobKind::Magnet) => is_valid_magnet_url(&self.source),
            Some(JobKind::Http) => is_valid_download_url(&self.source),
            None => false,
        }
    }

    pub fn wants_engine(&self) -> bool {
        if !self.is_valid() {
            return false;
        }
        if self.error.is_some() && self.status == "failed" {
            return false;
        }
        !matches!(self.status.as_str(), "done" | "stopped" | "failed")
    }

    pub fn after_probe(&self, present: bool, http_resumable: bool) -> RestoreAction {
        if !self.wants_engine() {
            return RestoreAction::Idle;
        }
        match self.kind_id() {
            Some(JobKind::Http) => {
                if self.downloaded > 0 && !http_resumable {
                    RestoreAction::Missing
                } else {
                    RestoreAction::Start
                }
            }
            Some(JobKind::Magnet) => {
                if self.downloaded > 0 && !present {
                    RestoreAction::Missing
                } else {
                    RestoreAction::Start
                }
            }
            None => RestoreAction::Idle,
        }
    }

    pub fn apply_to(&self, snapshot: &JobSnapshot) {
        snapshot
            .downloaded
            .store(self.downloaded, std::sync::atomic::Ordering::Relaxed);
        snapshot
            .total
            .store(self.total, std::sync::atomic::Ordering::Relaxed);
        if !self.title.is_empty() {
            snapshot.set_title(&self.title);
        }
        if self.status == "failed" {
            snapshot.set_error(self.error.clone().unwrap_or_else(|| "failed".into()));
            return;
        }
        if self.status == "stopped" {
            if let Some(error) = &self.error {
                snapshot.mark_stopped_with(error);
            } else {
                snapshot.mark_stopped();
            }
            return;
        }
        if self.status == "done" {
            snapshot.mark_finished();
            return;
        }
        let status = if self.paused {
            "paused"
        } else {
            self.status.as_str()
        };
        if !status.is_empty() {
            snapshot.set_status(status);
        }
        if self.paused {
            snapshot.set_paused(true);
        }
    }

    pub fn missing_reason(&self) -> &'static str {
        if self.kind_id() == Some(JobKind::Http) {
            HTTP_NOT_RESUMABLE
        } else {
            MISSING_FILES
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("failed to write {}", tmp.display()))?;
    replace_file(&tmp, path)
}

/// POSIX rename replaces. Windows cannot rename over an existing file, so the
/// live file is moved aside only for that replace, then restored if the new
/// name fails. Never unlink the live file first.
fn replace_file(from: &Path, to: &Path) -> Result<()> {
    if cfg!(windows) && to.exists() {
        let backup = to.with_extension("json.prev");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(to, &backup)
            .with_context(|| format!("failed to move aside {}", to.display()))?;
        if let Err(error) = std::fs::rename(from, to) {
            let _ = std::fs::rename(&backup, to);
            return Err(error).with_context(|| format!("failed to replace {}", to.display()));
        }
        let _ = std::fs::remove_file(&backup);
        return Ok(());
    }
    std::fs::rename(from, to).with_context(|| format!("failed to replace {}", to.display()))
}

fn recover_incomplete_save(path: &Path) {
    if path.exists() {
        return;
    }
    for aside in [
        path.with_extension("json.tmp"),
        path.with_extension("json.prev"),
    ] {
        if aside.exists() && std::fs::rename(&aside, path).is_ok() {
            return;
        }
    }
}

fn quarantine(path: &Path) {
    let bak = path.with_extension("json.bak");
    let _ = std::fs::remove_file(&bak);
    let _ = std::fs::rename(path, bak);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::JobSnapshot;

    fn sample(kind: &str, status: &str, downloaded: u64) -> PersistedJob {
        PersistedJob {
            id: 1,
            kind: kind.into(),
            source: if kind == "magnet" {
                "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10".into()
            } else {
                "https://example.com/file.bin".into()
            },
            dest: PathBuf::from("/tmp/torrenter-out"),
            seed: kind == "magnet",
            added: "2026-11-05".into(),
            title: "file".into(),
            downloaded,
            total: 100,
            status: status.into(),
            paused: status == "paused",
            error: None,
        }
    }

    #[test]
    fn roundtrip_and_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        let job = sample("magnet", "downloading", 10);
        save_jobs_to(&path, std::slice::from_ref(&job)).unwrap();
        match load_jobs_from(&path) {
            LoadJobs::Loaded(jobs) => assert_eq!(jobs, vec![job]),
            other => panic!("expected loaded, got {other:?}"),
        }
        let parsed: JobListFile = serde_json::from_str(
            r#"{"jobs":[{"id":2,"kind":"http","source":"https://example.com/a","dest":"/tmp/a","extra":true}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.jobs[0].id, 2);
        assert!(!parsed.jobs[0].paused);
        assert_eq!(parsed.jobs[0].downloaded, 0);
    }

    #[test]
    fn replace_keeps_live_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        let first = sample("magnet", "downloading", 10);
        save_jobs_to(&path, std::slice::from_ref(&first)).unwrap();
        assert!(path.exists());
        let second = sample("http", "queued", 0);
        save_jobs_to(&path, std::slice::from_ref(&second)).unwrap();
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
        match load_jobs_from(&path) {
            LoadJobs::Loaded(jobs) => assert_eq!(jobs, vec![second]),
            other => panic!("expected loaded, got {other:?}"),
        }
    }

    #[test]
    fn load_recovers_tmp_when_live_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        let job = sample("magnet", "paused", 10);
        save_jobs_to(&path, std::slice::from_ref(&job)).unwrap();
        std::fs::rename(&path, path.with_extension("json.tmp")).unwrap();
        assert!(!path.exists());
        match load_jobs_from(&path) {
            LoadJobs::Loaded(jobs) => assert_eq!(jobs, vec![job]),
            other => panic!("expected recovered, got {other:?}"),
        }
        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_file_moves_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        std::fs::write(&path, "not-json").unwrap();
        match load_jobs_from(&path) {
            LoadJobs::Corrupt(_) => {}
            other => panic!("expected corrupt, got {other:?}"),
        }
        assert!(!path.exists());
        assert!(path.with_extension("json.bak").exists());
    }

    #[test]
    fn skips_invalid_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        std::fs::write(
            &path,
            r#"{"jobs":[{"id":1,"kind":"magnet","source":"https://nope","dest":"/tmp/a"}]}"#,
        )
        .unwrap();
        match load_jobs_from(&path) {
            LoadJobs::Loaded(jobs) => assert!(jobs.is_empty()),
            other => panic!("expected loaded empty, got {other:?}"),
        }
    }

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        match load_jobs_from(&dir.path().join("jobs.json")) {
            LoadJobs::Empty => {}
            other => panic!("expected empty, got {other:?}"),
        }
    }

    #[test]
    fn probe_rules() {
        let magnet_live = sample("magnet", "downloading", 50);
        assert_eq!(
            magnet_live.after_probe(false, false),
            RestoreAction::Missing
        );
        assert_eq!(magnet_live.after_probe(true, false), RestoreAction::Start);
        let magnet_zero = sample("magnet", "queued", 0);
        assert_eq!(magnet_zero.after_probe(false, false), RestoreAction::Start);
        let http_live = sample("http", "downloading", 20);
        assert_eq!(http_live.after_probe(true, false), RestoreAction::Missing);
        assert_eq!(http_live.after_probe(false, true), RestoreAction::Start);
        let done = sample("magnet", "done", 100);
        assert_eq!(done.after_probe(true, false), RestoreAction::Idle);
        let paused = sample("magnet", "paused", 10);
        assert!(paused.wants_engine());
        assert_eq!(paused.after_probe(true, false), RestoreAction::Start);
        let stopped = sample("http", "stopped", 10);
        assert_eq!(stopped.after_probe(false, true), RestoreAction::Idle);
    }

    #[test]
    fn apply_paused_keeps_engine_active() {
        let snapshot = JobSnapshot::default();
        sample("magnet", "paused", 10).apply_to(&snapshot);
        assert!(snapshot.is_paused());
        assert!(snapshot.is_active());
        assert_eq!(snapshot.status(), "paused");
        assert_eq!(
            snapshot
                .downloaded
                .load(std::sync::atomic::Ordering::Relaxed),
            10
        );
    }

    #[test]
    fn apply_missing_is_stopped_not_failed() {
        let snapshot = JobSnapshot::default();
        let mut job = sample("magnet", "stopped", 40);
        job.error = Some(MISSING_FILES.into());
        job.apply_to(&snapshot);
        assert!(snapshot.is_stopped());
        assert!(!snapshot.is_active());
        assert_eq!(snapshot.error().as_deref(), Some(MISSING_FILES));
        assert_eq!(snapshot.status(), "stopped");
    }
}
