//! `machines.<m>.scheduler.drive` and `worker_threads` — how a `tokio` host
//! drives a machine's session runtime (SCE Protocol-Synthesis RFC §synth-5-K,
//! Session drive).
//!
//! What this file holds the deploy to:
//! - the two keys parse on a `tokio` scheduler and resolve to a profile;
//! - `cooperative` and `rt` refuse both keys, and a worker pool needs a stated
//!   count of at least 1 (`deploy/scheduler-drive-not-applicable`,
//!   `deploy/scheduler-worker-threads-invalid`);
//! - the Rust backend emits `{machine}_session_drive.rs` carrying the const the
//!   host reads, and a deploy that states no drive emits nothing, so every
//!   existing output stays byte-identical.

use sce_build::forge::generator::render_machine_session_drive_artifacts;
use sce_build::generator::Language;
use sce_build::mesh::deploy::{parse_deploy_str, SessionDriveProfile};
use sce_build::mesh::error::{DeployError, WorkerThreadsFault};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

fn template_root() -> PathBuf {
    let mut cursor = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = cursor.join("tools/codegen/templates");
        if candidate.is_dir() {
            return candidate;
        }
        assert!(
            cursor.pop(),
            "template root not found from CARGO_MANIFEST_DIR ancestry"
        );
    }
}

/// A deploy with one machine whose `scheduler:` body is `scheduler_yaml`.
fn deploy_with_scheduler(scheduler_yaml: &str) -> String {
    format!(
        r#"
version: "1.0"
topology:
  host:
    machines:
      ap_node:
        source: session_fsm.scxml
        scheduler:
{scheduler_yaml}
"#
    )
}

fn profile_of(scheduler_yaml: &str) -> Option<SessionDriveProfile> {
    let cfg = parse_deploy_str(&deploy_with_scheduler(scheduler_yaml))
        .unwrap_or_else(|e| panic!("deploy must parse: {e}"));
    cfg.topology["host"].machines["ap_node"]
        .scheduler
        .as_ref()
        .expect("scheduler present")
        .session_drive_profile()
}

fn rejection_of(scheduler_yaml: &str) -> DeployError {
    parse_deploy_str(&deploy_with_scheduler(scheduler_yaml)).expect_err("deploy must be refused")
}

// ── parse and resolve ─────────────────────────────────────────

#[test]
fn own_thread_resolves_to_its_profile() {
    let profile = profile_of("          kind: tokio\n          drive: own-thread");
    assert_eq!(profile, Some(SessionDriveProfile::OwnThread));
}

#[test]
fn worker_pool_resolves_with_its_stated_count() {
    let profile = profile_of(
        "          kind: tokio\n          drive: worker-pool\n          worker_threads: 2",
    );
    assert_eq!(
        profile,
        Some(SessionDriveProfile::WorkerPool { worker_threads: 2 })
    );
}

#[test]
fn a_tokio_scheduler_that_states_no_drive_resolves_to_none() {
    let profile = profile_of("          kind: tokio\n          tick_period_us: 1000");
    assert_eq!(profile, None);
}

#[test]
fn an_unknown_drive_spelling_is_refused_at_parse() {
    let err = rejection_of("          kind: tokio\n          drive: thread-per-core");
    assert!(
        matches!(err, DeployError::Yaml(_)),
        "an unknown drive must fail the typed parse, got {err:?}"
    );
}

// ── deploy/scheduler-drive-not-applicable ─────────────────────

#[test]
fn cooperative_refuses_drive() {
    let yaml = "          kind: cooperative\n          worker_stack_budget: 4096\n          \
                worker_slot_budget_us: 200\n          keepalive_jitter_budget_us: 5000\n          \
                drive: own-thread";
    match rejection_of(yaml) {
        DeployError::SchedulerDriveNotApplicable { machine, kind, key } => {
            assert_eq!(machine, "ap_node");
            assert_eq!(kind, "cooperative");
            assert_eq!(key, "drive");
        }
        other => panic!("expected SchedulerDriveNotApplicable, got {other:?}"),
    }
}

