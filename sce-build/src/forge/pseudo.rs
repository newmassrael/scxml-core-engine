// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A pseudocode rendering of a forge document — the surface a
//! specification author reviews instead of the XML.
//!
//! ```text
//!   algorithm crc16(data: bytes, seed: uint16) -> uint16
//!     const POLY: uint16 = 0x1021
//!     var crc: uint16 = seed
//!     foreach b in data:
//!       crc = crc ^ b
//!     return crc
//! ```
//!
//! # Why this is not the review table
//!
//! [`crate::forge::review_table`] already renders a non-statechart kind
//! for a human, and this module does not replace it. That one is a
//! *projection*: one row per requirement-bearing node, sorted so that
//! behaviour nobody required collects into a `(none)` block. A
//! projection answers "what does this document claim" and is
//! deliberately not invertible.
//!
//! This one answers a different question — *does this document say what
//! the specification says* — and to answer it the rendering must be
//! **total**: every field of the model reaches the output, so that a
//! reader who approves the pseudocode has approved the document and not
//! a summary of it.
//!
//! # ⚠ Totality is the contract, and it is why a kind is refused
//!
//! A renderer that silently omits a field produces a text that reads
//! perfectly well and is not the document. The reviewer signs it, and
//! the generated code carries something they never saw. That failure is
//! worse than having no review surface at all, because it converts an
//! unreviewed document into a signed one.
//!
//! So a kind is either rendered in full or refused by name
//! ([`Unsupported`]) — never rendered partially. The `match` in
//! [`render`] lists every kind on its own line rather than collapsing
//! the remainder behind `_`, so a kind added to
//! [`ForgeDocument`](crate::forge::model::ForgeDocument) stops the build
//! until somebody decides which of the two it is. That is the same
//! discipline [`crate::forge::requirement_nodes`] states for itself.
//!
//! # Determinism
//!
//! The output is a pure function of the model: no clock, no path, no
//! environment, and no hash-ordered container. Two runs of one build on
//! one document write the same bytes, which is what makes a round-trip
//! comparison an instrument rather than a coincidence. The same contract
//! `docs/SCE_CODEGEN_DETERMINISM.md` states for codegen applies here,
//! minus the two pinned inputs — this renderer prints neither a stamp
//! nor a provenance path, so it has nothing to pin.
//!
//! # Opaque text
//!
//! Identifiers and expressions are opaque by contract: an id may be any
//! run of non-whitespace and an expression any string the datamodel
//! language accepts, including one carrying a newline written as
//! `&#10;`. A newline reaching the output would end a line that the
//! grammar says continues, so every author-supplied string passes
//! through [`crate::comment_text::encode`] — the encoder the generated
//! comments already use, whose [`crate::comment_text::decode`] is its
//! exact inverse. Sharing it rather than writing a second escape
//! grammar is what lets one decoder serve both surfaces.
//!
//! # Deployment is not the document
//!
//! A mesh deployment changes what a `<send>` does without changing the
//! document that wrote it: the same `<send target="#motor"/>` becomes a
//! same-process call under one `deploy.yaml` and a SOME/IP request
//! under another. A rendering of the document alone cannot say which,
//! so a reviewer approves behaviour the deployment then decides —
//! `claudedocs/rfc-pseudocode-review-surface.md` §8 names this the
//! review gap, and [`render_with_deployment`] is what closes it.
//!
//! Those facts are **derived**, and the rendering says so in its syntax
//! rather than in its typography. Every derived line begins with
//! [`DEPLOYMENT_SIGIL`]:
//!
//! ```text
//!   machine brake (datamodel: null, initial: idle, binding: early)
//!     ! device ecu1
//!     state braking:
//!       on entry:
//!         send brake.activate:
//!           to #motor
//!             ! transport zenoh
//!             ! key sce/brake/motor/cmd
//! ```
//!
//! ⚠ The sigil is not a comment marker and not emphasis. It is the one
//! thing that tells the document's text apart from the deployment's,
//! and three properties rest on it:
//!
//! 1. **[`crate::forge::unpseudo::parse`] refuses any line carrying
//!    it.** A deployed rendering has no document to be read back into,
//!    because part of it came from `deploy.yaml`. The reader says so
//!    instead of quietly dropping those lines, which would let a round
//!    trip report that it had verified a text half of which it never
//!    looked at.
//! 2. **Removing every sigil line yields exactly [`render`]'s output.**
//!    A deployment adds and never alters, so a reviewer's approval of
//!    the authored half is the same approval either way.
//! 3. **No authored line can begin with it.** Every line this module
//!    writes starts with a keyword it chose, and author text reaches
//!    the output only after one — encoded, so it carries no line
//!    terminator to break out with.
//!
//! `sce-build/tests/a_deployment_annotation_is_not_the_document.rs`
//! holds all three to the corpus rather than to this comment.
//!
//! A sigil rather than a word (`derived`, `deployed`) because a word is
//! identifier-shaped: a construct spelled the same way could be added
//! to the grammar later and the two would collide in silence. `!`
//! cannot become a keyword.
//!
//! # Grammar
//!
//! Indentation is two spaces per level and carries block structure;
//! order of lines is order in the document.
//!
//! ```text
//!   machine <name> (datamodel: <d>, initial: <s>[, binding: <b>][, queue: <n>])
//!     context <id> [cpp-type <t>] [cpp-include <i>] [kt-type <t>]
//!     data <id>[: <type>] [src <s>] [= <expr>] [content <c>]
//!     (state|parallel|final) <id> [initial <s>] [initial-children <s>...]
//!         [history <h> default <s>] [unhandled <e>...]:
//!       req <id>
//!       on entry: / on exit: / on initial: / on history-default:
//!         <action>...
//!       [on <event> ]-> <target> [<type>] [when <cond>] [native-guard <g>]
//!         <action>...
//!       on sample <link> event <e> [callback <c>]
//!       invoke [<id>]:
//!         id-into <loc> / param <name>[=<expr>][@<loc>] / req <id>
//!         type (scxml|hybrid|mesh-rpc|<other>)
//!         [autoforward] [src <s>] [namelist <n>] [finalize <t>]
//!         [srcexpr <e>] [contentexpr <e>]
//!         [mesh-target <t>] [mesh-transport <t>]
//!         [target (src|srcexpr) <s>] [event <e>] [deadline <n>ms]
//!         [host-served]
//!         child:
//!           <the inline machine, rendered the same way>
//!       done:
//!         param <name> [= <expr>] [from <loc>]
//!         content (expr|text|literal) <v>
//!
//!   <action> is one of
//!     <location> = <expr>
//!     assign <location>:            (when it carries child content)
//!       [expr <e>] content <c>
//!     cancel [<sendid>] [expr <e>]
//!     log [<label>][: <expr>]
//!     raise <event>
//!     script <text>
//!     call <name>
//!     call <name>:
//!       arg <param>
//!     foreach <item> [index <i>] in <array>:
//!     if <cond>: / elif <cond>: / else:
//!     send <event>                       (when it carries nothing else)
//!     send [<event>]:
//!       eventexpr <e> / to <t> / to-expr <e> / type <t> / type-expr <e>
//!       after <d> / after-expr <e> / id <i> / id-into <l>
//!       namelist <n> / content <c> / content-expr <e>
//!       param <name>[=<expr>][@<loc>]
//!
//!   algorithm <name>(<param>: <type>, ...) [-> <type> [returns-max <n>]]
//!     const <name>: <type> = <expr>
//!     const <name>: array<<type>, <n>> = fold <var> in <a>..<b> -> <type>:
//!       <stmt>...
//!       yield <expr>
//!     var <name>: <type> [cap <n>] = <expr>
//!     <target> = <expr>
//!     append <target> <- <expr>
//!     if <cond>:
//!       <stmt>...
//!     else:
//!       <stmt>...
//!     while [max <n>] <cond>:
//!       <stmt>...
//!     foreach <item> in <source>:
//!       <stmt>...
//!     return [<expr>]
//!     call <target>
//!     call <target>:
//!       arg <expr>
//!     test <hex> -> (bool|uint|int) <literal> @line <n>
//!
//!   procedure <name> initial <state>
//!     in <id>: <type> <field-clause>...
//!     internal <id>: <type> <field-clause>...
//!     helper <name>(<type>, ...) -> <type> [returns-max <n>]
//!     state <id>:
//!       send <service>
//!       send <service>:
//!         subfunc <e> / addr <e> / payload <e> / response-max <n>
//!       [on <event> ]-> <target>[ when <cond>]
//!         <location> = <expr>
//!     final <id>:
//!       done <name> = <expr>
//!
//!   condition <name>
//!     <field>...
//!     when <expr>
//!
//!   transform <name>
//!     <field>...
//!
//!   validator <name>
//!     <field>...
//!     range <id> [min <v>] [max <v>]
//!     rate <id> max-delta <v> interval <n>ms
//!     plausibility <expr>
//!
//!   event-schema <name> event <event>
//!     <field>...
//!
//!   enum <name>: <type> [strict]
//!     variant <name> = <n> [@line <n>]
//!
//!   timer <name> period <n>us fire <event> [reset-on <e>] [cancel-on-exit <s>]
//!
//!   lookup <name>
//!     <field>          (the input, then the output)
//!     <key> -> <value> [req <id>...]
//!     miss default <v> | miss error
//!
//!   filter <name> <type> [window <n>] [alpha <f>]
//!     <field>          (the input, then the output)
//!
//!   observer <name> [domain <d>]
//!     <field>...
//!     monitor <id>:
//!       on-enter <ev> [on-leave <ev>] enter <e> [leave <e>]  (one per line)
//!
//!   interpolation <name> method <m> out-of-bounds <o>
//!     <field>...       (the inputs, then the output)
//!     axis <input> breakpoints <f>...
//!     values <f>...
//!
//!   bounded-collection <name> of <type> capacity (deploy-key <k>|const <n>)
//!       overflow <p> ordering <o> concurrency <c> [index-by <field>]
//!
//!   worker <name> link-rx <l> inbox depth <n> ordering <o> [outbox <x>]
//!
//!   buffer-pool <name> slots <n> size <n> section <s> align <n>
//!       cache <p> [dma <c>]
//!     reassembly max-fragments <n> timeout <n>ms per-peer-quota <n>
//!
//!   link <name> class <c> framer <f> backpressure <p>
//!       [accept-stage-copy-rate]
//!     rx-pool <p> / tx-pool <p> / stage-pool <p>
//!     inbound <event> [when <expr>]
//!     outbound <event> encode <e>
//!
//!   codec <name> endian <e> [input-length <n>]
//!     flag-input <name> width <n>
//!     field <id>: <type> at <byte>[.<bit>] size <bit-size>
//!       endian <e> / max-size <n> / length-field <f> / length-arith <i>
//!       max-count <n> / repeat-body <a> / tlv-body <a> / embed-body <a>
//!       embed-length-from <f> / dma-align <n> / quantity <s> <o> <u>
//!       present-if [not] (local|input):<field>.<flag> [or ...]
//!       flag <name> bit <n> width <n> [value <v>]
//!     variant [tag-field <f>] [tag-flag <f>] [peek-byte <id>]
//!       peek-flag <name> bit <n> width <n> [value <v>]
//!       arm <n> -> <alias> [default]
//!       default-arm <n> -> <alias> [default]
//!     test <hex> @line <n>
//!       <name> = (bool|uint|int|bytes|string) <literal>
//! ```
//!
//! A `<bit-size>` is `fixed <n>`, `tail`, `length-ref`, `vle <n>`,
//! `repeat (length-field <f>|until-eof)`, `embed`, or
//! `tlv-chain max-depth <n> on-overflow <p> terminate <s>`.
//!
//! A `<field>` is `(in|out|internal) <id>: <type> <field-clause>...`,
//! the direction printed as the keyword rather than inferred from which
//! list the field came out of.
//!
//! ⚠ **One free-text value per line, and it is the last thing on it.**
//! Identifiers and expressions are opaque, so a line with two of them
//! has no unambiguous split — an expression containing the next
//! keyword decides where the line breaks. Measured over 587 documents:
//! ` = `, ` to ` and ` with ` DO occur inside authored strings here, so
//! this is the rule that keeps the grammar readable back rather than a
//! bet that no author writes those words. It is why `send` and
//! `monitor` are blocks and why `= <expr>` ends its line.
//!
//! A `<field-clause>` is any of `= <expr>`, `max-size <n>`,
//! `quantity <scale> <offset> <unit>`, `retain <scope> initial <expr>`,
//! `default-covers <a> <b> ...`, each written only when the model
//! carries it, always in that order.
//!
//! A transition writes `on <event>` only when it has one and
//! `when <cond>` only when it has one, so an unconditional eventless
//! transition is the bare `-> <target>`.

