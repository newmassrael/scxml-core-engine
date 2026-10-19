//! `machines.<m>.scheduler.session_runtime` — how a `tokio` host configures the
//! runtime it creates for a machine's sessions (SCE Protocol-Synthesis RFC
//! §synth-5-K, Session runtime): `drive`, `worker_threads`, `rx_busy_poll_us`.
//!
//! What this file holds the deploy to:
//! - the block parses on a `tokio` scheduler and resolves to a profile, and the
//!   keys it names are the keys it takes (a flat `drive:` under `scheduler:` is
//!   refused, as is an unknown key in the block);
//! - `cooperative` and `rt` refuse the block, a worker pool needs a stated count
//!   of at least 1, and a busy-poll budget is at least 1 and no longer than the
//!   tick (`deploy/scheduler-session-runtime-not-applicable`,
//!   `deploy/scheduler-worker-threads-invalid`,
//!   `deploy/scheduler-rx-busy-poll-invalid`);
//! - the Rust backend emits `{machine}_session_runtime.rs` carrying the const
//!   the host reads, and a deploy that states no block emits nothing, so every
//!   existing output stays byte-identical.

use sce_build::forge::generator::render_machine_session_runtime_artifacts;
use sce_build::generator::Language;
use sce_build::mesh::deploy::{parse_deploy_str, SessionDriveProfile, SessionRuntimeProfile};
use sce_build::mesh::error::{DeployError, RxBusyPollFault, WorkerThreadsFault};
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

/// A deploy with one machine whose `scheduler:` body is `scheduler_yaml`, each
/// line already indented to sit under `scheduler:`.
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

/// A `tokio` scheduler whose `session_runtime:` block holds `block_lines`.
fn tokio_with_block(extra_scheduler: &str, block_lines: &[&str]) -> String {
    let mut yaml = String::from("          kind: tokio\n");
    if !extra_scheduler.is_empty() {
        yaml.push_str(&format!("          {extra_scheduler}\n"));
    }
    yaml.push_str("          session_runtime:\n");
    for line in block_lines {
        yaml.push_str(&format!("            {line}\n"));
    }
    yaml
}

fn profile_of(scheduler_yaml: &str) -> Option<SessionRuntimeProfile> {
    let cfg = parse_deploy_str(&deploy_with_scheduler(scheduler_yaml))
        .unwrap_or_else(|e| panic!("deploy must parse: {e}"));
    cfg.topology["host"].machines["ap_node"]
        .scheduler
        .as_ref()
        .expect("scheduler present")
        .session_runtime_profile()
}

fn rejection_of(scheduler_yaml: &str) -> DeployError {
    parse_deploy_str(&deploy_with_scheduler(scheduler_yaml)).expect_err("deploy must be refused")
}

// ── parse and resolve ─────────────────────────────────────────

#[test]
fn own_thread_resolves_to_its_profile_with_no_busy_poll() {
    let profile = profile_of(&tokio_with_block("", &["drive: own-thread"]));
    assert_eq!(
        profile,
        Some(SessionRuntimeProfile {
            drive: SessionDriveProfile::OwnThread,
            rx_busy_poll_us: 0,
        })
    );
}

#[test]
fn worker_pool_resolves_with_its_stated_count() {
    let profile = profile_of(&tokio_with_block(
        "",
        &["drive: worker-pool", "worker_threads: 2"],
    ));
    assert_eq!(
        profile,
        Some(SessionRuntimeProfile {
            drive: SessionDriveProfile::WorkerPool { worker_threads: 2 },
            rx_busy_poll_us: 0,
        })
    );
}

#[test]
fn a_busy_poll_budget_resolves_into_the_profile() {
    let profile = profile_of(&tokio_with_block(
        "",
        &["drive: own-thread", "rx_busy_poll_us: 10"],
    ));
    assert_eq!(profile.expect("a block resolves").rx_busy_poll_us, 10);
}

#[test]
fn a_tokio_scheduler_with_no_block_resolves_to_none() {
    let profile = profile_of("          kind: tokio\n          tick_period_us: 1000");
    assert_eq!(profile, None);
}

#[test]
fn an_unknown_drive_spelling_is_refused_at_parse() {
    let err = rejection_of(&tokio_with_block("", &["drive: thread-per-core"]));
    assert!(
        matches!(err, DeployError::Yaml(_)),
        "an unknown drive must fail the typed parse, got {err:?}"
    );
}

#[test]
fn a_block_that_does_not_say_how_it_is_driven_is_refused_at_parse() {
    let err = rejection_of(&tokio_with_block("", &["rx_busy_poll_us: 10"]));
    assert!(
        matches!(err, DeployError::Yaml(_)),
        "a block with no drive must fail the typed parse, got {err:?}"
    );
}