#[test]
fn rt_refuses_worker_threads_when_drive_is_absent() {
    match rejection_of("          kind: rt\n          worker_threads: 4") {
        DeployError::SchedulerDriveNotApplicable { kind, key, .. } => {
            assert_eq!(kind, "rt");
            assert_eq!(key, "worker_threads");
        }
        other => panic!("expected SchedulerDriveNotApplicable, got {other:?}"),
    }
}

#[test]
fn the_not_applicable_message_names_the_key_and_the_kind() {
    let message = rejection_of("          kind: rt\n          drive: worker-pool").to_string();
    assert!(
        message.contains("scheduler.drive") && message.contains("scheduler.kind 'rt'"),
        "the message must say which key cannot be used under which kind: {message}"
    );
}

// ── deploy/scheduler-worker-threads-invalid ───────────────────

fn worker_threads_fault(scheduler_yaml: &str) -> (WorkerThreadsFault, Option<u32>) {
    match rejection_of(scheduler_yaml) {
        DeployError::SchedulerWorkerThreadsInvalid {
            fault,
            worker_threads,
            ..
        } => (fault, worker_threads),
        other => panic!("expected SchedulerWorkerThreadsInvalid, got {other:?}"),
    }
}

#[test]
fn a_worker_pool_without_a_count_is_refused() {
    let fault = worker_threads_fault("          kind: tokio\n          drive: worker-pool");
    assert_eq!(fault, (WorkerThreadsFault::Missing, None));
}

#[test]
fn a_worker_pool_of_zero_workers_is_refused() {
    let fault = worker_threads_fault(
        "          kind: tokio\n          drive: worker-pool\n          worker_threads: 0",
    );
    assert_eq!(fault, (WorkerThreadsFault::BelowOne, Some(0)));
}

#[test]
fn a_count_beside_own_thread_is_refused() {
    let fault = worker_threads_fault(
        "          kind: tokio\n          drive: own-thread\n          worker_threads: 2",
    );
    assert_eq!(fault, (WorkerThreadsFault::WithoutWorkerPool, Some(2)));
}

#[test]
fn a_count_with_no_drive_is_refused() {
    let fault = worker_threads_fault("          kind: tokio\n          worker_threads: 2");
    assert_eq!(fault, (WorkerThreadsFault::WithoutWorkerPool, Some(2)));
}

// ── generated const ───────────────────────────────────────────

fn render(profile: Option<SessionDriveProfile>, language: Language) -> Vec<(String, String)> {
    render_machine_session_drive_artifacts(&template_root(), language, "ap_node", profile)
        .expect("render succeeds")
}

#[test]
fn own_thread_emits_the_const_the_host_reads() {
    let files = render(Some(SessionDriveProfile::OwnThread), Language::Rust);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "ap_node_session_drive.rs");
    assert!(
        files[0]
            .1
            .contains("pub const SESSION_DRIVE: SessionDrive = SessionDrive::OwnThread;"),
        "own-thread const missing:\n{}",
        files[0].1
    );
}

#[test]
fn worker_pool_emits_its_count_and_differs_from_own_thread() {
    let pool = render(
        Some(SessionDriveProfile::WorkerPool { worker_threads: 3 }),
        Language::Rust,
    );
    assert!(
        pool[0].1.contains(
            "pub const SESSION_DRIVE: SessionDrive = SessionDrive::WorkerPool { worker_threads: 3 };"
        ),
        "worker-pool const missing:\n{}",
        pool[0].1
    );
    let own = render(Some(SessionDriveProfile::OwnThread), Language::Rust);
    assert_ne!(pool[0].1, own[0].1);
}