use std::fmt::Write as _;

use crate::comment_text;
use crate::forge::model::{
    AlgorithmConst, AlgorithmConstType, AlgorithmModel, AlgorithmStmt, BackpressurePolicy, BitSize,
    BoundedCollectionModel, BufferPoolModel, BufferPoolVariant, CachePolicy, CapacitySource,
    CodecField, CodecModel, CodecTestVector, CodecVariant, CollectionOrdering, ConcurrencyMode,
    ConditionModel, CountRef, DecodedFieldValue, DecodedValue, Direction, Endian, EnumModel,
    EventSchemaModel, FilterModel, FilterType, FlagDef, FoldBody, ForgeDocument, ForgeField,
    InboxOrdering, InterpolationMethod, InterpolationModel, LinkClass, LinkModel, LookupModel,
    MissPolicy, ObserverModel, OutOfBounds, OverflowPolicy, PresentIfPredicate, PresentIfScope,
    ProcedureHelper, ProcedureModel, ProcedureState, ProcedureTransition, SceType, TestVector,
    TestVectorValue, TimerModel, TlvOverflowPolicy, TlvTerminateStrategy, TransformModel,
    ValidatorModel, WorkerModel,
};

/// Why a document has no pseudocode rendering, as opposed to an empty
/// one.
///
/// ⚠ Carries the kind's own name for the reason
/// [`crate::forge::requirement_nodes::ReviewScope`] gives: a bare marker
/// would make one unanswerable question look like every other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// `sce:kind` as authored, e.g. `codec`.
    pub kind: &'static str,
    /// The construct that stopped the rendering.
    ///
    /// ⚠ The refusal is per DOCUMENT, and that is a stronger contract
    /// than per kind rather than a weaker one. What a reviewer needs is
    /// "what I am shown is all of it"; a kind that renders nine
    /// documents in full and abbreviates the tenth would break that
    /// promise while looking covered. Naming the construct is what makes
    /// the refusal actionable — the kind alone says nothing about which
    /// document.
    ///
    /// All eighteen kinds render, so there is no "this kind has no
    /// rendering" case left to express. A kind added to
    /// [`ForgeDocument`](crate::forge::model::ForgeDocument) breaks the
    /// `match` in [`render`] until somebody either renders it or names
    /// what stops them.
    pub feature: &'static str,
}

impl Unsupported {
    /// A document carrying a construct this module does not render.
    fn feature(kind: &'static str, feature: &'static str) -> Self {
        Unsupported { kind, feature }
    }
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "this '{}' document carries {}, which SCE does not render yet — \
             the rest of it would read like the whole of it, so the document \
             is refused rather than abbreviated",
            self.kind, self.feature
        )
    }
}

/// The first token of every derived line. See the module comment.
///
/// One `const` rather than the character written at each site: the
/// renderer emits it, the reader refuses it, and the gate looks for it,
/// and a three-way agreement spelled three times is a disagreement
/// waiting to happen.
pub const DEPLOYMENT_SIGIL: &str = "!";

/// One thing a deployment settles that the document does not say.
///
/// `name` is a word this renderer and its builder agree on
/// (`transport`, `key`, `device`); `value` is whatever the deployment
/// holds — an opaque string from `deploy.yaml`, encoded on the way out
/// like any author text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// What the fact is about, e.g. `transport`.
    pub name: String,
    /// What the deployment settled it to, e.g. `zenoh`.
    pub value: String,
}

impl Fact {
    /// A fact from two things that can be spelled as strings.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Fact {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// A `<send>` the deployment adds to the machine.
///
/// The author did not write it: SCE_MESH.md §13 auto-symmetry adds an
/// `<onexit>` unsubscribe for a qualifying subscribe, and Session E
/// server detection adds the response leg of each detected RPC pair.
/// The generated code sends it, so a reviewer who is not shown it is
/// approving a machine that does something they never read.
///
/// No `PartialEq`: `crate::model::Action` has none, and giving
/// forty-eight fields an equality so this type can have one would put
/// the cost of a review surface onto the core model. Nothing here
/// compares two injected sends — [`crate::mesh::review::injected_sends`]
/// compares their serialised form instead, which is what it needs.
#[derive(Debug, Clone)]
pub struct InjectedSend {
    /// The state whose actions it was added to.
    pub state: String,
    /// When it fires, as
    /// [`SendSite::label`](crate::mesh::topology::SendSite::label)
    /// spells it — `on entry`, `on exit`, `on <event>`.
    pub site: String,
    /// The action itself, rendered by the renderer that draws every
    /// other send. A second way of drawing one would be a second thing
    /// to keep true.
    pub action: crate::model::Action,
}

/// What a deployment adds to a document's meaning.
///
/// Deliberately not a mesh type. `sce-build::mesh` resolves a
/// `deploy.yaml` into transports, service ids and pool plans; this
/// carries only the part of that answer a reviewer reads, so the
/// renderer takes no dependency on the mesh pipeline and a caller can
/// state a deployment without running one. The builder that turns a
/// real resolution into this lives on the mesh side, where the types it
/// reads already are.
#[derive(Debug, Clone, Default)]
pub struct Deployment {
    /// True of the deployed machine as a whole — the device it runs on,
    /// and any send target the deployment could not resolve statically.
    pub machine: Vec<Fact>,
    /// Per `<send>` target, keyed by the target exactly as the document
    /// writes it (`#motor`), which is also how `deploy.yaml` keys its
    /// bindings.
    pub targets: std::collections::BTreeMap<String, Vec<Fact>>,
    /// The sends the deployment adds to the machine, in the order the
    /// model walk finds them.
    pub injected: Vec<InjectedSend>,
}

impl Deployment {
    /// Nothing derived — the rendering is the document alone.
    pub fn is_empty(&self) -> bool {
        self.machine.is_empty() && self.targets.is_empty() && self.injected.is_empty()
    }
}

/// The empty deployment, so [`Out::new`] has one to borrow.
///
/// A borrowed empty value rather than an `Option<&Deployment>` on the
/// writer: an absent deployment and a deployment that settles nothing
/// render the same bytes, and giving the writer one shape to handle is
/// what keeps that true.
///
/// ⚠ `static`, not `const`. A `const` is substituted at each use and
/// would make a temporary the borrow outlives.
static NO_DEPLOYMENT: Deployment = Deployment {
    machine: Vec::new(),
    targets: std::collections::BTreeMap::new(),
    injected: Vec::new(),
};

/// The pseudocode for this document, or why there is none.
///
/// The returned string ends in a newline when it is non-empty, so a
/// caller writing it to a file or to stdout needs no terminator of its
/// own.
pub fn render(doc: &ForgeDocument) -> Result<String, Unsupported> {
    render_with_deployment(doc, &NO_DEPLOYMENT)
}

/// The pseudocode for this document under a deployment.
///
/// Identical to [`render`] except that each fact the deployment settles
/// is written as its own line beginning with [`DEPLOYMENT_SIGIL`]. See
/// the module comment for why the distinction is syntactic.
///
/// ⚠ The result is a review surface and not a document: the reader
/// refuses it. Read [`render`]'s output back, not this.
pub fn render_with_deployment(
    doc: &ForgeDocument,
    deployment: &Deployment,
) -> Result<String, Unsupported> {
    // A deployment binds `<send>` targets. No other kind has one, so a
    // deployment handed in with one is a caller error, and it is
    // refused by name rather than rendered without the facts it was
    // given — which would hand back a text that looks complete and is
    // missing exactly what the caller asked to see.
    if !deployment.is_empty() && !matches!(doc, ForgeDocument::Statechart(_)) {
        return Err(Unsupported::feature(
            doc.kind().as_attr(),
            "a deployment, which binds the <send> targets only a statechart has",
        ));
    }
    // One arm per kind, listed rather than collapsed behind `_`, so a
    // kind added to the enum stops the build until somebody decides
    // whether it is rendered or refused.
    match doc {
        ForgeDocument::Algorithm(m) => Ok(render_algorithm(m)),
        ForgeDocument::Procedure(m) => Ok(render_procedure(m)),
        ForgeDocument::Condition(m) => Ok(render_condition(m)),
        ForgeDocument::Transform(m) => Ok(render_transform(m)),
        ForgeDocument::Validator(m) => Ok(render_validator(m)),
        ForgeDocument::EventSchema(m) => Ok(render_event_schema(m)),
        ForgeDocument::Enum(m) => Ok(render_enum(m)),
        ForgeDocument::Timer(m) => Ok(render_timer(m)),
        ForgeDocument::Lookup(m) => Ok(render_lookup(m)),
        ForgeDocument::Filter(m) => Ok(render_filter(m)),
        ForgeDocument::Observer(m) => Ok(render_observer(m)),
        ForgeDocument::Interpolation(m) => Ok(render_interpolation(m)),
        ForgeDocument::BoundedCollection(m) => Ok(render_bounded_collection(m)),
        ForgeDocument::Worker(m) => Ok(render_worker(m)),
        ForgeDocument::BufferPool(m) => Ok(render_buffer_pool(m)),
        ForgeDocument::Link(m) => Ok(render_link(m)),
        ForgeDocument::Codec(m) => Ok(render_codec(m)),
        ForgeDocument::Statechart(m) => render_statechart(m, deployment),
    }
}

// ── Output buffer ──────────────────────────────────────────────

/// Accumulates lines at an indentation depth.
///
/// A plain `String` plus a depth counter rather than a formatter: the
/// grammar's only structural device is leading whitespace, and every
/// writer here appends whole lines.
struct Out<'d> {
    buf: String,
    depth: usize,
    /// The facts to interleave, borrowed for the whole rendering.
    ///
    /// On the writer rather than threaded through twenty rendering
    /// functions: the writer is already the one place every line goes
    /// through, so a derived line lands at its site without each caller
    /// learning what a deployment is.
    deployment: &'d Deployment,
    /// Which targets already had their facts written.
    ///
    /// A deployment can bind a target the document never names: a
    /// `deploy.yaml` `subscriptions:` entry, or an `<invoke>` whose
    /// target the renderer reaches by another clause. Those facts have
    /// no `to` line to sit under, and without this set they would be
    /// dropped — the review gap this whole surface exists to close,
    /// reappearing one level down. [`Out::annotate_unplaced`] writes
    /// whatever is left.
    annotated: std::collections::BTreeSet<String>,
}

