# ADR 0038 — A host action states whether the machine depends on it, and a host states whether it performs

- Status: Accepted and implemented (the owner asked on 2026-10-19 for the upstream form to be
  designed for the long term, without regard to the downstream whose request raised the question,
  and for all of it to be built)
- Date: 2026-10-19
- Scope: `<sce:action>` (`docs/SCE_ACCEPTED_SUBSET.md` §2.11), the six generated host interfaces
  and the Interpreter's `INativeActionHost`, the `sce-codegen` stdout manifest,
  `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`, `schemas/sce-manifest.v1.schema.json`
- Related: `docs/adr/0005-sce-static-closes-its-gaps-by-bounding-and-declaring.md`,
  `sce-build/src/host_processor_analyzer.rs` (the same "published, not enforced" shape),
  `SCE_WIRE_CONTRACTS.md`

## Context

A `<sce:action name="…"/>` names an operation the host performs. SCE lowers it to an interface the
host implements: a Rust trait, a C++ abstract struct, a Kotlin or Go interface, a Python protocol,
a C11 vtable. What the build then enforces is that an implementation **exists**. It enforces
nothing about whether the implementation **does what the name says**, and nothing in the document
says whether the machine's correctness depends on that.

Those are different facts, and they are the ones a reader of the machine needs:

- `showAttempts(count, exhausted)` is a notice. The machine is correct whether the host renders it,
  logs it or drops it.
- `start_keepalive_worker` is a duty. The machine enters a state that is only correct if something
  is emitting keepalives, and leaves it by calling `stop_keepalive_worker`. A host whose method
  increments a counter satisfies the compiler and breaks the machine.

The second case happened. A downstream host implemented both methods as trace counters, the
machine ran with no keepalive emitter for as long as the host's own loop happened to be free, and a
callback that blocked past the lease closed the session. Nothing failed at build time, in any test,
or at start: it was found by measurement. It is cited here as one witness. It is not the
requirement, and it is not what fixes the shape; the shape follows from what SCE can and cannot
know.

**What SCE cannot know** is whether a body performs. No declaration, type or check closes that;
a method can always be written to do nothing.

**What SCE can do** is make the question answerable by someone other than the author of the
host: say which actions the machine depends on, make the host state in its own type what it does
with them, refuse the combinations that contradict each other, and publish the list. That is
what `needs_host_processor` does for a `<send type>` nobody implements, and it is what was missing
here.

## Decisions

1. **An action states its effect.** `<sce:action name="…" effect="duty|notice">`.
   - `duty`: the machine's correctness depends on the host performing the operation.
   - `notice`: it does not; the host may observe, record or drop it.

   The attribute is **required**: an action whose effect is unstated is the gap this ADR closes,
   and a period in which it is optional would leave the gap open under a lint that asks for the
   fix. An action without it, or with a value that is neither word, is refused at the element by
   the grammar (`xml/schema-validation`, naming the attribute and both words); the parser holds
   the same two refusals as a missing and an invalid attribute for a document the grammar did not
   see. One name carries
   one effect at every call site, as it already carries one signature, because the effect is part
   of the one host method that serves them: a name that is a duty here and a notice there is
   refused as `validation/native-action-signature-conflict`.

   The attributes are written unqualified, as `name` and `expr` are: the element is SCE's own.

