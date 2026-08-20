# SCE Integration Fixture Layout (Non-W3C-IRP)

This document records the per-backend layout for hand-curated integration
fixtures — SCXML programs that exercise semantics not covered by the W3C
IRP suite. The W3C IRP suite (under `resources/<N>/test<N>.txml`, where
each fixture is regenerated to `test<N>.scxml` and consumed by the
per-backend W3C harnesses) remains the spec-conformance baseline; the
integration layer described here is strictly additive.

The stems are the directories under `integration_resources/`; this document
does not restate them. An earlier revision did, and the list was wrong within
one round of a stem being added — `event_origin_is_a_location` landed and the
sentence still named eleven.

A fixture placed there is a **seven-channel commitment** — C++ Interpreter,
C++ AOT, Rust, Go, Kotlin, Python, C11 — and the commitment splits in two.

The sites that GENERATE each channel are loud when they are missing:
`scripts/regen_<stem>{,_go,_kotlin,_python}.sh` (without them
`regen_all_committed_trees.sh` exits non-zero), the `pub mod <stem>;` in the
Rust integration tree (`rust-modrs-drift` blocks the push), and the two CMake
registrations (the build fails).

The sites that ASSERT are silent. Generated code that nobody runs still
compiles, so a stem can land with a machine in all seven channels and a test
driver in two while every gate stays green. That is not hypothetical:
measured 2026-08-11, `event_origin_is_a_location` had exactly that shape, and
when the five silent channels were finally asked, all five were violating the
clause the fixture exists to prove. `sce-build/tests/integration_stem_registration.rs`
now requires both halves — including this document's own entry, because a
fixture with no recorded axis is an axis nobody chose.

A fixture whose semantics only some backends implement does not belong here —
its committed trees would advertise coverage that does not exist. Close the
parity gap first, then add the fixture.

`autoforward_event_fields` covers W3C §6.4's exact-copy requirement for
`<invoke autoforward="true">`: the forwarded event must reach the child with
its `_event.data`, `_event.origin` and `_event.invokeid` intact. Every channel
asserts it — C++ Interpreter + AOT, Rust, Go, Kotlin, Python, C11 — because
each backend forwarded only the event name (Python: name + payload) until the
carrier landed.

**Fixtures stay on one axis.** `autoforward_event_fields` returns the child's
verdict as its own event rather than as `<donedata>`, so a regression in the
donedata lift cannot surface as an autoforward failure; `donedata_local_invoke`
owns that axis. An earlier revision coupled the two and a C11 donedata gap
masqueraded as an autoforward bug on that backend alone.

The donedata axis is split the same way. `donedata_local_invoke` pins the
payload *shapes* — a `<param>` table and a `<content>` scalar — on a child
whose initial configuration is already its top-level `<final>`, so the lift
and the `done.invoke.<id>` raise sit in the same call. `donedata_late_completion`
pins the *timing*: the child answers an event first and reaches `<final>` two
macrosteps in, which is a different completion-detection site in every AOT
backend. It reuses the sibling's `<param name="result" expr="42"/>` payload
verbatim so a shape the sibling already proves green cannot be what fails
there. Deleting the late-completion lift from
`tools/codegen/templates/c/invoke_methods.jinja2` reds
`c11_integration_donedata_late_completion` while
`c11_integration_donedata_local_invoke` stays green — the sibling is
structurally blind to that site.

`send_param_payload` covers W3C §6.2's `<param>` payload contract on the two
send paths that had no runtime witness — one `<send target="#_parent">` from a
`datamodel="null"` child, which needs no script engine, and one
`<send target="#_internal">` whose params must arrive as `_event.data`. Both
were fixed at the template layer while no committed fixture had a machine of
either shape, so every suite could show was that nothing regressed. The two
land in distinct final states (`failChildPayload` / `failInternalPayload`) so a
failure names the path rather than reporting "payload lost". Adding it closed
two C11 parity gaps that the missing fixture had hidden: a literal param was
formatted through the runtime Lua formatter, which does not compile in a
machine with no `lua_State`; and `<send target="#_internal">` with `<param>`
children fell through to the unenumerated-shape fallback and raised
`error.execution`.

The autoforward family is three fixtures on three questions, and each was
built blind to the other two. `autoforward_done_invoke` pins *which* events
are forwarded: Appendix D's `mainEventLoop` forwards whatever comes off the
external queue without consulting its name, so `done.invoke.<id>` is inside
the set and the only exclusion — the cancel event — is expressed as control
flow rather than as a name test. `autoforward_internal_queue` pins the
negative half, which the same loop expresses purely by position: the internal
drain has no forwarding step at all, so an `error.execution` raised there must
never reach a child, and must be excluded by *where it was raised* rather than
by a filter that recognised its name. `autoforward_dequeue_point` pins *when*:
the forward sits one statement after the dequeue and before transition
selection, which neither sibling can see, because both were deliberately built
to be blind to it.

`invoke_precedes_external_dequeue` and `invoke_precedes_dequeue_midrun` split
Appendix D's invoke-before-dequeue ordering the same way. The first pins the
start-up case: the external queue is named exactly once in `mainEventLoop` and
it is after `invoke(inv)`, so an engine that folds the external drain into the
macrostep loop consumes what `<onentry>` queued while the children do not yet
exist — a lost event, not a reordered one. The second pins that the ordering
is not a property of start-up: `statesToInvoke` is filled by `enterStates`, so
a state entered by an *external* event's transition arms an invoke that must
start before the next event comes off the queue. An engine that drains to
exhaustion inside one step passes the first and fails the second.

