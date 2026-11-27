# ADR 0148 — A machine whose document leaves Appendix D's main loop one step per event takes a reactive turn

- Status: Accepted (2026-11-27), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement (section "Measured").
- Amends: nothing decided. ADR 0135 (one `Turn` body for every machine) stands for every machine outside the class
  below; a machine of the class runs a second shared body.
- Related: ADR 0076 (the internal queue's proven bound), ADR 0125 and 0135 (the row walk and its class), ADR 0129
  decision 4 (a differential test binds two forms of a machine).
- Requested by: a downstream bare-metal consumer, 2026-11-27.

## Context

`Turn::run` is Appendix D's main event loop for any document: it completes each macrostep on eventless transitions
and internal events (`complete_macrostep`, `take_eventless_transitions`, the internal queue's dequeue and its
budget), starts the invocations the macrostep entered, applies what other threads posted, and takes one external
event. The consumer's link, fragment-slot and opening-session machines have no eventless transition, raise nothing
for themselves, invoke nothing and have no nested `<final>`: for them every step of the loop but the last has nothing
to do, and the image still carries `Turn::run` (1,296 bytes) and the procedures it calls.

## Decision

A machine is of the **reactive class** when its document and its policy prove that the main loop's other steps
have nothing to do:

- its tables are the row walk's (no `<history>`, no `<parallel>`, no transition of more than one target), no state
  has a cell of the null event (no eventless transition), and every `<final>` is a child of the `<scxml>` element, so
  no `done.state` event is raised: `DocumentTables::reactive_class`, a `const fn` of the rows the build keeps;
- its internal queue is proven empty (`INTERNAL_QUEUE_BOUND == Some(0)`), which the generator proves only where no
  executable content can raise an event or a platform error (§scxml-5.10), so no error event is lost;
- it has no invocation of any kind, no `<finalize>`, no autoforward and no child tick.

`StatePolicy::REACTIVE_TURN` is that conjunction, derived and never written, so a policy written by hand (which
proves no internal bound) takes the general turn. The main loop stays one body, `Turn::run(initial, reactive)`, and
`Engine::run_turn` hands it the constant: for a machine of the class the loop skips the steps that complete a
macrostep (the eventless selection and the internal queue) and start invocations, so each pass applies what other
threads posted, takes one external event and the one microstep it selects, and the run exits the interpreter when
it enters a top-level `<final>`. That is the order the general loop keeps for a machine of the class; a build with
debug assertions checks after each pass that the internal queue is still empty. Every caller hands a constant, so a
program whose machines all hand `true` has the general steps folded away by the compiler and links no macrostep
completion, and a program of both kinds links one loop whose two branches are a test each.

## Measured

- The class walker's differential test (`the_class_walker_runs_as_the_general_interpreter_does`) runs every
  generatable document of the corpus: for each one of the reactive class, the reactive loop and Appendix D's loop
  over the same seeded events give the same steps, configurations, journal and end, and the tables' answer is held
  to the document's (no eventless transition, every `<final>` at the top). The two machines of `sce-specs` are held to
  be of the class when their tests compile.
- The consumer's `pico-client-coop-nostd` library (its tree at its main of 2026-11-27, fat LTO, x86-64), whose three
  machines are all of the class: `.text` 222,757 to 222,070 (-687); `Turn::run` 1,296 to 688, with
  `complete_macrostep` and the eventless selection gone from the image.
- An image with machines outside the class (multicast and scouting on, the consumer's shipped peer form): `.text`
  770,335 to 770,479 (+144), of which 52 bytes carry no source line (alignment) and the rest is `Turn::run`'s two
  tests (+64) and the constant each machine's `run_turn` hands over. Its receive path, which tells no machine an
  event, is unchanged within the harness's noise.
- The nostd footprint probe's machines are not of the class; its figures move within the band.

## Alternatives considered

- **A second loop body, `Turn::run_reactive`, chosen by the constant.** Measured: the image of the class -623, but an
  image with machines of both kinds links both loops, +751; with the external step, the initial entry and the end of
  the run shared out of line between them, -578 and +220. The image of both kinds is a shipped form, so the loop
  stays one body. Refused.

- **A cargo feature that selects the turn.** A feature is one for the whole build, and the class is a machine's: an
  image with one general machine would have to give up the reactive turn for every machine. Refused.
- **One loop that tests the policy's constants at each step.** Keeps one body, but every machine of the class
  carries the general loop's code behind its tests. Refused: the point is that the image links none of it.
