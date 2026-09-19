# Authoring assist — the contract between a prose specification and a platform

This directory holds a **domain-free** core. It knows nothing about vehicles,
clusters, ignition, telltales or any other subject matter, and a test refuses
the package if a domain word appears in it (`tests/test_core_is_domain_free.py`).

Everything the core needs about a subject matter arrives as **data**, in the two
schemas under `schema/`. A subject matter that supplies those two files is
called a **pack**. The core plus a pack is what turns prose into SCXML.

    prose (1..n files)   the decision logic. Nothing else has it.
    interface model      what names exist outside the document, and what
                         values each one may take
    conventions          the five answers below

Nothing in the core is allowed to know which pack it is running against, and
nothing in a pack is allowed to be code.

---

## Why a pack has exactly five answers

They were not designed; they are what a converter actually asked for, over
twenty conversions of one subject matter. Each one is a question the prose does
not answer and cannot answer, because it is about the platform rather than about
the decision.

| | The question | Where it is answered |
|---|---|---|
| 1 | Given a name in the prose, what is it? | interface model (`names`) |
| 2 | Where does an output go, and what values may it take? | interface model (`fields`) |
| 3 | A precondition is written as a phrase. What expression is that? | conventions (`preconditions`) |
| 4 | When that precondition is false, what does the output become? | conventions (`gate_off`) |
| 5 | Which names must come from outside, and which does the document define itself? | conventions (`name_classes`) |

**Question 4 is the one that is always forgotten.** Prose states what a display
shows while a feature is active and stops there. Every platform has an answer
for the other case and none of them write it down.

---

## What the core does

    python3 -m sce_author brief     --pack <dir> --prose <file>...
    python3 -m sce_author questions --pack <dir> --prose <file>...
    python3 -m sce_author check     --pack <dir> --document <file.scxml>

**brief** assembles one page for whoever writes the document: the prose, the
addresses and value spaces it touches, the precondition vocabulary, and the
questions already known. It accepts any number of prose files and resolves
names across all of them, because one feature is frequently written across
several documents and a name introduced in one is used in another.

**questions** is the deliverable that matters for an author: what the prose does
not say. All domain-free; the ones an author acts on most are:

| class | what it means |
|---|---|
| `unknown-name` | the prose names something the interface model does not have, in a class the conventions say must come from outside |
| `name-is-indexed` | the prose names one subscripted thing and the platform publishes a numbered family of them, and nothing says which member an index picks |
| `value-not-in-space` | the prose compares a name against a symbol that name cannot take |
| `no-decision-logic` | an output exists with nothing in the prose that decides it |
| `gate-off-unstated` | an output is gated and the prose does not say what it becomes when the gate is false |
| `no-time-input` | the prose states a duration and no input can observe time passing |
| `example-shows-memory` | two cases drive the same inputs and require different results, so the component remembers something the prose never states |
| `depends-on-another-component` | the examples drive addresses another specification in the system writes, so this document is one of several and cannot be judged alone |

**check** judges a written document against the same model: every input it
declares must exist, every output it writes must be a real field, and every
literal it compares against must be in that field's value space.

**verify** RUNS the document against the pack's examples and says which cases
it fails and where. It is the only command that says whether a document
*behaves*: a document can satisfy `check` completely and compute the wrong
answer at every address, because `check` never executes anything. Until it
existed, the last step of the workflow was "and hope".

It needs nothing new. The examples carry values, so they are a test suite; the
binding says which address feeds which input and receives which output, so the
marshalling needs no subject knowledge; and the product already generates
runnable code from the document. ⚠ It refuses rather than skipping -- a kind
whose generated shape it cannot drive, an input rule it cannot evaluate, an
expected address the binding never writes. A verifier that quietly skips what
it does not understand reports a clean run for a document it never executed.

### Before any of that: what the file itself gives up

Every command starts by turning a file into text, and that step is where a
specification quietly loses the parts that decide things. A format is not a
subject matter, so the reader lives in the core and answers two things: the
text, and what it could not carry.

**What a document encloses is opened, not merely named.** A table pasted from
a spreadsheet is stored as the whole spreadsheet, and the body text keeps only
a reference to it. `.xlsx` and `.pptx` are zips of XML, so the rows come out
mechanically -- nothing to guess. Measured on one 22,669-line specification:
eleven enclosed files held 732 spreadsheet rows and 14 slides that the body
handed its requirements to, and every command downstream had been running
clean on the remainder. ⚠ The grid is preserved by placing each cell at the
column its reference names; a sparse row omits its empty cells, and reading
positionally turns "condition A gives X" into "condition A gives Y" with no
sign that it happened. A merged range is **counted and left as stored** --
inventing which rows it covered would manufacture rules nobody wrote.