#[test]
fn the_enum_takes_its_derives_from_the_policy() {
    let files = render(Some(SessionDriveProfile::OwnThread), Language::Rust);
    assert!(
        files[0]
            .1
            .contains("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum SessionDrive {"),
        "derive line must come from the SSOT policy:\n{}",
        files[0].1
    );
}

#[test]
fn no_stated_drive_emits_nothing() {
    assert!(render(None, Language::Rust).is_empty());
}

#[test]
fn a_backend_without_a_tokio_host_emits_nothing() {
    let profile = Some(SessionDriveProfile::OwnThread);
    assert!(render(profile, Language::C11).is_empty());
}

// ── through the orchestrator ──────────────────────────────────

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const SESSION_FSM: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="session_fsm"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##;

/// Orchestrate `session_fsm.scxml` against a deploy whose `ap_node` scheduler
/// body is `scheduler_yaml`, and return every file written under the output
/// directory, keyed by its path relative to it.
fn orchestrate(label: &str, scheduler_yaml: &str) -> BTreeMap<String, Vec<u8>> {
    let dir = ScratchDir::new(label);
    let scxml = dir.path().join("session_fsm.scxml");
    std::fs::write(&scxml, SESSION_FSM).expect("write scxml");
    let deploy = dir.path().join("deploy.yaml");
    std::fs::write(&deploy, deploy_with_scheduler(scheduler_yaml)).expect("write deploy");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("mkdir out");

    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("orchestrate")
        .arg("--scxml")
        .arg(&scxml)
        .args(["--language", "rust", "--output-dir"])
        .arg(&out_dir)
        .arg("--deploy")
        .arg(&deploy)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen orchestrate");
    assert!(
        out.status.success(),
        "orchestrate must succeed (stderr: {})",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut files = BTreeMap::new();
    collect(&out_dir, &out_dir, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).expect("read output dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .into_owned();
            files.insert(rel, std::fs::read(&path).expect("read output file"));
        }
    }
}

fn drive_file(files: &BTreeMap<String, Vec<u8>>) -> Option<(&String, &Vec<u8>)> {
    files
        .iter()
        .find(|(name, _)| name.ends_with("ap_node_session_drive.rs"))
}

#[test]
fn the_orchestrator_writes_the_drive_file_for_a_machine_with_no_links() {
    let files = orchestrate(
        "session-drive-emits",
        "          kind: tokio\n          drive: worker-pool\n          worker_threads: 2",
    );
    let (_, body) = drive_file(&files).unwrap_or_else(|| {
        panic!(
            "ap_node_session_drive.rs missing; wrote {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });
    let body = String::from_utf8_lossy(body);
    assert!(
        body.contains("SessionDrive::WorkerPool { worker_threads: 2 }"),
        "the file must carry the declared drive:\n{body}"
    );
}

#[test]
fn stating_a_drive_adds_one_file_and_changes_no_other() {
    let without = orchestrate("session-drive-control", "          kind: tokio");
    let with = orchestrate(
        "session-drive-treated",
        "          kind: tokio\n          drive: own-thread",
    );
    assert!(
        drive_file(&without).is_none(),
        "a deploy with no drive must emit no drive file"
    );
    let (drive_name, _) = drive_file(&with).expect("the treated run writes the drive file");
    let mut with_rest = with.clone();
    with_rest.remove(drive_name.as_str());
    assert_eq!(
        without_source_hash(&without),
        without_source_hash(&with_rest),
        "every other output must be byte-identical with and without the keys"
    );
}

/// The `source-hash` header covers the deploy file's text, so a deploy that
/// gains a key moves it in every file, and the `From:` header names the
/// scratch directory of the run; the body is what must stay put.
fn without_source_hash(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(name, bytes)| {
            let body = String::from_utf8_lossy(bytes)
                .lines()
                .filter(|line| {
                    !line.starts_with("// source-hash:") && !line.starts_with("// From:")
                })
                .collect::<Vec<_>>()
                .join("\n");
            (name.clone(), body)
        })
        .collect()
}