#[test]
fn a_key_the_block_does_not_take_is_refused_at_parse() {
    let err = rejection_of(&tokio_with_block(
        "",
        &["drive: own-thread", "stack_size: 65536"],
    ));
    assert!(
        matches!(err, DeployError::Yaml(_)),
        "an unknown key in the block must fail the typed parse, got {err:?}"
    );
}

#[test]
fn the_drive_key_belongs_to_the_block_and_not_to_the_scheduler() {
    let err = rejection_of("          kind: tokio\n          drive: own-thread");
    assert!(
        matches!(err, DeployError::Yaml(_)),
        "a flat drive under scheduler must fail the typed parse, got {err:?}"
    );
}

// ── deploy/scheduler-session-runtime-not-applicable ───────────

#[test]
fn cooperative_refuses_the_block() {
    let yaml = format!(
        "          kind: cooperative\n          worker_stack_budget: 4096\n          \
         worker_slot_budget_us: 200\n          keepalive_jitter_budget_us: 5000\n{}",
        tokio_with_block("", &["drive: own-thread"]).replace("          kind: tokio\n", "")
    );
    match rejection_of(&yaml) {
        DeployError::SchedulerSessionRuntimeNotApplicable { machine, kind } => {
            assert_eq!(machine, "ap_node");
            assert_eq!(kind, "cooperative");
        }
        other => panic!("expected SchedulerSessionRuntimeNotApplicable, got {other:?}"),
    }
}

#[test]
fn rt_refuses_the_block() {
    let yaml = tokio_with_block("", &["drive: worker-pool", "worker_threads: 4"])
        .replace("kind: tokio", "kind: rt");
    match rejection_of(&yaml) {
        DeployError::SchedulerSessionRuntimeNotApplicable { kind, .. } => assert_eq!(kind, "rt"),
        other => panic!("expected SchedulerSessionRuntimeNotApplicable, got {other:?}"),
    }
}

#[test]
fn the_not_applicable_message_names_the_block_and_the_kind() {
    let yaml = tokio_with_block("", &["drive: own-thread"]).replace("kind: tokio", "kind: rt");
    let message = rejection_of(&yaml).to_string();
    assert!(
        message.contains("scheduler.session_runtime") && message.contains("scheduler.kind 'rt'"),
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
    let fault = worker_threads_fault(&tokio_with_block("", &["drive: worker-pool"]));
    assert_eq!(fault, (WorkerThreadsFault::Missing, None));
}

#[test]
fn a_worker_pool_of_zero_workers_is_refused() {
    let fault = worker_threads_fault(&tokio_with_block(
        "",
        &["drive: worker-pool", "worker_threads: 0"],
    ));
    assert_eq!(fault, (WorkerThreadsFault::BelowOne, Some(0)));
}

#[test]
fn a_count_beside_own_thread_is_refused() {
    let fault = worker_threads_fault(&tokio_with_block(
        "",
        &["drive: own-thread", "worker_threads: 2"],
    ));
    assert_eq!(fault, (WorkerThreadsFault::WithoutWorkerPool, Some(2)));
}

// ── deploy/scheduler-rx-busy-poll-invalid ─────────────────────

fn rx_busy_poll_fault(scheduler_yaml: &str) -> (RxBusyPollFault, u32) {
    match rejection_of(scheduler_yaml) {
        DeployError::SchedulerRxBusyPollInvalid {
            fault,
            rx_busy_poll_us,
            ..
        } => (fault, rx_busy_poll_us),
        other => panic!("expected SchedulerRxBusyPollInvalid, got {other:?}"),
    }
}

#[test]
fn a_zero_budget_is_refused_and_left_off_by_omitting_it() {
    let fault = rx_busy_poll_fault(&tokio_with_block(
        "",
        &["drive: own-thread", "rx_busy_poll_us: 0"],
    ));
    assert_eq!(fault, (RxBusyPollFault::Zero, 0));
}

#[test]
fn a_budget_longer_than_the_tick_is_refused() {
    let fault = rx_busy_poll_fault(&tokio_with_block(
        "tick_period_us: 1000",
        &["drive: own-thread", "rx_busy_poll_us: 1500"],
    ));
    assert_eq!(
        fault,
        (
            RxBusyPollFault::ExceedsTick {
                tick_period_us: 1000
            },
            1500
        )
    );
}

#[test]
fn a_budget_equal_to_the_tick_is_kept() {
    let profile = profile_of(&tokio_with_block(
        "tick_period_us: 1000",
        &["drive: own-thread", "rx_busy_poll_us: 1000"],
    ));
    assert_eq!(profile.expect("a block resolves").rx_busy_poll_us, 1000);
}

#[test]
fn with_no_tick_declared_the_budget_has_no_ceiling_to_exceed() {
    let profile = profile_of(&tokio_with_block(
        "",
        &["drive: own-thread", "rx_busy_poll_us: 100000"],
    ));
    assert_eq!(profile.expect("a block resolves").rx_busy_poll_us, 100000);
}

// ── generated const ───────────────────────────────────────────

fn render(profile: Option<SessionRuntimeProfile>, language: Language) -> Vec<(String, String)> {
    render_machine_session_runtime_artifacts(&template_root(), language, "ap_node", profile)
        .expect("render succeeds")
}

fn own_thread(rx_busy_poll_us: u32) -> SessionRuntimeProfile {
    SessionRuntimeProfile {
        drive: SessionDriveProfile::OwnThread,
        rx_busy_poll_us,
    }
}

#[test]
fn own_thread_emits_the_const_the_host_reads() {
    let files = render(Some(own_thread(0)), Language::Rust);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "ap_node_session_runtime.rs");
    let body = &files[0].1;
    assert!(
        body.contains("drive: SessionDrive::OwnThread,") && body.contains("rx_busy_poll_us: 0,"),
        "own-thread const missing:\n{body}"
    );
}

