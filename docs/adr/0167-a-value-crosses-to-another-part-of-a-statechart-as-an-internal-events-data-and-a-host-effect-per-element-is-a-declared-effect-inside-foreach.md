# ADR 0167 — A value crosses to another part of a statechart as an internal event's data, and a host effect per element is a declared effect inside `<foreach>`

- Status: Accepted (2026-11-29) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review and a review by the
  downstream consumer as its user; amended on acceptance as listed in "Amended on acceptance". Proposed 2026-11-28 at
  that consumer's request: it is moving a protocol stack's hand-written logic into SCE documents, and its census names
  two missing elements under one code, in six rows: a step that reads a value another component computed in the same
  macrostep, and a host effect performed once per element of a collection. It asked for the first as an internal
  event's data, in the statechart's order and never by reading another component's state, and for the second as a
  declared effect inside `<foreach>`. This record finds the first in the language already, holds the message that
  carries it to the recommendation's rule (D1), gives a statechart the declaration that makes "never by reading
  another's state" a fact the build holds (D2), and adds the second (D3, D4).
- Landed: nothing. No step of "Order of work" is built, and no model or test of it has been run.
- Date: 2026-11-28 (proposed); 2026-11-29 (accepted).
- Amends (each amended document gains an "Amended by: ADR 0167" line in the landing that moves it):
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.11, "v1 acceptance contract": the placement rule holds under every data model
    but `sce-static` (D3);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15, "Host actions": a host action inside `<foreach>` and `<if>`, its arguments
    over a loop's names, one call per pass (D3, D4); and a `<parallel>` whose regions share nothing (D2);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15, the rule that a host call's failed argument lets its block go on ("the block
    goes on, as on every backend") and the rule that a `<send>` whose `<param>` fails leaves the pair out and still
    goes ("the message still goes"): a failed argument ends its block (§scxml-4.9), and a message with a failed
    `<param>` is not sent (§scxml-6.2), on every backend (D1, D4);
  - ADR 0076, decision 2: an `<sce:action>`'s failed argument and a `<send>`'s failed `<param>` each count one
    `error.execution` and end their block, so a loop's count falls from its passes times its actions to one (D4);
  - `docs/SCE_ACCEPTED_SUBSET.md`'s appendix: the code D6 adds;
  - ADR 0038: a duty taken inside a loop or a branch is a duty per call, and its release stays on a direct child of
    an `<onentry>` (D4);
  - `schemas/sce-forge-ext.xsd`: `sce:regions` on `<parallel>` (D2);
  - ADR 0115 D7, and ADR 0156 D3, whose list gains the step that has the model read a `sce-static` document: what the
    model generator reads of an event's data, a branch, a loop and a host call (D7);
  - the SCE Protocol-Synthesis RFC, §7 as amended on 2026-11-28: this record is the one it names for a value within a
    macrostep and a host effect per element.
- Scope: `sce-build` (the placement check of `forge/native_action.rs`, the static lowering of a host action inside a
  loop and a branch, the failure rules of a host call and a `<send>`, the check of D2, the model generator), every
  lane that lowers `<sce:action>` or `<send>` under `sce-static`, and the scenarios of
  `sce-build/tests/fixtures/static_datamodel` with every driver that replays them. The Interpreter runs a host action
  wherever executable content stands already (§2.11), so its lowering changes where D1, D2 and D4 change the rules.
- Related: ADR 0038 (a host action's effect and its realization), ADR 0076 (an internal queue sized by what its
  document can raise), ADR 0084 (one inbox, one door, a macrostep per session), ADR 0086 (bounded types; section 16,
  no collection is unbounded), ADR 0108 (a table by key; D5, a map's walk), ADR 0109 D5 to D7 (host operations that
  answer a value), ADR 0111 D5 (an ask answered later), ADR 0115 D7 and D8 (a document's model, and every lane's run
  read against it), ADR 0151 D10 (what leaves a machine is certified from the program's text), ADR 0156 D1, D2, D3, D5
  and D9 (one semantics, the model of every document, theorems for every size).

## Context

What exists, read from the tree at `662e4d060cc`:

- **Within one session, a value already crosses as an event's data.** Under `sce-static`, `<send target="#_internal"
  event="E">` with a `<param>` for each field of `E`'s imported schema puts `E` and its typed data on the session's
  internal queue (§scxml-6.2.4), and a transition of any state takes it in the same macrostep and reads
  `_event.data.<field>`, typed by the schema (`docs/SCE_ACCEPTED_SUBSET.md` §2.15; the fixtures
  `static_composed_send` and `presence_peer` of `sce-build/tests/fixtures/static_datamodel` do it). `<raise>` puts
  an event with no data there (§scxml-4.2). The order is Appendix D's: the internal queue is first in, first out and
  is drained before the next external event is taken (`mainEventLoop`), and the transitions of one microstep run
  their content in document order (§scxml-3.13). The internal queue is sized by what the document can raise, a
  `<foreach>`'s passes counted by its list's bound (ADR 0076, as ADR 0086 amended it).
- **Within one session, a value is also read in place.** A document's data are one datamodel (§scxml-5.3): any
  region of a `<parallel>` reads and writes any variable, and `In()` reads another region's configuration. Nothing
  in SCE says which region a variable belongs to.
- **Between sessions nothing is read.** A session's data are its own. An input from another context enters through
  the door and is applied at a macrostep boundary (ADR 0084), so a value of another session reaches a session only
  as an event, in a later macrostep of the receiver.
- **A host effect per element is refused.** A `<sce:action>` is admitted only as a direct child of a `<transition>`,
  an `<onentry>`, an `<onexit>` or initial content. Nested in `<if>` or `<foreach>` it is
  `validation/native-action-placement` under every data model, because "its call site is conditional or iterated,
  which v1 does not lower" (`check_placement`, `sce-build/src/forge/native_action.rs`). Under `sce-static` an
  argument is any typed expression over the scope (§2.15, "Host actions"), so the reason the rule was first written
  for, an argument that could name only a field of the triggering event's payload, does not hold there. A host
  call's argument that fails is computed first, the host is not called, `error.execution` is raised in the call's
  place, and the block goes on, on every backend (§2.15).
- **A failed `<param>` does not stop its message.** Under `sce-static` a `<send>` whose `<param>` cannot be computed
  leaves that pair out, raises `error.execution`, and the message still goes (§2.15; ADR 0076, decision 2). The
  recommendation discards such a message (§scxml-6.2), so a receiving part would take a typed event missing a field
  that no conforming processor sends. W3C test 553 holds the recommendation's rule, as test 156 (§scxml-4.6) and test
  159 (§scxml-4.9) hold the end of a block at an error.
- **The model generator** (`sce-codegen model`, ADR 0115 D7) reads `<assign>`, `<raise>` and a `<send>` with no
  data to the session's own external queue. It refuses a `sce-static` document whole (`Construct::Datamodel`), the
  data of an event, a `<send>` with a target, `<foreach>`, `<if>` and `<sce:action>`, each by name (`Construct`,
  `sce-build/src/formal/document_model.rs`). ADR 0156 D3 orders its breadth and lists no step that reads a
  `sce-static` document.

The consumer's rows are of three shapes: the fan of one event across several registries, where one registry's
verdict (it still holds a local match; a pending request owns this id) is used by another in the same macrostep; a
drain that performs one host effect per staged element, in the elements' order (every reply of a request before its
final); and a source asked once per registered entry.

So the first element exists inside a session and lacks a rule that keeps a part from reading another part's state;
the second is missing.

## The textbook structure

- **Orthogonal components coordinate by broadcast, and in a statechart by shared data too.** In a statechart an event
  generated in one orthogonal component is sensed by every component, and a component may also read the others' data
  and test their states with `in(S)` (Harel, "Statecharts: a visual formalism for complex systems", Science of
  Computer Programming 8(3), 1987). In STATEMATE's asynchronous time model a reaction to one input is a sequence of
  steps run until the system is stable, and the events a step generates are sensed, as a set, in the next one (Harel
  and Naamad, "The STATEMATE semantics of statecharts", ACM TOSEM 5(4), 1996).
- **SCXML orders what STATEMATE senses as a set.** A macrostep is that reaction. The internal queue holds the events a
  session generates for itself, by `<raise>` and by `<send target="#_internal">`, first in, first out, one taken per
  microstep, before the next external event (§scxml-3.12, §scxml-3.13, §scxml-6.2.4, Appendix D). The transitions of
  one microstep run their content in document order (§scxml-3.13), so a region that reads what a sibling region wrote
  in the same microstep reads a value the document's order fixes. A `<param>`'s value is read when its send runs
  (§scxml-5.7), and a message whose `<param>` fails is discarded (§scxml-6.2).
- **Sharing nothing is a modularity discipline here, not a condition of determinism.** Esterel refuses a variable
  written in one branch of a parallel and used in another because the branches of one instant have no order (Berry,
  *The Esterel v5 Language Primer*, 2000); SCXML gives them one, so reading across regions is deterministic. What
  sharing nothing buys a SCXML document is information hiding (Parnas, "On the criteria to be used in decomposing
  systems into modules", CACM 15(12), 1972): a region's data are read by that region alone, a value reaches another
  part only as a message's content, as between CSP's processes (Hoare, "Communicating sequential processes", CACM
  21(8), 1978) and between actors (Agha, *Actors: A Model of Concurrent Computation in Distributed Systems*, MIT Press,
  1986), and a microstep's effect on the data does not depend on the order of its regions; only the events raised and
  the host calls made keep the document's order.
- **A machine's output is a sequence of actions per reaction** (Mealy, "A method for synthesizing sequential
  circuits", Bell System Technical Journal 34(5), 1955; the actions of a statechart's transitions, Harel 1987). An
  iteration over a finite sequence that performs an action per element is a structural recursion over that sequence,
  so its output is a finite sequence in the sequence's order.
- **A custom action element stands where executable content stands** (§scxml-4.10, §scxml-G-7). `<foreach>` walks a
  shallow copy of its array, in order, taken when the loop begins, and an error of any child ends the loop and the
  block that holds it (§scxml-4.6, §scxml-4.9).

## Decision

### D1. A value of another part of a statechart crosses as an internal event's data

This is the form the language has. It is stated here as the one way a part of a statechart hands another a value in
the same macrostep:

```xml
<parallel id="session" sce:regions="share-nothing">
  <state id="registry">
    <transition event="face.closed">
      <sce:call expr="DropFace(faces, _event.data.face)"/>
      <send target="#_internal" event="matching.changed">
        <param name="key" expr="_event.data.key"/>
        <param name="matched" expr="HasLocalMatch(faces, _event.data.key)"/>
      </send>
    </transition>
  </state>
  <state id="fan">
    <transition event="matching.changed">
      <sce:action name="tell_matching" effect="duty">
        <sce:arg expr="_event.data.key"/>
        <sce:arg expr="_event.data.matched"/>
      </sce:action>
    </transition>
  </state>
</parallel>
```

- The sender puts `E` on the session's internal queue with `<send target="#_internal" event="E">`, a `<param>` for
  each field of `E`'s imported schema (§scxml-6.2.4). A transition of any region takes it in a later microstep of the
  same macrostep, in the queue's order, and reads `_event.data.<field>` typed by the schema.
- The value is the one the sender held when its send ran (§scxml-5.7); a later write of the sender's variable changes
  nothing the receiver reads.
- **A message is whole or not sent.** Under `sce-static` a `<send>` whose `<param>` cannot be computed is not sent:
  `error.execution` is raised and its block ends (§scxml-6.2, §scxml-4.9), on every backend and to every target. The
  rule that left the pair out and sent the rest is withdrawn (Amends), so a receiver never takes a typed event that
  lacks a field.
- **What a region does not take is lost to it.** The event is sensed by every region, and a region that has no
  enabled transition for it in its active states does not take it; within a region a descendant's enabled transition
  pre-empts its ancestors' (`selectTransitions` and `removeConflictingTransitions`, Appendix D). A region that must
  see every value holds a transition for the event in each of its states, or at its root with no descendant taking it.
- `<raise>` stays the recommendation's: it names an event and carries no data (§scxml-4.2). It gains no `<param>`
  (Alternatives rejected, 1).
- Between sessions nothing changes. A value crosses as an event through the door, in a later macrostep of the
  receiver (ADR 0084). Parts that must agree within one macrostep are parts of one session: regions of one document,
  or a pure function of data the session holds, which it computes by calling an algorithm (`<sce:call>`,
  `SCE_FORGE.md` §4.12). A table that the logic of several sessions reads within one macrostep is therefore a map of
  one machine (ADR 0108) whose regions are those parts, never a table one session reads of another.

### D2. A `<parallel>` may declare that its regions share nothing

`sce:regions="share-nothing"` on a `<parallel>` makes D1's form the only one between its regions:

- **Whose content is it.** Executable content belongs to the region that holds the transition's domain
  (`getTransitionDomain`, Appendix D), or that holds the `<onentry>` or `<onexit>`; a transition whose domain is the
  `<parallel>` or above it, and the `<parallel>`'s own `<onentry>` and `<onexit>`, are the composite's content.
- **What a region writes.** A `<data>` is written by a region when content of that region names it as the `location`
  of an `<assign>` or of a `<sce:call>`, the `target` of an `<sce:append>`, `<sce:put>`, `<sce:remove>` or
  `<sce:clear>`, a `<sce:call>`'s `inout` argument, an `idlocation`, or the location a host operation answers into
  (ADR 0109, D5 to D7); and when it is declared in a state of that region and bound late, since its binding is a write
  of that region (§scxml-5.3).
- **The refusals.** A `<data>` that one region writes is read and written by no guard, expression or executable content
  of a sibling region. No expression of a region calls `In()` of a state of a sibling region, and no transition of a
  region targets a state of a sibling region. A `<data>` that no region writes, because only the composite's content
  writes it or nothing does, may be read by every region.
- **Its reach.** The declaration holds the regions of the `<parallel>` that states it. A `<parallel>` nested in one of
  them is part of that region, and its own regions share nothing with each other only where it declares so itself.
- What a region writes is read inside that region alone, and reaches a sibling only as the data of an internal
  event, in the queue's order. "Never by reading another's state" is then a fact the build holds, checked on the
  document's text, and a microstep's effect on the data is the same in whatever order its regions are written.

It is a declaration and not the rule of every document. A document's data are one datamodel (§scxml-5.3), Harel's
statecharts coordinate orthogonal components through shared data and `in(S)`, the recommendation's own documents read
across regions, and SCXML's document order already makes such a read deterministic; refusing it everywhere would refuse
documents that conform, for a discipline and not for a defect. A `<parallel>` that declares nothing keeps the
recommendation's meaning. The declaration states which discipline the document keeps and the build holds it to it, as
`<sce:closable/>` makes a queue's type say what it can answer (ADR 0157 D2).

It is read under `sce-static` alone. Under a script data model an expression is text an engine evaluates, and the
build cannot list what it reads; `sce:regions` there is refused (D6).

### D3. A host effect stands wherever executable content stands, under `sce-static`

```xml
<transition event="request.done">
  <foreach array="staged" item="reply" index="i">
    <sce:action name="send_reply" effect="duty">
      <sce:arg expr="reply.request"/>
      <sce:arg expr="reply.part"/>
    </sce:action>
  </foreach>
  <sce:action name="send_final" effect="duty">
    <sce:arg expr="_event.data.request"/>
  </sce:action>
  <sce:clear target="staged"/>
</transition>
```

- Under `sce-static` a `<sce:action>` may stand inside `<foreach>` and inside `<if>`, `<elseif>` and `<else>`, at any
  depth, in a transition, an `<onentry>`, an `<onexit>` or initial content.
- Its arguments are typed expressions over the scope where it stands: the triggering event's payload where one is in
  scope, the machine's variables, and a loop's `item`, `index` and `sce:key` (ADR 0108 D5). Each is a value a host
  argument may be (a number, a `bool`, a text, a byte string, an optional of one, §2.15); a record's field is passed
  as its field, and a record, an enum, a list or a map is no argument, as today.
- One name has one signature at every site, a site inside a loop among them, so one host method serves them all
  (`validation/native-action-signature-conflict`, as today).
- Under every other data model the placement rule stands. An argument there names a field of the triggering event's
  payload, which no pass of a loop changes, and a loop of a script data model is its engine's.

### D4. One call per pass, in the walk's order, and a failed argument ends the walk and its block

- **Per element.** A `<foreach>` walks its list or map as it was when the loop began (§scxml-4.6, ADR 0108 D5), so the
  host is called once per element of that copy, in its order: a list by index, a map in the order its keys were put.
  Each call's arguments are computed in its own pass.
- **A failed argument ends the walk and its block.** Where an argument of one pass fails, the host is not called for
  that pass, `error.execution` is raised, and the walk and the block that holds it end (§scxml-4.6, §scxml-4.9); the
  calls made are those of the earlier passes. In D3's example a reply whose `part` fails stops the drain: the replies
  before it are sent, no final is sent, and `staged` is left as it was, so no reply is lost. The same rule holds for a
  host call wherever it stands: §2.15's "the block goes on" is withdrawn on every backend (Amends), so a call means one
  thing wherever it is written. The document takes the error and counts it as it counts any. ADR 0076's count of the
  errors a loop can raise falls from its passes times its actions to one, since the first ends the loop.
- **The order of a macrostep's calls** is the order the content runs: the transitions of a microstep in document
  order (§scxml-3.13), each block in order, each loop by element. A scenario's `host_calls` lists them so
  (`docs/SCE_ACCEPTED_SUBSET.md` §2.15, the replay scenarios), and a recording host records them so.
- **A duty in a loop is a duty per call** (ADR 0038). A `released-by` stays on an action that is a direct child of an
  `<onentry>` (`validation/native-action-release`): a release is performed once on the state's exit, and could not
  name which of a loop's calls it releases.
- **No call answers the machine.** A host's answer per element, where one is needed later, comes back as an event: a
  host-run `<invoke>`'s result, or an ask answered later (ADR 0111 D5). An answer the same step needs, a clock read or
  the first of several sources that can answer, is not this record's: a sans-I/O core reads no clock of its own, so it
  is read by the seam and handed in, or it is a host operation that answers a value inside the step (ADR 0109, D5 to
  D7).
- **The calls of one macrostep are finite.** A loop walks a collection whose bound is part of its type (ADR 0086,
  section 16), so a macrostep makes at most as many calls as the bounds of the loops it runs allow.

### D5. Lanes

Every lane that lowers `<sce:action>` under `sce-static` lowers it inside a loop and a branch, each through the call
it already makes: Rust and Kotlin through the host's trait and interface, C++ through its abstract struct, Go through
its interface, Python through its protocol, Swift where it lowers an action, and C11 through its vtable, once per pass
and with no buffer, since each argument is read from the element when the call is made. Every lane that lowers a
`<send>` under `sce-static` discards a message whose `<param>` fails (D1). A lane that has no lowering of a loop's
call yet refuses it by name (`generate/unsupported-feature`, naming this record) until its step lands. The Interpreter
admits the placement already, and takes D1's and D4's failure rules with every other lane.

### D6. Refusals

- `validation/native-action-placement` (existing): under a data model other than `sce-static`, a `<sce:action>` inside
  `<if>` or `<foreach>`.
- `validation/native-action-release` (existing): a `released-by` on an action that is not a direct child of an
  `<onentry>`.
- `validation/native-action-argument` (existing): an argument that is a record, an enum, a list or a map, or one that
  reads `_event.data` where no payload is in scope.
- `validation/region-reads-sibling` (new): under `sce:regions="share-nothing"`, a variable a region writes and a
  sibling region reads or writes, `In()` of a state of a sibling region, or a transition into a state of a sibling
  region, naming the variable, the state or the transition, the region that owns it, and the place that crossed.
- `scxml/static-datamodel-rule` (existing): `sce:regions` under a data model other than `sce-static`, on an element
  other than `<parallel>`, or with a value other than `share-nothing`.

The new code lands in the acceptance document's appendix with its first emitter (`acceptance_doc_covers_every_code`).

### D7. How the model generator reads it

The model reads none of this until it reads a `sce-static` document at all: `Construct::Datamodel` refuses one whole,
and ADR 0156 D3's list has no step that lifts it. This record adds that step to ADR 0156 D3 (Amends), and step 5 of
"Order of work" depends on it. Then the generator reads these as follows, each written once beside
`docs/formal/AppendixD.tla` and the only definition the generator reads (ADR 0156 D2):

- **An event is a name and its data.** An entry of either queue is a record of its name and its data, the data a
  record of the event schema's fields. A value of a type with a bound is one of finitely many (ADR 0086, section 16.1,
  item 6). An event from outside takes its data from the exploration's domains (ADR 0115 D7); an internal event's data
  are computed by the document and need none. `<raise>`'s data are the empty record.
- **`<send target="#_internal">`** appends to the internal queue the event with each `<param>`'s value as the
  datamodel holds it when the send runs, and `_event.data.<field>` reads the field of the event being taken; a send
  whose `<param>` fails appends nothing, raises `error.execution` and ends its block (D1). The generator's refusal
  of a `<send>` with "a target" is lifted for `#_internal`. The internal queue's length is at most the bound the build
  proves (ADR 0076), a constant of the module, and the bound invariant of ADR 0156 D4 holds it.
- **`<if>`, `<elseif>` and `<else>`** take the first branch whose condition holds; a condition that cannot be
  evaluated counts as false and raises `error.execution` (§scxml-5.9.1).
- **`<foreach>`** is a fold over the copy's elements, in order: each pass's effect is composed onto the one before,
  its assignments in sequence and its events and calls appended, and the fold stops at the first pass that fails,
  which raises `error.execution` and ends the block (D4). A loop's model is its sequence of effects and nothing else.
  A map is held in the model as the sequence of its key and value pairs in the order its keys were put, a put of a
  present key replacing its value in place and a remove taking its pair out (ADR 0108 D2), so a walk is that sequence's.
- **`<sce:action>`** is an output. A block's effect gains the sequence of calls it makes, each a record of the
  action's name and its arguments, appended in the order run. A model's step is a macrostep (ADR 0115 D7), so its
  output is the macrostep's calls, the sequence a lane's run reports as `host_calls`, and ADR 0115 D8's check of each
  lane's run compares them call by call. That the host performs a duty is an assumption of the environment, named
  with every verdict (ADR 0156 D1, item 4; ADR 0038's `performs`).
- **`sce:regions="share-nothing"`** is a check of the text (D2) and adds nothing to the model.

## Not in scope

- A host operation that answers a value inside a step, a clock read among them (ADR 0109, D5 to D7; D4).
- A value another session computed, used in the same macrostep: sessions share no macrostep (ADR 0084); parts that
  must are one session (D1).
- A `<param>` on `<raise>` (Alternatives rejected, 1).

## Consequences

- The consumer's rows have a form. A fan across registries is one document whose registries are regions that share
  nothing, each verdict crossing as an internal event's data in the macrostep, and the tables the registries share are
  maps of that one machine; a drain is a `<foreach>` over a bounded list with one declared effect per element, in the
  list's order, which a failed element stops before the final is sent; a source asked per entry is asked by an effect
  and answers by an event, or, where the step needs its answer, is read by the seam.
- A document that reads across regions today is unchanged; only a `<parallel>` that declares `share-nothing` is held.
- Every lane's host call whose argument fails now ends its block, and every lane's `<send>` whose `<param>` fails is
  not sent: a change of behaviour on every backend, held by the scenarios that state it (`static_send_params` among
  them), which change with it.
- Every lane that lowers `<sce:action>` under `sce-static` gains a loop and a branch around a call; the C11 lane calls
  through its vtable once per pass and holds no buffer for it.
- The model's effect of a block gains the calls it makes, and every lane's trace is read against them.

## Alternatives rejected

1. **A `<param>` on `<raise>`.** It would be a second spelling of `<send target="#_internal">`, which the
   recommendation already gives data, and `<raise>` has no children there (§scxml-4.2).
2. **Share-nothing as the rule of every document.** It refuses documents that conform: a document's data are one
   datamodel (§scxml-5.3), and the recommendation's own documents read across regions. Esterel's reason for the rule,
   branches with no order, does not hold where document order orders them (D2).
3. **A region's variables private by where they are declared** (a `<datamodel>` inside a region). A `<data>` inside
   a state is a variable of the one datamodel, whose binding time alone its place decides (§scxml-5.3); giving its
   place a scope would change what the recommendation's documents mean.
4. **An effect per element made by an algorithm.** An algorithm is a function of its arguments (`SCE_FORGE.md` §4.12)
   and calls nothing of its host; an algorithm with effects would no longer be the operator the model reads it as.
5. **One host call that takes the whole list.** The host would walk it, so the order and each element's failure would
   be the host's and outside the model, and a list is no host argument (§2.15).
6. **Letting the walk go on past a failed argument** (the proposed text). It keeps one rule of SCE's for a host call
   against two of the recommendation's (§scxml-4.6, §scxml-4.9) and SCE's own rule for a loop's other elements, and
   in D3's example it sends the final with a reply missing. The host call's rule is brought to the recommendation's
   instead (D4).
7. **Sending a message with a failed `<param>` left out.** A receiver would take a typed event missing a field, which
   no conforming processor sends (§scxml-6.2).
8. **A host effect per element only inside `<foreach>`, and not inside `<if>`.** The refusal of both had one reason,
   a call site that is conditional or iterated, which no longer holds for either under `sce-static`.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The placement** (D3, D6): `check_placement` admits a nested action under `sce-static`, registers its signature as
   a direct one's and types its arguments over the loop's names.
   `sce-build/tests/a_host_effect_per_element_is_a_declared_effect_inside_foreach.rs` accepts an action inside a
   `<foreach>` over a list of records (arguments `item.<field>` and `index`), over a map (`sce:key`), inside an
   `<if>`, and two loops deep; it refuses the same document under `ecmascript`
   (`validation/native-action-placement`), a `released-by` inside a loop (`validation/native-action-release`), a
   record item as an argument (`validation/native-action-argument`), and a loop's call whose argument types differ
   from a direct call of the same name (`validation/native-action-signature-conflict`). Red without the step: the
   first accepted case is refused as `validation/native-action-placement`.
3. **The failure rules** (D1, D4) on every lane and the Interpreter: a host call's failed argument ends its block, and
   a `<send>`'s failed `<param>` discards its message and ends its block; ADR 0076's count follows. The scenarios that
   state the old rules change with them on every driver that replays them (`static_send_params` and
   `static_host_call_arguments` among them): `bad` now sends nothing and ends with one refusal. Red: a lane that still
   sends the message or still runs the block's next element.
4. **The lowering** (D4, D5) on every lane that lowers `<sce:action>` under `sce-static`, with the fixture
   `sce-build/tests/fixtures/static_datamodel/static_host_call_each.scxml` and `scenarios/static_host_call_each.json`,
   replayed by every driver that replays that directory (Rust, Kotlin, Python, Go, Swift, C++, C11 and the
   Interpreter). Its cases: three staged records, a call per record and then the final call, in that order; no
   staged record, the final call alone; a record whose argument fails in its pass (an `int8` sum past its range),
   the calls of the records before it, one `error.execution` taken and counted, no final call, and `staged`
   unchanged; a map walked in the order its keys were put after a remove and a put again. Red: a lowering that calls
   once per loop, that goes on past the failed pass or runs the block after it, or that walks a map in another order.
5. **The regions** (D2, D6): `sce:regions` in the grammar and the check.
   `sce-build/tests/regions_that_share_nothing_cross_values_as_events.rs` refuses, as
   `validation/region-reads-sibling`, a guard of one region that reads a variable its sibling writes, an `<assign>`
   that writes one, an `<sce:append>` to one, a host operation answering into one, `In()` of a sibling's state and a
   transition into a sibling's state; it accepts a variable that only the composite's content writes, read by both
   regions, a transition whose domain is the `<parallel>` writing it, a nested `<parallel>` that declares nothing, and
   D1's example as the fixture `static_region_value.scxml`, whose scenario takes `face.closed` and holds, in that step,
   the fan's host call made with the value the registry sent. It refuses `sce:regions` under `ecmascript`
   (`scxml/static-datamodel-rule`). A second test, `a_share_nothing_microstep_does_not_depend_on_region_order`,
   generates each accepted document with its regions' document order swapped and compares the data after every
   microstep of its scenario. A mutation casefile, `sce-build/tests/mutations/a_region_reads_only_what_it_writes.cases`,
   drops the guards from the reads the check collects and must turn the guard's case red.
6. **The model** (D7): first the step added to ADR 0156 D3, which has the model read a `sce-static` document; then an
   event's data, `<send target="#_internal">`, `_event.data`, `<if>`, `<foreach>`, a map as a sequence and
   `<sce:action>`. `static_host_call_each` and `static_region_value` are entered in `docs/formal/models.json` with a
   run that holds, and their runs on every lane are read against their models (`docs/formal/traces.json`, ADR 0115
   D8), with a trace that drops one host call, one that swaps two, and one that calls past a failed pass, each refused.
   Red without the step: `sce-codegen model` refuses both documents by name.
7. The documents named in "Amends".

## Open questions

None. The proposed text's one question, whether a failed argument in a pass ends the walk, was decided on acceptance:
it ends the walk and its block (D4).

## Amended on acceptance

Items 1 to 8 are the independent review's, by its number there; items 9 and 10 the consumer's review as its user.
Each was accepted by the main session on 2026-11-29.

1. D4: a failed argument ends the walk and its block (§scxml-4.6, §scxml-4.9), the host-call rule "the block goes on"
   is withdrawn on every backend, ADR 0076's count follows, step 4's case is the drain that stops, and the open
   question is closed; Alternatives rejected, 6, is now the rejected proposal (review 13).
2. D1: a `<send>` whose `<param>` fails is not sent under `sce-static` (§scxml-6.2), amending §2.15 and ADR 0076, with
   step 3 (review 14).
3. The textbook section and D2: share-nothing is a modularity discipline, since SCXML's document order makes reads
   across regions deterministic where Esterel's unordered branches do not; the region-order test of step 5
   (review 15).
4. D2: content belongs to the region of its transition's domain, a transition into a sibling is refused, a host
   operation answering into a location and a late binding are writes, and the declaration does not reach a nested
   `<parallel>` (review 16).
5. D7: the step that has the model read a `sce-static` document is added to ADR 0156 D3, and the model reads a
   branch, a map as a sequence, a fold that stops at a failure and a `<send>` to `#_internal` (review 17).
6. The examples drop `type="internal"`, which means nothing on a transition with no target (§scxml-3.5), and D1 states
   what a region does not take (review 18).
7. The textbook section: STATEMATE senses a step's events as a set, SCXML one per microstep in order; D4 notes ADR
   0076's count for a loop (review 19).
8. "Amends" names the RFC's §7 (review 22).
9. D4 and "Not in scope": an answer the same step needs, a clock read among them, is read by the seam or is ADR 0109's
   host operation, from the consumer's review.
10. D1: a table the logic of several sessions reads within one macrostep is a map of one machine (ADR 0108), from the
    consumer's review.

## Sources

- W3C, *State Chart XML (SCXML): State Machine Notation for Control Abstraction*, Recommendation, 1 September 2015:
  sections 3.5, 3.12, 3.13, 4.2, 4.6, 4.9, 4.10, 5.3, 5.7, 5.9.1, 6.2, 6.2.4, G.7, and Appendix D; its tests 156, 159
  and 553.
- D. Harel, "Statecharts: a visual formalism for complex systems", Science of Computer Programming 8(3), 1987.
- D. Harel and A. Naamad, "The STATEMATE semantics of statecharts", ACM TOSEM 5(4), 1996.
- G. Berry, *The Esterel v5 Language Primer*, 2000.
- D. L. Parnas, "On the criteria to be used in decomposing systems into modules", Communications of the ACM 15(12),
  1972.
- C. A. R. Hoare, "Communicating sequential processes", Communications of the ACM 21(8), 1978.
- G. Agha, *Actors: A Model of Concurrent Computation in Distributed Systems*, MIT Press, 1986.
- G. H. Mealy, "A method for synthesizing sequential circuits", Bell System Technical Journal 34(5), 1955.