impl<'d> Out<'d> {
    fn new() -> Out<'static> {
        Out::with_deployment(&NO_DEPLOYMENT)
    }

    fn with_deployment(deployment: &'d Deployment) -> Out<'d> {
        Out {
            buf: String::new(),
            depth: 0,
            deployment,
            annotated: Default::default(),
        }
    }

    /// One line at the current depth.
    fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.buf.push_str("  ");
        }
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    /// One derived line at the current depth.
    ///
    /// Both halves are encoded. The value is opaque `deploy.yaml` text
    /// and could carry a newline; the name is chosen by the builder and
    /// could not, but encoding it anyway costs nothing and means no
    /// future builder can make this the one unescaped site.
    fn derived(&mut self, fact: &Fact) {
        self.line(&format!(
            "{DEPLOYMENT_SIGIL} {} {}",
            text(&fact.name),
            text(&fact.value)
        ));
    }

    /// One derived line carrying text this module already produced.
    ///
    /// Used for an injected send, whose body is drawn by the ordinary
    /// action renderer and is therefore encoded already. Encoding it a
    /// second time would show the reviewer the escapes rather than the
    /// send.
    fn derived_rendered(&mut self, already_rendered: &str) {
        self.line(&format!("{DEPLOYMENT_SIGIL} {already_rendered}"));
    }

    /// The deployment's facts about a send target, one level under it.
    ///
    /// `target` is the raw attribute value, not the encoded one —
    /// `deploy.yaml` keys its bindings by what the document wrote.
    fn annotate_target(&mut self, target: &str) {
        // Read the facts out through the deployment's own lifetime
        // before writing, so the buffer is free to be borrowed.
        let deployment: &'d Deployment = self.deployment;
        let Some(facts) = deployment.targets.get(target) else {
            return;
        };
        self.annotated.insert(target.to_string());
        self.nested(|out| {
            for f in facts {
                out.derived(f);
            }
        });
    }

    /// The deployment's facts about the machine, at the current depth.
    fn annotate_machine(&mut self) {
        let deployment: &'d Deployment = self.deployment;
        for f in &deployment.machine {
            self.derived(f);
        }
    }

    /// The facts for every target no clause in the document named.
    ///
    /// ⚠ Written LAST, because which targets those are is only known
    /// once the document has been walked. A `deploy.yaml`
    /// `subscriptions:` entry binds a target the machine never
    /// `<send>`s to, and measured over `tests/mesh` eleven fixtures do
    /// exactly that — so without this block eleven deployments would
    /// have bound a transport that no reviewer was ever shown.
    ///
    /// The target heads its own derived line and its facts nest under
    /// it, so the shape reads like the `to` case rather than inventing
    /// a second one.
    /// Every `<send>` the deployment adds to the machine.
    ///
    /// ⚠ Drawn by [`render_scxml_action`], the same function that draws
    /// an authored send, and then prefixed line by line. A second
    /// renderer for "the same thing, but derived" is two things to keep
    /// true, and the one that is read less often is the one that rots.
    ///
    /// The prefix goes on EVERY line of the send, not only its head, so
    /// a multi-line send is removed whole by a stripper that only ever
    /// looks at a line's first token.
    fn annotate_injected(&mut self) {
        let deployment: &'d Deployment = self.deployment;
        for inj in &deployment.injected {
            self.derived(&Fact::new(
                "injected-send",
                format!("in {} {}", inj.state, inj.site),
            ));
            let mut body = Out::new();
            render_scxml_action(&inj.action, &mut body);
            self.nested(|out| {
                for line in body.buf.lines() {
                    out.derived_rendered(line);
                }
            });
        }
    }

    fn annotate_unplaced(&mut self) {
        let deployment: &'d Deployment = self.deployment;
        for (target, facts) in &deployment.targets {
            if self.annotated.contains(target) {
                continue;
            }
            self.derived(&Fact::new("bound-without-a-send", target));
            self.nested(|out| {
                for f in facts {
                    out.derived(f);
                }
            });
        }
    }

    /// Run `body` one level deeper.
    fn nested(&mut self, body: impl FnOnce(&mut Self)) {
        self.depth += 1;
        body(self);
        self.depth -= 1;
    }

    /// An already-rendered block, re-indented under the current depth.
    ///
    /// Used where a rendering nests one of its own kind — an `<invoke>`
    /// carrying an inline child machine. Re-indenting the finished text
    /// keeps one renderer for a machine instead of a second one that
    /// takes a starting depth, which is how the two would drift.
    fn block(&mut self, rendered: &str) {
        for line in rendered.lines() {
            self.line(line);
        }
    }
}

/// Author-supplied text, encoded so no newline or escape introducer
/// reaches the output. See the module comment.
fn text(s: &str) -> std::borrow::Cow<'_, str> {
    comment_text::encode(s)
}

// ── Algorithm ──────────────────────────────────────────────────

fn render_algorithm(m: &AlgorithmModel) -> String {
    let mut out = Out::new();

    let params: Vec<String> = m
        .signature
        .params
        .iter()
        .map(|p| format!("{}: {}", text(&p.name), p.sce_type.as_attr()))
        .collect();
    let mut head = format!("algorithm {}({})", text(&m.name), params.join(", "));
    if let Some(ret) = &m.signature.return_type {
        let _ = write!(head, " -> {}", ret.as_attr());
        if let Some(max) = m.signature.returns_max_size {
            let _ = write!(head, " returns-max {max}");
        }
    }
    out.line(&head);

    out.nested(|out| {
        for c in &m.consts {
            render_const(c, out);
        }
        for stmt in &m.body {
            render_stmt(stmt, out);
        }
        for tv in &m.test_vectors {
            render_test_vector(tv, out);
        }
    });

    out.buf
}

fn render_const(c: &AlgorithmConst, out: &mut Out<'_>) {
    match (&c.sce_type, &c.init, &c.fold) {
        // Scalar form. `compute_at_build` is false here by the parser's
        // "exactly one of init / fold" invariant, so the keyword below
        // carries it without a second spelling.
        (AlgorithmConstType::Scalar(t), Some(init), _) => {
            out.line(&format!(
                "const {}: {} = {}",
                text(&c.name),
                t.as_attr(),
                text(init)
            ));
        }
        (AlgorithmConstType::Array { elem, len }, _, Some(fold)) => {
            out.line(&format!(
                "const {}: array<{}, {}> = {}",
                text(&c.name),
                elem.as_attr(),
                len,
                fold_head(fold)
            ));
            out.nested(|out| {
                for stmt in &fold.body {
                    render_stmt(stmt, out);
                }
                out.line(&format!("yield {}", text(&fold.yield_expr)));
            });
        }
        // The parser admits no other combination — a scalar with a fold
        // body, or an array with a literal init, is refused at parse
        // time. Rendering the shape verbatim rather than guessing keeps
        // this total without inventing a reading the parser forbids.
        (t, init, fold) => {
            out.line(&format!(
                "const {}: {} = <unrepresentable init={} fold={}>",
                text(&c.name),
                const_type_text(t),
                init.is_some(),
                fold.is_some()
            ));
        }
    }
}