#[test]
fn worker_pool_emits_its_count_and_budget_and_differs_from_own_thread() {
    let pool = render(
        Some(SessionRuntimeProfile {
            drive: SessionDriveProfile::WorkerPool { worker_threads: 3 },
            rx_busy_poll_us: 10,
        }),
        Language::Rust,
    );
    let body = &pool[0].1;
    assert!(
        body.contains("drive: SessionDrive::WorkerPool { worker_threads: 3 },")
            && body.contains("rx_busy_poll_us: 10,"),
        "worker-pool const missing:\n{body}"
    );
    assert!(body.contains(".worker_threads(3)"));
    let own = render(Some(own_thread(0)), Language::Rust);
    assert_ne!(pool[0].1, own[0].1);
}

#[test]
fn changing_only_the_busy_poll_budget_changes_the_output() {
    let off = render(Some(own_thread(0)), Language::Rust);
    let on = render(Some(own_thread(10)), Language::Rust);
    assert_ne!(off[0].1, on[0].1);
}

#[test]
fn the_types_take_their_derives_from_the_policy() {
    let files = render(Some(own_thread(0)), Language::Rust);
    let derive = "#[derive(Debug, Clone, Copy, PartialEq, Eq)]";
    for item in ["pub enum SessionDrive {", "pub struct SessionRuntime {"] {
        assert!(
            files[0].1.contains(&format!("{derive}\n{item}")),
            "derive line must come from the SSOT policy for `{item}`:\n{}",
            files[0].1
        );
    }
}

#[test]
fn no_stated_block_emits_nothing() {
    assert!(render(None, Language::Rust).is_empty());
}

#[test]
fn a_backend_without_a_tokio_host_emits_nothing() {
    assert!(render(Some(own_thread(0)), Language::C11).is_empty());
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

fn runtime_file(files: &BTreeMap<String, Vec<u8>>) -> Option<(&String, &Vec<u8>)> {
    files
        .iter()
        .find(|(name, _)| name.ends_with("ap_node_session_runtime.rs"))
}

#[test]
fn the_orchestrator_writes_the_runtime_file_for_a_machine_with_no_links() {
    let files = orchestrate(
        "session-runtime-emits",
        &tokio_with_block(
            "",
            &[
                "drive: worker-pool",
                "worker_threads: 2",
                "rx_busy_poll_us: 10",
            ],
        ),
    );
    let (_, body) = runtime_file(&files).unwrap_or_else(|| {
        panic!(
            "ap_node_session_runtime.rs missing; wrote {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });
    let body = String::from_utf8_lossy(body);
    assert!(
        body.contains("SessionDrive::WorkerPool { worker_threads: 2 }")
            && body.contains("rx_busy_poll_us: 10,"),
        "the file must carry the declared runtime:\n{body}"
    );
}

#[test]
fn stating_a_block_adds_one_file_and_changes_no_other() {
    let without = orchestrate("session-runtime-control", "          kind: tokio");
    let with = orchestrate(
        "session-runtime-treated",
        &tokio_with_block("", &["drive: own-thread"]),
    );
    assert!(
        runtime_file(&without).is_none(),
        "a deploy with no block must emit no runtime file"
    );
    let (name, _) = runtime_file(&with).expect("the treated run writes the runtime file");
    let mut with_rest = with.clone();
    with_rest.remove(name.as_str());
    assert_eq!(
        without_run_headers(&without),
        without_run_headers(&with_rest),
        "every other output must be byte-identical with and without the block"
    );
}

/// The `source-hash` header covers the deploy file's text, so a deploy that
/// gains a block moves it in every file, and the `From:` header names the
/// scratch directory of the run; the body is what must stay put.
fn without_run_headers(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
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
