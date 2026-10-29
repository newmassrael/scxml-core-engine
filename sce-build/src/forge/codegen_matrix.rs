// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! SCE Forge — codegen kind × language matrix (RFC §synth-5-J-4 / §synth-5-J-5).
//!
//! Single source of truth for which `(ForgeKind, Language)` pairs are
//! emit-eligible per the SCE Protocol-Synthesis RFC §synth-5-J-4 matrix. Two
//! invariants live here:
//!
//! 1. Every `ForgeKind` declares a `KindClass` (`Generic` or
//!    `McuClass`). Compile-time exhaustive over the enum — adding a
//!    kind variant without classifying it fails `cargo check`.
//! 2. Every emit-eligible `(kind, lang)` pair has a shipped Jinja2
//!    template in `tools/codegen/templates/forge/<lang>/`. Generic
//!    kinds list every backend explicitly so a missing template is a
//!    matrix edit, never a silent skip.
//!
//! The walker turns matrix violations into the two A1 diagnostics:
//! - `codegen/mcu-class-kind-on-non-mcu-language` — MCU-class kind
//!   targeting `{cpp, kotlin, go, python}`.
//! - `codegen/generic-kind-backend-emit-missing` — generic-class kind
//!   whose backend template has not yet shipped.
//!
//! Trust-boundary rationale: SCE owns the contract, so this matrix
//! is declared in SCE code (X form). It extends the existing
//! `SIX_BACKENDS` const at
//! `sce-build/tests/forge_conformance.rs` from a backend-only list to
//! a `(kind, backend)` pair set.

use crate::forge::error::GenerateError;
use crate::forge::model::{ForgeKind, QueueAlgorithm, QueueProgress};
use crate::generator::Language;

/// Class of a kind per RFC §synth-5-J-4. Drives which `(kind, language)`
/// pairs are reachable; combined with `template_ships` it determines
/// which A1 diagnostic fires when an unreachable pair is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindClass {
    /// Emits on all six backends. Missing template is an SCE bug
    /// (`codegen/generic-kind-backend-emit-missing`).
    Generic,
    /// Protocol-synthesis primitive whose emitter shape is only
    /// meaningful on `(rust, c11)`. Authoring against any other
    /// language is `codegen/mcu-class-kind-on-non-mcu-language`.
    McuClass,
    /// A document read by the documents that import it, with nothing of its
    /// own to emit on any backend: generating one writes no file, and that
    /// is not a missing template. The feature model of ADR 0057 §2 is one:
    /// its importers' conditions read it, and the code that relies on its
    /// *requires* edges carries their check itself.
    Declaration,
}

/// Outcome of `lookup` for a `(kind, language)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitOutcome {
    /// Template ships on this backend and the pair is on-policy.
    Emit,
    /// Generic-class kind whose backend template is absent.
    TemplateMissing,
    /// MCU-class kind targeting a non-MCU language.
    McuClassOnNonMcuLanguage,
    /// A [`KindClass::Declaration`]: generation succeeds and writes nothing.
    Declaration,
    /// A kind whose document is valid on every backend and whose runtime row has
    /// not landed on this one ([`awaits_runtime`]): [`check`] lets it through to
    /// the dispatch, which refuses the document by name with the kind's own code
    /// (`broadcast-log/runtime-missing`). Not an emitting backend, so
    /// [`emitting`] and the catalog's `backends` leave it out. A backend whose
    /// row and template have landed answers [`EmitOutcome::Emit`] for the same
    /// kind.
    RuntimeMissing,
}

