# ADR 0038 — A host action states whether the machine depends on it, and a host states whether it performs it

- Status: Proposed (the owner asked on 2026-10-19 for the upstream form to be designed for the
  long term, without regard to the downstream whose request raised the question)
- Date: 2026-10-19
- Scope: `<sce:action>` (`docs/SCE_ACCEPTED_SUBSET.md` §2.11), the six generated host interfaces
  and the Interpreter's `INativeActionHost`, the `sce-codegen` stdout manifest,
  `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`
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
what `needs_host_processor` does for a `<send type>` nobody implements, and it is what is missing
here.

## Decisions

1. **An action declares its effect.** `<sce:action name="…" sce:effect="duty|notice">`.
   - `duty`: the machine's correctness depends on the host performing the operation.
   - `notice`: it does not; the host may observe, record or drop it.

   One name carries one effect everywhere it appears, as it already carries one signature
   (`validation/native-action-signature-conflict`); a name with two effects is refused by a code of
   its own. The attribute is absent on every document today. The end state is that it is
   **required**, because an action whose effect is unstated is the gap this ADR closes. It gets
   there in two steps rather than one: first optional, with the manifest listing the actions
   that state none (`actions_without_effect`) and `sce-codegen --lint` refusing them; required
   once the committed corpus states it. The first step is not a workaround, since the end state is
   fixed here and the lint is what drives the migration.

2. **A duty entered on `<onentry>` names its release.** `sce:released-by="stop_keepalive_worker"`.
   The release must be an action of the same machine and must sit on the `<onexit>` of the same
   state, so every way out of the state releases what the entry took. A duty with no release is
   legal (a duty that lasts for the machine's life), and says so by omitting the attribute. A
   `released-by` that names nothing, or that sits elsewhere, is refused.

3. **A host states what it does with duties, in its own type.** Each generated interface carries
   one required member with no default, and its values are `performs` and `simulates`:

   | backend | the member |
   |---|---|
   | Rust | an associated const of the actions trait |
   | C++ | a `constexpr` static of the abstract struct |
   | Kotlin / Go / Python | a required property / method |
   | C11 | a field of the vtable |

   A host that `performs` is a host whose author has said so beside the code. A host that
   `simulates` is a test double. The machine refuses a `simulates` host unless the caller opts in
   by name: a compile-time assertion on Rust and C++ (the host type is known), a refused
   construction elsewhere (`_init_with_actions` already returns `false` for an unfilled vtable, and
   this is the same refusal). A machine with no duty takes any host, as now. The ambiguous third
   state, a host that implements the method and does nothing, is not a value: writing `performs`
   over an empty body is the author's false statement, in a line a reviewer can grep for, rather
   than an unwritten fact.

4. **The generated `Recording…` host simulates.** It is a recorder by construction. It exists for
   Kotlin and Rust today (`kotlin_recording_host`, `rust_recording_host` in
   `sce-build/src/forge/native_action.rs`); wherever it exists it declares `simulates`, so a harness
   that wants to drive a machine with duties through it says so at the call and a production
   wiring that reaches for it by accident is refused.

5. **The manifest publishes the duties.** `sce-codegen`'s stdout manifest gains `host_duties`
   (for each: the action, the state that enters it, its release) beside `needs_host_processor`. It
   is published and not enforced here, for the reason `host_processor_analyzer` gives: the
   consumer decides whether to gate on it, and a build that deliberately drives a machine with a
   test double keeps working.

6. **The Interpreter says the same.** `INativeActionHost` already answers whether the host
   performed an operation, and an unanswered one raises `error.execution`. It gains the same
   declaration, so the Interpreter and the six generated engines refuse the same hosts.

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
- **Tie a duty's concurrency need to `scheduler.session_runtime.drive`.** Looks natural, is wrong: a host may
  satisfy a concurrent duty on a thread of its own outside the session runtime (the witness did
  exactly that), so an `own-thread` session runtime does not contradict a concurrent duty.
- **A runtime witness that a duty ran** (a heartbeat the machine checks). It has to be written
  per duty, and what it proves is that the witness ran.

## Consequences

- The attribute `sce:effect` and the manifest fields are wire surface: `schemas/sce-forge-ext.xsd`,
  `apis/forge-ast.v1.schema.json` and the manifest each change under `SCE_WIRE_CONTRACTS.md`, and
  its registry rows move with them.
- New diagnostic codes, each with its acceptance-doc row: a name with two effects, a
  `released-by` naming nothing, a `released-by` off the state's `<onexit>`, and the lint code for
  an undeclared effect.
- The six interfaces and the two `Recording…` hosts change shape, so every committed generated tree
  that declares an action is regenerated, and every host that implements an interface adds the
  member. That is the cost, and it is paid once.
- Decision 4 does not extend the `Recording…` host to the four backends without one.
  `docs/SCE_ACCEPTED_SUBSET.md` §2.11 still says Kotlin alone has it; Rust has had it since, so
  that sentence is stale and is corrected when stage 2 lands.

## Open

- Whether `notice` needs to be spelled at all, or is what an action is when it says nothing and
  `duty` is the only attribute. The lint in decision 1 reads more clearly with both spelled;
  the migration is cheaper with one.
- Whether a duty should be able to state the context it needs (concurrent, interrupt-safe) so a
  host's declaration can say which of them it provides. Nothing here needs it, and the rejected
  scheduler tie above is the warning against inventing it from one witness.

## Order

1. The model, the validation and the three wire surfaces for decisions 1 and 2, with the lint and
   the manifest fields (decision 5). No backend changes.
2. The realization member in the six backends and the Interpreter, with the `Recording…` host
   (decisions 3, 4 and 6).
3. The corpus migration, then the flip of decision 1 from optional to required.
