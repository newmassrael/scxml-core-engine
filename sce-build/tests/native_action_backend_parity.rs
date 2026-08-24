// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-G-7 `<sce:action>` — every backend lowers it, and lowers the same
//! three things.
//!
//! Until 2026-08-24 native host dispatch was Rust-only, and
//! `reject_native_actions_in_unsupported_lang` refused the construct for the
//! other five by name. Removing that refusal is a CLAIM that a path exists,
//! and a claim made by deleting something has to be paid for somewhere. The
//! six runtime channels named on the fixture are most of that payment — each
//! drives a real host implementation and asserts the effects fired. This file
//! is the rest: it asks the same questions of all six emitters from one place,
//! so a backend that stops emitting one of them fails HERE rather than in
//! whichever language's suite happens to run first.
//!
//! The questions, and why each is separate:
//!
//! 1. **An interface exists, with a method per declared operation.** The
//!    document names its operations symbolically; without an emitted interface
//!    the host has nothing to implement and the generated call names a
//!    function nobody wrote.
//! 2. **The machine takes the host where it is constructed.** `idle`'s
//!    `<onentry>` performs an act, so a host installed after construction
//!    arrives one act too late. This is one rule in six spellings, and it is
//!    exactly the kind of thing a new backend gets wrong quietly — a setter
//!    compiles, and the missed act is a silent no-op.
//! 3. **Every call site is emitted**, including the two operations that appear
//!    in no transition at all.
//! 4. **The arg-bearing call is guarded by that backend's typed-payload tag.**
//!    An event that arrives without its typed payload carries nothing the
//!    arguments can be read from; firing anyway would hand the host a zero
//!    value it would take for data. The runtime channels measure the effect;
//!    what is measured here is that the guard is emitted at all, on every
//!    backend, from one lowering.
//! 5. **That guard's other arm answers with `error.execution`, in one
//!    wording.** A guard says what does NOT happen; this says what does. The
//!    document itself can put a payload-typed event on the queue with no
//!    payload — `<raise>` is legal SCXML — so the arm is reachable without any
//!    host mistake, and both "skip in silence" and "abort the process" are
//!    answers a generator owes the author instead of §scxml-3.12.2's. Question 4
//!    passed for months while the six disagreed about this one.
//!
//! Engine-freedom is asserted from the manifest rather than from the absence
//! of a substring — see `native_action_alone_needs_no_script_engine_on_any_backend`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The binary under test, resolved the way every other CLI test here does:
/// `CARGO_BIN_EXE_*` makes it a build dependency, so a change to the emitter
/// is visible to this test instead of being measured against a stale binary.
fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// The one document all six backends are asked about — the same one the six
/// runtime channels drive. Named rather than derived, so a fixture that moves
/// fails against a written-down answer instead of agreeing with itself.
const FIXTURE: &str = "sce-build/tests/fixtures/event_schema/statechart_native_action.scxml";

/// The operations the fixture declares, language-neutral. The per-backend
/// spellings are derived from these, so "the fixture grew an operation" and "a
/// backend stopped emitting one" are different failures with different
/// messages.
const OPERATIONS: [&str; 4] = [
    "append_fragment_payload",
    "reset_slot",
    "on_idle_entry",
    "on_assembling_exit",
];

/// What one backend must show for the same document.
struct Expectation {
    lang: &'static str,
    /// The emitted artifacts carrying the interface and the call sites. C++
    /// and C11 split declaration from body, so the pair is read together.
    files: &'static [&'static str],
    /// The interface declaration, exactly as that language spells it.
    interface: &'static str,
    /// How the machine is handed its host. One per backend because each
    /// language expresses "required at construction" differently; what they
    /// have in common is that none of them is a setter.
    host_seam: &'static str,
    /// The receiver prefix every emitted call carries.
    receiver: &'static str,
    /// The tag check wrapping the arg-bearing call.
    payload_guard: &'static str,
    /// How this backend raises `error.execution` from the arm an unreadable
    /// delivery takes. Six spellings, one behaviour — the point of reading it
    /// here is that a backend which quietly went back to skipping, or which
    /// grew its own diagnostic instead, stops matching.
    error_arm: &'static str,
    /// `operation -> spelling`, in that language's convention.
    method: fn(&str) -> String,
}