2. **A duty entered on `<onentry>` names its release.** `released-by="stop_keepalive_worker"`.
   The release is an action of the same machine, a duty itself (a release the host may drop leaves
   the entry's duty held), performed on the `<onexit>` of the same state, so every way out of the
   state releases what the entry took. A duty with no `released-by` is legal and lasts the
   machine's life. A `released-by` off an `<onentry>`, on a notice, naming no action, or naming one
   that is not a duty on this state's `<onexit>`, is refused as
   `validation/native-action-release`, with the reason in the message.

3. **A host states what it does with duties, in its own type.** The generated interface of a
   machine that states a duty carries one member with no default, whose value is `performs` or
   `simulates`. A machine with no duty has no such member and takes any host, as before.

   | backend | the member | refusal of a simulating host | accepting one on purpose |
   |---|---|---|---|
   | Rust | `const REALIZATION: ::sce_rust_runtime::ActionRealization` | compile time: `Policy::new` evaluates an associated const | `Policy::new_simulated` |
   | C++ | `virtual SCE::ActionRealization realization() const = 0` | `std::invalid_argument` at construction | a trailing `SCE::SimulatedHost::Accept` constructor argument (an enum, so a `bool` cannot be passed by accident) |
   | Kotlin | `val realization: ActionRealization` | `require` at construction | `acceptSimulatedHost = true` |
   | Go | `Realization() sce.ActionRealization` | `New<Machine>Policy` panics | `NewSimulated<Machine>Policy` |
   | Python | `realization: ActionRealization` | `ValueError` at construction | `accept_simulated_host=True` |
   | C11 | `sce_action_realization_t realization` in the vtable (`SCE_ACTION_UNSTATED` is zero, so a table that never says is refused as a missing member is) | `_init_with_actions` returns false | `_init_with_simulated_actions` |

   `Performs` is a statement the host's author makes about the bodies of the methods, in a line a
   reviewer can find. SCE cannot read the bodies; an empty body under `Performs` is that author's
   false statement and not an unwritten fact. The refusal is made where the machine is built with
   a host, and a child machine that a parent's host answers is built the same way, so a child's
   duties are checked at the child's construction and no check is repeated at the parent.

4. **The generated `Recording…` host simulates.** It is a recorder by construction. It exists for
   Kotlin and Rust (`kotlin_recording_host`, `rust_recording_host` in
   `sce-build/src/forge/native_action.rs`); wherever it exists it declares `simulates`, so a harness
   that drives a machine with duties through it says so at the call and a production wiring that
   reaches for it by accident is refused.

5. **The manifest publishes the duties.** `sce-codegen`'s stdout manifest gains `host_duties`, one
   record per site that states a duty (`machine`, `action`, `state`, `on` — `entry`, `exit`,
   `transition`, `initial` or `history-default` — and `released_by` when there is one), beside
   `needs_host_processor`. It is published and not enforced, for the reason
   `host_processor_analyzer` gives: the consumer decides whether to gate on it, and a build that
   deliberately drives a machine with a test double keeps working.

6. **The Interpreter says the same.** `INativeActionHost` already answers whether the host
   performed an operation, and an unanswered one raises `error.execution`. It gains the same
   declaration (`realization()`, pure virtual), and `StateMachine::start()` refuses a simulating
   host of a document that states a duty unless the host was installed through
   `setSimulatedNativeActionHost`, so the Interpreter and the six generated engines refuse the
   same hosts. Its parser reads the two attributes and refuses an action that states no effect.

7. **The pseudocode says it too.** The reading of a statechart that a specification's owner is
   shown renders the effect and the release as clauses of the `call` block, and reads them back,
   so what the owner reads is what the machine is.

## Considered and not chosen

- **Infer a duty from the name** (`start_*`, `stop_*`). A heuristic over spelling decides a
  correctness property; the first action named `begin_` is silently a notice.
- **Declare realization in the deploy** (`machines.<m>.actions.<name>.realization`). A claim made
  in `deploy.yaml` is a claim made away from the code it is about, and the deploy is not what the
  host's compiler reads. The deploy keeps what it already owns (scheduler, links, queues).
- **A typed handle that `start` returns and `stop` consumes.** It ties the pair together in the
  signature and is worth having, but a host can produce a dummy handle as easily as an empty
  body, so it adds a shape without adding a refusal. It may follow decision 2 later as a
  strengthening.
- **Tie a duty's concurrency need to `scheduler.session_runtime.drive`.** Looks natural, is wrong:
  a host may satisfy a concurrent duty on a thread of its own outside the session runtime (the
  witness did exactly that), so an `own-thread` session runtime does not contradict a concurrent
  duty.
- **A runtime witness that a duty ran** (a heartbeat the machine checks). It has to be written
  per duty, and what it proves is that the witness ran.
- **Make the effect optional first and require it once the corpus states it.** A period of
  "optional" is a period in which the gap is open, and the corpus this tree owns is migrated in the
  same change. A consumer's document that lacks it is refused with a code that names the missing
  attribute and its two values, which is the repair.

## Consequences

- The attributes and the manifest field are wire surface: `schemas/sce-forge-ext.xsd`,
  `apis/forge-ast.v1.schema.json` and `schemas/sce-manifest.v1.schema.json` change under
  `SCE_WIRE_CONTRACTS.md`; all three are `pre-release`.
- One new diagnostic code, `validation/native-action-release`, with its acceptance-doc row; the
  other refusals reuse `xml/schema-validation`, `validation/missing-attribute`,
  `validation/invalid-attribute` and `validation/native-action-signature-conflict`.
- Every `<sce:action>` in a committed document states its effect, and every committed generated
  tree is regenerated. A host of a machine that states a duty adds one member; a host of a
  machine of notices is untouched.
- Decision 4 does not extend the `Recording…` host to the four backends without one.
  `docs/SCE_ACCEPTED_SUBSET.md` §2.11 said Kotlin alone has it; Rust has had it since, and that
  sentence is corrected with this change.

## Open

- Whether a duty should be able to state the context it needs (concurrent, interrupt-safe) so a
  host's declaration can say which of them it provides. Nothing here needs it, and the rejected
  scheduler tie above is the warning against inventing it from one witness.
- The attributes are not checked for a misspelled sibling (`releasedby`): `<sce:action>` is SCE's
  own element and carries no unknown-attribute refusal today. `schemas/sce-forge-ext.xsd` declares
  its attributes and refuses an undeclared one where a document is validated against it.