**A picture is not read, and the report says where the unread ones sit.** This
core makes no model calls, so it cannot say what an image shows. But the count
alone -- "71 pictures were not read" -- leaves two piles a person cannot tell
apart: a screenshot beside a paragraph that already states the rule, and a
diagram a clause hands its whole content to. The second is the requirement.
Which one it is *can* be decided mechanically, by asking whether the numbered
clause around the picture states anything at all, so that is what is reported,
with the clause number attached. On the specification above the answer was
**none**: every clause showing a picture also states something in text.

**And then the picture is read by whoever can read it.** Refusing to read a
diagram is not the same as refusing to use one. A reading may come from a
person, a model, or a phone call with whoever drew it, and it enters the
document the way every other guess does -- marked `sce:assumed`, with the
clause it came from written in the reason. `verify` runs the document and, if
a case refutes that value, reports *the guess you recorded* rather than *your
document is wrong*. The four hops are one route and it is tested as one:

    ingest     names the clause whose whole content is a picture
    questions  puts that in front of the author
    the author reads it and writes the rule, marked
    verify     runs it and hands the author's own sentence back

What is refused is not the picture. It is a reading of a picture entering as a
fact, where nothing downstream can ever disagree with it.

⚠ A cheaper discriminator was built first and measured wrong: "a stretch of
pictures with no text between them" was true of 156 drawings out of 156,
because a word processor anchors a picture in a paragraph of its own. A test
every instance passes says nothing. And a document with no numbering says so
rather than reporting "no clause hands its content to a picture", which would
be true of it trivially and read as reassurance.

### What "precision" means here, and where it can actually go wrong

Most of these classes are not empirical claims. `ambiguous-name` fires exactly
when one name reaches two addresses; `value-not-in-space` fires exactly when a
compared symbol is outside a declared value space; `example-shows-memory`
fires exactly when two cases drive equal inputs and require different results.
Given the pack, each of those is a **theorem, not a measurement** -- counting
how often it is "right" on a corpus measures the corpus, not the check.

What can genuinely be wrong is the PACK, and it fails in exactly two ways:

| the pack is wrong about | which classes then lie | how to see it |
|---|---|---|
| the spellings an address goes by (`names`) | `no-decision-logic`, and every class that skips an address it thinks unmentioned | an address reported undecided that the prose plainly decides under another spelling |
| which text belongs to which address | `gate-off-unstated` above all | the share of the document `blocks()` attributes, and how many blocks are one or two lines |

### One class this cannot have, and what stands in for it

The commonest real gap in a specification is a table whose rows can both apply
and which never says which wins. It is the cause of both conversions this
corpus could not settle, and it is NOT in the class list. Three instruments
were built for it and all three failed:

| instrument | result |
|---|---|
| count conditional clauses in the output's prose block | **zero** on the known-bad addresses; these documents state logic as tables, and only 11 blocks of 1257 hold even two clauses |
| count table ROWS in the block | the known-bad address ranked 1099th of 1257 |
| count how many of the field's OWN symbols the block names | both known-bad addresses scored zero, ranking 1020th and 1183rd |

The three fail for one reason, and it is structural rather than a matter of a
better predicate: **the deciding table is keyed by something other than the
output's name** -- an event identifier, a stage number, a bare quantity -- so
attributing text by name gives those addresses almost nothing, and the
addresses with the most complex decisions are exactly the ones it starves.

Reading the table instead would mean parsing the specification's LOGIC, and
this package deliberately does not: every class here compares a text against a
model, which is what lets one core serve any subject matter. A reader who wants
this class back should know they are asking for a different tool.

⚠ What stands in for it is not a question but a marker plus verification. The
author writes `sce:assumed` on the value they had to choose, with the reason;
the build proceeds; and `verify` reports the assumption BY NAME the first time
a case contradicts it. Measured: that is exactly how the one such gap in this
corpus is currently surfacing.

**Recall has a ground truth nothing human has to write.** Take a document the
tool is quiet about for some output -- meaning the prose does answer there --
delete the lines that carry the answer, and the matching class must now fire.
The expected result is fixed before the tool runs, so the test cannot be tuned,
and a specification supplies as many trials as it has answered outputs. Doing
that for `gate-off-unstated` gave 38 of 49, and every one of the eleven misses
was one defect: the off value was sought as a substring, so `DISPLAY_OFF` read
as the document having said `OFF`. Fixed, the same trials give 49 of 49.

⚠ What this cannot measure is a silence there is no class for. Recall here is
recall WITHIN the vocabulary -- the difference between "a class exists" and "it
fires when it should", which is worth having and is not the whole question.

So a reader judging this tool should ask about the pack, not about the class
list. ⚠ And a number measured on the corpus a check was designed against is
in-sample: it says the check does what it was built to do, which was never in
doubt. Held-out numbers need a subject matter whose gaps the author did not
plant -- the fixture under `tests/fixtures/` is a second subject matter, but
its gaps ARE planted, so it proves the classes carry across a different
document shape and vocabulary, not that they are precise on unseen text.