/// Classify a kind per the §synth-5-J-4 matrix.
///
/// Exhaustive over `ForgeKind` so a new variant fails `cargo check`
/// until classified — same lock-in pattern as `ForgeKind::ALL_ATTR_NAMES`.
pub const fn kind_class(kind: ForgeKind) -> KindClass {
    match kind {
        ForgeKind::Statechart
        | ForgeKind::Transform
        | ForgeKind::Lookup
        | ForgeKind::Condition
        | ForgeKind::Codec
        | ForgeKind::Procedure
        | ForgeKind::Validator
        | ForgeKind::Filter
        | ForgeKind::Interpolation
        | ForgeKind::Timer
        | ForgeKind::Observer
        | ForgeKind::Algorithm
        // RFC §synth-5-L lines 2566-2581: BoundedCollection ships on all 6
        // backends with language-builtin primitives (no MCU-specific
        // hardware constraint like DMA section/cache for BufferPool, no
        // cross-core atomics constraint like Worker). Generic class.
        | ForgeKind::BoundedCollection
        // Enum lowers to a backend-native typed
        // enum on every language (`enum class : <int>` / `#[repr(<int>)]
        // enum` / `IntEnum` / etc.) — no MCU-specific constraint.
        // Generic class.
        | ForgeKind::Enum
        // EventSchema lowers to a backend-native
        // payload struct on every language (`struct <S>Payload` /
        // `pub struct <S>Payload` / `data class <S>Payload` / etc.) —
        // typed-Enum fields resolve through the Enum kind's emitted
        // type via the import context. No MCU-specific hardware
        // constraint; Generic class matches Enum's stance.
        | ForgeKind::EventSchema
        // RFC §synth-5-P: a Queue is specified for all six backends (Python
        // at `blocking` only). It is Generic, so a backend whose runtime has
        // not landed yet is `codegen/generic-kind-backend-emit-missing`.
        | ForgeKind::Queue
        // ADR 0062: a broadcast log is specified for all six backends (Python
        // at `blocking` only). A backend whose runtime row or template has not
        // landed yet is `broadcast-log/runtime-missing`, not a missing
        // template ([`awaits_runtime`]).
        | ForgeKind::BroadcastLog => KindClass::Generic,
        // RFC §synth-5-C / §synth-5-J-4: Link is the first MCU-class kind.
        // Authoring it on cpp/kotlin/go/python raises
        // `codegen/mcu-class-kind-on-non-mcu-language`. The (rust, *)
        // and (c11, bare_metal) substrate is in `sce-link-runtime`
        // (rust trait + per-OS downstream impls) and the c11 lower.
        ForgeKind::Link => KindClass::McuClass,
        // RFC §synth-5-E / §synth-5-J-4: BufferPool is the second MCU-class kind.
        // Same matrix as Link: `(rust, *)` + `(c11, bare_metal)` only;
        // cpp/kotlin/go/python rejected via the same diagnostic; rust
        // and c11 templates both ship.
        ForgeKind::BufferPool => KindClass::McuClass,
        // RFC §synth-5-D / §synth-5-J-4: Worker is the third MCU-class kind.
        // Same matrix as Link + BufferPool: `(rust, *)` + `(c11, bare_metal)`
        // only; cpp/kotlin/go/python rejected via the same diagnostic;
        // Worker dual-emits rust + c11 codegen.
        ForgeKind::Worker => KindClass::McuClass,
        // ADR 0057 §2: the feature model is read by its importers' conditions.
        ForgeKind::FeatureModel => KindClass::Declaration,
    }
}

