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
//!    An event raised by name carries no payload; firing anyway would hand the
//!    host a zero value it would take for data. The runtime channels measure
//!    the effect; what is measured here is that the guard is emitted at all,
//!    on every backend, from one lowering.
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
    /// `operation -> spelling`, in that language's convention.
    method: fn(&str) -> String,
}

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
            method: pascal,
        },
        Expectation {
            lang: "kotlin",
            files: &["statechart_native_actionSm.kt"],
            interface: "interface StatechartNativeActionActions {",
            host_seam: "private val actions: StatechartNativeActionActions,",
            receiver: "actions.",
            payload_guard: "pendingFragmentReceivedPayload?.let {",
            method: camel,
        },
        Expectation {
            lang: "python",
            files: &["statechart_native_action_sm.py"],
            interface: "class StatechartNativeActionActions(Protocol):",
            host_seam: "def __init__(self, actions: StatechartNativeActionActions) -> None:",
            receiver: "self._actions.",
            payload_guard: "if (_p := self._pending_fragment_received_payload) is not None:",
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
            method: snake,
        },
    ]
}

/// Run `sce-codegen generate` for `lang` into a per-language scratch directory.
///
/// The directory is derived from the language rather than randomised and is
/// emptied first, so a previous run's artifacts cannot be read as this one's.
/// `--no-format` keeps the emitted text the emitter's own: a formatter that is
/// absent on one machine would otherwise move the assertions' ground.
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
fn generate(lang: &str, files: &[&str]) -> BTreeMap<String, String> {
    let (out, _) = generate_into(lang, "native_action_backend_parity");
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
        let files = generate(e.lang, e.files);
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
        let files = generate(e.lang, e.files);
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
        let files = generate(e.lang, e.files);
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
        let files = generate(e.lang, e.files);
        let text = emitted(&files);
        assert!(
            text.contains(e.payload_guard),
            "{}: the arg-bearing call is not guarded by the typed-payload tag. An event \
             raised by NAME carries no payload; firing anyway hands the host a zero \
             value it would take for data, and the machine reaches the same state \
             either way so no configuration assertion can see it. Expected to \
             find:\n  {}",
            e.lang,
            e.payload_guard
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