`nested_final_not_terminal` covers W3C SCXML 3.7: only a `<final>` whose parent
is the `<scxml>` element ends the session. Appendix D's `enterStates` splits
the two cases in one branch, so `isFinalState(s)` is a structural question and
not by itself the completion criterion. An engine that answers "has this
session ended?" with the bare structural predicate reports completion the
moment a compound state finishes, while the machine is still live. The trap is
the naming: Appendix D's separate `isInFinalState(s)` is a third thing again,
asking whether a compound or parallel state has completed for the done.state
computation.

`parallel_regions_take_own_transitions` covers W3C SCXML 3.4: when one event
enables a transition in more than one region of a `<parallel>`, every such
region takes its own in the same microstep. The fixture is asymmetric on
purpose, because a symmetric one passes under a wrong exit set as readily as
under a right one — one region's transition is an external self-transition,
whose domain Appendix D resolves through `findLCCA` over `getProperAncestors`,
candidates that never include the state itself. Answering with the state left
`computeExitSet`'s climb without a stopping point: it ran to the document root,
the exit set named the enclosing `<parallel>`, and conflict resolution preempted
the other region. The verdict is a top-level `<final id="settled">` guarded on
both regions' assignments having run, so a region that moved without executing
its transition content still fails — and a top-level final is the one
observable every backend exposes, including the ones that report a single
current leaf rather than a configuration.

`parallel_self_transition_keeps_its_leaf` owns the axis immediately after that
one: a region that took an external self-transition still holds an atomic
state, so it answers the NEXT event. The two are separate fixtures because the
sibling's question can be answered correctly by a configuration that is already
broken — a region can take its transition and run its content exactly as
required and still be left holding no leaf, which is present by every ancestor
test while nothing inside it can ever fire again. Measured 2026-08-14 with the
defect `sce-build/tests/mutations/parallel_microstep_owns_exit_and_entry.cases`
restores: the sibling's whole C++ AOT driver stays green (SURVIVED, 0/2 red)
and this one's goes red (CAUGHT, 1/2). What differs is which region the
engine's single current-state pointer is resting in when the event arrives —
re-exiting a leaf the microstep has already moved out of is harmless, and
re-exiting one it just re-entered is the loss — so the self-transitioning
region is first in document order here and second in the sibling. The deeper
region answers `e` once and then goes quiet, which leaves the second `e`
addressed to the self-transitioning region alone, and `settled` is guarded on
`n == 1 && m == 2`.

`ancestor_entry_is_not_default_entry` covers W3C SCXML 3.3 + Appendix D: a
compound state entered only because the transition's target lies inside it does
not take its default initial child. Appendix D asks that as two functions —
`addDescendantStatesToEnter` for the target, `addAncestorStatesToEnter` for
everything between the target and the LCCA — and an engine with one entry
function answers both with the first, leaving two children of one compound
state in the configuration.

Two things had to be true at once for that to survive the W3C IRP suite, and
the fixture is shaped by both. The first is that the block computing the
default child is emitted inside `{% if model.has_parallel_states %}` in every
backend's entry-action template, so a machine with no `<parallel>` never grows
the code that has the defect — measured 2026-08-15, this same document with
`drive` promoted to the root passes on the Rust channel while the defect is
still in the templates, which is why the second region `idle` is load-bearing
and answers nothing. The second is that the extra child is a *sibling* of the
target rather than an ancestor of it, so every structural check still holds and
the engine's current-leaf pointer is right; a document only notices when the
spurious state DOES something. The verdict is therefore a counter written by
that state's `<onentry>`, and `settled` is guarded on
`targeted == 1 && defaulted == 0` so a run that never reached the target fails
the same way one that entered both does.

It was found by running `examples/ai_loop/ai_loop.scxml`, not by reading: there
the wrongly-entered state's `<onentry>` sends the opening prompt, so the
supervised session was re-introduced to itself every time a person answered a
dialog — on both AOT engines, with every W3C fixture green.

`event_data_arrives_as_sent` covers W3C SCXML 5.10 + B.2: a payload a HOST
injects reaches the datamodel as a value. Every other fixture here drives its
machine with an empty payload — measured 2026-08-16, the data argument was
`""` in every call on every channel — so the boundary an embedder calls was
covered by nothing, while the payload paths the W3C IRP suite does exercise
(`<send><content>`, `<param>`, `<donedata>`) all originate INSIDE the document
and are lowered on separate code.

The document asks five claims and lands each failure in a `<final>` of its own:
`mangled` when a JSON object did not become addressable properties, `garbled`
when text did not arrive as that string, `evaluated` when a payload shaped like
an expression was RUN, `flattened` when a well-formed XML payload did not become
a DOM, `swallowed` when a payload that opens like XML but is not a document did
not reach the document at all, and `settled` when all five held.

The last two arrived on 2026-08-19 and are one question asked twice.
§scxml-B-2-8-1 conditions the DOM reading on the content BEING a document and
then closes with a MUST of its own — "Otherwise, the Processor MUST treat the
content as a space-normalized string literal" — so a leading `<` is a guess
about which reading applies rather than the reading itself. Nine script engines
gave four answers to that guess going wrong: Rust and Go answered nil, the two
C++ engines built a DOM out of whatever fragment happened to parse
(`XMLDocument::isValid` asked whether the tree was non-empty rather than whether
the parse succeeded), and C11 had no XML rung on the event path at all, so it
answered a string where its six siblings answered a document. Nothing sent such
a payload until the round before, which filled `_event.data` in at 192 `error.*`
raise sites with messages that name the failing construct — every platform error
opens with `<` and is not a document.