/// The one message all six carry. Spelled once here so "the six agree" is a
/// thing this file can assert rather than a thing six string literals happen
/// to do — the failure mode `error.execution` has already been through on this
/// codebase is six backends wording the same failure six ways.
const UNREADABLE_ARG_MESSAGE: &str = "<sce:action name='append_fragment_payload'> needs the typed \
     payload of 'fragment.received', which this delivery did not carry";

fn snake(op: &str) -> String {
    op.to_string()
}

fn pascal(op: &str) -> String {
    op.split('_')
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn camel(op: &str) -> String {
    let p = pascal(op);
    let mut c = p.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn expectations() -> Vec<Expectation> {
    vec![
        Expectation {
            lang: "rust",
            files: &["statechart_native_action_sm.rs"],
            interface: "pub trait StatechartNativeActionActions {",
            // The only backend where the requirement is a type parameter: a
            // `Policy<A: …Actions>` cannot be constructed without one, so
            // "the host was supplied" is a fact the compiler already holds.
            host_seam: "pub fn new(actions: A)",
            receiver: "self.actions.",
            payload_guard: "match &self.pending_payload {",
            error_arm: "_ => { engine.raise(sce_rust_runtime::EventWithMetadata::platform_error(\
                        StatechartNativeActionEvent::ErrorExecution,",
            method: snake,
        },
        Expectation {
            lang: "go",
            files: &["statechart_native_action_sm.go"],
            interface: "type StatechartNativeActionActions interface {",
            host_seam:
                "func NewStatechartNativeActionPolicy(actions StatechartNativeActionActions)",
            receiver: "p.actions.",
            payload_guard:
                "p.pendingPayloadTag == StatechartNativeActionPayloadTagFragmentReceived",
            error_arm:
                "engine.Raise(sce.NewPlatformError(StatechartNativeActionEventErrorExecution,",
            method: pascal,
        },
        Expectation {
            lang: "kotlin",
            files: &["statechart_native_actionSm.kt"],
            interface: "interface StatechartNativeActionActions {",
            host_seam: "private val actions: StatechartNativeActionActions,",
            receiver: "actions.",
            payload_guard: "pendingFragmentReceivedPayload?.let {",
            error_arm: "?: run { raiseInternal(StatechartNativeActionEvent.Error.Execution,",
            method: camel,
        },
        Expectation {
            lang: "python",
            files: &["statechart_native_action_sm.py"],
            interface: "class StatechartNativeActionActions(Protocol):",
            host_seam: "def __init__(self, actions: StatechartNativeActionActions) -> None:",
            receiver: "self._actions.",
            payload_guard: "if (_p := self._pending_fragment_received_payload) is not None",
            error_arm: "else self._raise_error_execution(engine,",
            method: snake,
        },
        Expectation {
            lang: "cpp",
            files: &[
                "statechart_native_action_sm.h",
                "statechart_native_action_sm.inl",
            ],
            interface: "struct StatechartNativeActionActions {",
            host_seam: "explicit statechart_native_action(StatechartNativeActionActions& actions)",
            receiver: "actions_->",
            payload_guard:
                "pendingPayloadTag_ == StatechartNativeActionPayloadTag::FragmentReceived",
            error_arm: "engine.raise(typename Engine::EventWithMetadata(Event::Error_execution,",
            method: camel,
        },
        Expectation {
            lang: "c11",
            files: &[
                "statechart_native_action_sm.h",
                "statechart_native_action_sm.c",
            ],
            interface: "} statechart_native_action_actions_t;",
            // The one backend whose "required at construction" is a runtime
            // check rather than a type: C cannot make an unfilled function
            // pointer a compile error, so `_init_with_actions` refuses the
            // vtable instead — see the C11 channel's
            // `a_vtable_missing_an_operation_is_refused`.
            host_seam: "bool statechart_native_action_init_with_actions(",
            receiver: "sm->actions.",
            payload_guard:
                "sm->pending_payload.tag == STATECHART_NATIVE_ACTION_PAYLOAD_FRAGMENT_RECEIVED",
            error_arm: "statechart_native_action_raise_platform_error(sm, \
                        STATECHART_NATIVE_ACTION_EVENT_ERROR_EXECUTION,",
            method: snake,
        },
    ]
}

/// Run `sce-codegen generate` for `lang` into a scratch directory belonging to
/// one CASE and one language.
///
/// The directory is derived rather than randomised and is emptied first, so a
/// previous run's artifacts cannot be read as this one's. `--no-format` keeps
/// the emitted text the emitter's own: a formatter that is absent on one
/// machine would otherwise move the assertions' ground.
///
/// `tag` must be unique per `#[test]`, not merely per file. Rust runs the tests
/// in this binary as concurrent threads of one process, so a tag shared across
/// tests has several of them calling `remove_dir_all` on the same path while
/// the others are reading out of it. That is not a hypothetical: it failed in
/// CI on 2026-08-25 with four of six tests down, one of them reporting an
/// artifact as `No such file or directory` — and the same binary passed under
/// `--test-threads=1` on the same commit, which is what a shared scratch
/// directory looks like from the outside.
fn generate_into(lang: &str, tag: &str) -> (PathBuf, std::process::Output) {
    let out = repo_root().join("target").join(tag).join(lang);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("scratch dir");

    let result = Command::new(codegen_bin())
        .args([
            "generate",
            FIXTURE,
            "-l",
            lang,
            "-o",
            out.to_str().expect("utf-8 path"),
            "--no-format",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");

    assert!(
        result.status.success(),
        "`sce-codegen generate -l {lang}` refused {FIXTURE}. §scxml-G-7 is lowered \
         by every backend; a refusal here is the retired \
         `reject_native_actions_in_unsupported_lang` coming back.\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (out, result)
}

/// The named artifacts for one backend, keyed by filename.
fn generate(lang: &str, files: &[&str], case: &str) -> BTreeMap<String, String> {
    let (out, _) = generate_into(lang, case);
    files
        .iter()
        .map(|name| {
            let path = out.join(name);
            let body = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "{lang}: expected artifact {name} at {}: {e}",
                    path.display()
                )
            });
            ((*name).to_string(), body)
        })
        .collect()
}

/// Every artifact for one backend, concatenated. The split between header and
/// source is a per-language layout detail; the question being asked is whether
/// the backend emitted the construct at all.
fn emitted(files: &BTreeMap<String, String>) -> String {
    files.values().cloned().collect::<Vec<_>>().join("\n")
}

#[test]
fn every_backend_emits_a_host_interface_for_a_native_action() {
    for e in expectations() {
        let files = generate(e.lang, e.files, "native_action_parity_host_interface");
        let text = emitted(&files);
        assert!(
            text.contains(e.interface),
            "{}: no host interface emitted. The document names its operations \
             symbolically, so without one the host has nothing to implement and the \
             generated call names a function nobody wrote. Expected to find:\n  {}",
            e.lang,
            e.interface
        );
        for op in OPERATIONS {
            let spelled = (e.method)(op);
            assert!(
                text.contains(&spelled),
                "{}: nothing declares `{spelled}` (from `<sce:action name=\"{op}\">`). \
                 Every declared operation needs a method, including the two that \
                 appear in no transition — an eventless-only action is still an act \
                 the document declared.",
                e.lang
            );
        }
    }
}

#[test]
fn every_backend_takes_its_host_where_the_machine_is_constructed() {
    for e in expectations() {
        let files = generate(e.lang, e.files, "native_action_parity_host_injection");
        let text = emitted(&files);
        assert!(
            text.contains(e.host_seam),
            "{}: the host is not taken at construction. `idle`'s `<onentry>` performs \
             an act, so a host installed afterwards arrives one act too late — and a \
             setter compiles, which is what makes that failure silent. Expected to \
             find:\n  {}",
            e.lang,
            e.host_seam
        );
    }
}

#[test]
fn every_backend_emits_a_call_site_for_every_declared_operation() {
    for e in expectations() {
        let files = generate(e.lang, e.files, "native_action_parity_call_sites");
        let text = emitted(&files);
        for op in OPERATIONS {
            let call = format!("{}{}(", e.receiver, (e.method)(op));
            assert!(
                text.contains(&call),
                "{}: no call site for `<sce:action name=\"{op}\">`. A declared act that \
                 reaches no host is the silence this construct exists to remove. \
                 Expected to find:\n  {call}",
                e.lang
            );
        }
    }
}

#[test]
fn every_backend_guards_the_arg_bearing_call_with_its_typed_payload_tag() {
    for e in expectations() {
        let files = generate(e.lang, e.files, "native_action_parity_payload_guard");
        let text = emitted(&files);
        assert!(
            text.contains(e.payload_guard),
            "{}: the arg-bearing call is not guarded by the typed-payload tag. An event \
             that arrives without its typed payload carries nothing the arguments can \
             be read from; firing anyway hands the host a zero value it would take for \
             data. Expected to find:\n  {}",
            e.lang,
            e.payload_guard
        );
    }
}

/// The arm an unreadable delivery takes must SAY so, identically, on all six.
///
/// Two deliveries reach it and only one is a host mistake: a host reaching past
/// the generated typed inject, and the DOCUMENT'S OWN `<raise>` of a
/// payload-typed event — legal SCXML this generator accepts, so "blame the
/// caller" is not available as an answer. §scxml-3.12.2 supplies the one that
/// is: an error "arising from expression evaluation" is signalled as
/// `error.execution` on the internal event queue, which the document can
/// answer with a transition.
///
/// Measured on 2026-08-24, the six did not agree. Five skipped in silence and
/// Rust alone aborted through a `debug_assert!` that `--release` compiled away,
/// so one legal document either killed a development build or did nothing at
/// all depending on the profile. This test is what makes that a red rather than
/// something only a reader would notice.
///
/// The shared message is asserted alongside the per-backend spelling because
/// six wordings of one failure is the OTHER way a single contract turns back
/// into six, and it is a drift this codebase has already paid for once.
#[test]
fn every_backend_answers_an_unreadable_argument_with_error_execution() {
    let all = expectations();
    // Lower bound. A sweep that lost rows reports "every backend agrees" by
    // asking fewer of them, and an empty loop is indistinguishable from a pass.
    assert_eq!(
        all.len(),
        6,
        "§scxml-G-7 is lowered by six backends; this table stopped asking all of them"
    );

    for e in all {
        let files = generate(e.lang, e.files, "native_action_parity_unreadable_arg");
        let text = emitted(&files);
        assert!(
            text.contains(e.error_arm),
            "{}: an unreadable argument is not answered with error.execution. Silence \
             here is the seam deciding, on the host's behalf, that a failure it can see \
             is not worth reporting — and the document has a transition for it. \
             Expected to find:\n  {}",
            e.lang,
            e.error_arm
        );
        assert!(
            text.contains(UNREADABLE_ARG_MESSAGE),
            "{}: the raise carries a different message from the other backends. One \
             failure, one wording — a per-backend phrasing is how a shared contract \
             becomes six. Expected to find:\n  {}",
            e.lang,
            UNREADABLE_ARG_MESSAGE
        );
    }
}

/// The construct is engine-free BY DEFINITION — it never degrades to a runtime
/// fallback — so a document whose every effect is a native action needs no
/// script engine on ANY backend.
///
/// Read off the manifest rather than inferred from the emitted text: an absent
/// substring is not evidence, and `needs_script_engine` is the field a build
/// system actually consults. The C11 channel pays the same claim a second way,
/// by linking its target without the Lua libraries at all.
#[test]
fn native_action_alone_needs_no_script_engine_on_any_backend() {
    for e in expectations() {
        let (_, result) = generate_into(e.lang, "native_action_backend_parity_manifest");
        let stdout = String::from_utf8_lossy(&result.stdout);
        let line = stdout
            .lines()
            .last()
            .unwrap_or_else(|| panic!("{}: no manifest on stdout", e.lang));
        let manifest: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|err| panic!("{}: manifest is not JSON ({err}): {line}", e.lang));

        assert_eq!(
            manifest.get("needs_script_engine"),
            Some(&serde_json::Value::Bool(false)),
            "{}: a document whose every effect is a `<sce:action>` reported that it \
             needs a script engine. The construct is engine-free by definition and \
             never degrades to a runtime fallback, so this is the lowering having \
             reached for one.\nmanifest: {line}",
            e.lang
        );
    }
}