fn const_type_text(t: &AlgorithmConstType) -> String {
    match t {
        AlgorithmConstType::Scalar(s) => s.as_attr(),
        AlgorithmConstType::Array { elem, len } => {
            format!("array<{}, {}>", elem.as_attr(), len)
        }
    }
}

fn fold_head(fold: &FoldBody) -> String {
    format!(
        "fold {} in {}..{} -> {}:",
        text(&fold.iter_var),
        fold.range_start,
        fold.range_end,
        fold.elem_type.as_attr()
    )
}

fn render_stmt(stmt: &AlgorithmStmt, out: &mut Out<'_>) {
    match stmt {
        AlgorithmStmt::Var {
            name,
            sce_type,
            init,
            capacity,
        } => {
            let cap = match capacity {
                Some(n) => format!(" cap {n}"),
                None => String::new(),
            };
            out.line(&format!(
                "var {}: {}{} = {}",
                text(name),
                sce_type.as_attr(),
                cap,
                text(init)
            ));
        }
        AlgorithmStmt::Assign { target, expr } => {
            out.line(&format!("{} = {}", text(target), text(expr)));
        }
        AlgorithmStmt::Append { target, expr } => {
            out.line(&format!("append {} <- {}", text(target), text(expr)));
        }
        AlgorithmStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.line(&format!("if {}:", text(cond)));
            out.nested(|out| {
                for s in then_body {
                    render_stmt(s, out);
                }
            });
            // An absent `else` and an empty one are different documents
            // — `<sce:else/>` with no children is a block the author
            // wrote — so the keyword is printed whenever the model
            // carries `Some`, even for an empty body.
            if let Some(body) = else_body {
                out.line("else:");
                out.nested(|out| {
                    for s in body {
                        render_stmt(s, out);
                    }
                });
            }
        }
        AlgorithmStmt::While {
            cond,
            body,
            max_iter,
        } => {
            // The bound goes BEFORE the condition, which is the only
            // free-text value on the line. Written the other way round
            // a condition containing ` max ` would decide where the
            // line breaks — no fixture does today (measured: 6
            // conditions, none of them), and the rule is not a bet on
            // that staying true.
            let max = match max_iter {
                Some(n) => format!("max {n} "),
                None => String::new(),
            };
            out.line(&format!("while {}{}:", max, text(cond)));
            out.nested(|out| {
                for s in body {
                    render_stmt(s, out);
                }
            });
        }
        AlgorithmStmt::Foreach { item, source, body } => {
            out.line(&format!("foreach {} in {}:", text(item), text(source)));
            out.nested(|out| {
                for s in body {
                    render_stmt(s, out);
                }
            });
        }
        AlgorithmStmt::Return { expr } => match expr {
            Some(e) => out.line(&format!("return {}", text(e))),
            None => out.line("return"),
        },
        AlgorithmStmt::Call { target, args } => {
            // One argument per line. Joined with `, ` they were several
            // free-text values on one line, and an argument containing
            // a comma-space would have decided where they split. ⚠ No
            // fixture in this tree calls anything (measured: zero
            // arguments across 587 documents), so this arm is written
            // to the rule rather than to an example.
            if args.is_empty() {
                out.line(&format!("call {}", text(target)));
            } else {
                out.line(&format!("call {}:", text(target)));
                out.nested(|out| {
                    for a in args {
                        out.line(&format!("arg {}", text(a)));
                    }
                });
            }
        }
    }
}

fn render_test_vector(tv: &TestVector, out: &mut Out<'_>) {
    let hex: String = tv.hex.iter().map(|b| format!("{b:02x}")).collect();
    let value = match &tv.value {
        TestVectorValue::Bool(b) => format!("bool {b}"),
        TestVectorValue::Uint(v) => format!("uint {v}"),
        TestVectorValue::Int(v) => format!("int {v}"),
    };
    // ⚠ `source_line` is printed although it is a position, not
    // behaviour. Unlike every other position in the IR it is NOT named
    // `source_location`, so a round-trip comparison that strips that one
    // key by name would still see this field differ. Printing it keeps
    // the rendering total and keeps the strip list at exactly one key;
    // the alternative — a second key in the strip list — is the
    // hand-kept exclusion list this design exists to avoid.
    out.line(&format!("test 0x{hex} -> {value} @line {}", tv.source_line));
}

// ── Procedure ──────────────────────────────────────────────────

fn render_procedure(m: &ProcedureModel) -> String {
    let mut out = Out::new();
    out.line(&format!(
        "procedure {} initial {}",
        text(&m.name),
        text(&m.initial)
    ));

    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        for f in &m.internals {
            render_field(f, out);
        }
        for h in &m.helpers {
            render_helper(h, out);
        }
        for s in &m.states {
            render_state(s, out);
        }
    });

    out.buf
}

/// One `<data>` field, with every clause the model carries.
///
/// `direction` is printed as the leading keyword rather than inferred
/// from which list the field came out of: the two agree today, and a
/// rendering that relies on them continuing to agree would be a
/// rendering that stops being total the day they do not.
fn render_field(f: &ForgeField, out: &mut Out<'_>) {
    let dir = match f.direction {
        Direction::In => "in",
        Direction::Out => "out",
        Direction::Internal => "internal",
    };
    let mut line = format!("{} {}: {}", dir, text(&f.id), f.sce_type.as_attr());
    if let Some(expr) = &f.expr {
        let _ = write!(line, " = {}", text(expr));
    }
    if let Some(max) = f.max_size {
        let _ = write!(line, " max-size {max}");
    }
    if let Some(q) = &f.quantity {
        let _ = write!(line, " quantity {} {} {}", q.scale, q.offset, q.unit);
    }
    if let Some(r) = &f.retain {
        let _ = write!(
            line,
            " retain {} initial {}",
            text(&r.scope),
            text(&r.initial)
        );
    }
    if !f.default_covers.is_empty() {
        let covers: Vec<String> = f
            .default_covers
            .iter()
            .map(|c| text(c).into_owned())
            .collect();
        let _ = write!(line, " default-covers {}", covers.join(" "));
    }
    out.line(&line);
}

fn render_helper(h: &ProcedureHelper, out: &mut Out<'_>) {
    let args: Vec<String> = h.args.iter().map(SceType::as_attr).collect();
    let mut line = format!(
        "helper {}({}) -> {}",
        text(&h.name),
        args.join(", "),
        h.returns.as_attr()
    );
    if let Some(max) = h.returns_max_size {
        let _ = write!(line, " returns-max {max}");
    }
    out.line(&line);
}

fn render_state(s: &ProcedureState, out: &mut Out<'_>) {
    let keyword = if s.is_final { "final" } else { "state" };
    out.line(&format!("{} {}:", keyword, text(&s.id)));
    out.nested(|out| {
        for send in &s.on_entry_sends {
            // A block when it carries anything past the service:
            // `subfunc`, `addr` and `payload` are author expressions,
            // and three free-text values on one line have no
            // unambiguous split.
            let clauses: Vec<(&str, String)> = [
                ("subfunc", send.subfunc.clone()),
                ("addr", send.addr.clone()),
                ("payload", send.payload.clone()),
                (
                    "response-max",
                    send.response_max_size.map(|n| n.to_string()),
                ),
            ]
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect();

            if clauses.is_empty() {
                out.line(&format!("send {}", text(&send.service)));
            } else {
                out.line(&format!("send {}:", text(&send.service)));
                out.nested(|out| {
                    for (name, value) in clauses {
                        out.line(&format!("{name} {}", text(&value)));
                    }
                });
            }
        }
        for t in &s.transitions {
            render_transition(t, out);
        }
        for p in &s.done_params {
            out.line(&format!("done {} = {}", text(&p.name), text(&p.expr)));
        }
    });
}

fn render_transition(t: &ProcedureTransition, out: &mut Out<'_>) {
    let mut line = String::new();
    // An eventless transition is the bare arrow: `on` with no event
    // would read as an event named by the empty string.
    if let Some(e) = &t.event {
        let _ = write!(line, "on {} ", text(e));
    }
    let _ = write!(line, "-> {}", text(&t.target));
    // The guard goes LAST, after the target, because it is the only
    // free-text value on the line and the rule puts those at the end.
    // Written before the arrow, a condition containing ` -> ` would
    // decide where the line breaks — none of the 232 conditions in
    // this tree does, and the rule is not a bet on that.
    if let Some(c) = &t.cond {
        let _ = write!(line, " when {}", text(c));
    }
    out.line(&line);

    out.nested(|out| {
        for a in &t.assigns {
            out.line(&format!("{} = {}", text(&a.location), text(&a.expr)));
        }
    });
}

// ── Declarative kinds ──────────────────────────────────────────
//
// Each of these is its whole document: the model carries what the
// author wrote and nothing a backend derived, which is why the
// rendering is a transcription rather than a walk. `render_field` is
// shared with the procedure above for the same reason the review table
// shares its row type — two spellings of one field is how they drift.

fn render_condition(m: &ConditionModel) -> String {
    let mut out = Out::new();
    out.line(&format!("condition {}", text(&m.name)));
    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        out.line(&format!("when {}", text(&m.expr)));
    });
    out.buf
}

fn render_transform(m: &TransformModel) -> String {
    let mut out = Out::new();
    out.line(&format!("transform {}", text(&m.name)));
    out.nested(|out| {
        for f in m.inputs.iter().chain(&m.outputs) {
            render_field(f, out);
        }
    });
    out.buf
}

fn render_validator(m: &ValidatorModel) -> String {
    let mut out = Out::new();
    out.line(&format!("validator {}", text(&m.name)));
    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        for r in &m.rules.ranges {
            let mut line = format!("range {}", text(&r.id));
            if let Some(v) = &r.min {
                let _ = write!(line, " min {}", text(v));
            }
            if let Some(v) = &r.max {
                let _ = write!(line, " max {}", text(v));
            }
            out.line(&line);
        }
        for r in &m.rules.rate_of_changes {
            out.line(&format!(
                "rate {} max-delta {} interval {}ms",
                text(&r.id),
                text(&r.max_delta),
                r.sample_interval_ms
            ));
        }
        if let Some(p) = &m.rules.plausibility {
            out.line(&format!("plausibility {}", text(p)));
        }
    });
    out.buf
}