---

## Writing a pack

A pack is a directory containing

    interface-model.yaml     (or .json, or several files — all are merged)
    conventions.yaml
    examples.yaml            (optional, and the tool is much weaker without it)

and nothing else that the core reads. Getting a platform's own model into
`interface-model.yaml` is the pack author's work and belongs with the pack, not
here — a converter script for one subject matter is domain knowledge, which is
exactly what may not live in this directory.

Both files carry `version: 1`. The schemas are in `schema/` and the loader
refuses a file that does not validate, naming the path that failed.

### interface-model

    version: 1
    entries:
      - address: Some.Qualified.Address
        role: input                               # or output, or internal
        names: [WhatTheProseCallsIt, AnotherSpelling]
        values: {NONE: 0, LOW: 1, HIGH: 2}        # a scalar address

      - address: Another.Address
        role: output
        names: [WhatTheProseCallsIt]
        fields:                                   # a record address
          Stat:  {values: {NONE: 0, OFF: 1, ON: 2}}
          Value: {type: number}

      - address: Another.Unit.Output
        role: upstream                            # another SPECIFICATION writes it
        names: [WhatTheProseCallsIt]
        type: number

`names` is what makes question 1 answerable: prose writes names, platforms have
addresses, and no document anywhere states the correspondence. It is a list
because prose is inconsistent.

**`role: upstream` is how a pack says one specification is not one program.**
An address the examples drive and the model does not declare reads as a
document reaching outside what it declares -- which is a defect report, and
for some documents it is simply wrong. Measured over 129 subject packs against
a 244-component platform: of 180 such addresses, **45 are output by another
component**, and they concentrate rather than spread -- 19 of one pack's 20,
9 of another's 9, against 2 of the largest pack's 33. Declared `upstream`, the
tool says the useful thing instead: this document is one of several and cannot
be judged alone, with the addresses to go and look up. Both sentences are said
**once per pack**, not once per address; 180 findings fell on 33 packs, and
thirty-three copies of one sentence bury every other class.

### conventions

    version: 1

    name_classes:            # question 5
      - pattern: '\b(?:Input|Inter)_[A-Za-z0-9_]+\b'
        role: supplied       # from outside -> a question when unknown
      - pattern: '\bPrivate_[A-Za-z0-9_]+\b'
        role: own_internal   # the document defines it
      - pattern: '\bOutput_[A-Za-z0-9_]+\b'
        role: own_output     # the document produces it

    preconditions:           # question 3
      inputs:                # names the document declares to receive
        powerOn: "the supply is present"
      phrases:
        "supply on": "powerOn"
        "supply off": "!powerOn"

    gate_off:                # question 4, an ordered cascade
      - {when: has_symbol,     symbol: "OFF",  use: "OFF"}   # ⚠ quoted
      - {when: unique_suffix,  suffix: _OFF,   use: matched}
      - {when: binary,                         use: last}
      - {when: always,                         use: first}

`gate_off` is ordered and the first clause that applies wins. It is a cascade
rather than a set of rules because a generator has to choose one value; a rule
set that offers three candidates has not answered anything.

A convention may also DEFINE a reading idiom rather than only naming one:

    protocols:
      last-incremented:
        parameters: [on_counter, off_counter]
        latch:
          set_when_changed: on_counter
          clear_when_changed: off_counter
          both: last           # set | clear | last  — who wins when both move
          initial: clear
          cumulative: [Ladder.Rung0, Ladder.Rung500, Ladder.Rung3500]

Naming a protocol without defining it is still allowed and still means "the
core does not know what this is" — `verify` then declines rather than guessing.
⚠ Every field of `latch` was put there by a case that a shorter definition got
wrong: `cumulative` because reaching a later rung includes the earlier ones and
without it an input asking "has it been on at all" goes false the moment a
longer reading passes; `both` because the round where both move is ordinary and
all three answers occur; `last` because the record's own order says which was
more recent, which is what the idiom is named for.

### examples

The second thing that can expect something, and therefore the second thing that
can reveal a silence. An interface model catches a specification that says
something the platform cannot do; only an example catches a specification that
does not say something the platform does.

    version: 1
    origin: the product's own shipped tests
    independent_cases: true      # each `given` is the WHOLE input
    ordered: true                # the cases are in the order they happened
    cases:
      - name: a train approaches
        variant: MAINLINE        # which build this record was taken from
        elapsed_ms: 4123         # how long the situation had HELD, not a clock time
        given:  {plant/in/approach: APPROACHING, plant/in/power: OK}
        drove:  [plant/in/approach]     # what this entry SET, not what merely held
        expect: {plant/out/signal.value: FLASHING}