The XML payload carries leading whitespace on purpose: the reading is chosen by
the first non-blank character, and the scan past it is small enough to look
redundant. It is what a mutation to the C11 rung's scan is caught by.

The first guard tests `_event.data` before indexing it, which is not defensive style: a
backend that drops the payload leaves the variable nil, and the run would then
report an evaluation error rather than the missing payload.

Three defects, all silent, all found by giving the fixture a channel. C11 built
the assignment as Lua SOURCE (`_event.data = (<raw>)`) and discarded the load
result, so a JSON object parsed as a Lua array, a JSON object with a string
value did not parse at all and left the PREVIOUS event's data in place, and a
payload was executable code. C++ AOT emitted `currentEventMetadata_` only for
machines hosting an `<invoke>`, so the base engine's public `processEvent(Event,
const EventMetadata &)` did not compile for any other machine; and once it did,
that overload stored the metadata without populating the policy's pending
fields, which is what `_event` is bound from. The `<content>` path was green
throughout on all three.

`parallel_completion_raises_done_state` covers W3C SCXML 3.4 + 3.7: a
`<parallel>` is done once every region has reached a `<final>`, and that raises
`done.state.<parallel>`. A parallel owns no `<final>` of its own — the regions
do — so a rule that registers the event by walking from a final to its direct
parent never reaches it, while the C++ and C11 emitters raise it from the
grandparent regardless. The result was generated code naming an enumerator the
model never declared, which `check` cannot see because acceptance is decided
before anything is compiled. Nothing in the fixture listens for
`done.state.run`: an event named by a transition is collected from that
attribute, which would register it no matter what the `<final>` walk does and
leave the fixture unable to fail.

`parallel_done_state_is_delivered` covers the other half of the same clause,
and the split is forced rather than chosen. Its sibling above cannot listen for
`done.state.run`: an event named by a transition is collected from that `event`
attribute, so a listener would register the enumerator no matter what the
`<final>` walk does and leave that fixture unable to fail. What it proves is
therefore that the event is *declared*, which the build proves by compiling.

Declared is not delivered. A backend that names the enumerator and never raises
it, or raises it where nothing selects from, compiles clean and passes there.
Measured 2026-08-13: all six code-generating backends do raise it, and no
channel asserted so — every driver on the pair checked that the regions reached
`a2`/`b2`, which is the precondition and not the event. So this fixture
listens, and its verdict is the top-level `<final>` `settled`, which the
completion event alone can reach and which the Kotlin channel can observe
through the single current leaf it exposes.

One shape worth recording, because it cost a round of red drivers: completion
is selected in the SAME macrostep as the regions' finals, so once the step
returns the parallel has been exited and `a2`/`b2` are gone. Asserting them as
a precondition fails against an engine that has done exactly the right thing.
Each driver therefore makes one assertion and puts the configuration in its
failure message, where `a1`/`b1` and `a2`/`b2` name the two different defects.

`event_origin_is_a_location` covers W3C SCXML Appendix C.1: the origin of a
delivered event is the `location` the sending session published for the SCXML
Event I/O Processor, and that location is a usable `<send>` target. The public
IRP suite cannot separate the two halves — test336 and test350 both check
`_event.origin` by sending to it with the sender and the receiver being the
same session, so any value at all round-trips. This fixture puts a peer session
on the other end, which is the only arrangement where a bare session id and a
published location differ. A mismatch lands in `fail`; a routing violation
leaves the parent parked in `await_reply` and the harness times out, which is a
weaker signal on purpose, because a target that resolves nowhere produces no
event to transition on.

The fixture is single-axis to the point of comparing the two strings for
equality rather than testing a prefix: the guard is evaluated by whichever
engine the backend embeds, and a failed evaluation raises `error.execution` and
reads as a false condition — so a probe written with a method one engine lacks
reports a violation that is not there.

`session_ids_are_distinct` covers W3C SCXML 5.10: `_sessionid` is bound to the
system-generated id for the *current session*, so two sessions are two ids.
Appendix C.1.1 derives the address a session publishes from that id, which is
what makes a shared id a routing defect rather than a cosmetic one — two live
sessions reading one id publish one address, and a `<send>` addressed to
either reaches both. The public IRP suite cannot ask: every test that reaches
`_sessionid` runs a single session, so a processor handing the same value to
every session it starts passes all of them, which is what the C11 backend did
until this fixture was added.

Its reach is stated in the fixture rather than implied. The two children are
separate inline documents, so a processor deriving an id from the document
still tells them apart — this pins the property across the channels and does
not by itself reproduce the C11 defect. The same-document case, two sessions
of one machine told apart only by the ordinal the processor issues, is pinned
by `sce-build/tests/c11_session_identity.rs`, which instantiates one generated
machine twice; expressing it as a fixture would need a second canonical
document per stem, which the single-document regen contract above does not
carry.

Every channel asserts it, and the reason is the history: the C++ pair landed
first and its own comment claimed the other five answered the same question.
They had no driver at all. All five were violating the clause, in five
different ways — two names for one child session (Rust, Go), no conversion at
the `_event` boundary (all five), a `#_<invokeid>` origin that is §6.4's
addressing form rather than C.1's published location (C11), and two buffers
sized for an id rather than an address (C11), where a truncated address is
indistinguishable from a spec violation.