fn render_event_schema(m: &EventSchemaModel) -> String {
    let mut out = Out::new();
    out.line(&format!(
        "event-schema {} event {}",
        text(&m.name),
        text(&m.event_name)
    ));
    out.nested(|out| {
        for f in &m.fields {
            render_field(f, out);
        }
    });
    out.buf
}

fn render_enum(m: &EnumModel) -> String {
    let mut out = Out::new();
    let mut head = format!("enum {}: {}", text(&m.name), m.underlying_type.as_attr());
    if m.strict_variants {
        head.push_str(" strict");
    }
    out.line(&head);
    out.nested(|out| {
        for v in &m.variants {
            // `source_line` is printed for the reason the algorithm's
            // test vector prints its own: it is a position that does NOT
            // travel under the `source_location` key, so a round trip
            // stripping that one key would still see it differ.
            let mut line = format!("variant {} = {}", text(&v.name), v.value);
            if let Some(l) = v.source_line {
                let _ = write!(line, " @line {l}");
            }
            out.line(&line);
        }
    });
    out.buf
}

fn render_timer(m: &TimerModel) -> String {
    let mut out = Out::new();
    let mut head = format!(
        "timer {} period {}us fire {}",
        text(&m.name),
        m.period_us,
        text(&m.fire_event)
    );
    if let Some(e) = &m.reset_on_event {
        let _ = write!(head, " reset-on {}", text(e));
    }
    if let Some(s) = &m.cancel_on_state_exit {
        let _ = write!(head, " cancel-on-exit {}", text(s));
    }
    out.line(&head);
    out.buf
}

/// An `f64` as the shortest text that reads back as the same value.
///
/// ⚠ This is deterministic but NOT the author's spelling: a breakpoint
/// written `1.0` prints as `1`, because the model holds an `f64` and the
/// literal's text was already gone before this module saw it. Same
/// class as an enum variant written `0x10` arriving as `16`. It costs
/// nothing under a model-level round trip and it costs a reviewer
/// something, which is why it is written down rather than left to be
/// rediscovered.
fn num(v: f64) -> String {
    format!("{v}")
}

fn render_filter(m: &FilterModel) -> String {
    let mut out = Out::new();
    let kind = match m.filter_type {
        FilterType::MovingAverage => "moving-average",
        FilterType::LowPass => "low-pass",
        FilterType::Debounce => "debounce",
    };
    let mut head = format!("filter {} {}", text(&m.name), kind);
    if let Some(w) = m.window {
        let _ = write!(head, " window {w}");
    }
    if let Some(a) = m.alpha {
        let _ = write!(head, " alpha {}", num(a));
    }
    out.line(&head);
    out.nested(|out| {
        render_field(&m.input, out);
        render_field(&m.output, out);
    });
    out.buf
}

fn render_observer(m: &ObserverModel) -> String {
    let mut out = Out::new();
    let mut head = format!("observer {}", text(&m.name));
    if let Some(d) = &m.event_domain {
        let _ = write!(head, " domain {}", text(d));
    }
    out.line(&head);
    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        for mon in &m.monitors {
            // A BLOCK, not a line. `enter` and `leave` carry author
            // expressions, and two free-text values on one line have no
            // unambiguous split: an expression containing the word
            // `leave` would decide where the line breaks. One free-text
            // value per line, at the end of it, is the rule this whole
            // grammar follows — measured over 587 documents, the tokens
            // that do occur inside authored strings (` = `, ` to `,
            // ` with `) are exactly the ones that sit beside free text.
            out.line(&format!("monitor {}:", text(&mon.id)));
            out.nested(|out| {
                out.line(&format!("on-enter {}", text(&mon.on_enter)));
                if let Some(e) = &mon.on_leave {
                    out.line(&format!("on-leave {}", text(e)));
                }
                out.line(&format!("enter {}", text(&mon.enter_expr)));
                if let Some(e) = &mon.leave_expr {
                    out.line(&format!("leave {}", text(e)));
                }
            });
        }
    });
    out.buf
}

fn render_interpolation(m: &InterpolationModel) -> String {
    let mut out = Out::new();
    let method = match m.method {
        InterpolationMethod::Linear => "linear",
        InterpolationMethod::Bilinear => "bilinear",
    };
    let oob = match m.out_of_bounds {
        OutOfBounds::Clamp => "clamp",
        OutOfBounds::Extrapolate => "extrapolate",
        OutOfBounds::Error => "error",
    };
    out.line(&format!(
        "interpolation {} method {} out-of-bounds {}",
        text(&m.name),
        method,
        oob
    ));
    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        render_field(&m.output, out);
        for a in &m.axes {
            let bps: Vec<String> = a.breakpoints.iter().copied().map(num).collect();
            out.line(&format!(
                "axis {} breakpoints {}",
                text(&a.input_id),
                bps.join(" ")
            ));
        }
        let vals: Vec<String> = m.values.iter().copied().map(num).collect();
        out.line(&format!("values {}", vals.join(" ")));
    });
    out.buf
}

fn render_bounded_collection(m: &BoundedCollectionModel) -> String {
    let mut out = Out::new();
    let capacity = match &m.capacity {
        CapacitySource::DeployKey { key } => format!("deploy-key {}", text(key)),
        CapacitySource::CompileConst { value } => format!("const {value}"),
    };
    let overflow = match m.on_overflow {
        OverflowPolicy::DiagnosticEvent => "diagnostic-event",
        OverflowPolicy::Reject => "reject",
        OverflowPolicy::OldestWins => "oldest-wins",
    };
    let ordering = match m.ordering {
        CollectionOrdering::Insertion => "insertion",
        CollectionOrdering::SortedByIndex => "sorted-by-index",
    };
    let concurrency = match m.concurrency {
        ConcurrencyMode::SingleWriter => "single-writer",
        ConcurrencyMode::MultiWriter => "multi-writer",
    };
    let mut head = format!(
        "bounded-collection {} of {} capacity {capacity} overflow {overflow}",
        text(&m.name),
        text(&m.element_type)
    );
    let _ = write!(head, " ordering {ordering} concurrency {concurrency}");
    if let Some(ix) = &m.index_by {
        let _ = write!(head, " index-by {}", text(ix));
    }
    out.line(&head);
    out.buf
}

// ── Statechart ─────────────────────────────────────────────────
//
// The one kind whose model is mostly NOT the document: of its 92
// fields, 39 are written by the analyzer (measured — see
// `sce-build/tests/the_analyzer_declares_which_fields_it_writes.rs`),
// and showing any of them to a reviewer would be showing SCE's
// arithmetic rather than their machine. What is rendered here is the
// authored core: the states, their executable content, the datamodel
// and the transitions.
//
// ⚠ Five of those 39 are authored fields the analyzer REWRITES —
// `datamodel`, `states`, `transitions`, `variables` and a transition's
// `type`. They are rendered from the model as given, which is correct
// on the `--emit-ast` path (no analyzer pass) and is the reason the
// contract says a rendering must come from before that pass.

/// The constructs a statechart document may carry that this module does
/// not render yet, checked before anything is written.
///
/// Returning the FIRST one found rather than a list: the author needs
/// to know the document is not fully rendered, and one name says that
/// as well as five do.
fn statechart_gap(m: &crate::model::SCXMLModel) -> Option<&'static str> {
    // The root's `invokes` is a flat index of the ones the states own,
    // so the rendering walks the states and skips the index. That is
    // only sound while the two agree: an invoke reachable from the root
    // and from no state would be dropped in silence, which is the one
    // thing this module may not do.
    let owned: usize = m.states.values().map(|s| s.invokes.len()).sum();
    if m.invokes.len() != owned {
        return Some("an <invoke> no state owns");
    }
    None
}

fn render_statechart(
    m: &crate::model::SCXMLModel,
    deployment: &Deployment,
) -> Result<String, Unsupported> {
    if let Some(gap) = statechart_gap(m) {
        return Err(Unsupported::feature("statechart", gap));
    }

    let mut out = Out::with_deployment(deployment);
    let datamodel = match m.datamodel {
        crate::model::Datamodel::Null => "null",
        crate::model::Datamodel::EcmaScript => "ecmascript",
    };
    let mut clauses = vec![
        format!("datamodel: {datamodel}"),
        format!("initial: {}", text(&m.initial)),
    ];
    if !m.binding.is_empty() {
        clauses.push(format!("binding: {}", text(&m.binding)));
    }
    if let Some(cap) = m.event_queue_capacity {
        clauses.push(format!("queue: {cap}"));
    }
    out.line(&format!(
        "machine {} ({})",
        text(&m.name),
        clauses.join(", ")
    ));

    let mut nested: Result<(), Unsupported> = Ok(());
    out.nested(|out| {
        // The machine's own derived facts come first, before anything
        // the document says, so a reader meets the deployment before
        // the behaviour it reinterprets rather than after it.
        out.annotate_machine();
        // ⚠ `context_objects`, not `context_object_ids`: the parser
        // fills the id set alongside the list from the same elements, so
        // rendering both would print each object twice — the shape the
        // invoke rendering was caught doing.
        for c in &m.context_objects {
            let mut line = format!("context {}", text(&c.id));
            if !c.cpp_type.is_empty() {
                let _ = write!(line, " cpp-type {}", text(&c.cpp_type));
            }
            if !c.cpp_include.is_empty() {
                let _ = write!(line, " cpp-include {}", text(&c.cpp_include));
            }
            if !c.kt_type.is_empty() {
                let _ = write!(line, " kt-type {}", text(&c.kt_type));
            }
            out.line(&line);
        }
        for v in &m.variables {
            render_variable(v, out);
        }
        for s in &m.global_scripts {
            render_scxml_action(s, out);
        }
        // ⚠ NOT `m.invokes`. The model keeps every invoke twice — once
        // on the state that owns it and once in a flat index on the
        // root — so rendering both printed each `<invoke>` a second
        // time, and a reviewer would have counted two where the
        // document has one. Measured on test187. The invariant that
        // makes the flat index redundant is asserted above.
        // States are held in a map, so the order a reviewer reads must
        // come from `document_order` — the field the model keeps
        // precisely because the container lost it.
        let mut states: Vec<&crate::model::State> = m.states.values().collect();
        states.sort_by_key(|s| (s.document_order, s.id.clone()));
        for s in states {
            if let Err(e) = render_scxml_state(s, out) {
                nested = Err(e);
            }
        }
        // Last, once every clause that could have claimed a target has
        // had its turn. See `Out::annotate_unplaced`.
        out.annotate_injected();
        out.annotate_unplaced();
    });
    nested?;

    Ok(out.buf)
}