/// Whether the per-`(kind, language)` Jinja2 template has shipped.
///
/// Exhaustive over both axes. The 11 baseline kinds shipped on all 6
/// backends at `758aea3f` (commit "close C11 byte-golden parity with
/// 5 backends"). When a new kind variant lands in `ForgeKind`, a new
/// outer `match` arm must list every `Language` arm — silent
/// fall-through to a default branch is forbidden, otherwise a kind
/// can ship a template on backend A while the matrix records it as
/// missing on A.
pub const fn template_ships(kind: ForgeKind, lang: Language) -> bool {
    match kind {
        ForgeKind::Statechart
        | ForgeKind::Transform
        | ForgeKind::Lookup
        | ForgeKind::Condition
        | ForgeKind::Codec
        | ForgeKind::Procedure
        | ForgeKind::Validator
        | ForgeKind::Filter
        | ForgeKind::Interpolation
        | ForgeKind::Timer
        | ForgeKind::Observer => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => true,
            Language::Swift => false,
        },
        // RFC §synth-5-A Algorithm: closed across all six backends.
        // Item A3 landed Rust + Cpp; item A5 closed C11 (RFC §synth-7
        // line 4031); a follow-up trio added Go + Kotlin + Python
        // in three independent commits.
        ForgeKind::Algorithm => match lang {
            Language::Rust
            | Language::Cpp
            | Language::C11
            | Language::Go
            | Language::Kotlin
            | Language::Python => true,
            Language::Swift => false,
        },
        // RFC §synth-5-C Link: rust and c11 templates ship.
        // cpp/kotlin/go/python are MCU-class-rejected by `kind_class`
        // ahead of this lookup, so their `template_ships` value never
        // enters the diagnostic flow — `false` here documents "no
        // shipped template" without falsely claiming emission.
        ForgeKind::Link => match lang {
            Language::Rust | Language::C11 => true,
            Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::Swift => false,
        },
        // RFC §synth-5-E BufferPool: rust and c11 templates ship (c11:
        // `__attribute__((section, aligned))` storage table +
        // sidecar linker fragment + section attribute round-trip).
        // cpp/kotlin/go/python are MCU-class-rejected by `kind_class`
        // ahead of this lookup — `false` documents "no shipped
        // template" without falsely claiming emission.
        ForgeKind::BufferPool => match lang {
            Language::Rust | Language::C11 => true,
            Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::Swift => false,
        },
        // RFC §synth-5-D Worker: dual-emit codegen on Rust + C11. The worker's
        // inbox is the queue it imports (`<sce:inbox ref>`, RFC §synth-5-P
        // Migration): the Rust template re-exports that queue's type as the
        // worker's inbox, and the C11 template includes the queue's header and
        // names its type, so neither carries a ring of its own and the memory
        // ordering is the queue algorithm's. cpp/kotlin/go/python remain
        // MCU-class-rejected by `kind_class` ahead of this lookup.
        ForgeKind::Worker => match lang {
            Language::Rust | Language::C11 => true,
            Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::Swift => false,
        },
        // RFC §synth-5-L BoundedCollection: the 6-backend codegen matrix is
        // closed per spec lines 2571-2581. `Rust` ships (heapless::Vec
        // <T, N> std / no_std + 16/16 packed Handle u32 + insert/remove/
        // get/find_by_index/iter/len/capacity per spec lines 2609-2622),
        // `Cpp` (std::array + std::bitset + uint32_t generation
        // + forward-iterator begin/end per spec line 2576-2577) + `Kotlin`
        // (Array<T?>(N) + BooleanArray + IntArray generation + Iterable<T>
        // per spec line 2578), `Go` ([N]T + [N]bool + [N]uint32
        // + ForEach callback per spec line 2579) + `Python` (list[Optional
        // [T]] + bytearray mask + bytearray gen + __iter__ + Optional[H]
        // overflow per spec lines 2580-2581) + `C11` (struct slots[N] +
        // generation[N] + bitmap[(N+31)/32] + count with snake_case
        // _insert/_remove/_get/_find_by_index/_foreach API per spec
        // lines 2573-2575). All 6 backends emit; the §synth-5-L matrix is
        // entirely closed. Swift has no emitter yet.
        ForgeKind::BoundedCollection => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => true,
            Language::Swift => false,
        },
        // Enum ships on all 6 backends
        // via `tools/codegen/templates/forge/<lang>/enum.<ext>.jinja2`
        // — `generator.rs::render_enum` lowers `EnumModel` to a typed
        // backend-native enum. Matches the
        // §synth-5-J-4 Generic-class contract; matrix dispatch lets every
        // `(Enum, lang)` pair through to the per-language render arm. Swift
        // has no emitter yet.
        ForgeKind::Enum => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => true,
            Language::Swift => false,
        },
        // EventSchema ships on all 6
        // backends via `tools/codegen/templates/forge/<lang>/event_schema.<ext>.jinja2`
        // — `generator.rs::render_event_schema` lowers
        // `EventSchemaModel` to a per-backend payload struct
        // (`struct <Schema>Payload` / `data class <Schema>Payload` /
        // `@dataclass` / `typedef struct ... <Schema>Payload_t`).
        // Matches the §synth-5-J-4 Generic-class contract; matrix dispatch
        // lets every `(EventSchema, lang)`
        // pair through to the per-language render arm. Swift writes the kind in
        // part ([`ships_in_part`]): a struct of scalar fields.
        ForgeKind::EventSchema => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => true,
            Language::Swift => false,
        },
        // RFC §synth-5-P Queue: the algorithms live once per language in
        // that language's forge runtime (SCE_FORGE.md §2.1), so a backend
        // can emit a queue only once its runtime has them. Rust's has
        // (`sce_forge_runtime::queue`: the Lamport ring and SCQ) and so has
        // C++'s (`sce/forge/queue.h`: the same two), Go's
        // (`sce-forge-runtime/queue`: the same two) and Kotlin's
        // (`com.sce.forge.runtime.queue`: the same two) and Python's
        // (`sce_forge_runtime.queue`: one ring under one lock, `blocking`) and
        // C11's (`sce/forge/queue.h`: the Lamport ring and SCQ over 64-bit and
        // 32-bit entries, and `queue_irq.h` for a target with no
        // read-modify-write atomic). Swift has no forge runtime yet, so it has
        // none either.
        ForgeKind::Queue => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Go
            | Language::Kotlin
            | Language::Python
            | Language::C11 => true,
            Language::Swift => false,
        },
        // A declaration has no template on any backend: there is nothing to render.
        ForgeKind::FeatureModel => match lang {
            Language::Rust
            | Language::Cpp
            | Language::Go
            | Language::Kotlin
            | Language::Python
            | Language::C11
            | Language::Swift => false,
        },
        // ADR 0062 BroadcastLog: the algorithm lives once per language in that
        // language's forge runtime, and the template that stores a document's
        // element over it ships per language (the order of landing is Rust,
        // then C11 and C++, then Go, Kotlin and Python; Swift stays refused).
        // Rust's has landed (`forge/rust/broadcast_log.rs.jinja2`, over
        // `sce_forge_runtime::broadcast_log`); each other backend flips to
        // `true` in the change that gives it the template, and
        // [`awaits_runtime`] stops answering for it then.
        ForgeKind::BroadcastLog => match lang {
            Language::Rust => true,
            Language::Cpp
            | Language::Go
            | Language::Kotlin
            | Language::Python
            | Language::C11
            | Language::Swift => false,
        },
    }
}