`invoke_unsupported_type` covers W3C §6.4.1: an `<invoke>` whose `type` names
no processor the platform implements is valid SCXML that must raise
`error.execution`, not a document to reject. Both engines were silent here in
different ways — the Interpreter substituted its SCXML handler for the unknown
type and started a child session the author never asked for, while AOT dropped
the `<invoke>` from the model outright and produced no observable at all. The
fixture is single-axis to the point of carrying no `src` and no `<content>`:
§6.4.1 classifies on `type` alone, before any child document would be
resolved, so a fixture that supplied one would let a child-materialization
regression masquerade as an unsupported-type regression. It is also why this
stem is the only one whose CMake registration passes no
`SYNTH_INVOKE_CHILDREN` — there is no child to synthesize.

Every channel asserts it, because wiring one backend does not close the
contract for the rest: the `Invoke::Unsupported` model variant is skipped by
each template's `scxml`-family filter until that backend is wired explicitly,
which moves the silent drop from the parser into the templates rather than
removing it. Measured directly during the work that added this fixture —
after the C++ lowering landed, five backends still emitted zero raise sites
while reporting successful generation. C11 needed three separate gates opened
(the entry-action switch, the `scxml_family` include guards, and an
`execute_pending_invokes` arm past the `| scxml` filter's index space), and
Rust passed a generated-source assertion while still resting in `probe` at
runtime, so the emit-site check and the runtime channel are not
substitutes for one another.

`xml_data_is_a_dom_tree` covers W3C §B.2: a `<data>` element's XML content is
"the corresponding DOM structure" the appendix obliges the Processor to create,
and a document walks it with DOM Level 1 Core's vocabulary. Every backend
carried three methods instead — `getElementsByTagName`, `getAttribute` and a
non-standard `getTagName`, which are the two names the W3C IRP suite reads plus
one — so `doc.tagName`, `doc.firstChild` and `doc.childNodes.length` answered
nil on all seven channels with 204/204 W3C fixtures green.

What this fixture owns that the per-binding tests do not is the SEAM. Each
binding is measured directly against `tests/ecmascript/dom_read_surface.json`,
which every channel's unit test reads; a binding being right does not say a
document reaches it, because the path from document to binding is the generated
`<data>` initializer plus the guards the frontend lowered. For the C11 channel
the fixture is the ONLY witness there can be: `sce_lua_dom_push_object` and its
metatable have no caller but generated code.

It found one immediately. The C11 Lua binding's element metatable still pointed
`__index` at itself while the document metatable had been moved to the property
dispatcher, so a document handle answered every member and a node reached by
`documentElement` answered none — invisible to the unit tests, which measure the
tree layer, and to the W3C suite, whose two methods live on the metatable
either way.

Every transition in it is eventless, so the verdict is reached in the first
macrostep. That is deliberate: an event would pull in the `_event.data` payload
path that `event_data_arrives_as_sent` owns, and a failure there would surface
here as a DOM failure.

`late_tick_honours_cancel` covers W3C SCXML 6.2 + 6.3: `<cancel>` drops a
delayed `<send>` that has not been dispatched, and "not dispatched yet" is a
fact about the engine's delivery order rather than about the host's clock.
Every pull-driven backend keeps its scheduled sends in a queue ordered by fire
time and drains that queue inside `tick` (Python: `advance_time`); draining it
to exhaustion before running a macrostep puts everything past due on the
external queue together, and the `<cancel>` the first entry's transitions
execute then finds nothing left to drop. The fixture is the settle-timer shape
a supervising host actually writes — arm a long timer, cancel it when the short
signal arrives first — and every driver deliberately waits past BOTH fire times
before its first tick, because a host that wakes between them passes under
either dispatch order. That is why no existing suite saw it: measured
2026-08-19 the document reached `cancelLost` on Rust, Go, Python and C11 alike,
the Python one on a virtual clock where the host's step size alone decided it.

It owns only the dispatch order. `donedata_late_completion` owns delayed
completion detection, and this document reads no data model, hosts no
`<invoke>` and has no `<parallel>`, so a regression in any of those cannot
surface here. The verdict `finish` is itself scheduler-driven, so a channel
whose tick loop stopped working entirely fails rather than passing by never
leaving `waiting`. The Interpreter channel is asserted alongside the six
pull-driven ones for a different reason than usual: `EventSchedulerImpl` owns a
thread and fires each entry at its own deadline, so it cannot coalesce them —
it pins the verdict the document is supposed to reach, and a pull-driven
backend that disagrees is diverging from an engine in the same repository
rather than from a rule written only in a test.

`discarded_event_is_observable` covers W3C SCXML 3.1.2: "If no transition
matches in any state, the event is discarded." Discarding is the clause;
being unable to say that it happened is what the fixture is about. Three
outcomes leave the configuration byte-identical — a self transition (`poke`), a
targetless internal transition (`nudge`, which exits and enters nothing at all)
and a discard (`settle`, declared in `busy` and therefore nameable by the host
but unmatched in `idle`) — so a host that feeds a machine external events, which
is every host that supervises one, cannot tell them apart through any accessor
that existed before. Each driver asserts that indistinguishability directly, so
the fixture fails if it ever stops measuring what it claims.

The Interpreter channel is not a mirror here, it is the reason the axis exists:
`StateMachine::processEvent` has always returned a `TransitionResult` whose
`success` is false for the third case, and `getStatistics().failedTransitions`
has always counted them, while the six generated engines computed the same fact
at the same point of Appendix D's `mainEventLoop` and dropped it — so a document
that grew up on the Interpreter and shipped as AOT lost a signal its host was
reading. `nudge` is in the document because the generated engines' own
"did anything happen" bool means "the configuration changed": a count keyed off
it would report a handled event as discarded. Kotlin is asserted through its
sync entry point, and the coroutine mode's channel path records the same fact,
because that engine has two external-event entry points for one queue.

It owns only what the host learns about an event it injected: no delayed
`<send>`, no `<invoke>`, no `<parallel>`. `late_tick_honours_cancel` owns
scheduler dispatch order.

`unhandled_error_is_observable` covers W3C SCXML 3.12.2: the processor MUST
signal its own failures by raising `error.*` events into the **internal** queue,
and the same paragraph says they "are ignored if no transition is found that
matches them". Being ignored is the clause; being unable to say it happened is
what this fixture is about. It is the sibling of the entry above and exists
because that one drew its boundary at the external queue, on the stated ground
that an unmatched internal event is the document's own business with both ends
inside the document. That reasoning is exactly right for an author's `<raise>`
and exactly wrong for an error event, whose sender is the **engine**: the host
never wrote the document, cannot see the failure anywhere in the configuration,
and is the only party positioned to act on it.

Four outcomes leave the configuration on the same state — `poke` (handled, no
error), `whisper` (the author's own `<raise event="unheard"/>`, unmatched and
deliberately **not** counted), `boom` in `idle` (error, unmatched, counted) and
`boom` in `guarded` (the same failure, answered by the document, not counted).
`boom` is one event name routed to two outcomes by state, so a count cannot be
keyed off the event or the action, only off what the configuration did with the
error. The failure itself is `<assign location="">` — W3C 5.3's invalid location,
which every backend rejects at generation time rather than through its script
engine, so the error is raised identically in all seven channels.

C11 answers the membership question differently on purpose: it has no
event-name table to consult at run time, so its generated code compares against
the `error.*` enum members the document declares, while the other five ask the
same question of a name they already carry. The Interpreter channel asserts the
parity claim from the other side — its raise sites pass the failure text and
run whether or not the document declares a handler.

It owns only what a host learns about an error its machine raised and dropped:
no delayed `<send>`, no `<invoke>`, no `<parallel>`.

`error_cascade_is_bounded` covers the other side of the same clause. W3C SCXML
3.12.2 bounds what happens to an error event nothing matches; it says nothing
about one that **is** matched, by a handler that fails the same way every time.
The failure raises `error.execution`, the same transition answers it, the
handler fails again, and the drain never empties. Measured 2026-08-19, that is
not a hang: the Python engine turned 37,000 links a second on a two-line
document while its configuration never moved and the machine reported itself
running, and the C++ Interpreter did not return from `processEvent` at all
(its executable content dispatches into the raiser again, so each link was a
stack frame). An unattended supervisor reads a healthy idle machine and a
pinned core.

Four outcomes again, and the third is the one that makes the count mean
something — `poke` (handled, no error), `boom` (one error, unmatched: the
sibling entry's own case, and the cascade count must **not** move for it),
`settle` (a chain that ends by itself after three links, when its
`repairs < 3` guard stops matching) and `spin` (a chain that cannot end). A
ceiling that could not tell `settle` from `spin` would report every document
that fails often as one that cannot stop failing, so the chain is measured
handler-to-handler rather than failure-to-failure: any other internal event
resets it, and a second entry into `runaway` buys a full chain again.

Both error states answer `poke` with a **targetless** transition on purpose —
a self transition would re-run `<onentry>`, start a fresh chain, and leave the
driver measuring its own probe instead of the engine.

It owns only the error a handler answers with the same failure forever: no
delayed `<send>`, no `<invoke>`, no `<parallel>`.

`eventless_macrostep_is_bounded` asks the same question of a chain built from
transitions that need no event at all. W3C SCXML 3.13 defines a macrostep as a
chain of microsteps ending in a configuration where nothing is enabled by NULL,
and Appendix D's Principles and Constraints then say that end need not exist:
*"A microstep always terminates. A macrostep may not. ... This is currently
allowed."* A document with a cyclic eventless transition is therefore
conformant, and an engine that runs it to the letter never returns — so a
ceiling is the engine declining a document the specification permits, which is
why the decline is published rather than logged.

Measured 2026-08-20, the seven engines answered it three ways: the Python
engine had no ceiling and did not return from `initialize()`; the C++ AOT
engine called `stop()`, so the same document came back dead there and merely
paused elsewhere; the other five stopped the chain and said nothing a program
could read. `truncated_macrosteps` and `last_truncated_macrostep_state` are the
seven-channel surface that separates a machine resting in a stable
configuration from one this engine stopped walking.

Four outcomes, and the second is what makes the count mean something — `poke`
(one ordinary transition), `bounded` (a chain that stops by itself after
exactly the ceiling's length, which is where an off-by-one lands: two engines
reported it as a runaway), `spin` (a chain that cannot stop) and `reset` (the
way back out, so a driver can run the chain twice). `reset` is a
state-changing transition on purpose: the two C++ engines complete a macrostep
only after a transition that moves the machine, so a targetless one would leave
the eventless chain unvisited on those two channels alone.

It owns only the macrostep that cannot end: no `<send>`, no `<invoke>`, no
`<parallel>`, and no failing expression.

`internal_chain_is_bounded` owns the other branch of that same inner loop. W3C
SCXML 3.13 ends a macrostep where nothing is enabled by NULL **and the internal
queue is empty**, so a `<raise>` answered by a transition that raises again
never reaches either condition — it is the same allowance Appendix D grants the
eventless cycle, reached by different code in every engine here. Measured
2026-08-20, six of the seven did not return from it at all (the eventless
ceiling budgeted the branch that was not running) and the Kotlin engine stopped
at a hundred internal iterations and said nothing.

Five outcomes: `poke` (the control), `bounded` (a chain exactly the ceiling's
length that ends on its own — the count must stay zero), `spin` (a chain that
cannot end), `resume` (a chain half again as long as the ceiling, which the
first macrostep refuses and the second finishes — the only outcome that can
tell a refusal that *left* the queue from one that swallowed it) and
`alternate` (one `<raise>` and one eventless transition, turn and turn about).

`alternate` is why the budget is one number. Neither branch reaches the ceiling
on its own there, so an engine that budgets them separately runs the document
forever with both counters half spent — which is what the Kotlin engine
shipped. `alts == 500`, half of the shared thousand, is the arithmetic that
holds it.

Every chain in this fixture is built from targetless transitions inside one
state, and each outcome has its own machine. That is not economy: the state a
truncation names has to be one all seven channels can agree on, and the C11
profile keeps its configuration in a bitmap with no current-state scalar to
report, so it names the source of the transition the drain last took. On a
chain that never leaves one state those readings are the same state.

**The ceiling moved from a hundred to a thousand when this fixture landed**, in
both bounded-chain fixtures and all seven engines. The general ceiling is a
backstop and it was sitting on top of a sharper one: `error_cascade_is_bounded`
allows a hundred links and a handler that logs before it fails costs two
microsteps a link, so an equal number here cut that document at fifty links and
its `error_cascade_events` never moved. A general ceiling has to be loose
enough for the specific ones to stay reachable.

It owns only the macrostep an internal chain cannot end: no `<send>`, no
`<invoke>`, no `<parallel>`, and no failing expression.

`targetless_transition_completes_macrostep` owns the other half of that
sentence — whether the chain is entered at all. W3C SCXML Appendix D's main
event loop returns to `selectEventlessTransitions()` after every microstep and
drains the internal queue in the same inner loop, without asking whether the
microstep moved the machine; W3C SCXML 3.13 defines a transition with no
`target` as one that exits and enters nothing and runs its content in place, so
that content can enable an eventless transition or raise an internal event and
both belong to the same macrostep.

Measured 2026-08-20, three of the seven engines ended the macrostep there, in
three different shapes: the C++ Interpreter never entered the chain, the C++ AOT
engine never entered it and also stranded the raise, and the Kotlin engine
entered the chain but stopped at its first targetless link — which its code
generator had dropped from `processNullEvent` while still emitting the
transition's actions.

Three outcomes, and the first is what makes the other two mean anything —
`quiet` (a targetless transition that enables nothing: the control that
separates a stopped macrostep from a lost event), `arm` (a targetless
transition that enables an eventless chain whose LAST link is targetless too,
so `chained == 1, polished == 0` names the engine that walks a chain only while
the machine keeps moving) and `ping` (a targetless transition whose `<raise>`
must be answered before the host gets control back).

It owns only whether a targetless transition ends the macrostep: no `<send>`,
no `<invoke>`, no `<parallel>`, no delay, and nothing that can fail.

The full uniformity roadmap (per-backend layout migration, AOT/Interpreter
two-channel parity, SSoT canonical fixture path) lives in
`claudedocs/rfc-donedata-5-backend-layout.md`. This document records the
**current state** at HEAD; rows update as each phase of that RFC lands.

## Why this layer exists

W3C SCXML §5.5 (`<donedata>`) and §6.3.1 (`done.invoke.<id>` event
emission) interact when an inline child machine reaches a top-level
`<final>` carrying `<donedata>` and the parent reads `_event.data` on
the `done.invoke.<id>` event. No public W3C IRP fixture exercises this
combination directly — the IRP `<donedata>` tests cover machine-level
done emission, not the child-invoke-to-parent round trip. A repository
grep `for f in resources/*/test*.txml; do donedata && invoke; done`
confirms zero W3C IRP fixtures combine both.

The mesh suite covers the same contract for the AOT wire-18 path via
`test_mesh_session_f_donedata`. The integration layer documented here
covers the parallel *local-invoke* path.

## Two architectural axes

The 6 backends differ along two orthogonal axes that the previous
revision of this document conflated under a single "Interpreter
first-class vs AOT-only" framing.

### Axis 1 — Committed generated tree vs build-time generation

| Backend | Committed generated tree? | Source |
|---|---|---|
| Rust | yes (W3C `src/generated/` + integration `src/integration/`) | hand-committed; regen via `sce-codegen generate-w3c -l rust` + per-fixture scripts |
| Kotlin | yes (W3C `…/com/sce/generated/` + integration intermixed under same) | hand-committed; regen via `sce-codegen generate-w3c -l kotlin` + per-fixture scripts |
| Go | hybrid — W3C `backends/go/tests/generated/` is `.gitignore`d (CI regen), donedata `backends/go/tests/donedata_local_invoke/` is committed | mixed |
| C++ | no — `${CMAKE_CURRENT_BINARY_DIR}/w3c_static_generated/` | CMake build-time |
| Python | no — `backends/python/tests/generated/` is `.gitignore`d | CI regen via `sce-codegen generate-w3c -l python` |
| C11 | no — `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/generated/` | CMake build-time |

Committed-tree backends are §6.2.6 drift-gated (per-context source-hash
+ template-hash invariant via `b9_drift_detection::verify_passes_on_real_committed_*`);
build-time backends rely on CMake to regenerate on every build, so the
build process itself is the §6.2.6 freshness invariant.

That invariant holds only while every codegen step declares the
templates as an input. CMake learns the SCXML dependency from `DEPENDS`
and the ~120 template dependencies only from a `DEPFILE` written by
`sce-codegen --write-deps`. Steps missing it were measured to reuse
stale artefacts after a template edit — 0 of 21 C++ integration outputs
regenerated, 74 of 270 C11 — while the build reported success. All ten
steps now carry it, and `sce-build/tests/codegen_depfile_coverage.rs`
holds them there; each site was individually mutated to confirm the gate
catches its removal.

### Axis 2 — Engine path (Interpreter vs AOT)

| Backend | Interpreter channel | AOT channel |
|---|---|---|
| Rust | n/a (AOT-only backend) | committed integration tree |
| Kotlin | n/a (AOT-only backend) | committed integration tree |
| Go | n/a (AOT-only backend) | committed integration tree |
| C++ | `tests/integration/DonedataLocalInvokeTest.cpp` (gtest against `runtime/StateMachine.h`) | `tests/integration/DonedataLocalInvokeAotTest.cpp` against build-time `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/donedata_local_invoke_sm.{h,inl}` (CMake `sce_generate_static_integration_test`) |
| Python | `backends/python/bindings/tests/test_donedata_local_invoke.py` (pybind11 wrapping `ReadySCXMLEngine` over C++ Interpreter, commit `0589bb35`) | `backends/python/tests/integration/donedata_local_invoke/test_donedata_local_invoke_aot.py` against gitignored `*_sm.py` (regen via `scripts/regen_donedata_local_invoke_python.sh` / `sce-codegen generate-integration -l python`) |
| C11 | n/a (AOT-only backend) | `backends/c/tests/integration/test_donedata_local_invoke.c` against build-time `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/integration_generated/donedata_local_invoke_sm.{h,c}` (CMake `sce_generate_static_integration_c_test`) |

C++ and Python are the only backends with both engine paths in
production: Interpreter (embedded usage — consumer loads SCXML at
runtime) and AOT (codegen-compiled consumers). The layout RFC adds the
AOT channel without removing the Interpreter channel — both are
production code paths whose execution traces differ, so both are
verified independently.

The other 4 backends (Rust / Kotlin / Go / C11) are AOT-only — they
have no Interpreter and the AOT channel is the canonical contract test.

## Per-backend coverage at HEAD

| Backend | Coverage form | Location | Drift-verify CI gate |
|---|---|---|---|
| Rust | AOT-generated tree | `backends/rust/tests/src/integration/donedata_local_invoke/` | yes |
| Kotlin | AOT-generated tree | `backends/kotlin/tests/src/main/kotlin/com/sce/generated/donedata_local_invoke/` | yes |
| Go | AOT-generated tree | `backends/go/tests/donedata_local_invoke/` | yes |
| C++ | Interpreter gtest + AOT build-time | Interpreter: `tests/integration/DonedataLocalInvokeTest.cpp`; AOT: `tests/integration/DonedataLocalInvokeAotTest.cpp` against `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/` | n/a (build-time generation, freshness invariant = CMake build) |
| Python | Interpreter via pybind11 + Python AOT | pybind11: `backends/python/bindings/tests/test_donedata_local_invoke.py`; AOT: `backends/python/tests/integration/donedata_local_invoke/test_donedata_local_invoke_aot.py` against gitignored `*_sm.py` | n/a (`*_sm.py` gitignored, regenerated by CI before pytest; mirrors W3C IRP Python pattern) |
| C11 | codegen donedata literal-shape + AOT integration fixture | Literal-shape: `tools/codegen/templates/c/state_machine.c.jinja2` (`6eec3a95`), verified by W3C IRP donedata tests 294/527/528/529/176/179/186/578/298. Cross-SM `done.invoke.<id>._event.data` lift: `tools/codegen/templates/c/invoke_methods.jinja2` (Phase E) + `tools/codegen/templates/c/scriptengine.jinja2` (`_sce_donedata_to_lua_literal`). Integration fixture: `backends/c/tests/integration/test_donedata_local_invoke.c` against build-time `${CMAKE_CURRENT_BINARY_DIR}/backends/c/tests/integration_generated/` | n/a (build-time generation, freshness invariant = CMake build) |

The 3 AOT-generated trees are regenerated by per-backend scripts
(`scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`) and guarded by
`.github/workflows/drift-verify.yml` plus the
`scripts/hooks/pre-commit` drift-verify trigger.

### C11 coverage detail

The c11 codegen template lifts `<donedata>` literal shape via lua stash
+ JSON-quoted `_event.data` carry (SSoT mirror of cpp
`DoneDataHelper::emitContentLiteral`, commit `6eec3a95` 2026-04-29).
W3C IRP donedata tests _294/527/528/529/176/179/186/578/298_ are
generated under the c11 backend at CMake build time and verify the
literal-shape contract end-to-end (`backends/c/tests/CMakeLists.txt`).

Phase E (LANDED) closed the `<donedata> + <invoke> +
done.invoke.<id>._event.data` *combination* contract that the W3C IRP
itself does not test (Phase 0 grep confirmed zero W3C IRP fixtures
combine all three). The fix added cross-SM payload carry at every
`done.invoke` raise site in `tools/codegen/templates/c/invoke_methods.jinja2`
(execute_pending_invokes scxml/hybrid + drive_active_children
scxml/hybrid, gated on `invoke_info.child_needs_script_engine`) plus a
generic Lua-source serializer (`_sce_donedata_to_lua_literal` in
`scriptengine.jinja2`'s `lua_init_engine`) that converts the child's
`_pending_donedata` lua global into a Lua-source expression. The
parent's existing `process_event_queues` external dequeue path
(`state_machine.c.jinja2:3493-3498`) rebinds `_event.data = (<literal>)`
on its own `sm->L` so the parent's `done.invoke.<id>` transition cond
(`_event.data.result === 42`, `_event.data === 'hello_content'`)
evaluates against the typed donedata value. Mirrors cpp's
`donedataAtFinal()` carried through `EventMetadataHelper::createDoneInvokeEvent`
— cpp ships JSON because `EventWithMetadata` is engine-agnostic; C11
ships Lua source per its P1 lock-in (Lua 5.4 only) which is the
round-trip-free equivalent producing a typed value rather than forcing
a re-parse.

The C11 integration fixture
(`backends/c/tests/integration/test_donedata_local_invoke.c`) drives both the
`<param name="result" expr="42"/>` (table-shaped donedata) and
`<content expr="'hello_content'"/>` (scalar-shaped donedata) cond
branches through the parent's `done.invoke.{inv_param,inv_content}`
transitions, asserting the run reaches the `pass` final state — a
regression on either the lift macro or the parent's external-dequeue
override trips the test immediately.

### Python coverage detail

The Python channel is `pybind11 → ReadySCXMLEngine → C++ Interpreter`.
Commit `0589bb35` 2026-04-24 added
`backends/python/bindings/tests/test_donedata_local_invoke.py` (109 LOC) and the
fixture (85 LOC) without any template or runtime change — the pybind11
binding wraps the C++ Interpreter's `pendingDonedataAtFinal_` +
`SCXMLInvokeHandler` completion path, so the donedata stash/lift
contract is inherited automatically. The script verifies both `<param>`
(=== 42) and `<content>` (=== `'hello_content'`) branches and was
authored with load-bearing bites (param 42→99 reaches fail; content
`'hello_content'`→`'goodbye_content'` reaches fail; both restored reach
pass), so a future regression in either the C++ Interpreter or the
pybind11 wrapper trips this script.

The Python AOT channel (Phase D of the layout RFC) is separate from
this pybind11 path and verifies the codegen-emitted Python state-table
code independently.

## Adding a new custom integration fixture

When a future SCXML contract requires this layer:

1. Author the source `.scxml` at the canonical fixture root:
   `integration_resources/<stem>/<stem>.scxml` (per-fixture dir,
   mirroring the W3C IRP `resources/<N>/test<N>.txml` convention).
   The top-level `integration_resources/` dir sits outside
   `resources/` because `compute_source_hash` recurses through the
   input root — nesting integration under `resources/` would fold
   the integration fixture into the W3C source-hash domain.
2. Author per-backend regen scripts following the
   `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh` pattern.
   These are now thin wrappers around the canonical CLI surface
   (see step 3); each script encodes the per-language TMP staging,
   `--input-root` override, and post-processing (Rust `mod.rs`
   synthesis, Kotlin `// Source:` rewrite, Kotlin
   `--kotlin-package-prefix com.sce.integration`).
3. Bulk regenerate via the uniform CLI:
   `sce-codegen generate-integration -l <rust|kotlin|go> --stem <stem>`
   (single fixture) or omit `--stem` to walk every
   `integration_resources/<stem>/` dir. The
   `scripts/regen_all_committed_trees.sh` master script bundles W3C
   + integration + forge round-trip so a template touch lands one
   coherent commit across every drift context.
4. Register the new sub-module in each backend's integration entry
   point: Rust `backends/rust/tests/src/integration/mod.rs`, Kotlin
   `backends/kotlin/tests/src/main/kotlin/com/sce/integration/package-info.kt`,
   Go `backends/go/tests/integration/doc.go`.
5. Wire the §6.2.6 drift-verify CI gate to each backend's new
   generated directory in `.github/workflows/drift-verify.yml` and
   the `scripts/hooks/pre-commit` drift-verify trigger.
6. For cpp / C11 / Python pybind11 channels (no committed tree),
   wire the fixture into the per-backend build/CI entry point
   (`tests/CMakeLists.txt` for cpp; Python CI workflow;
   `backends/c/tests/CMakeLists.txt` for C11).

## RFC reference

The full long-term-correct end state is defined in
`claudedocs/rfc-donedata-5-backend-layout.md` (locked 2026-05-22, 9
Q-locks decided). Key end-state guarantees once all phases land:

- Single canonical fixture source `integration_resources/<stem>/<stem>.scxml`
  for all 6 backends (Q-8 + Q-8a per-fixture dir, separate top-level
  from W3C `resources/` to keep drift contexts disjoint).
- Committed-tree backends (Rust / Kotlin / Go) share canonical
  `integration/` layout sibling to W3C `generated/` (Q-1).
- Per-language anchor file convention: Rust `mod.rs` /
  Kotlin `package-info.kt` / Go `doc.go` (Q-1a).
- Build-time backends (C++ / C11) share canonical
  `sce_generate_static_integration_test` CMake function (Q-2).
- C++ / Python retain both Interpreter and AOT channels (Q-3, Q-4).
- `sce-codegen generate-integration -l <lang> [--stem <stem>]`
  subcommand parallel to `generate-w3c` (Q-6, LANDED).
- `scripts/regen_all_committed_trees.sh` master regen script
  bundling W3C + integration + forge round-trip (Q-7, LANDED).
- Every backend has ≥1 channel for the `donedata_local_invoke`
  contract — "uncovered" eliminated.