/// One `<invoke>`, in whichever of its four shapes.
///
/// ⚠ What is deliberately NOT printed, named here so the omission is
/// stated rather than silent: `field_suffix` and `state_name` on
/// [`InvokeBase`](crate::model::InvokeBase), and `child_name`,
/// `use_specific_event`, `child_needs_script_engine`,
/// `child_has_send_to_parent`, `child_needs_event_scheduler` and
/// `child_datamodel_vars` on
/// [`InvokeSessionCommon`](crate::model::InvokeSessionCommon). Every one
/// is a codegen-facing conclusion SCE draws about the child — symbol
/// naming, whether the child needs an engine — and not a thing the
/// author wrote. Printing them would put SCE's reasoning in front of a
/// reviewer as if it were their machine.
///
/// ⚠⚠ `inline_child_xml` is also absent, and for a different reason: it
/// is the same content as `inline_child`, in its source form. The
/// parsed child is rendered; repeating its XML underneath would be one
/// document shown twice.
fn render_invoke(inv: &crate::model::Invoke, out: &mut Out<'_>) -> Result<(), Unsupported> {
    use crate::model::Invoke;

    let base = match inv {
        Invoke::Scxml(i) => &i.common.base,
        Invoke::Hybrid(i) => &i.common.base,
        Invoke::MeshRpc(i) => &i.base,
        Invoke::Unsupported(i) => &i.base,
    };

    let mut head = String::from("invoke");
    if !base.invoke_id.is_empty() {
        let _ = write!(head, " {}", text(&base.invoke_id));
    }
    out.line(&format!("{head}:"));

    let mut nested: Result<(), Unsupported> = Ok(());
    out.nested(|out| {
        if !base.idlocation.is_empty() {
            out.line(&format!("id-into {}", text(&base.idlocation)));
        }
        for p in &base.params {
            out.line(&format!("param {}", render_param(p)));
        }
        for id in &base.req {
            out.line(&format!("req {}", text(&id.to_string())));
        }

        match inv {
            Invoke::Scxml(i) => {
                out.line("type scxml");
                if i.common.autoforward {
                    out.line("autoforward");
                }
                if !i.src.is_empty() {
                    out.line(&format!("src {}", text(&i.src)));
                }
                if !i.namelist.is_empty() {
                    out.line(&format!("namelist {}", text(&i.namelist)));
                }
                if let Some(t) = &i.remote_mesh_target {
                    out.line(&format!("mesh-target {}", text(t)));
                    // An `<invoke>` names a target too, and the
                    // deployment binds it exactly as it binds a
                    // `<send>`'s. Annotated here so the facts sit at
                    // the clause that names them rather than in the
                    // trailing block, which is where they would
                    // otherwise land — correct, and further from the
                    // thing they are about.
                    out.annotate_target(t);
                }
                if let Some(t) = &i.remote_mesh_transport {
                    out.line(&format!("mesh-transport {}", text(t)));
                }
                if !i.finalize_content.is_empty() {
                    out.line(&format!("finalize {}", text(&i.finalize_content)));
                }
                if let Some(child) = &i.inline_child {
                    // ⚠ The child renders WITHOUT the parent's
                    // deployment, and that is the honest answer rather
                    // than a shortcut: a deployment binds one machine's
                    // send targets, and an inline child is a different
                    // machine whose `deploy.yaml` entry is its own. A
                    // child's own deployment is a thing this surface
                    // does not yet show — stated here because the
                    // alternative, lending it the parent's facts, would
                    // print bindings that are not its.
                    match render_statechart(child, &NO_DEPLOYMENT) {
                        Ok(rendered) => {
                            out.line("child:");
                            out.nested(|out| out.block(&rendered));
                        }
                        // A child the renderer cannot finish makes the
                        // PARENT unrenderable: a machine shown without
                        // the machine it invokes is the abbreviation
                        // this module exists to refuse.
                        Err(e) => nested = Err(e),
                    }
                }
            }
            Invoke::Hybrid(i) => {
                out.line("type hybrid");
                if i.common.autoforward {
                    out.line("autoforward");
                }
                if !i.srcexpr.is_empty() {
                    out.line(&format!("srcexpr {}", text(&i.srcexpr)));
                }
                if !i.contentexpr.is_empty() {
                    out.line(&format!("contentexpr {}", text(&i.contentexpr)));
                }
            }
            Invoke::MeshRpc(i) => {
                out.line("type mesh-rpc");
                match &i.target {
                    crate::model::MeshRpcTarget::Src { src } => {
                        out.line(&format!("target src {}", text(src)))
                    }
                    crate::model::MeshRpcTarget::SrcExpr { srcexpr } => {
                        out.line(&format!("target srcexpr {}", text(srcexpr)))
                    }
                }
                out.line(&format!("event {}", text(&i.mesh_event)));
                if let Some(d) = i.deadline_ms {
                    out.line(&format!("deadline {d}ms"));
                }
            }
            Invoke::Unsupported(i) => {
                out.line(&format!("type {}", text(&i.invoke_type)));
                if !i.src.is_empty() {
                    out.line(&format!("src {}", text(&i.src)));
                }
                if i.host_served {
                    out.line("host-served");
                }
            }
        }
    });
    nested
}

fn render_variable(v: &crate::model::Variable, out: &mut Out<'_>) {
    let mut line = format!("data {}", text(&v.id));
    if !v.var_type.is_empty() {
        let _ = write!(line, ": {}", text(&v.var_type));
    }
    if !v.src.is_empty() {
        let _ = write!(line, " src {}", text(&v.src));
    }
    // `= <expr>` and `content` are the free-text clauses, so they close
    // the line. No variable in this tree carries an expression together
    // with a `src` or a content body (measured: 0 of 425), so the order
    // changes nothing anyone has written — it removes the shape in
    // which a future one could not be read back.
    if !v.expr.is_empty() {
        let _ = write!(line, " = {}", text(&v.expr));
    }
    if !v.content.is_empty() {
        let _ = write!(line, " content {}", text(&v.content));
    }
    out.line(&line);
}

fn render_scxml_state(s: &crate::model::State, out: &mut Out<'_>) -> Result<(), Unsupported> {
    let keyword = if s.is_final {
        "final"
    } else if s.is_parallel {
        "parallel"
    } else {
        "state"
    };
    let mut head = format!("{keyword} {}", text(&s.id));
    if !s.initial.is_empty() {
        let _ = write!(head, " initial {}", text(&s.initial));
    }
    if !s.initial_children.is_empty() {
        let kids: Vec<String> = s
            .initial_children
            .iter()
            .map(|k| text(k).into_owned())
            .collect();
        let _ = write!(head, " initial-children {}", kids.join(" "));
    }
    if !s.initial_history_id.is_empty() {
        let _ = write!(
            head,
            " history {} default {}",
            text(&s.initial_history_id),
            text(&s.initial_history_default_target)
        );
    }
    if !s.unhandled.is_empty() {
        let u: Vec<String> = s.unhandled.iter().map(|k| text(k).into_owned()).collect();
        let _ = write!(head, " unhandled {}", u.join(" "));
    }
    out.line(&format!("{head}:"));

    let mut nested: Result<(), Unsupported> = Ok(());
    out.nested(|out| {
        for id in &s.req {
            out.line(&format!("req {}", text(&id.to_string())));
        }
        for v in &s.datamodel {
            render_variable(v, out);
        }
        for inv in &s.invokes {
            if let Err(e) = render_invoke(inv, out) {
                nested = Err(e);
            }
        }
        // ⚠ Per state, not from the root's `on_sample_links`: that set
        // is filled by the ANALYZER from these same blocks
        // (`analyzer.rs`), so it is a derived index and printing it too
        // would say each link twice.
        let mut samples: Vec<&crate::model::OnSampleNode> = s.on_sample_blocks.iter().collect();
        samples.sort_by_key(|n| n.document_order);
        for n in samples {
            let mut line = format!("on sample {} event {}", text(&n.link), text(&n.event));
            if let Some(c) = &n.callback {
                let _ = write!(line, " callback {}", text(c));
            }
            out.line(&line);
        }
        for block in &s.on_entry_blocks {
            out.line("on entry:");
            out.nested(|out| {
                for a in block {
                    render_scxml_action(a, out);
                }
            });
        }
        for block in &s.on_exit_blocks {
            out.line("on exit:");
            out.nested(|out| {
                for a in block {
                    render_scxml_action(a, out);
                }
            });
        }
        if !s.initial_transition_actions.is_empty() {
            out.line("on initial:");
            out.nested(|out| {
                for a in &s.initial_transition_actions {
                    render_scxml_action(a, out);
                }
            });
        }
        if !s.initial_history_default_actions.is_empty() {
            out.line("on history-default:");
            out.nested(|out| {
                for a in &s.initial_history_default_actions {
                    render_scxml_action(a, out);
                }
            });
        }
        // `transition_index` is what the model keeps document order in;
        // selection depends on it, so a reviewer must read them in it.
        let mut ts: Vec<&crate::model::Transition> = s.transitions.iter().collect();
        ts.sort_by_key(|t| t.transition_index);
        for t in ts {
            render_scxml_transition(t, out);
        }
        if let Some(d) = &s.donedata {
            render_donedata(d, out);
        }
    });
    nested
}