/// Whether `kind` is a valid kind whose absence of a template on `lang` is a
/// runtime row that has not landed there yet, and not a defect of the generator.
///
/// A [`KindClass::Generic`] kind with no template on a backend is, by default,
/// `codegen/generic-kind-backend-emit-missing`: the matrix says the backend
/// should have emitted it, and none does. A kind that is parsed and checked
/// before a language's runtime has its row is not that: its document is valid
/// and the refusal is the kind's own code, which names the document and the
/// language (`broadcast-log/runtime-missing`, ADR 0062). [`lookup`] answers
/// [`EmitOutcome::RuntimeMissing`] for it, and the generator's dispatch builds
/// the refusal where the document is in hand.
///
/// The answer is per language because the rows land one language at a time: a
/// language whose template ships ([`template_ships`]) is out of it, and the
/// kind stops being answered for at all when the last language has landed.
pub const fn awaits_runtime(kind: ForgeKind, lang: Language) -> bool {
    match kind {
        ForgeKind::BroadcastLog => match lang {
            Language::Rust => false,
            Language::Cpp
            | Language::Go
            | Language::Kotlin
            | Language::Python
            | Language::C11
            | Language::Swift => true,
        },
        _ => false,
    }
}

/// Whether the template for `(kind, lang)` ships but lowers only part of what the
/// kind admits, refusing the rest by name: a backend on its way to
/// [`template_ships`], that already writes what it lowers.
///
/// Kept apart from [`template_ships`] on purpose. That one is the claim that a
/// backend writes the kind the way every other does, and a check that holds every
/// backend to what one of them writes ([`emitting`]) is right to include the
/// backend only when the claim is true: a partial backend refusing a document the
/// others accept would turn every such check red for a reason that is not a defect.
/// What a partial backend does lower is held by the checks written for it
/// ([`emitting_in_part`]), and a backend moves to [`template_ships`] when nothing
/// it refuses is left.
///
/// Swift's statechart lowers `datamodel="null"` documents so far
/// (`generator::reject_swift_unsupported_features` names each construct it does
/// not yet), its algorithm a body of scalars
/// (`forge::generator::swift_algorithm_unlowered` names each construct past
/// those), and its event schema a struct of scalar fields
/// (`forge::generator::swift_event_schema_unlowered` names each field past those).
pub const fn ships_in_part(kind: ForgeKind, lang: Language) -> bool {
    matches!(
        (kind, lang),
        (ForgeKind::Statechart, Language::Swift)
            | (ForgeKind::Algorithm, Language::Swift)
            | (ForgeKind::EventSchema, Language::Swift)
    )
}

/// The backends that write `kind` in whole or in part, in [`Language::ALL`] order:
/// [`emitting`] and the backends [`ships_in_part`] names.
pub fn emitting_in_part(kind: ForgeKind) -> impl Iterator<Item = Language> {
    Language::ALL.iter().copied().filter(move |lang| {
        matches!(lookup(kind, *lang), EmitOutcome::Emit) || ships_in_part(kind, *lang)
    })
}

/// Walk the matrix for `(kind, language)`. Returns `EmitOutcome::Emit`
/// when the template ships and the pair is on-policy; otherwise the
/// outcome names the §synth-5-J-4 violation.
pub const fn lookup(kind: ForgeKind, lang: Language) -> EmitOutcome {
    match kind_class(kind) {
        KindClass::McuClass => match lang {
            Language::Rust | Language::C11 => {
                if template_ships(kind, lang) {
                    EmitOutcome::Emit
                } else {
                    EmitOutcome::TemplateMissing
                }
            }
            Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::Swift => EmitOutcome::McuClassOnNonMcuLanguage,
        },
        KindClass::Generic => {
            if template_ships(kind, lang) {
                EmitOutcome::Emit
            } else if awaits_runtime(kind, lang) {
                EmitOutcome::RuntimeMissing
            } else {
                EmitOutcome::TemplateMissing
            }
        }
        KindClass::Declaration => EmitOutcome::Declaration,
    }
}

/// The backends that emit `kind`, in [`Language::ALL`] order: the ones
/// [`lookup`] answers [`EmitOutcome::Emit`] for.
///
/// A check that holds every backend to what one of them writes iterates this,
/// not [`Language::ALL`]: a backend with no emitter yet is not one of the
/// backends that write it, and is refused by [`check`] instead, which is a
/// check of its own over the backends this leaves out.
pub fn emitting(kind: ForgeKind) -> impl Iterator<Item = Language> {
    Language::ALL
        .iter()
        .copied()
        .filter(move |lang| matches!(lookup(kind, *lang), EmitOutcome::Emit))
}

