# ADR 0129 — A document of the static class is printed either for the shared interpreter or as its own step function; the image chooses, and a differential test binds the two forms

- Status: Accepted (2026-11-25), on the owner's decisions, under the owner's standing rule: the most long-term
  correct, textbook structure, cost ignored. The Rust part is implemented in the order of the last section; its
  figures are the footprint gate's and the consumer's measurement at the landing (section "Acceptance").
- Date: 2026-11-25
- Amends: ADR 0064 (decision 6, "there is one structure and no build mode": for a document of the static class the
  image chooses between two forms of one structure, per document, and no runtime feature or `cfg` chooses).
- Related: ADR 0053 (the runtime's features only add), ADR 0066 (a program pays for what its documents use), ADR 0068
  (the main loop is one copy), ADR 0076 (the internal queue's proven depth), ADR 0118 (Proposed: a document's cost is
  computed from its plan), ADR 0123 (the core decides where a run ends), ADR 0125 (what a document fixes of a
  microstep is computed when it is built), ADR 0127 (one selection reads the rows of every machine; both forms read
  it, decision 2), ADR 0131 (a policy's hook is handed the engine's state apart from the policy, in both forms).
- Requested by: a downstream bare-metal consumer, 2026-11-25.

## Context

The consumer measured its release image (fat LTO, `opt-level = 3`, read with `nm` and `objdump`). It links two
generated machines:

| machine | shape |
|---|---|
| a unicast session machine | 13 states, 2 of them compound, 30 transitions, 19 events; 1 `cond`, 1 `<raise>`, 2 `<send>`, 9 `<onentry>`, 2 `<onexit>`, 1 `<script>`; no `<parallel>`, `<history>` or `<invoke>` |
| a reassembly slot machine | flat, 6 states, 12 transitions |

The shared runtime body is 4,713 bytes:

| symbol | bytes |
|---|---|
| `Turn::run` (the main loop, with what is inlined into it) | 1,632 |
| `exit_states_from_rows` | 656 |
| `enter_states_from_rows` | 616 |
| `select_transitions` | 544 |
| `Turn::take_microstep` | 390 |
| `exit_interpreter` | 256 |
| the rest | 619 |

The per-machine shells are 973 and 713 bytes. `Turn::run` makes 48 calls, 45 of them through the `dyn` hooks of
ADR 0064 decision 3. With two machines the interpreter costs between 816 bytes (`Turn::run` alone) and 3,200 bytes
for each machine. A step function per document is estimated at 1,150 to 1,700 bytes for both machines, and the
reassembly machine runs once for every fragment of a message, so its calls through `dyn` hooks are on the image's
hottest path.

ADR 0064 chose the table-driven form for every document because it is smaller once an image holds enough machines,
and ADR 0125 listed "generating each machine's exits and entries as code" as the per-machine copy ADR 0064 removed.
Both judgements were about a program of many machines. For an image of two, the shared body is larger than what it
shares, and which side of that line an image is on is a fact of the image, not of the document.

## Decision

### 1. Two forms of one algorithm, as compiler construction has them

The textbook basis is the choice every scanner generator offers between a table-driven automaton and a directly
coded one: one driver reading per-language tables, against code generated for each language with the tables
written into its branches (Aho, Lam, Sethi and Ullman, *Compilers: Principles, Techniques, and Tools*, chapter 3;
re2c generates directly coded automata where flex reads tables). Both implement one semantics. Here the
semantics is W3C SCXML Appendix D (§scxml-D, its main event loop §scxml-D-mainEventLoop and the macrostep of
§scxml-3.13), the table-driven form is the shared interpreter of ADR 0064 over the rows of ADR 0125, and the directly
coded form is a step function printed for one document.

The directly coded form is specialised to a class, as a directly coded scanner is to its language. A document of the
static class has

- no `<parallel>`, so a configuration is one atomic state and its ancestors, and a selection takes one transition at
  most;
- no `<history>`, so what a transition enters depends on the transition alone;
- no transition of more than one target, and no compound state whose initial transition does not enter one of its own
  descendants, so what a transition enters is one path down the tree (ADR 0125 decision 2);
- no `<invoke>`, so a macrostep starts no session.

Its internal queue's bound is known when it is built in every case: the generator proves a depth from the document
(ADR 0076), or gives the queue the external rule's depth (the capacity the document or its deployment states, or the
runtime's default) where it proves none. The queue is the engine's in both forms and the step function reads nothing
of it, so the class asks nothing more of it. Asking for a proven depth would refuse every document that raises an
event it answers, since ADR 0076 proves no depth for such a document; the session-shaped fixture of decision 4 is one.

In that class what a transition exits from a given atomic state is fixed too: every active state below its domain,
which is the atomic state and its ancestors up to the domain. So each step is a list of constants, and the step
function writes them out as the arms of a `match`:

- for each atomic state, the states a selection asks, the state first and each ancestor with transitions outward;
- for each written transition and each atomic state it can be selected from, the states it exits, whether its
  content runs, the states it enters with whether each is entered by default, the atomic state it leaves the machine
  in, and whether that state ends the run;
- for each atomic state, the states the end of the run exits;
- the states the document's initial transition enters.

`sce_build::step_function::StepFunction::of` computes them, from the same `DocumentTables` and entry paths the row
walk reads, so the arms and the rows have one producer.

### 2. The whole macrostep, the same hooks, the same names

The step function replaces what reads the document: the selection walk and the exit and entry walks. It does not
replace the main loop, and it does not replace the policy.

- **One main loop, compiled per form.** The loop of ADR 0068 (`engine::Turn`) is written once and is generic over
  the form: over `&mut dyn MainLoop` with the shared procedures, which a program compiles once for every machine
  printed for the interpreter, or over the engine itself with the policy's step function, which it compiles for that
  machine with every hook a direct call. Eventless transitions are taken to completion, the internal queue is
  processed before the next external event, the macrostep and external-event budgets bound both, the error chain is
  counted, the external queue is drained unseen at the end of a run and the interpreter is exited: by the same lines
  in both forms.
- **One selection.** The arms of `select_transition` name which states a selection asks and in what order, the atomic
  state first and each ancestor with transitions outward, which is the walk the shared `select_transitions` makes up
  the tree. Each state asked is answered by `Steps::enabled`, which is `DocumentTables::first_enabled` over the
  selection rows of the policy's own tables (ADR 0127): the rows say which of the state's transitions the event
  enables, in the order written, and the policy's `guard_holds` is asked for each of those until one holds. That is
  the call the shared selection makes for each state of its chain, over the same rows, so the two forms ask the same
  guards in the same order by construction, and neither form generates a `first_enabled_transition`: the selection
  rows are table data of every generated machine, written in both forms. `first_enabled` is generic over the
  machine it asks: the shared selection calls it with `&mut dyn Behaviour`, one copy for every interpreted machine,
  and a step function with the engine itself, so its guard calls are direct.
- **The same hooks.** Each arm calls the engine's `Steps`: `enabled`, `exit`, `content` and `enter`, which run the
  generated policy's `guard_holds`, `execute_exit_actions`, `execute_transition_content` and
  `execute_entry_actions` exactly as the shared procedures do, and keep the run store as they do. So the guards and
  their errors (a `cond` that fails is false and raises `error.execution`, §scxml-5.9), the order of `<onexit>`,
  transition content and `<onentry>`, `done.state.<id>` raised on entering a `<final>` child of a compound state
  (the policy's entry hook raises it), and a test component's take step are one code in both forms. Each hook is
  dispatched as ADR 0131 dispatches it, in both forms: the policy as `&mut self` and the engine's state as
  `engine: &mut EngineState<Self>`, two fields of the engine borrowed apart.
- **Where a run ends.** An arm that enters a `<final>` child of `<scxml>` reports it as `Entered::ended_in`, as the
  shared `enter_one` does from the tables (ADR 0123 decision 2); the generator knows it when it writes the arm.
- **The same names.** The policy keeps its type, its `<Machine>Actions` host trait, its constructors, its
  `<Machine>State` and `<Machine>Event` enums and their `ID_` constants, its tables and its `Engine<P>` API. A
  host's binding code is the same for both forms; only the build line that generates the machine differs.

In the runtime this is:

- `StepFunction`, the trait the generated policy implements: `enter_initial_configuration`, `select_transition`,
  `microstep` and `exit_configuration`, each generic over `M: Steps`;
- `Steps`, what the arms call, implemented by the engine alone;
- `StepFunctionForm<P>`, the four procedures as the engine holds them, and `StatePolicy::STEP_FUNCTION`, `None` by
  default and `Some(StepFunctionForm::of())` for a printed policy. The engine reads the constant where it builds a
  turn (`Engine::run_turn`, `Engine::exit_interpreter`), so a machine names the code of its own form and no other,
  and a program none of whose machines is interpreted references none of the shared loop or walks;
- `NO_WALK`, the value a printed document's tables carry as their walk, with no entry rows: no shared procedure walks
  its microsteps, and the two procedures it holds stop the program if one is ever asked to, as tables that name a
  feature without its procedures do. The tables are otherwise the document's, as the interpreted form has them, so
  what reads them besides the walks (the selection rows of ADR 0127, which `Steps::enabled` reads, the transition
  observer of ADR 0089, `is_in_final_state`, `enter_at`'s check, the error events) reads the same rows;
- the workspace of a printed document holds one transition and nothing else.

The atomic state an arm is chosen by is read from the configuration the engine keeps: the state of the largest number
in it, since every other active state is one of its ancestors and a state is numbered after its ancestors. A
restored configuration (`Engine::enter_at`, a saved state) is read the same way.

### 3. The image chooses, document by document

The build that generates the machines of one image states the form of each: `sce-codegen generate --step-function`,
`check --step-function` with the same verdict, or `StatechartCodegenOptions::form` for a build script. The default
is the interpreted form, and a document without the option is printed byte for byte as before, but for the
template hash every generated header carries. A document a run reaches (an import, a child) is printed for the
interpreter unless its own run says otherwise.

A step function asked of a document outside the class is refused with `codegen/step-function-outside-class`, which
names every reason and is located on the element the first is about. It is never printed for the interpreter in its
place: the build asked for a form, and silently getting the other would move the image's size and speed without a
word.

Choosing the form by cost, automatically, is not decided here. It belongs to ADR 0118, which computes a document's
cost from the plan its code is printed from, once its byte model is calibrated against linked images: the choice is a
property of the image (how many machines share the interpreter), and only a model of the linked image can make it.
Until then the build states it.

### 4. A differential test binds the two forms

For every document of the class in the corpus the Rust lane compiles both forms of, seeded and reproducible random
sequences of external events drive both, and every observation is compared, step for step: the configuration, the
atomic state, whether the machine runs and where it ended, each transition taken (by the row of its transition table,
ADR 0089), each call of the host's actions, and the counts of discarded, unseen and unanswered error events, which is
where a raised or sent event the other form would not have taken shows. The test is
`backends/rust/tests/tests/a_step_function_runs_as_the_shared_interpreter_does.rs` over
`sce_rust_tests::step_function_differential`. Its documents are two consumer-shaped fixtures (the shapes of the
table above, in `tests/integration/step_function/`) and the class documents of `integration_resources` (the flat
and the nested lifecycle).

The W3C lane registers one machine per fixture (`docs/SCE_W3C_TEST_REGISTRATION.md`): `generate-w3c` writes a module
and a `SimpleAotTest` per fixture, and the registry carries no second form. Running the class fixtures in both
forms is the next landing, in this order: `generate-w3c` gains `--step-function`, which generates every fixture of
the class (and skips, by the refusal's exit status, every other) into `backends/rust/tests/src/generated_step_function`
with test modules `tests/w3c_step_function.rs`; `tools/codegen/trees/rust-w3c.toml` gains that generation; and
`backends/rust/tests/build.rs` declares the second module tree. Each fixture's machine then runs the conformance
check in both forms, and the differential driver of this ADR runs over each pair.

### 5. The other backends

The C11 and C++ backends have the same two forms to offer, from the same `StepFunction::of`: each is its own landing
with its own measurement, as ADR 0125 decision 7 has it. This ADR decides their direction only.

## Why this is not the build mode ADR 0064 decision 6 refused

Decision 6 refused a feature or `cfg` that replaced how the algorithm is dispatched, for three reasons, and none
applies:

- **ADR 0053, features only add.** No runtime feature or `cfg` is added. The form is a constant of each generated
  policy, the way the walk of ADR 0125 is a constant of each document's tables.
- **Two copies of Appendix D's parameterisation.** The main loop is one source in both forms, and the hooks are one
  code. What the step function writes out is the set computation of ADR 0125, from the same producer; the
  differential test holds the two forms of every class document to each other, and the shared procedures stay the
  reference.
- **Speed and size are measured, not assumed.** The footprint gate weighs an image of interpreted machines as before,
  and gains a configuration of printed ones (section "Order of the changes").

## Consequences

- An image whose machines are all printed as step functions links none of `Turn<dyn MainLoop>`, `select_transitions`,
  the row walk or the general walk; each machine carries its own loop and arms. An image that mixes the forms links the
  shared body once and each printed machine's own code beside it.
- ADR 0064 decision 7's third figure (no core symbol carries a machine type) holds of the interpreted form. A printed
  machine's turn is its own by design and carries its type; the gate tells the two apart by the form of the probe's
  machines.
- `StatePolicy` gains a defaulted constant, so a hand-written policy compiles unchanged and is interpreted.
- `DocumentTables::walk` gains a third value, `NO_WALK`; the debug name of a walk is its constant's name.
- ADR 0127's selection rows are the selection of both forms. A printed document's arms choose which states are asked,
  and each is answered by `DocumentTables::first_enabled` over the rows, which asks `guard_holds`; no generated
  policy of either form writes `first_enabled_transition`, which stays for tables written by hand. The runtime test
  `a_printed_step_function_selects_by_the_rows_of_its_tables` runs one hand-written document in both forms over
  the same rows and holds every hook call of the one to the other.

## Alternatives considered

- **One form for every document** (ADR 0064 as it stood). An image of two machines pays more for the shared body
  than it shares, measured above. Refused for the class; kept as the default.
- **Choosing by a runtime feature.** Refused by ADR 0053, as in ADR 0064 decision 6.
- **Choosing automatically by a size estimate in the generator.** Without the byte model of ADR 0118 calibrated
  against linked images, the estimate would be a guess the build could not check. Deferred to ADR 0118.
- **A step function that prints the main loop too, as a second copy of it per document.** The loop decides nothing
  the document fixes; a second copy of it is the duplication ADR 0068 removed. Monomorphising the one loop over the
  form gives the directly coded loop without a second source.
- **Exits read from the configuration at run time, as the row walk reads them.** That keeps a walk in the printed
  form. The arms are per atomic state instead, which is what a directly coded automaton is: the state is the branch.
- **Classes beyond the static one.** A `<parallel>` makes the selection a set and the exit set depend on several
  atomic states; a `<history>` makes what is entered a record read at run time. Their step functions would carry a
  walk of their own. Refused: they keep the shared interpreter.

## Order of the changes

1. This ADR, and "Amended by: ADR 0129" on ADR 0064.
2. The runtime: `Turn` generic over the form, `Form` with the shared interpreter's `Tables`, `StepFunction`,
   `Steps`, `StepFunctionForm`, `StatePolicy::STEP_FUNCTION` and `NO_WALK`.
3. `sce-build`: `step_function`, `RustMachineForm` and `StatechartCodegenOptions::form`, `--step-function` on
   `generate` and `check`, `codegen/step-function-outside-class`, and the Rust template's step function.
4. The differential test, its two consumer-shaped fixtures, and the generations of both forms in
   `tools/codegen/trees/rust-integration.toml`.
5. The main session regenerates the committed trees (the template hash moves), runs the Rust lanes and clippy, and
   adds the footprint gate's configuration of two printed machines beside `plain-documents`.
6. The W3C lane's second form (decision 4).
7. The consumer measures its image with both machines printed as step functions.

## Acceptance

Accepted on all of these: the differential test passes for every document it drives; the Rust lanes pass with every
interpreted machine's generated text unchanged but for its header; the footprint gate's image of printed machines
links no symbol of the shared loop or walks; and the consumer's image of two printed machines is smaller than the
interpreted one and its turnaround does not regress.