fn render_scxml_transition(t: &crate::model::Transition, out: &mut Out<'_>) {
    let mut line = String::new();
    if !t.event.is_empty() {
        let _ = write!(line, "on {} ", text(&t.event));
    }
    if t.target.is_empty() {
        line.push_str("-> (no target)");
    } else {
        let _ = write!(line, "-> {}", text(&t.target));
    }
    if !t.transition_type.is_empty() {
        let _ = write!(line, " [{}]", text(&t.transition_type));
    }
    // The guard goes last: it is the line's only free-text value, and
    // no transition in this tree carries both a condition and a native
    // guard (measured over 587 documents), so "last" is unambiguous.
    if !t.cond.is_empty() {
        let _ = write!(line, " when {}", text(&t.cond));
    }
    if !t.native_payload_guard.is_empty() {
        let _ = write!(line, " native-guard {}", text(&t.native_payload_guard));
    }
    out.line(&line);

    out.nested(|out| {
        for id in &t.req {
            out.line(&format!("req {}", text(&id.to_string())));
        }
        for a in &t.actions {
            render_scxml_action(a, out);
        }
    });
}

fn render_donedata(d: &crate::model::DoneData, out: &mut Out<'_>) {
    out.line("done:");
    out.nested(|out| {
        for p in &d.params {
            let mut line = format!("param {}", text(&p.name));
            if let Some(e) = &p.expr {
                let _ = write!(line, " = {}", text(e));
            }
            if let Some(l) = &p.location {
                let _ = write!(line, " from {}", text(l));
            }
            out.line(&line);
        }
        match &d.content {
            crate::model::DoneDataContent::None => {}
            crate::model::DoneDataContent::Expression(e) => {
                out.line(&format!("content expr {}", text(e)))
            }
            crate::model::DoneDataContent::InlineText(e) => {
                out.line(&format!("content text {}", text(e)))
            }
            crate::model::DoneDataContent::Literal(e) => {
                out.line(&format!("content literal {}", text(e)))
            }
        }
    });
}

/// One item of executable content.
///
/// The field each tag reads is `Action::authored_fields`, measured over
/// 587 documents; the derived half is deliberately absent, because a
/// reviewer approving `cond_cpp` would be approving SCE's lowering.
fn render_scxml_action(a: &crate::model::Action, out: &mut Out<'_>) {
    match a.action_type.as_str() {
        "assign" => {
            // The common shape is one line with the expression last. An
            // `<assign>` carrying child content instead becomes a block:
            // `= <expr> content <c>` put free text before a keyword, and
            // one assign in this tree does carry content.
            // ⚠ The block is also what an EMPTY location gets. W3C
            // requires one, but the model can hold a document that
            // omitted it, and rendering that as ` = 1` produces a line
            // starting with the operator — which reads as nothing and
            // cannot be read back. Two fixtures in this tree do it.
            if a.content.is_empty() && !a.location.is_empty() {
                out.line(&format!("{} = {}", text(&a.location), text(&a.expr)));
            } else {
                let id = text(&a.location);
                out.line(
                    &(if id.is_empty() {
                        "assign:".to_string()
                    } else {
                        format!("assign {id}:")
                    }),
                );
                out.nested(|out| {
                    if !a.expr.is_empty() {
                        out.line(&format!("expr {}", text(&a.expr)));
                    }
                    out.line(&format!("content {}", text(&a.content)));
                });
            }
        }
        "cancel" => {
            let mut line = String::from("cancel");
            if !a.sendid.is_empty() {
                let _ = write!(line, " {}", text(&a.sendid));
            }
            if !a.sendidexpr.is_empty() {
                let _ = write!(line, " expr {}", text(&a.sendidexpr));
            }
            out.line(&line);
        }
        "log" => {
            let mut line = String::from("log");
            if !a.label.is_empty() {
                let _ = write!(line, " {}", text(&a.label));
            }
            if !a.expr.is_empty() {
                let _ = write!(line, ": {}", text(&a.expr));
            }
            out.line(&line);
        }
        "raise" => out.line(&format!("raise {}", text(&a.event))),
        "script" => out.line(&format!("script {}", text(&a.content))),
        "native_action" => {
            // One argument per line, as the algorithm's `call` does and
            // for the same reason. ⚠ No native action in this tree
            // carries an argument (measured: zero), so this arm is
            // written to the rule rather than to an example.
            if a.params.is_empty() {
                out.line(&format!("call {}", text(&a.native_action_name)));
            } else {
                out.line(&format!("call {}:", text(&a.native_action_name)));
                out.nested(|out| {
                    for p in &a.params {
                        out.line(&format!("arg {}", render_param(p)));
                    }
                });
            }
        }
        "foreach" => {
            // The array is an expression and goes last; the index is an
            // identifier and moves in front of it. Written the other
            // way round, an array containing ` index ` would decide
            // where the line breaks — and 8 foreach in this tree do
            // carry both an array and an index.
            let mut head = format!("foreach {}", text(&a.item));
            if !a.index.is_empty() {
                let _ = write!(head, " index {}", text(&a.index));
            }
            let _ = write!(head, " in {}", text(&a.array));
            out.line(&format!("{head}:"));
            out.nested(|out| {
                for inner in &a.actions {
                    render_scxml_action(inner, out);
                }
            });
        }
        "if" => {
            out.line(&format!("if {}:", text(&a.cond)));
            out.nested(|out| {
                for inner in &a.then_actions {
                    render_scxml_action(inner, out);
                }
            });
            for b in &a.elseif_branches {
                out.line(&format!("elif {}:", text(&b.cond)));
                out.nested(|out| {
                    for inner in &b.actions {
                        render_scxml_action(inner, out);
                    }
                });
            }
            if !a.else_actions.is_empty() {
                out.line("else:");
                out.nested(|out| {
                    for inner in &a.else_actions {
                        render_scxml_action(inner, out);
                    }
                });
            }
        }
        "send" => render_send(a, out),
        other => out.line(&format!("<unrendered action {}>", text(other))),
    }
}

fn render_param(p: &crate::model::Param) -> String {
    let mut s = text(&p.name).into_owned();
    if !p.expr.is_empty() {
        let _ = write!(s, "={}", text(&p.expr));
    }
    if !p.location.is_empty() {
        let _ = write!(s, "@{}", text(&p.location));
    }
    s
}

/// A `<send>`, as a block when it carries more than its event.
///
/// ⚠ The block is not decoration. `content`, `contentexpr` and the
/// expression clauses carry author text, and while they sat mid-line
/// after other clauses a value containing ` to ` or ` with ` decided
/// where the line broke. Both tokens DO occur inside authored strings
/// in this tree (measured: 5 and 2 occurrences). One free-text value
/// per line, at the end of it, removes the question instead of betting
/// that no author writes the word.
///
/// A bare `send <event>` stays one line: with nothing after the event
/// there is nothing to be ambiguous about, and most sends are that.
fn render_send(a: &crate::model::Action, out: &mut Out<'_>) {
    let clauses: Vec<(&str, &str)> = [
        ("eventexpr", a.eventexpr.as_str()),
        ("to", a.target.as_str()),
        ("to-expr", a.targetexpr.as_str()),
        ("type", a.send_type.as_str()),
        ("type-expr", a.typeexpr.as_str()),
        ("after", a.delay.as_str()),
        ("after-expr", a.delayexpr.as_str()),
        ("id", a.id.as_str()),
        ("id-into", a.idlocation.as_str()),
        ("namelist", a.namelist.as_str()),
        ("content", a.content.as_str()),
        ("content-expr", a.contentexpr.as_str()),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .collect();

    if clauses.is_empty() && a.params.is_empty() {
        out.line(&format!("send {}", text(&a.event)));
        return;
    }

    let mut head = String::from("send");
    if !a.event.is_empty() {
        let _ = write!(head, " {}", text(&a.event));
    }
    out.line(&format!("{head}:"));
    out.nested(|out| {
        for (name, value) in clauses {
            out.line(&format!("{name} {}", text(value)));
            // The deployment binds a target, so its facts belong under
            // the clause that names one. `to-expr` is deliberately not
            // annotated: an expression is resolved at run time and the
            // deployment has nothing to say about it — the builder
            // records that as a machine-level fact instead, where it
            // reads as "this send could not be resolved" rather than as
            // an absence nobody notices.
            if name == "to" {
                out.annotate_target(value);
            }
        }
        for p in &a.params {
            out.line(&format!("param {}", render_param(p)));
        }
    });
}

// ── Codec ──────────────────────────────────────────────────────
//
// The widest declarative kind: `CodecField` alone carries eighteen
// fields. A field is therefore rendered as a BLOCK rather than as one
// line — a line naming all eighteen would be unreadable, and an
// unreadable rendering fails the only purpose this module has.

fn endian_word(e: Endian) -> &'static str {
    match e {
        Endian::Big => "big",
        Endian::Little => "little",
        Endian::Native => "native",
    }
}

fn bit_size_words(b: &BitSize) -> String {
    match b {
        BitSize::Fixed { bits } => format!("fixed {bits}"),
        BitSize::Tail => "tail".to_string(),
        BitSize::LengthRef => "length-ref".to_string(),
        BitSize::Vle { width_bits } => format!("vle {width_bits}"),
        BitSize::Repeat { count_ref } => match count_ref {
            CountRef::LengthField(f) => format!("repeat length-field {}", text(f)),
            CountRef::UntilEof => "repeat until-eof".to_string(),
        },
        BitSize::TlvChain {
            max_depth,
            on_overflow,
            terminate_on,
        } => {
            let overflow = match on_overflow {
                TlvOverflowPolicy::Reject => "reject",
                TlvOverflowPolicy::Truncate => "truncate",
            };
            let terminate = match terminate_on {
                TlvTerminateStrategy::ExhaustOrDepth => "exhaust-or-depth".to_string(),
                TlvTerminateStrategy::EntryFlag { flag_name } => {
                    format!("entry-flag {}", text(flag_name))
                }
            };
            format!("tlv-chain max-depth {max_depth} on-overflow {overflow} terminate {terminate}")
        }
        BitSize::Embed => "embed".to_string(),
    }
}