/// Whether `lang` emits at least one kind: the backends a run that sweeps "every
/// backend" means, in [`Language::ALL`] order, as opposed to the ones that are
/// known and render nothing yet.
pub fn emits_anything(lang: Language) -> bool {
    emitting(ForgeKind::Statechart).any(|emits| emits == lang)
        || ForgeKind::ALL_ATTR_NAMES
            .iter()
            .filter_map(|name| ForgeKind::from_attr(name))
            .any(|kind| matches!(lookup(kind, lang), EmitOutcome::Emit))
}

/// Whether `lang` writes anything at all, in whole or in part: [`emits_anything`],
/// or a kind in part ([`ships_in_part`]).
///
/// What a route that writes files serves, and what a refusal means by a backend
/// that "emits nothing yet": a backend that writes a statechart in part is asked
/// to generate one, and refuses by the construct what it does not lower. A sweep
/// over "every backend" ([`emits_anything`]) leaves it out until nothing it
/// refuses is left, since a verdict that refuses most documents says nothing
/// about the document.
pub fn writes_something(lang: Language) -> bool {
    emits_anything(lang)
        || ForgeKind::ALL_ATTR_NAMES
            .iter()
            .filter_map(|name| ForgeKind::from_attr(name))
            .any(|kind| ships_in_part(kind, lang))
}

/// Reject `(kind, lang)` with the matching `GenerateError` if the
/// matrix walker says the pair is unreachable; return `Ok(())` when
/// emit-eligible. Dispatch sites in `forge/generator.rs` call this
/// before routing to the per-kind `render_*` helper.
///
/// A kind a backend writes in part ([`ships_in_part`]) passes: its renderer
/// refuses by name, with the document in hand, what it does not lower yet.
pub fn check(kind: ForgeKind, lang: Language) -> Result<(), GenerateError> {
    if ships_in_part(kind, lang) {
        return Ok(());
    }
    match lookup(kind, lang) {
        // A declaration passes: each dispatcher's arm for it writes no file.
        // A kind that awaits its runtime passes too: each dispatcher's arm for
        // it refuses the document by name, which only the dispatcher can do,
        // since the document is not in hand here.
        EmitOutcome::Emit | EmitOutcome::Declaration | EmitOutcome::RuntimeMissing => Ok(()),
        EmitOutcome::TemplateMissing => Err(emit_missing(kind, lang)),
        EmitOutcome::McuClassOnNonMcuLanguage => {
            Err(GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                kind: kind.to_string(),
                language: language_wire_name(lang).to_string(),
            })
        }
    }
}

/// The refusal a dispatch site answers with for a backend that has no emitter
/// for `kind` at all: `codegen/generic-kind-backend-emit-missing`.
///
/// [`check`] builds it from the matrix. A dispatch `match` over `Language`
/// whose arm for a backend with no emitter yet has nothing to route to builds
/// it here instead, so the refusal is the diagnostic the matrix gives and not a
/// panic; the arm is replaced by the real route in the commit that makes
/// [`template_ships`] answer `true` for the pair.
pub fn emit_missing(kind: ForgeKind, lang: Language) -> GenerateError {
    GenerateError::CodegenGenericKindBackendEmitMissing {
        kind: kind.to_string(),
        language: language_wire_name(lang).to_string(),
    }
}

/// What an arm for Swift inside a renderer's helper answers.
///
/// Swift emits no kind whole yet ([`template_ships`] is `false` for every
/// kind). [`check`] refuses each kind it does not write at all at the dispatch
/// site, and a kind it writes in part ([`ships_in_part`]) is refused by name,
/// construct by construct, before its renderer reaches a helper for one it does
/// not lower. Those helpers are therefore never handed Swift, but each is a
/// `match` the compiler makes name the backend, so the arm exists and says why
/// it is not a route. A helper that is handed Swift is a dispatch site, or an
/// in-part refusal, that let a construct through, and the panic names it.
///
/// The arms are replaced by real ones as the Swift emitters land, one construct
/// at a time.
pub(crate) fn no_swift_emitter(site: &str) -> ! {
    unreachable!(
        "{site}: swift has no lowering here, and codegen_matrix::check or the kind's \
         in-part refusal names what reaches it before a renderer runs"
    )
}