⚠ The four properties are separate because they answer separate questions, and
each of them cost a measurement to separate:

| property | what it lets the core do | what happens without it |
|---|---|---|
| values in `given`/`expect` | run the document at all | every check that compares two cases declines |
| `independent_cases` | compare two cases | the memory check declines rather than reading a delta as a whole input |
| `ordered` | replay something that carries state | protocols and `previous_of`/`state_of` decline |
| `drove` | know what a case ASSERTED | a reading restated at the same value looks like nothing happening |

⚠ `variant` and `elapsed_ms` sit on the case rather than in `given` for the
same reason: neither is a signal. Nothing drives them, they have no address and
no value space. A record tagged with a build and never setting a configuration
signal is ordinary — the tag IS the statement — and until the variant had a
place to live, no binding could read such a case at all.

`elapsed_ms` is a DURATION, not a moment on a timeline — it restarts whenever
the situation does, and a record where it goes backwards is ordinary. Time is
its own category: no address, no value space, nothing drives it, which is why
it sits on the case rather than in `given`.

### binding — the third artefact

Written per document rather than per pack, and named on the command line. It
says which address feeds which of the document's own inputs and receives which
of its outputs. ⚠ This is the artefact that did not exist before: a prose
specification decides, an interface model declares, and neither says which of
the model's addresses the document's names are.

    version: 1
    document: controller.scxml
    inputs:
      approaching: {address: plant/in/approach, equals: APPROACHING}
      supplyOn:    {unresolved: "the platform list is not available yet"}
      anyWarning:  {address: plant/in/state, equals_any: [WARN, FAULT]}
      notClear:    {address: plant/in/state, not_equals: CLEAR,
                    note: "a negation stays right when the enum grows"}
      silent:      {address: plant/in/state, absent: true}
      level:       {address: plant/in/level, number: true, range: [0, 255],
                    when_absent: 0}
      onMainline:  {variant_is: [MAINLINE, BRANCH]}
      sinceRise:   {clock: true, when_absent: 0}
      wasDown:     {previous_of: approaching}
      lastShown:   {state_of: signal, initial: 0}
      supplyOn:    {protocol: last-incremented,
                    parameters: {on_counter: plant/count/on,
                                 off_counter: plant/count/off}}
    outputs:
      signal:  {address: plant/out/signal, field: value,
                map: {0: "DARK", 1: "FLASHING"},
                when: {1: {blink: "ON"}}, also: {source: "LOCAL"}}
      reading: {address: plant/out/reading, field: value, passthrough: true}
      held:    {internal: true}

### Writing the document before the addresses exist

⚠ The document is ALREADY independent of the platform — it uses its own
identifiers and this file is the dictionary — so the decision logic can be
written in full before anybody has produced the platform's list of addresses.
That is the ordinary situation when a specification arrives first.

`unresolved` is how a rule says so. It is written instead of `address`, and
the string is the reason, for whoever can answer it:

    supplyOn: {unresolved: "the source calls this the supply signal and
                            the platform list is not available yet"}

`check` then reports an address still missing rather than a name that does not
exist; `verify` says it cannot run rather than running the document on a value
nobody supplied. When the list arrives, only this file changes.

⚠⚠ This exists because ABSENCE HAS TO BE WRITABLE OR IT DOES NOT GET WRITTEN.
Asked for a complete binding with no list to hand, an author — human or model
— produces a complete-looking one, and a plausible wrong address is invisible
in a way a missing one never is. The document's `sce:unresolved` has stopped
exactly this for VALUES since before this package existed; this is its peer for
addresses.

⚠ `when_absent` is required for a NUMBER whose address a case may not drive: a
symbol comparison needs no such declaration, because an address that is not
reporting is not any symbol and that is already the answer — but a number has
no such fallback, and folding absence into zero made seven cases on one corpus
look as though the specification had been misread.

⚠⚠ Quote every symbol. YAML 1.1 reads a bare `ON`, `OFF`, `YES`, `NO`, `TRUE`
and `FALSE` as booleans, so `equals: ON` becomes `equals: true` and a map of
`{0: OFF}` becomes `{0: false}`. The loader refuses these and names the
position, because writing this rule cost seven encounters with the same trap.

---

## The boundary is tested, not asserted

`tests/test_core_is_domain_free.py` reads every source file in `sce_author/`
and refuses a word from a vocabulary of subject matters. That test is the
reason this README can claim the core is general: the claim is measured on
every run rather than maintained by care.

⚠ The vocabulary a test refuses is necessarily a list, and a list is only as
wide as what someone thought of. The second guard is structural and does not
depend on anyone's imagination: the core reads **no file it was not given on
the command line**, so a domain fact has no route in except through a pack.