/// A `present-if` predicate, including the `or_with` chain.
///
/// The scope is printed as a word (`local` / `input`) rather than in the
/// attribute's own encoding. The distinction decides which document the
/// flag is read from, so a reviewer has to see it; spelling it out is
/// the only form that says so without the reader knowing the attribute
/// grammar.
fn present_if_words(p: &PresentIfPredicate) -> String {
    let scope = match p.scope {
        PresentIfScope::Local => "local",
        PresentIfScope::Input => "input",
    };
    let mut s = format!(
        "{}{scope}:{}.{}",
        if p.negate { "not " } else { "" },
        text(&p.field_id),
        text(&p.flag_name)
    );
    if let Some(next) = &p.or_with {
        let _ = write!(s, " or {}", present_if_words(next));
    }
    s
}

fn render_codec(m: &CodecModel) -> String {
    let mut out = Out::new();
    let mut head = format!(
        "codec {} endian {}",
        text(&m.name),
        endian_word(m.default_endian)
    );
    if let Some(n) = m.input_length {
        let _ = write!(head, " input-length {n}");
    }
    out.line(&head);

    out.nested(|out| {
        for fi in &m.flag_inputs {
            out.line(&format!("flag-input {} width {}", text(&fi.name), fi.width));
        }
        for f in &m.fields {
            render_codec_field(f, out);
        }
        if let Some(v) = &m.variant {
            render_codec_variant(v, out);
        }
        for tv in &m.test_vectors {
            render_codec_test_vector(tv, out);
        }
    });
    out.buf
}

fn render_codec_field(f: &CodecField, out: &mut Out<'_>) {
    let at = match f.bit_offset {
        Some(b) => format!("{}.{b}", f.byte_offset),
        None => f.byte_offset.to_string(),
    };
    out.line(&format!(
        "field {}: {} at {at} size {}",
        text(&f.id),
        f.sce_type.as_attr(),
        bit_size_words(&f.bit_size)
    ));
    out.nested(|out| {
        if let Some(e) = f.endian {
            out.line(&format!("endian {}", endian_word(e)));
        }
        if let Some(n) = f.max_size {
            out.line(&format!("max-size {n}"));
        }
        if let Some(v) = &f.length_field {
            out.line(&format!("length-field {}", text(v)));
        }
        if let Some(n) = f.length_arith {
            out.line(&format!("length-arith {n}"));
        }
        if let Some(n) = f.max_count {
            out.line(&format!("max-count {n}"));
        }
        if let Some(v) = &f.repeat_body_alias {
            out.line(&format!("repeat-body {}", text(v)));
        }
        if let Some(v) = &f.tlv_chain_body_alias {
            out.line(&format!("tlv-body {}", text(v)));
        }
        if let Some(v) = &f.embed_body_alias {
            out.line(&format!("embed-body {}", text(v)));
        }
        if let Some(v) = &f.embed_length_from {
            out.line(&format!("embed-length-from {}", text(v)));
        }
        if let Some(n) = f.dma_burst_align {
            out.line(&format!("dma-align {n}"));
        }
        if let Some(q) = &f.quantity {
            out.line(&format!("quantity {} {} {}", q.scale, q.offset, q.unit));
        }
        if let Some(p) = &f.present_if {
            out.line(&format!("present-if {}", present_if_words(p)));
        }
        for fl in &f.flags {
            render_flag_def("flag", fl, out);
        }
    });
}

fn render_flag_def(keyword: &str, fl: &FlagDef, out: &mut Out<'_>) {
    let mut line = format!(
        "{keyword} {} bit {} width {}",
        text(&fl.name),
        fl.bit,
        fl.width
    );
    if let Some(v) = fl.value {
        let _ = write!(line, " value {v}");
    }
    out.line(&line);
}

fn render_codec_variant(v: &CodecVariant, out: &mut Out<'_>) {
    let mut head = String::from("variant");
    if let Some(f) = &v.tag_field {
        let _ = write!(head, " tag-field {}", text(f));
    }
    if let Some(f) = &v.tag_flag {
        let _ = write!(head, " tag-flag {}", text(f));
    }
    if let Some(p) = &v.peek_byte {
        let _ = write!(head, " peek-byte {}", text(&p.id));
    }
    out.line(&head);
    out.nested(|out| {
        if let Some(p) = &v.peek_byte {
            for fl in &p.flags {
                render_flag_def("peek-flag", fl, out);
            }
        }
        for a in &v.arms {
            let mut line = format!("arm {} -> {}", a.value, text(&a.body_alias));
            if a.is_default {
                line.push_str(" default");
            }
            out.line(&line);
        }
        if let Some(a) = &v.default_arm {
            let mut line = format!("default-arm {} -> {}", a.value, text(&a.body_alias));
            if a.is_default {
                line.push_str(" default");
            }
            out.line(&line);
        }
    });
}

fn render_codec_test_vector(tv: &CodecTestVector, out: &mut Out<'_>) {
    let hex: String = tv.hex.iter().map(|b| format!("{b:02x}")).collect();
    out.line(&format!("test 0x{hex} @line {}", tv.source_line));
    out.nested(|out| {
        let DecodedValue::Plain { fields } = &tv.decoded;
        for f in fields {
            let v = match &f.value {
                DecodedFieldValue::Bool(b) => format!("bool {b}"),
                DecodedFieldValue::Uint(u) => format!("uint {u}"),
                DecodedFieldValue::Int(i) => format!("int {i}"),
                DecodedFieldValue::Bytes(b) => {
                    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
                    format!("bytes 0x{h}")
                }
                DecodedFieldValue::String(s) => format!("string {}", text(s)),
            };
            out.line(&format!("{} = {v}", text(&f.name)));
        }
    });
}

// ── MCU kinds ──────────────────────────────────────────────────
//
// ⚠ The enum spellings below come from `schemas/sce-forge-ext.xsd`'s
// `xs:enumeration` values, NOT from the serde rename on the Rust enum.
// The two disagree: `LinkClass` and `InboxOrdering` carry
// `rename_all = "snake_case"` / `"kebab-case"` while the grammar spells
// them `raw_eth` and `acq_rel`, and a kebab rendering would print words
// no author ever wrote. What a reviewer compares against the
// specification is the authored spelling.

fn render_worker(m: &WorkerModel) -> String {
    let mut out = Out::new();
    let ordering = match m.inbox.ordering {
        InboxOrdering::AcqRel => "acq_rel",
        InboxOrdering::Relaxed => "relaxed",
    };
    let mut head = format!(
        "worker {} link-rx {} inbox depth {} ordering {ordering}",
        text(&m.name),
        text(&m.link_rx),
        m.inbox.depth
    );
    if let Some(o) = &m.outbox {
        let _ = write!(head, " outbox {}", text(o));
    }
    out.line(&head);
    out.buf
}

fn render_buffer_pool(m: &BufferPoolModel) -> String {
    let mut out = Out::new();
    let cache = match m.cache_policy {
        CachePolicy::Maintain => "maintain",
        CachePolicy::NonCacheable => "non-cacheable",
        CachePolicy::None => "none",
    };
    let mut head = format!(
        "buffer-pool {} slots {} size {} section {} align {} cache {cache}",
        text(&m.name),
        m.slot_count,
        m.slot_size,
        text(&m.section),
        m.alignment
    );
    if let Some(c) = &m.dma_channel {
        let _ = write!(head, " dma {}", text(c));
    }
    out.line(&head);
    match &m.variant {
        BufferPoolVariant::Default => {}
        BufferPoolVariant::Reassembly(r) => out.nested(|out| {
            out.line(&format!(
                "reassembly max-fragments {} timeout {}ms per-peer-quota {}",
                r.max_fragments_per_message, r.reassembly_timeout_ms, r.per_peer_quota
            ));
        }),
    }
    out.buf
}

fn render_link(m: &LinkModel) -> String {
    let mut out = Out::new();
    let class = match m.class {
        LinkClass::Udp => "udp",
        LinkClass::Tcp => "tcp",
        LinkClass::Serial => "serial",
        LinkClass::Websocket => "websocket",
        LinkClass::RawEth => "raw_eth",
    };
    let backpressure = match m.backpressure {
        BackpressurePolicy::Drop => "drop",
        BackpressurePolicy::Block => "block",
        BackpressurePolicy::SignalEvent => "signal-event",
    };
    let mut head = format!(
        "link {} class {class} framer {} backpressure {backpressure}",
        text(&m.name),
        text(&m.framer)
    );
    if m.accept_stage_copy_rate {
        head.push_str(" accept-stage-copy-rate");
    }
    out.line(&head);
    out.nested(|out| {
        for p in [
            ("rx-pool", &m.rx_pool),
            ("tx-pool", &m.tx_pool),
            ("stage-pool", &m.stage_pool),
        ] {
            if let Some(v) = p.1 {
                out.line(&format!("{} {}", p.0, text(v)));
            }
        }
        for e in &m.inbound {
            let mut line = format!("inbound {}", text(&e.event));
            if let Some(w) = &e.when {
                let _ = write!(line, " when {}", text(w));
            }
            out.line(&line);
        }
        for e in &m.outbound {
            out.line(&format!(
                "outbound {} encode {}",
                text(&e.event),
                text(&e.encode)
            ));
        }
    });
    out.buf
}

fn render_lookup(m: &LookupModel) -> String {
    let mut out = Out::new();
    out.line(&format!("lookup {}", text(&m.name)));
    out.nested(|out| {
        render_field(&m.input, out);
        render_field(&m.output, out);
        for e in &m.entries {
            let mut line = format!("{} -> {}", text(&e.key), text(&e.value));
            if !e.requirements.is_empty() {
                let ids: Vec<String> = e
                    .requirements
                    .iter()
                    .map(|r| text(&r.to_string()).into_owned())
                    .collect();
                let _ = write!(line, " req {}", ids.join(" "));
            }
            out.line(&line);
        }
        match &m.miss_policy {
            MissPolicy::Default(v) => out.line(&format!("miss default {}", text(v))),
            MissPolicy::Error => out.line("miss error"),
        }
    });
    out.buf
}