/// SCE Protocol-Synthesis RFC §5.2 MCU section-attribute reject: when
/// `deploy.yaml`'s `platform.c11_section_attribute` is present but the
/// target codegen backend is not C11, surface
/// `mcu/section-attribute-on-non-mcu-target`. The section attribute
/// injects `__attribute__((section("...")))` which has no equivalent
/// emit on the non-MCU backends (cpp / rust / kotlin / go / python);
/// silently dropping it would let the directive vanish on a non-C11
/// compile, matching the §synth-5-I extern-sidecar non-MCU reject pattern.
///
/// Caller-supplied flag: the section attribute lives on `deploy.yaml`
/// not on `ForgeKind`, so this helper takes a boolean rather than a
/// kind. Co-located with `check` so the codegen-matrix module owns
/// every "this target rejects feature X" gate in one place.
///
/// `class` is the declared section name when the block carries one. It
/// is validated here rather than at deploy-parse time because this is
/// the gate the C11 emission path already runs through, and the name's
/// constraint comes from how the emitter spells it — see
/// [`GenerateError::McuSectionAttributeNameInvalid`].
pub fn check_c11_section_attribute(
    section_attribute_present: bool,
    class: Option<&str>,
    lang: Language,
) -> Result<(), GenerateError> {
    if !section_attribute_present {
        return Ok(());
    }
    if !matches!(lang, Language::C11) {
        return Err(GenerateError::McuSectionAttributeOnNonMcuTarget {
            backend: language_wire_name(lang).to_string(),
        });
    }
    if let Some(name) = class {
        if let Some(reason) = section_name_rejection(name) {
            return Err(GenerateError::McuSectionAttributeNameInvalid {
                name: name.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Why `name` cannot be emitted as a section name, or `None` when it
/// can. Named per-character so the author sees which one to remove
/// rather than a blanket "invalid".
fn section_name_rejection(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("the name is empty".to_string());
    }
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '$' | '-');
        if !ok {
            return Some(match c {
                '"' => "it contains a double quote, which would close the \
                        string literal the name is emitted into"
                    .to_string(),
                '\\' => "it contains a backslash, which would start an escape \
                         sequence in the string literal the name is emitted into"
                    .to_string(),
                c if c.is_whitespace() => {
                    format!("it contains whitespace (U+{:04X})", c as u32)
                }
                c if c.is_control() => {
                    format!("it contains a control character (U+{:04X})", c as u32)
                }
                c => format!("it contains `{c}`"),
            });
        }
    }
    None
}

/// Stable wire-name for a `Language`, matching `Language::from_str`
/// accept-set so diagnostic `actual` values round-trip cleanly.
/// Wire-format name for a backend.
///
/// The single spelling table for the six backends: the codegen matrix's
/// own diagnostics, `lib.rs`'s `codegen/mcu-class-kind-on-non-mcu-language`
/// family, and the stdout manifest's per-language verdict all read it
/// here. A second table would be free to disagree — and did, until this
/// one absorbed it.
pub const fn language_wire_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::Cpp => "cpp",
        Language::Kotlin => "kotlin",
        Language::Go => "go",
        Language::Python => "python",
        Language::C11 => "c11",
        Language::Swift => "swift",
    }
}

/// The strongest progress a target can give any `queue` document (SCE
/// Protocol-Synthesis RFC §synth-5-P, Backends), and why it gives no more when it
/// gives less than the selection table's strongest. The selection table judges a
/// document against the algorithm its storage and cardinality pick
/// (`queue/progress-unreachable`); this is the judgement against the platform,
/// which no choice of storage can lift:
///
/// - Python has no compare-and-swap primitive, so its queue is one ring under one
///   lock and gives `blocking`.
/// - A C11 target whose `platform.atomic_rmw_width` is 0 has no read-modify-write
///   atomic, so every row runs in an interrupt-masked critical section and gives
///   `blocking`.
///
/// Rust, C++, Kotlin and Go take nothing from the platform here, because none of
/// them lowers a row on the deploy's width: C11 alone has a critical-section form,
/// chosen when `platform.atomic_rmw_width` is 0. Their output checks the atomics a
/// row needs where the target is built or run (the RFC's Backends paragraph), so
/// a row they give is the row the selection table already judged. A Rust row its
/// target has no atomic for is refused at generation by
/// [`rust_runtime_row_atomics`] (`queue/storage-runtime-missing`), not weakened
/// here.
///
/// `atomic_rmw_width` is the deploy's `platform.atomic_rmw_width` in bits, `None`
/// when the deploy states none (or there is no deploy), and only C11 lowers on it.
pub const fn queue_progress_ceiling(
    lang: Language,
    atomic_rmw_width: Option<u32>,
) -> (QueueProgress, &'static str) {
    match lang {
        Language::Python => (
            QueueProgress::Blocking,
            "it has no compare-and-swap primitive",
        ),
        Language::C11 => match atomic_rmw_width {
            Some(0) => (
                QueueProgress::Blocking,
                "platform.atomic_rmw_width is 0, so every row runs in an interrupt-masked critical section",
            ),
            _ => (QueueProgress::WaitFree, ""),
        },
        // Swift emits no queue yet (`template_ships`), so this answer is never
        // read for it; it stays with the backends that take nothing from the
        // platform until the Swift runtime's atomics decide it.
        Language::Rust | Language::Cpp | Language::Kotlin | Language::Go | Language::Swift => {
            (QueueProgress::WaitFree, "")
        }
    }
}

/// The strongest progress a target can give any `broadcast-log` document (ADR 0062
/// §4, Per backend). The selection table judges a document against the row its
/// producer cardinality picks (`broadcast-log/progress-unreachable`); this is the
/// judgement against the language, which no document can lift:
///
/// - Python keeps one ring under one lock, so its log gives `blocking`, the ceiling
///   [`queue_progress_ceiling`] gives it too.
/// - Every other backend's row is built from atomic loads and stores of one stamp
///   and one word, with no read-modify-write atomic, so a target with only those
///   (a C11 target whose `platform.atomic_rmw_width` is 0 included) gives
///   `wait-free`. A deploy's width does not enter, unlike the queue's.
///
/// Swift has no runtime yet (`template_ships`), so its answer is never read; it
/// stays with the backends that take nothing from the platform until its atomics
/// decide it.
pub const fn broadcast_log_progress_ceiling(lang: Language) -> (QueueProgress, &'static str) {
    match lang {
        Language::Python => (
            QueueProgress::Blocking,
            "its log is one ring under one lock",
        ),
        Language::Rust
        | Language::Cpp
        | Language::Kotlin
        | Language::Go
        | Language::C11
        | Language::Swift => (QueueProgress::WaitFree, ""),
    }
}

/// The read-modify-write atomic a row of the Rust forge runtime needs, as the
/// widest `platform.atomic_rmw_width` the deploy can state that has it, and what the
/// row does with it. The Rust runtime has no critical-section form of any row (C11 has
/// one, for a single core), so a row it cannot build for a target is refused when the
/// deploy states the target, and not left to be found as a module that is not there:
///
/// - the Lamport ring loads and stores, and is on every target;
/// - the Vyukov list swaps the tail of a `u32` link, and the linked Lamport rings
///   compare-and-swap the flag that hands out each of their two handles, which a target
///   whose atomics are loads and stores alone (ARMv6-M, width 0) has neither of;
/// - SCQ and LSCQ keep 64-bit entries, which a 32-bit target does not have.
///
/// A deploy that states no width says nothing about the target, and the generated Rust
/// carries a `compile_error!` for each of these where the target lacks the atomic.
pub const fn rust_runtime_row_atomics(algorithm: QueueAlgorithm) -> (u32, &'static str) {
    match algorithm {
        QueueAlgorithm::LamportRing => (0, ""),
        QueueAlgorithm::VyukovMpsc | QueueAlgorithm::VyukovListWithConsumerFlag => (
            32,
            "the list swaps the tail of its 32-bit links, and a target whose atomics are loads and stores alone has no swap",
        ),
        QueueAlgorithm::LinkedLamportRings => (
            32,
            "the rings' two handles are each taken once by a compare-and-swap of a flag, which a target whose atomics are loads and stores alone does not have",
        ),
        QueueAlgorithm::Scq | QueueAlgorithm::Lscq => (
            64,
            "the ring's entries are 64-bit atomics, which a target whose widest read-modify-write atomic is smaller does not have",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Rust runtime's rows against the widths a deploy can state: every row but the Lamport
    /// ring needs a read-modify-write atomic, and the ring-based rows need 64 bits.
    #[test]
    fn the_rust_runtime_says_what_atomic_each_row_needs() {
        for width in [0u32, 32, 64] {
            let ring = rust_runtime_row_atomics(QueueAlgorithm::LamportRing);
            assert_eq!(ring.0, 0, "the Lamport ring loads and stores");
            assert!(width >= ring.0, "{width}");
        }
        for algorithm in [
            QueueAlgorithm::VyukovMpsc,
            QueueAlgorithm::VyukovListWithConsumerFlag,
            QueueAlgorithm::LinkedLamportRings,
        ] {
            let (needs, why) = rust_runtime_row_atomics(algorithm);
            assert_eq!(needs, 32, "{algorithm:?}");
            assert!(!why.is_empty(), "{algorithm:?}");
        }
        for algorithm in [QueueAlgorithm::Scq, QueueAlgorithm::Lscq] {
            let (needs, why) = rust_runtime_row_atomics(algorithm);
            assert_eq!(needs, 64, "{algorithm:?}");
            assert!(!why.is_empty(), "{algorithm:?}");
        }
    }

    /// All 11 baseline kinds emit on every backend that emits any of them
    /// (all six at `758aea3f`). A backend ships the baseline whole or not at
    /// all: one that emitted some kinds and not others would be a half-shipped
    /// backend, and is the shape this refuses.
    #[test]
    fn baseline_kinds_emit_on_all_backends() {
        // `Language::ALL` rather than a local restatement: a new backend
        // joins this coverage claim automatically.
        const ALL_LANGS: &[Language] = Language::ALL;
        const BASELINE_KINDS: &[ForgeKind] = &[
            ForgeKind::Statechart,
            ForgeKind::Transform,
            ForgeKind::Lookup,
            ForgeKind::Condition,
            ForgeKind::Codec,
            ForgeKind::Procedure,
            ForgeKind::Validator,
            ForgeKind::Filter,
            ForgeKind::Interpolation,
            ForgeKind::Timer,
            ForgeKind::Observer,
        ];
        for &k in BASELINE_KINDS {
            assert_eq!(kind_class(k), KindClass::Generic, "kind {k:?} class");
        }
        let mut whole = 0usize;
        for &l in ALL_LANGS {
            let emits: Vec<bool> = BASELINE_KINDS
                .iter()
                .map(|&k| lookup(k, l) == EmitOutcome::Emit)
                .collect();
            assert!(
                emits.iter().all(|e| *e) || emits.iter().all(|e| !*e),
                "{l:?} emits only some of the baseline kinds: {emits:?}"
            );
            for &k in BASELINE_KINDS {
                // A kind the backend writes in part is refused by its renderer,
                // construct by construct, and not by the matrix.
                if emits[0] || ships_in_part(k, l) {
                    assert!(check(k, l).is_ok(), "check({k:?}, {l:?})");
                } else {
                    assert!(check(k, l).is_err(), "check({k:?}, {l:?}) admitted");
                }
            }
            whole += usize::from(emits[0]);
        }
        assert!(whole > 0, "no backend emits the baseline");
    }

    /// RFC §synth-5-A §synth-5-J-4 matrix closure: Algorithm ships on every
    /// backend that emits a machine, after the post-A6 Go / Kotlin / Python
    /// follow-up trio. No such backend remains gated by
    /// `codegen/generic-kind-backend-emit-missing`.
    #[test]
    fn algorithm_kind_ships_on_all_backends() {
        assert_eq!(kind_class(ForgeKind::Algorithm), KindClass::Generic);
        for lang in emitting(ForgeKind::Statechart) {
            assert_eq!(
                lookup(ForgeKind::Algorithm, lang),
                EmitOutcome::Emit,
                "({:?}, {:?}) should emit",
                ForgeKind::Algorithm,
                lang,
            );
            assert!(
                check(ForgeKind::Algorithm, lang).is_ok(),
                "check({:?}, {:?}) should succeed",
                ForgeKind::Algorithm,
                lang,
            );
        }
    }

    /// ADR 0062: a broadcast log is emitted by Rust, whose runtime row and
    /// template have landed, and every other language still awaits its
    /// runtime. The per-language answer of `awaits_runtime` and the matrix's
    /// `template_ships` are never both true for one pair, and a pair that
    /// ships is never the missing-runtime one.
    #[test]
    fn a_broadcast_log_is_emitted_by_rust_alone() {
        assert_eq!(
            lookup(ForgeKind::BroadcastLog, Language::Rust),
            EmitOutcome::Emit
        );
        assert!(template_ships(ForgeKind::BroadcastLog, Language::Rust));
        assert!(!awaits_runtime(ForgeKind::BroadcastLog, Language::Rust));
        for &lang in Language::ALL {
            if lang == Language::Rust {
                continue;
            }
            assert_eq!(
                lookup(ForgeKind::BroadcastLog, lang),
                EmitOutcome::RuntimeMissing,
                "{lang:?}"
            );
            assert!(!template_ships(ForgeKind::BroadcastLog, lang), "{lang:?}");
            assert!(awaits_runtime(ForgeKind::BroadcastLog, lang), "{lang:?}");
        }
        assert_eq!(
            emitting(ForgeKind::BroadcastLog).collect::<Vec<_>>(),
            vec![Language::Rust]
        );
    }

    /// No kind but the broadcast log awaits a runtime, and no pair that ships
    /// a template awaits one.
    #[test]
    fn only_a_pair_with_no_template_awaits_its_runtime() {
        for name in ForgeKind::ALL_ATTR_NAMES {
            let kind = ForgeKind::from_attr(name).expect("every attribute name parses");
            for &lang in Language::ALL {
                if awaits_runtime(kind, lang) {
                    assert_eq!(kind, ForgeKind::BroadcastLog, "{kind:?} on {lang:?}");
                    assert!(!template_ships(kind, lang), "{kind:?} on {lang:?}");
                }
            }
        }
    }

    #[test]
    fn language_wire_names_match_from_str_accept_set() {
        use std::str::FromStr;
        let cases = [
            (Language::Rust, "rust"),
            (Language::Cpp, "cpp"),
            (Language::Kotlin, "kotlin"),
            (Language::Go, "go"),
            (Language::Python, "python"),
            (Language::C11, "c11"),
            (Language::Swift, "swift"),
        ];
        for (lang, name) in cases {
            assert_eq!(language_wire_name(lang), name);
            assert_eq!(Language::from_str(name).unwrap(), lang);
        }
    }
}
