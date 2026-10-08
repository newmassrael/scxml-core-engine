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

### One-time MCP setup for a specification owner

The owner needs no checkout and no Rust toolchain. Someone with the tree
builds a bundle once:

```sh
scripts/package_sce_author.sh dist/
```

It writes `dist/sce-author/` and a `.tar.gz` of it, containing the code
generator, the templates it renders, this MCP server, the license files, and
the launcher `bin/sce-author-mcp`. On the owner's machine, unpack it and
register `bin/sce-author-mcp` (by absolute path) as a local stdio MCP server
in the AI client. It needs Python 3 with PyYAML and jsonschema and nothing else
(`sce_author/needs.py` lists them, and `--check` asks about each). The exact
registration UI or configuration key depends on the client. Once connected,
the tool list should include `scxml_kinds`, `validate_scxml` and
`render_scxml_pseudocode`. `tests/test_the_bundle_runs_where_the_tree_is_not.py`
starts a bundle's launcher outside the tree and holds it to the protocol.

For a client that is not on the owner's machine — an assistant reached
through an API, or a hosted one — the same bundle serves over HTTP:

```sh
bin/sce-author-mcp --http 0.0.0.0:8765 --token-file TOKEN_FILE
```

The client connects to `http://HOST:8765/mcp` and sends
`Authorization: Bearer <token>`. Off loopback the server refuses to start
without a token. A remote client hands every file over as text
(`document_text`, `manifest_text`, `documents_text`), and a path from it is
refused, since a path names the server's files rather than the caller's.

A design from a remote caller is read and checked and **not played** unless the
operator says how safely it may be: `--run-designs-under process+rlimit` names the
weakest isolation they accept (`--help` lists the levels, weakest first), and a host
that gives less does not start. Without it `compare` still answers every level that
runs nothing and says `not judged` for behaviour, and `scxml_scenarios` reads the set
and answers `not run`, each with the way to allow it. The reason is what playing is:
a child process under the kernel's limits stops a design that runs away, not one that
reads a file or opens a socket, so a server open to strangers does not run their
designs until its operator has decided that is acceptable. Over stdio the designs are
the owner's own and are always played.

From a checkout, `scripts/sce_author_mcp.sh` runs the same server out of
the tree after `cargo build -p sce-build --features cli --bin sce-codegen`,
and takes the same `--http` flags.

`--check` (on either launcher) starts nothing and asks whether the server could do its work:
it exits 0 and says `ready`, or exits 1 and says, one line each, what is missing (the
generator and how to point at it, `sce-work`, each Python module of `sce_author/needs.py` that
is not installed). An application that starts an AI client with this server asks first, so that
a server that cannot start is told to the owner in these words and not as an AI that never
answers. It goes with no other option. That file is the one list of what the server needs of its
Python: the workbench's `.deb` declares a package for each (`app/src-tauri/tauri.conf.json`), and
`scripts/verify_installed_app.sh` holds the `.deb` that was built to it.

The launcher and MCP server run locally, but the AI client may send the prose,
SCXML, tool results, and pseudocode to its model service. Local MCP does not
guarantee local-only handling of a specification. Before using restricted
material, check the AI client's data handling against the applicable agreement
and use an approved model environment. The launcher contains no subject-matter
documents or customer-specific configuration.

The specification owner then provides a prose file or accessible link and
asks the AI to create and check a pseudocode draft. The AI reads the prose,
writes an SCXML-root document, calls SCE, and shows the diagnostics and
pseudocode. Before writing, the AI must choose a document kind from the
behavior. Event-driven behavior is a statechart; Forge kinds such as
`transform`, `lookup`, `condition`, `procedure`, `timer`, and `codec` describe
other shapes. A Forge document declares its choice with `sce:kind` in the SCE
namespace. If that attribute is absent, SCE reads the file as a statechart;
`check` validates that chosen route but cannot decide from prose whether it was
the right choice. The AI should explain its choice and ask the owner about
ambiguous behavior. The evidence is the prose's stated inputs, outputs,
events, retained state, timing, and data format; the AI should cite the
relevant clauses, not infer a kind from a filename or the statechart default.

What the choice rests on comes from the product, not from this repository's
documents or an assistant's memory of them. `scxml_kinds` returns the catalog
compiled into `sce-codegen` (`sce-codegen kinds`,
`schemas/sce-kind-catalog.v1.schema.json`): for every kind, the evidence a
specification offers for it, the behaviour that separates it from the kinds it
is confused with, the defaults the source has to decide, and a checked-in
example document that `check --lint` accepts and `pseudo` renders. An author
working outside this tree therefore reads the same catalog as the binary that
will judge the document, and a test holds every example to that binary. After
`validate_scxml`, the manifest's `document_kind` names the kind SCE read the
document as, and `document_kind.declared` is `false` when no `sce:kind` was
found and the statechart default applied, so a forgotten or misspelled
declaration shows up without any diagnostic being needed.

The choice is recorded in the document as well as explained in the
conversation. A `<sce:kind-basis>` directly under the root holds an
`<sce:evidence>` for each clause the choice rests on, with its provenance
anchor, and an `<sce:rejected kind="…">` for each neighbouring kind ruled
out (`docs/SCE_ACCEPTED_SUBSET.md` §2.10.1; the catalog's
`declaration.basis` carries the grammar and an example). The pseudocode page
shows it under the document's head line, so the owner reviews the reason
along with the document. The manifest's `document_kind.basis_recorded` says
whether one was written. A basis that rules out the document's own kind is
refused. When the specification does not decide the kind and a draft is
written anyway, the basis carries the `sce:unresolved` marker with the other
candidate kinds, so `--strict-unresolved` refuses to build it until the
owner decides.

Neither is a prose-to-kind classifier. The catalog describes kinds and the
manifest reports a reading; when the source states neither side of the
behaviour that separates two candidate kinds, the choice is the owner's, and
the AI should ask.

Whether that is enough for a client with none of this repository's context is
measured, not assumed. `eval/kind_choice_cases.json` holds prose
specifications written for the purpose — each determines one kind, or leaves
the choice open between named candidates — and `eval/kind_choice.py` hands
each one, in an empty directory, to a client that has only the SCE MCP
server, with the request above. It scores what the product read, not what the
client said: the written document goes through `validate_scxml`, and the
manifest's `document_kind` is compared with the case. A case whose text
leaves the kind open counts as right when no document was written, or when
the draft marks its kind open on its `<sce:kind-basis>` (`left_open`).
Writing it down is the same question, put where the owner and the strict
build see it.

```sh
python3 tools/authoring/eval/kind_choice.py --client claude-restricted --out /tmp/kind-eval
```

The report names how isolated its client was: `claude-bare` reads no
CLAUDE.md, memory or hooks and needs `ANTHROPIC_API_KEY`; `claude-restricted`
runs with no repository in reach but still reads a user-level CLAUDE.md.

Measured on 2026-09-29 with `claude-restricted` (the model current that day).
These are the numbers of one run each, not guarantees:

| | Before `<sce:kind-basis>` | With it |
|---|---|---|
| Determined cases, kind correct | 24 / 24 | 24 / 24 |
| Drafts recording the basis | — | 27 / 27 |
| Open cases asked or marked open | 1 / 3 (2 decided silently) | 3 / 3 |
| Determined cases wrongly marked open | — | 0 |

#### When the same specification is drafted more than once

The model that writes the draft runs in the owner's AI client, which SCE
does not control: nothing in the product can make two drafts of one
specification equal. What it can do is say where they part.
**compare** (the MCP tool of that name, or `python3 -m sce_author compare
--document a.scxml --document b.scxml …`) puts two or more drafts side by
side at every level a reader has: the bytes, the canonical XML, the logic
the build compiles (SCE annotations removed), the product's review table,
the pseudocode page, the open questions each draft marks, the vocabulary,
and — for statecharts — what the drafts do when driven alike. Each level
comes back as the classes of drafts that agree.

A draft that starts a child session (`<invoke src="child.scxml">`) is
compared with its child: the documents its static `src` names are found beside
the draft, the ones those start in turn too, and built with it, and a `src` that
climbs out of the draft's directory or names no file is left alone (the draft then
cannot start it, and `undriven` says so in the engine's words). Over MCP the child
documents ride with the draft that starts them (`documents_text[].companions`), and
every draft is built in a directory of its own: the drafts being compared were
written in separate conversations, so each calls its child `child.scxml` and means
a different document, and one directory for all of them could hold only one.
`companions_text` is for a child every draft starts and is the same document for
all; neither kind is a draft, and neither is compared.

Behaviour is compared by driving each draft's Python lowering with the
same seeded random drives, each draft in its own event names. Two drafts
whose inputs are named differently are tried under every renaming of one
alphabet onto the other, and the renaming that makes them alike is
reported; two that part come with a witness — a drive after which they
end in different states, reduced until no single step can be dropped —
spelled in the drafts' own event names so the owner can read it against
the prose. ⚠ Three things
the verdict says rather than hides: a draft that cannot be built is named
and left out of the classes; drives that moved nothing (one observation
per draft, typically because every input carries data no drive sent) are
`not judged`, never "alike"; and the drives are a bound, printed with the
verdict, not a proof — `--drives` sets how many (300 by default) and
`--steps` how long each is (25). Agreement is not correctness either —
drafts can agree and all be wrong.

`eval/reproducibility.py` drafts each case of `eval/reproducibility_cases.json`
several times through the same client and request as `kind_choice.py`
(Sonnet by default, the model the owner fixed for drafting), and reports
each case's classes per level. Measured on 2026-09-29, five drafts per
case, one run (1 means all five agree):

| Case | Kind | Bytes | Logic | Table | Open questions | Page | Behaviour |
|---|---|---|---|---|---|---|---|
| door-with-auto-close | statechart | 5 | 3 | 3 | 4 | 5 | 1, one event renamed |
| connection-keeper | statechart | 5 | 5 | 4 | 4 | 5 | 1, one event renamed |
| keep-alive | timer | 5 | 1 | 1 | 2 | 5 | — |
| version-query | procedure | 5 | 2 | 1 | 3 | 5 | — |
| fan-curve | interpolation | 5 | 4 | 1 | 4 | 5 | — |
| vending-controller | statechart | 5 | 5 | 5 | 5 | 5 | not judged |

What the prose decided came out alike; what moved was names and which
open questions each draft chose to mark. The vending case leaves its
interface open, and each draft invented a different one.

`--profile` runs the same cases under an owner's authoring profile, and
`--requirements <dir>` hands every draft of a case the SAME requirement list
(`<case id>.manifest_text.json` and `.sidecar_text.json`, made once by
`scxml_requirement_set`). Each draft is then measured against that list by the
product's own `requirements` records, so the drafts of a case share a
denominator instead of counting against ids their authors made up (twelve of
fifteen drafts did, when left to number their own). Measured on 2026-10-01,
five drafts per case, one run, a profile that closes the interface and requires
every state and transition to claim a requirement, and the lists built from the
prose with their quotes checked verbatim:

| Case | Requirements | Cited an id the list lacks | Requirement missing | Marked unresolved by the draft | Behaviour classes |
|---|---|---|---|---|---|
| door-with-auto-close | 6 | 0 of 5 | 0 of 5 | none | 2 |
| connection-keeper | 7 | 0 of 5 | 0 of 5 | none | 1 |
| vending-controller | 12 | 0 of 5 | 0 of 5 | 8 marks on 4 requirements | not judged |

Every draft kept to the profile and none left a requirement of the list
unaccounted for: each was claimed by a part of the design or marked open by the
draft itself. ⚠ What this does not show. `implemented` means a state or
transition claims the id; the product does not check that the part does what the
sentence says, so a draft can claim a requirement and be wrong (no draft piled
ids on a node to satisfy the profile: at most three on one). The vending drafts
agree on the list and not on which requirements to call open (`R6` in three
drafts, `R10` and `R12` in two, `R7` in one), and that choice is the draft's.
The two door classes are two real behaviours, not spellings: an open request
while open restarts the 20 seconds in three drafts and is ignored in the other
two. ⚠ This table first said three classes and gave a reason for the third that
was wrong: it blamed the closed interface for making one draft declare its
timer's event as a driven input. The cause was the Python runtime's scheduler,
which remembered a cancelled `<send>` id and dropped the next send that reused
it, so a draft that cancelled its timer on exit lost the timer of its second
entry and a draft that did not cancel kept it (fixed 2026-10-01,
`backends/python/tests/scheduler/`). Every behaviour figure of the earlier arms
was recomputed under the repaired runtime from the saved drafts, and only this
one moved; the recomputation under the old runtime reproduced the figures as
first reported. The loose levels sit inside the band of the earlier runs with no
profile (logic 15 against 13 and 14, table 14 against 13 and 14, open 15 against
15 and 15, vocabulary 13 against 13 and 14); the door's behaviour column is the
one that moved (2 against 1 in all three earlier arms), and one run per arm
cannot say whether that is the profile or the sample. The three cases are the
ones the product was shaped on and the lists were written by the owner's side
before the drafts, so this is not a blind test.

#### When the specification is revised

A revised specification lapses an acceptance, and the next draft is written
afresh. Saying which requirements the revision touched needs an id to mean the
same requirement in both revisions. It used to not: `scxml_requirement_set`
numbered the quoted requirements in reading order and left `rev` at `1`, so a
sentence inserted anywhere but the end renumbered everything after it.
`eval/revision_identity.py` measures that without a model: it applies an owner's
edit to a specification and to its requirement list by rule and reports what
became of every id. Over three specifications and 16 edits, 32 of 127 ids named
a different requirement in the second revision with nothing to flag it.

The tool now hands back a **lineage** (`lineage_text`, kept beside the manifest and
the sidecar as `requirements.lineage.json`): which id was issued for which
requirement, by digest, with no word of the specification in it. Build a revision
by giving it back (`lineage`, or `lineage_text`) and the sidecar that came with it
(`previous_sidecar`): a requirement whose words are unchanged keeps its id, one
whose wording changed slightly keeps it too and is listed `changed`, one the
revision lost is retired and its id is never issued again, and `rev` follows the
text (a reflow is not a revision). A rewording too large to recognise can be
stated (`continues`, `{quote: id}`); a list made before lineages existed is
revised from its `previous_manifest` and `previous_sidecar`. The answer's `delta`
says which ids were carried, changed, new or retired and which sentences were
added or removed. On the same 16 edits that is 0 ids naming another requirement
unnoticed; the one thing it cannot do is tell a reworded requirement from a
replaced one when the new sentence is nearly the old, and then it lists it
`changed` so a person reads it. The figures, the constraints behind the design
and the steps it opens are in
`docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`. Not done: keeping
the lineage with a work in the application.

An acceptance record also keeps, per requirement, digests of the rows the report
showed the owner for it (`evidence`, with `unclaimed` for the rows that claim
nothing, and `succeeds` for the record it replaced: ADR 0007). **scxml_acceptance_delta**
sets that against the design as it is now: for each requirement `unchanged`,
`changed` (`moved`, the places in the design now of rows the record lacks, and
`gone`, how many recorded rows nothing matches), `new` (`at`) or `dropped`. Give it
the record, the root its paths are read against and, for another draft of the design,
`design`. It is a report and says what a second look should be about, never that a
requirement is still met: `unchanged` is the product's closure of what the requirement
depends on reading the same, and the acceptance still lapses by its bytes. A record
taken before records kept this evidence is refused. Local servers only
(`docs/adr/0008-an-acceptance-says-what-moved-since-it-was-taken.md`).

**Revising from the accepted design.** When the specification was revised and the
design's own files did not move, `scxml_accepted_for` answers `lapsed` and also hands
the accepted design back (`revise_from`: its document, its text and its page): revise
that, do not draft afresh. When a design file moved the accepted bytes are gone and no
base is offered. Build the requirement list again against its lineage
(`scxml_requirement_set` with `lineage` and `previous_sidecar`), change only what its
`delta` says moved (it names its specification, the revision it starts from and the digest
of the manifest of that list, and is refused unless all three are the acceptance record's
manifest's), then
**scxml_revision_check** joins, per requirement, what happened
to its words (carried, changed, new, retired) with what happened to the design's
evidence for it (`scxml_acceptance_delta`): `outside-reach` when a design moved where
the words did not (`moved-without-reason`) or still cites what the specification dropped
(`retired-still-cited`), `within-reach` otherwise, with every place a second look should
go (`look`). A requirement whose words did not change but whose evidence moved ONLY at
places a changed or new requirement also stands on (several requirements cite one node) is
a `look`, `moved-with-a-changed-neighbour`, and names that neighbour (`shared_with`): check
that it still holds there. A requirement that no node cites, before or now, was not compared: it is
`uncited`, counted apart (`summary.uncovered`) and never carried over, and when
`summary.seen` is 0 (no requirement has evidence, or the kind of document has nowhere to
cite one) the verdict says nothing about the design and the page says so.
**scxml_revision_report** renders the same join as the page the owner reads:
what carries over folded into one line, everything else listed with the places that
moved, and a requirement's sentence printed only when given the sidecar of the revised
list (the page then says it carries someone else's sentences). `within-reach` is not
"right": it says the design's changes are accounted for by changes in the words, on the
product's closure of what a requirement depends on, and the scenarios of an unchanged
requirement are still played with `scxml_scenarios`. Local servers only
(`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`).

#### Examples a design is played against

A requirement met by something NOT happening ("nothing is sent after the
response") has no node to point at, so `scxml_requirements` can only call it
`needs-scenario`. **scxml_scenarios** (the MCP tool) is the evidence column that
outcome asks for. The client writes a scenario set from the specification, the
owner confirms it, and the tool plays it into the design and says whether the
design behaved as the examples say. The set is `schemas/sce-scenario-set.v1.schema.json`:
each example is anchored to a sentence of the specification word for word (give
`specification` and every quote is checked), names its inputs and outputs in an
`interface` the owner accepts with it, and says after every step what must be
observed, including what must NOT be sent. An example the specification leaves
open is `awaiting-decision`, and one that waits on a fact nobody has decided is
`blocked`; neither is run.

⚠ The tool is reached through the check the client already makes. Measured
2026-10-02 (Sonnet, only this server's tools, three runs of "make pseudocode
from this specification"): none called `scxml_scenarios`, and the same three
runs asked also to check that the design behaves as the specification says all
did. A step the instructions ask for in a second call is skipped when the owner
did not name it, so an accepted statechart's `validate_scxml` (and
`validate_scxml_set`) answer carries `behaviour: {"verdict": "not played"}` and
a sentence in `show` that says the design was checked and not run, and what to
do before calling it finished. The field and the sentence are written together
in `_behaviour_say`, and a document accepted as another kind, which has nothing
to play, carries neither.

Given the owner's requirement list (`manifest`), `scxml_scenarios` also says what
the examples make of each requirement. A scenario names the requirements it is
about (`requirements`); a `shall_not` that no node can show, whose every scenario
passed, is `scenario-passed`, and a requirement with a failed scenario is
`scenario-failed`. The tool types none of it: it writes the driver's trace beside
the design and the product holds the list against the set
(`sce-codegen requirements --scenarios --trace`, which judges the set itself from
the trace), so the answer's `requirements` is the product's records passed through,
with a `scenario-evidence` record naming the set's digest and `origin` and the
engine. ⚠ `scenario-passed` is not `implemented`: it says these examples passed on
that engine, and `says` in the answer tells the client so, and that the examples are
the author's (`ai-proposed`) until the owner confirms them.

The product does two things and the authoring package one. `sce-codegen
scenarios` says whether a set is usable, and nothing is run from one that is not.
`sce-codegen judge-scenarios` turns a set and an observation trace into
`pass`, `fail`, `not-judged`, `blocked` or `awaiting-decision` per example, with
each failed check and each gap. `sce_author/scenario_driver.py` is the one driver
written so far: it generates the design to Python (the lowering `verify` and
`compare` drive), plays each example in virtual time, and writes the trace,
naming itself `Python lowering`, because a verdict is about an engine. The
verdicts are the product's, passed through. A second driver on another engine
needs only to write the same trace.

⚠ A design is code, and it is never played in the process that serves the
client. Each example is played by `sce_author/scenario_play.py` in a child
process of its own, started by `sce_author/process.py` (the one place this
package starts a program, with its launcher `_bound.py`;
`test_core_is_domain_free` holds it to those two). The child
gets a session of its own, the kernel's limits on processor time, memory, output
size and open files, no core dump, no new privileges, a scrubbed environment and
a clock outside it, and the server process never imports the engine or the Lua
binding. A hostile design costs exactly the one example it was playing.
Measured 2026-10-01 against the tool as a client calls it, before and after:

| Design | In the server's process | In a supervised child |
|---|---|---|
| Re-sends itself at zero delay | never returned | `not-judged`, cause `environment`, "had not finished" |
| Endless `<script>` loop | never returned; a 3 s `SIGALRM` handler in the same process never ran, because the loop is inside Lua's C code | `not-judged`, cause `environment`, stopped by processor time |
| Doubles a string for ever | took the memory the host had | `not-judged`, stopped by the memory limit or by an error nobody answered |
| Nine fine examples and a tenth that loops | the whole call hung | nine `pass`, the tenth `not-judged` |
| Cyclic `<raise>`, deep recursion, a timer every millisecond | ended, in the engine or the driver's bound | the same, cause `design` |

⚠ The first row changed on 2026-10-02. A design that sends itself an external event
and answers it by sending it again ends every macrostep, so the microstep ceiling
never applied, and the engine call never returned: the processor-time limit stopped
it after 25.5 s and called it the machine's doing, when a design that does this does
it on every host. The engine now takes at most `max_external_events_per_call`
external events in one invocation of its main event loop (10,000 unless the host
chooses another; a `send_event` is one invocation, and `advance_time` runs one per
due scheduled entry), hands the call back with the rest still queued, and counts it
(`truncated_event_chains`, with `last_truncated_event`). The driver reads that
through `lowering.stopped_run`, so the same design is `not-judged`, cause `design`,
in about 3 s, naming the event it was still taking. It is a ceiling this engine
chooses and not a rule of W3C SCXML, as the microstep ceiling is. The contract the
other runtimes are held to is ARCHITECTURE.md "External-Event Budget"; none of them
has the ceiling yet.

A refusal carries a `cause`. `design` means what the design did made the example
unplayable (an error no state answered, an open route, a macrostep the engine
cut short, a chain of external events the engine handed back, a design that would
not start) and another machine would refuse it too. `environment` means the machine that ran it stopped it (time, memory,
output, a crash) and another machine may play it to the end, so a second run may
differ; the answer says so. `decision` means the design leaves open a question the
example needs answered, and the example is `blocked` by it, named, rather than
failed: a design that sends to its parent while the specification never says who
the caller is is played only when the interface routes an output through
`#_parent`, and the driver never makes a parent up. Every reply states the `isolation` the run really had
(`process+rlimit` on Linux, and a plain `process` where the kernel does not
enforce the limits) and the `limits` it was bounded by. What this does NOT give
is a namespace, a cgroup or a seccomp filter: those are further layers that a
host may not offer (a namespace sandbox needs unprivileged user namespaces, which
AppArmor forbids on a current Ubuntu, so `bwrap` and `unshare -Urn` fail there),
and a deployment that needs one will ask for it by name and be refused where it
is absent. A memory-safety defect in the Lua binding is contained by none of what
is here.

`verify` and `compare` hold to the same rule, and the way they do it differs
because they drive one machine through many small steps, where a scenario is one
process per example. The generated module is imported by a worker process
(`sce_author/worker.py`), one per verification and one per draft, under the same
limits and a clock for every exchange; the server holds only references to what
the worker keeps (`sce_author/sandbox.py`). Plain values are copied, a dataclass
is copied field by field, and anything else (an engine, an enumeration member)
stays where it is and is named by a number. The wire is JSON and nothing else,
never `pickle`: a child that runs a hostile design must not be able to hand the
server anything but a wrong value. Three consequences are worth knowing.

- A worker that is stopped is not asked again. `verify` ends the run with a
  refusal that says the machine stopped and that a second run may differ;
  `compare` leaves that one draft out (`undriven`, with the reason) and compares
  the rest, and says "not judged" if the stop came while two drafts were being set
  side by side.
- `compare` runs a whole drive inside the worker and asks once (`procedures.trace`).
  Asking per step cost 247 requests and 208 ms for a drive that takes 12 ms in
  process, because each request wakes two processes; one request per drive costs
  20 ms.
- Children are started through `sce_author/_bound.py`, which applies the limits to
  itself and then runs the program, not through `preexec_fn`, which Python
  documents as unsafe when threads are running. On Linux a limit that cannot be
  applied is a refusal to start, never a child that runs unbound.

`compare` plays by the driver's rules, not rules of its own. Time moves from one
deadline to the next, and a draft whose engine stopped a macrostep that would not
end (W3C SCXML 3.13), handed a call back from a chain of external events that would
not end, or whose time would stop at more than 50,000 instants, is
left out of the classes (`undriven`, with the engine's words and no suggestion that
a bigger machine would play it) instead of being classed. Measured 2026-10-02 by an
outside review, a cyclic eventless draft was classed with one that waits, and two
retry drafts that pass the same examples were told apart because `compare` moved
600 ms in one jump. Both rules are written once, in `sce_author/lowering.py`, and a
scenario and a comparison both go through them. So does `verify`, which read a
record's `elapsed_ms` of 600 by jumping to it: a signal armed by two timers of 200 ms
read DARK there and failed a case it should pass, and the same behaviour as one timer
of 400 ms passed. The time to an observation is walked deadline to deadline there
too, in one exchange with the worker.

Whatever a child started is ended with it, however it ended: on its own, by a
limit, or by the clock. The group is signalled while the child is still uncollected,
because a collected child's pid belongs to nobody (`process._Leader`). Measured the
same day, a child that exited normally had left a process behind, and so had a
session whose worker ended before `close()`.

⚠ What the driver does not do is as much of the design as what it does. It
reports what it saw and fills no hole:

- An example is **refused**, not run, when the engine raised an `error.*` that no
  state answered (W3C SCXML 3.12.2). The usual cause is a send whose route is an
  open decision: the send fails, the entry block ends there (W3C SCXML 4.9), the
  timer after it is never armed, and the machine would then fail an example about
  timing it was never allowed to keep. The refusal names the open decisions the
  author wrote, and the product reports `not-judged`, never `fail`.
- An example is refused too when the engine stopped a macrostep that would not
  end (W3C SCXML 3.13): every other reading of such a machine says it is fine.
  Measured 2026-10-01 by an outside review, a cyclic eventless transition passed
  an example that says the machine waits in its state. So is one whose engine call
  was handed back from a chain of external events it was still taking (the machine
  sends itself an event and answers it by sending it again): the verdict is the
  design's, and names the event.
- The interface the examples were accepted with is held to what the design
  presents, in both directions, and the answer says where they part (`interface`).
  The product writes the design's side on its manifest (`surface`: the events a
  caller can deliver, the events it sends out of the session, its states and its
  data) and the driver copies it into the trace, so neither this package nor the
  judge re-derives what the analyzer already knows. Measured 2026-09-29, five
  drafts of one specification invented five interfaces, and an example that names a
  state the design calls something else used to FAIL, which reads as the design
  misbehaving when it is two names for one thing. It is `not-judged` now (cause
  `design`), and the reason lists the states the design has. Everything else is
  reported and moves no verdict: an output the design never sends still fails the
  example that expects it, because a design that leaves it out is a defect and not
  a spelling.
- Virtual time moves one scheduled instant at a time. The engine dates a timer
  from the end of the move that fires it, so 600 ms in one step and 200 ms three
  times were two runs of one machine (the retry machine passed the second and
  failed the first). The same time now passes the same way however an example
  splits it. A child session is played too: a statechart that starts one
  (`<invoke src>`) is handed over with the child among its documents (the first is
  the statechart, the others what it uses), every document is built beside it, and
  the parent imports the child's module by its bare name from that directory. The
  engine's next deadline counts each active child's own, because `advance_time`
  ticks the children by the same delta: before, a child that re-arms a timer was
  dated from the end of a long move (measured 2026-10-02: a child that speaks at
  400 ms by two timers of 200 had spoken at 600 ms in three moves and had not in
  one). A document the product refuses refuses the design, and the reason names it.
- An input is delivered under the name the example gives it. The generated
  engines carry an event as the descriptor the design declares, and W3C SCXML
  3.12.1 lets `request.new` match a transition on `request`, so a design reading
  `_event.name` used to be told `request`. The longer name now travels with the
  event, and every way of reading it sees it: a guard, a helper function, a
  variable, a computed key. (A first version scanned the design's text for
  `_event.name` and refused the example; an outside review showed a helper
  function and a computed key walk past a text scan, so the cause was fixed
  instead of detected.)
- An output the interface sends through a route (`via`) counts only when the
  design sent it through that route. A design that leaves by another door is not
  what the owner accepted, and counting its event would pass it.
- A data item the generated module has no reader for is a gap, with the
  generator's reason when it gave one: a C++ keyword, or a name equal to the
  document's own, which the C++ class takes (measured 2026-10-01: `credit` read
  nothing in `credit.scxml` and read fine in `counter.scxml`). An item declared
  with no initial value has neither a reader nor a reason, and the gap says
  exactly that. A send over BasicHTTP is refused: the driver does not observe it.
- A `pass` says the design behaved as these examples say, on this engine, over
  these inputs. It does not say the design is right, and the examples are the
  client's reading until the owner confirms them.

Measured 2026-10-01 on the retry client, with a set of six examples written from
the specification before any draft was played
(`sce-build/tests/fixtures/scenario_sets/`):

| Design | Result |
|---|---|
| A correct retry machine | 6 of 6 `pass` |
| The same machine retrying once too often | 5 `pass`, the count example `fail` at the step that should have timed out |
| The correct machine on the Python runtime BEFORE the scheduler repair | 5 `pass`, the count example `fail` |
| A GPT draft whose caller route was left open | 6 `not-judged`, 0 `fail`, the draft's own open routing decision quoted |

The third row is the reason the engine is in every verdict. The defect that made
two drafts of one door look like two behaviours (`backends/python/tests/scheduler/`)
would have shown here as a failed example, and as a failure ON the Python
lowering: a failure only one engine shows is a place to look at the engine first.
⚠ The set is not a blind test, and the machine of the first rows is written to
the specification by hand. What is measured is that a real run produces the
observations the judge was tested on, and that a draft that cannot be played is
told so instead of being failed.

#### The owner's decision record

What a draft does where the specification is silent is the difference
that matters most to an owner, and every draft meets those places on its
own. The **decision record** (`schema/decisions.v1.schema.json`) is a file
beside the specification where the owner answers each question once:

    {
      "record": "sce-decision-record",
      "v": 1,
      "specification": {"doc_id": "door-spec", "rev": "3"},
      "decisions": [
        {"id": "D1",
         "anchor": {"doc_id": "door-spec", "section": "1", "at": {"page": 2}},
         "question": "Does an open request while the door is open restart the 20 seconds?",
         "answer": "Yes, it restarts the full 20 seconds.",
         "candidates": ["true", "false"],
         "chosen": "true",
         "answered": "2026-09-29"},
        {"id": "D2",
         "question": "What does an open request do while the door is closing?"}
      ]
    }

A decision without an `answer` is a question asked and still open. The
`anchor` has the fields of the product's `spec_provenance` record — the
fields a document's `sce:provenance` is read into — and names the clause
the question is about; `candidates` and `chosen` are for an answer that is
a value. The `specification` names which specification the answers are
for; the acceptance record, not this, pins its files (`--source`).

A draft cites the record: `sce:assumed="D1"` where it applies an answer,
`sce:unresolved="D2"` where it asks a question the owner has not answered,
and a new id only for a question the record does not hold. **decisions**
(the MCP tool of that name, or `python3 -m sce_author decisions --document
draft.scxml --decisions decisions.json`) reads the draft's markers through
the product (`sce-codegen unresolved`) and holds them to the record. It
refuses a guess that cites no decision, a guess on a question not yet
answered (a draft may ask it, not guess it), a question the owner already
answered, and a decision variable — a `<data>` whose `expr` is the decided
value, with `sce:assumed-candidates` — holding a value other than `chosen`.
It reports, and never refuses, a new question the record has not seen,
shown beside every recorded question on the same clause, since whether two
sentences ask the same thing is the owner's reading; and an answer no
marker cites. `--codegen` names the product's generator and `--out` writes
the whole report as JSON. The record may quote the specification: keep it
where the specification is kept.

Connecting SCE supplies tools and usage instructions; it
does not supply the prose or decide policies absent from it. A host may choose
not to pass MCP server instructions to the AI, so confirm the tool calls in the
client's transcript.

With SCE MCP connected to an AI assistant, a specification owner can attach a
prose specification and ask, "Choose the SCE document kind that fits this
behavior, make a pseudocode draft, check it with SCE, and show anything the
specification leaves undecided." The
AI writes the SCXML draft and calls the tools; the owner does not provide an
SCXML path or a tool name. The server announces this workflow to MCP clients
in its initialization instructions. A client may choose whether to pass those
instructions to the AI, so this is guidance, not a substitute for reviewing
the pseudocode against the source.

An assistant that paraphrases `render_scxml_pseudocode`'s page instead of
showing it can hand the owner a summary that reads clean where the actual
page would not, or a line the source never stated inside what looks like
rendered output. The server's instructions ask the assistant to show that
text verbatim, in a fenced block, and to keep any note about what the
source leaves open outside it. A client that drops MCP server instructions
drops this guidance too, so confirm it the same way as the tool calls
themselves: read what the assistant actually showed against what the tool
actually returned.

The MCP server exposes tools that need no pack or binding:
`scxml_kinds` runs `sce-codegen kinds` and returns the kind catalog, whole or
for one kind; the rest take their files either by path, on the machine the
server runs on, or as text (`document_text` with `document_name`, and likewise
for a manifest). A file given as text is read under the name it was given, in
a directory that lasts for one call, so diagnostics name it and documents given
together import one another by name. Without `out`, `render_scxml_diagram`
returns each figure's SVG text. `validate_scxml_set` checks documents that
refer to one another as one set, such as a statechart and the event schemas
it imports, or a worker, its link and the link's codec. It passes each as
`sce-codegen check --document`, so the product reads each document's kind. Its
answer carries `open` and `next` like `validate_scxml`'s: what one member of the
set leaves open is what the set leaves open. The design-time lints already run
on every statechart of a set, reporting the first finding; `sce-codegen check
--lint --document …` reports every one.
`validate_scxml` runs `sce-codegen check --lint --error-format=json` and returns
every diagnostic record with the verdict and manifest, as JSON. When the product
ACCEPTS, the pseudocode page of the same document follows the JSON as its own
raw block, and `pages` names its sha256 and which block is whose;
`validate_scxml_set` does the same with one page per document, in the order
given. Why on the check and not a second call: measured 2026-09-30, fifteen
drafts by a real client asked for "a pseudocode draft, checked with SCE" showed
a page in one, and skipped the rendering tool fourteen times of fifteen (saying
it needed a pack, which it does not). A page that comes with the verdict is
there every time the design is checked and is the page of the bytes checked; the
digest is taken before the check and again after the page, and a document that
changed in between gets no page. A refused document has none.
`validate_scxml` also takes the owner's requirement list (`manifest`, or
`manifest_text`): when the product accepts the document, the answer carries
`requirements` -- the product's own `requirements --manifest` records passed
through whole, a count per outcome, and the ids by outcome (everything but
`implemented`). The list is the owner's, made once by `scxml_requirement_set`
from quotes they can read against their own words and kept beside the
specification, so every draft of it is measured against the same R1..Rn: where
the list a client builds for itself agrees in content and not in where a clause is
cut, and a client left to number its own `sce:req` made the ids up in twelve of
fifteen drafts (2026-10-01). It is data and not a verdict; `dangling` is an id the
document cites that the list does not hold, `needs-scenario` a requirement met by
something not happening, and `denominator` says whether the list is the
specification's own or a reading of it (`synthesized`). A list the product cannot
read is said as that and the check stands.
`validate_scxml_set` takes the same list, and that is the call a statechart that
closes its interface goes through, since it is checked with the event schemas it
imports. The documents are measured as ONE design -- the product pools their
claims, so a schema that claims nothing does not make every requirement `missing`
-- and each node path names its document (`draft.scxml#states.idle`). Until
2026-10-01 only `validate_scxml` took the list, and a design with companion files
could not be given it in the check it had to make.
Both checks say what to tell the owner about requirements, in `show`, and set
`requirements` to match: with no list, `{"verdict": "not measured"}` and a
sentence that a requirement table the client writes is its own reading, with how
to have SCE measure it; with a measured list, a sentence that `implemented` means
a state or transition carries the id and SCE has not checked that it does what the
sentence says. Measured 2026-10-01 (Sonnet, four cases, five runs each, the owner
asking for the requirements as verbatim quotes and where each is reflected, no
profile and no list): before the sentences, `scxml_requirement_set` was called in 12
of 20 runs, and none of the 13 whose check was measured said what `implemented`
does not show; after, it was called in 20 of 20 and 12 of the 17 measured runs said
it. The remaining five reported the outcomes with no such word. One run per arm,
the arms run at different times, and the labels read by a person from the final
reply. GPT (Codex, reasoning effort none) gave a table with no word about its
source and called no requirement tool (2026-10-01): that client is not measured
here.
The list and its sidecar are the only durable record of what R1..Rn mean, and
no client kept them: none of the 20 Sonnet runs saved either file, and a later
GPT run saved the list and not the sidecar, after writing a list by hand as YAML
and being refused three times, because the `manifest` input said only "the
requirement manifest". The `next` of `scxml_requirement_set`, the `manifest`
input's description and the measured check's `show` now say what the file is
(the JSON the tool returned, unchanged) and to save `manifest_text` and
`sidecar_text` as `requirements.manifest.json` and `requirements.sidecar.json`
beside the design and tell the owner where they are. With the owner's request
unchanged (it never mentions the list), 19 of 20 runs saved both, every one
byte for byte what the tool returned, and the list reached a check in 20 (17
before); the one that did not said so and offered to. One run per arm. The
product's acceptance report, rendered from the saved pair, sets each
requirement's sentence beside the nodes that carry it; without the sidecar it
says `(no sidecar supplied)` under every requirement.
`render_scxml_pseudocode` runs `sce-codegen pseudo` and returns the review page
as its FIRST block and, as a second block, a note on what the page was rendered
from: the sha256 of the document, the product's check of that same document
alone (accepted, or refused with its first code), what it leaves open, and that
no owner acceptance is recorded. The check runs inside the same call on the
same bytes, and a document that changes while it runs gets no page. The digest
is a note about the page and never in it, because `compare` asks whether two
drafts render to the same page; the owner's `profile` is accepted so the check
is held to it. It answers for one document: a statechart that imports schemas is
checked with them by `validate_scxml_set`, and that answer is the one to quote;
`render_scxml_diagram` runs `sce-codegen diagram` and writes one print figure
(SVG) per container of a statechart, or, for any other kind, that kind's own
picture and the table of every value the document states, refusing a figure
too large for the page rather than shrinking it; `scxml_unresolved` and `scxml_requirements` report the document's
`sce:unresolved` markers and its requirements (their outcomes, given the
manifest). `scxml_requirement_set` makes that manifest, and the sentences
sidecar beside it, from the words of the specification a client quotes as stating
each requirement: it refuses a quote that is not in the specification word for
word or that points at more than one place, gives the ids from where the quotes
sit in reading order, and lists the sentences no requirement quotes. It is the
deterministic half of reading a specification; the list is still the client's
reading (`ids: synthesized`, and the answer says so). Measured 2026-10-01, Sonnet,
three specifications, five independent extractions each: every quote was in the
source (35 of 35), and the lists agreed in content -- identical for one
specification, five of six requirements on all five runs for another (one
clause quoted whole or in two), eleven of twelve for the third -- where the
drafts of that third specification fell into five classes at every level. With
the ids on the elements (`sce:req`), `scxml_acceptance_report` puts each
sentence beside what the design does for it, and lists the requirements no
element carries and the elements no requirement asked for;
`scxml_acceptance_report` renders the page the owner reads before
accepting; `scxml_accept` records that acceptance and `scxml_acceptance_check`
asks whether it still holds. Each JSON answer carries `verdict`, the command's
output, and every `diagnostics` record. `scxml_accept` states a person's
decision: call it only on the owner's word, after they have read the report.

`accepted` from `validate_scxml` is the product's verdict, and a document can
be accepted without being finished: a draft that left a count
`sce:unresolved`, and one that sent an output to `#_parent` with nothing that
invokes it, are both accepted. So the answer also carries
`open` — one line for each thing the run leaves to a person — and `next`,
which says to settle them before the design is shown as finished. A run that
leaves nothing has neither field.

The lines are the product's, not this server's: `sce-codegen check` and
`generate` publish them as the manifest's `open` (a question the
specification leaves open, a value chosen without it, event-schemas imported
by a statechart that does not declare its interface closed, a parent the
machine needs, a processor the host must serve), the acceptance report prints the
same words at the top of its block B, and the acceptance record keeps them as
`open_at_acceptance`, so what an owner accepted with is written down. This
server relays them and adds no sentence of its own. `--strict-unresolved`
remains the way to refuse a document with an open question outright, and an
acceptance is not refused for one: accepting with a question open is the
owner's decision, and the record says that they did.

The record also says which surface stated the acceptance (`channel`). This
server's `scxml_accept` is a client's door: what it records is that a client
reported the owner accepted, in a conversation the product did not see, so it
states `relayed` and never `direct`. The workbench application, which the owner
presses a button on, is the surface that states `direct`. Either is the caller's
word, recorded and never verified, and a record from before the field says
nothing.

A statechart's output needs a receiver. A `<send>` with no `target` and no
`type` goes to the machine's own queue, and one no transition takes is thrown
away (`check --lint`: `scxml/self-send-discarded`). What the machine tells its
surroundings goes to a processor the host serves (`<send type="…">`), and
`target="#_parent"` is right only when the specification names the statechart
that invokes this one. Which receiver a specification means is the owner's to
say; the server's instructions ask the assistant to ask, not to choose.

An acceptance can also pin what the design was **authored from**: the
specification files (`sources`) and the owner's decision record
(`decisions`), each by sha256 (`sce-codegen accept --source … --decisions
…`). A revised specification then lapses the acceptance as an edited
document does, saying which of the two moved. And `scxml_accepted_for`,
asked with the same record, a specification and a decision record before
anything is written, answers `accepted-design` — the document, its text and
its page — when the record still holds and the design was authored from
exactly those files, compared by content (`acceptance-check --source …`), so
the owner's copy of the prose under another name still matches. This is the
only way a second request for the same specification gets the same
document: the model that drafts runs in the owner's client, and two of its
drafts are never the same file (see "When the same specification is drafted
more than once" above). A role left out of the question is part of the
answer — the same prose without the decision record the design followed is
a different set of inputs.

A new draft cannot be the same file, but it can be held to the same
boundary. A statechart whose root says `sce:interface="closed"` takes from
outside and sends outside only the events its imported event-schemas
declare, and every event it sends itself has to be taken by one of its
transitions (`docs/SCE_ACCEPTED_SUBSET.md` §2.16); anything else is refused
as `scxml/undeclared-interface-event`. Without the declaration an import
does not close anything — an event no schema declares is still accepted.
The server's instructions ask the assistant to write the event-schemas
first and close the interface: that is the choice the vending case above
left open, and each of its drafts made it differently.

An event that carries no data is declared like any other, so the interface
can stay closed: its schema says `<datamodel sce:payload="none"/>` when the
specification says the event carries nothing, and marks the `<datamodel>`
`sce:unresolved`, with the reason, when the specification does not say. A
schema with neither and no field is refused, since an empty `<datamodel>`
reads the same as a field the author forgot. The second form is an open
question like any other: the check accepts the design and its answer says, in
`open`, that the payload is still unsettled, and the strict check refuses it
until the owner has decided. `sce:assumed` on the same `<datamodel>` is
accepted for a payload the author chose without the specification: it is an
assumed value the owner confirms or corrects, and it does not block the
strict check. A statechart that imports event-schemas but does not declare its
interface closed is accepted too, and its answer says in `open` that the
boundary the schemas describe is not held. A fieldless schema changes nothing about how the
machine runs — the machine generated for it is the one generated with no
schema — and no field can be read from or sent with such an event
(`docs/SCE_ACCEPTED_SUBSET.md`, EventSchema kind, "Fieldless schema").

Whether a statechart that imports schemas and leaves its interface open is a
mistake or a choice is the owner's to say, and the product does not guess it
from any feature of the document: a statechart with no schema is a W3C
conformance document, a legacy machine, or a design about to be shown to an
owner, and nothing in it says which. The owner says it in an **authoring
profile**, a small JSON file kept beside the specification
(`{"record": "sce-authoring-profile", "v": 1, "name": "owner-review",
"interface": "closed"}`; `docs/SCE_ACCEPTED_SUBSET.md` §2.17). Handed to
`validate_scxml`, `validate_scxml_set`, `scxml_accept`,
`scxml_acceptance_check` and `scxml_accepted_for` as `profile` or
`profile_text`, it holds the design to what it says: a statechart that is valid
and is not what the profile asks for is refused, every statechart of a set is
judged and every departure listed, and an accepted manifest names the profile
by digest (`profile`: `name`, `sha256`, and `judged`, the number of documents
it was applied to — zero when no setting reaches any document of the run).
Without a
profile the tools hold the design to nothing the owner asked for, and the
manifest carries no `profile`: that absence is how "checked under none" reads.

What a profile can hold a design to is what a machine can decide from the
text. `interface: "closed"` asks that the statechart declare its interface
closed (`profile/interface-not-closed`). `evidence: "anchored"` asks that every
`<sce:evidence>` say where the specification states it
(`profile/evidence-unanchored`). `names` asks how the names the document
DEFINES are spelled, by class — the document's `name`, its state ids, its
events and its data ids — as a style (`snake`, `upper_snake`, `camel`,
`pascal`, `kebab`, `lower`), a maximum length, forbidden words and a required
prefix, and for events the number of dot-separated tokens, the tokens a name
may begin with, and that no event name is a token prefix of another
(`profile/name-style`, `profile/name-limit`, `profile/event-structure`,
`profile/event-prefix-of-another`). The event-schema documents an interface is
made of are judged too — the event one declares and the ids of its fields — since
that is where a boundary event is spelled once; a statechart that takes the name
from the schema it imports is not judged for it, nor for the platform's own
`error.*` and `done.*`, which its author did not choose. `guidance` is a list of standing
instructions for whoever writes the draft, handed over as written and checked
by nothing: the manifest says how many there were (`profile.guidance`), so
`accepted` under a profile that has guidance is not read as though it had been
held to. Which term in the specification a name stands for is a reading of the
prose, and no setting here judges that: measured 2026-09-30, five drafts of one
door specification under a profile of spellings all kept to it, and their
words still parted (`door.open_request`, `request.open`, `hold.elapsed`), so a
naming rule makes drafts agree on how a name is written and not on which name.

`house_rules` are the owner's standing answers to gaps that recur (`{"id":
"H1", "rule": "An event a state does not mention is ignored."}`). A draft that
meets such a gap applies the rule instead of asking, and cites it with
`sce:assumed="H1"` on the element it applies to. The product lists every
citation apart from the values chosen without an answer (`open`, kind
`house-rule`; `house_rule` on the marker's record), and the `decisions` tool,
given the profile, does not refuse the citation as an uncited guess. What
nothing can see is a rule applied without its citation, which is the limit the
decision record has too. What the product can see it says: the check's answer
carries `house_rules: {held, cited}` and, when the profile holds rules and the
design cites none, a sentence saying so and how to cite. Measured 2026-10-02 (eight
headless runs, a profile of three rules): without the sentence all eight applied
the rules and cited none; with it all eight cited, each on the element the rule
applies to.

An acceptance pins the profile beside the specification and the decision
record, so a design accepted under one profile is not the answer for another
(`scxml_accepted_for` says the design was *held to* a different profile, and a
role left out is part of the answer), and `scxml_accept` refuses a statechart
that departs from the profile instead of recording the owner accepting it.
The scenario set whose examples closed a requirement is pinned the same way
(`scenarios` on the three acceptance tools, role `examples`): a set edited
afterwards lapses the acceptance. And the record keeps the text of each house rule
the design applied, with the places that cite it (`applied_rules`, which
`scxml_accept` returns so the owner is told what each rule said), so a profile that
changes later is reported rule by rule: `house rule H1, which the design applied at
3 places, now says "…"; it said "…" when the design was accepted`. A rule the design
never applied lapses the profile and is not named.
This server passes the file to the product untouched. Only the product reads a
profile, so a setting it adds needs no edit here, and a profile it cannot
fully read — an unknown setting, another version — is refused whole as
`cli/profile-unusable` rather than applied in part. The server's instructions
tell the assistant never to write a profile or to edit one to make a draft
pass: which boundary a design is held to is the owner's decision.

**scxml_house_rule** is the one way a rule gets into a profile without anyone
writing JSON. The owner says a standing rule in their own words; the assistant
brings those words with the rule it makes of them (`rules: [{quote, rule}]`, and
the owner's text as `owner_words` when it holds it, which every quote is then held
to word for word). The first call saves nothing: it returns `tell_the_owner`, the
rule beside the owner's own words. Only when the owner says yes to the rules as
worded does the assistant call it again with `owner_confirmed: true`, and the tool
returns the profile text with each new rule as
`{id, rule, quote, confirmation: "relayed"}`. On a local server `out` writes it:
the profile is written whole or not at all, the answer is `saved` with the path and
the sha256, a file already at `out` is replaced only when it was given as `profile`
and still holds exactly those bytes (an edit made since is never overwritten), and
asking again after success adds nothing, since the rule is then already held. The
profile's revision is its sha256, which every acceptance record that applied one of
its rules already pins. `relayed` is a client's report that
the owner said yes: the product was not in the conversation and says only that.
Ids are the lowest `H<n>` not in use unless one token is given; a rule that
repeats one the profile holds, a quote that is not in the owner's words, or an id
that names two rules is refused; and the profile it returns is read by the product
before it is offered. Every acceptance that applied a rule repeats the owner's
words and the word `relayed` in `applied_rules` and says, rule by rule, on what
authority it stands (`applied_rules_standing`). A rule written into the profile by
hand carries neither, and the answer says so of it.

**scxml_acceptance_impact** answers the question a shared profile raises: someone
edited a rule, which specifications applied it? Given the acceptance records
(`records`) and the directory their paths are read against (`root`), it rechecks each
for the variant it was taken for and answers `summary`, `records` (each with `holds`
and its `lapses` as the product's data beside the sentence) and `by_rule`: for every
rule that moved, the records that APPLIED it, with the places, the words it had and
the words it has (`current` is null when the rule is gone). A record accepted under
the profile that did not apply the rule still lapses (`source`: the file is other
bytes), is not indexed under any rule, and one accepted under no profile holds. A
record that cannot be read is `unusable`, counted apart, and the scan goes on.
Local servers only (it reads the owner's own tree); nothing is re-accepted. The
product's `sce-codegen acceptance-impact` is the same report as lines of JSON.

The requirement and acceptance tools answer for a document of any kind. A
statechart's report shows, for each requirement, the transitions it depends
on. A forge document's report shows the review-table lines that claim it,
and it is classified by the same implementation. A kind that carries
`sce:req` on none of its nodes yet (every kind but lookup today) says so at the
top of its report, so its `missing` rows are read as a property of the kind:
the owner judges those requirements against the pseudocode page, which covers
every kind. An acceptance record pins the document and every file it reads,
including what it imports. `render_scxml_diagram` draws a statechart as its
figures and any other kind as its own picture (a codec's byte layout, an
interpolation's curve, a buffer pool's slots, an observer's thresholds, a
timer's timeline, a lookup's table, a link's imports, a procedure's states)
beside the table of every value the document states, field by field; the
requirement checklist is a statechart's alone, since its rows name a
statechart's boxes, and for any other kind the requirements are in the
acceptance report. The picture is an aid and the table is the total reading
(every value of the document appears in it once, less where a node was
written in its file). An algorithm has no picture of its own: it is reviewed
on the pseudocode page, whose steps are the reading, and in its field table.
The review artefact follows the kind's shape.

**Works: the specification the owner keeps in the workbench application.**
`works_list`, `works_read`, `works_save_model`, `works_save_requirements` and
the three that take, finish and give up a generation (`works_begin_generation`,
`works_finish_generation`, `works_fail_generation`)
connect this server to the application (`app/`, `app-core/`). The owner writes the specification there and
asks an AI client to model it; the client reads the text from the work, writes
and checks the model as above, and saves it back, where the owner sees the
figures SCE draws of it beside their text. They do it by running `sce-work`,
the application's own command layer, and never by opening the folder: the
desktop application and this server both save, a save is a compare-and-swap on
the revision the writer read under a lock, and a second implementation of that
here would be a second definition of when two saves conflict. `sce-work` is
found as `SCE_WORK` names it, else `target/debug/sce-work` of the tree
(`cargo build -p sce-app-core --features cli --bin sce-work`), and a bundle
carries it when packaged with `--work PATH`. The works folder is the one the
application opens (`SCE_WORKS_DIR`, else the per-user data directory).

- `works_read` gives the text with its `revision` and the model with its
  `revision`, the text revision it was `written_for`, and its `standing`
  (`current`, `behind` or `unstated`: the command layer's own word, which the
  application shows the same way). A model of one document is its `text`; a model of
  several documents that name each other (a statechart and the event schemas it
  imports, which is how a closed interface is written) is its `entry` and its
  `documents`, each under the file name its imports know it by, and no `text`.
  It also gives `answers`, what the owner
  answered in the application to the questions the model left open (each
  question's id, their words, and when the words last changed), and
  `decisions_text`, **the decision record those answers make**
  (`decisions.compose_record`): every `sce:unresolved` question of the model in
  the model's own words, answered where the owner has answered, and every answer
  the owner gave to a question the current model no longer asks, because the
  next draft may cite it. The workbench keeps a plain map of ids to words, and
  this package writes the record from it, so the format of that record stays where
  it is read (`decisions.v1.schema.json`).
- `works_save_model` takes `model_text` (one document) or `documents_text` with an
  `entry_name` (several), never both. It runs the product's check first, exactly
  as `validate_scxml` does for one document and `validate_scxml_set` for several
  (and takes the owner's `profile` the same way): a model it refuses is not
  saved and comes back with the product's records, so a work's model chain holds
  documents SCE accepts. When the owner has answered anything, the draft is also
  held to those answers by the product's own `decisions` check, as the tool of
  that name runs it: a draft that leaves an answered question `sce:unresolved`
  (`answered-left-open`) or guesses with `sce:assumed` where no answer licenses it
  is not saved either, and what the check only REPORTS (a question nobody had
  asked, `new-question`) comes back as `decisions` for the client to put to the
  owner. A work the owner has answered nothing of is not held to a record that
  does not exist. A save from a `base` that is no longer the current model is a
  `conflict` and writes nothing. The client says which text it wrote the model
  from (`source_revision`); without it the application can only say that nobody
  recorded it.
- `works_save_requirements` takes the `manifest_text` and `sidecar_text` that
  `scxml_requirement_set` made from the work's text, unchanged, with the
  `source_revision` they were read from and the `base` (the `requirements.revision`
  `works_read` returned; none for a first list). The product's own loader runs
  first, against an empty statechart because loading a list does not look at a
  design: a manifest it refuses is not saved and comes back in its words, so a
  work's list chain holds lists the product loads. A stale `base` is a `conflict`.
  `works_read` gives the list back as `requirements`: `manifest_text` and
  `sidecar_text` byte for byte (the owner's acceptance pins their hash), the text
  revision it was `written_for`, and its `standing` against the text, so a list
  read from an earlier text is `behind` and the client is told to build it again.
- **A model can be asked for, and written for the request.** The owner asks for a
  model in the application (`works_read` gives `request`: `queued` means nobody
  has taken it). `works_begin_generation` takes that request, or makes one for a
  work nobody asked for, and answers with the work as `works_read` gives it and
  `generation` (`request`, `attempt`, and the `source` revision the request is about).
  `works_save_model` and `works_save_requirements` then take that `request`: what
  they save is written for the request and is not the work's model until
  `works_finish_generation`, which has SCE check the model itself and publishes the
  model and the requirement list together as one bundle, so the owner never sees a
  model of one draft beside a list of another. What the client ran that the core
  cannot (the decisions the draft was held to) is kept beside the core's check as
  the client reported it. A draft is written for the text its request is about:
  a `source_revision` that says another is refused. `works_fail_generation` says
  the model could not be written and why, and the owner is told in the application.
  - **This process keeps the claim alive.** A request is held for a lease that runs
    out, and a client that thinks for longer never says it is still there; the
    server renews the lease itself (`works.Generations`, a thread, every fifteen
    seconds of a sixty-second lease) for as long as it runs, so the owner reads
    the request as running, and as interrupted if the process dies. A request the
    owner called off, or whose text was saved, is refused at the client's next word
    (`request-ended`, then `generation-ended`), and nothing is renewed for it again.
    A request the application's own executor holds is refused (`request-held`).
  - **A work that has had a model published this way refuses the plain saves**
    (`bundled-work`): one half saved alone is the pair the bundle exists to keep
    from being read. A work that never asked for a generation is saved to as it
    always was. `works_read` reads the text, model, answers and list as one state of
    the work, and says the `bundle` they are the ones of.
- **The owner's acceptance is read and never written here.** `works_read` gives
  `acceptance` for a work that has a list and a model: `standing` is `none`,
  `holds`, or `lapsed`, with the product's own sentence of what moved (`lapse`,
  one sentence passed on whole: the product joins its lapses with `; ` and a
  single lapse can contain one, so it is not split), when (`accepted_at`), the
  `channel` it was stated on and what the design left `open` when the owner
  accepted. The owner accepts in the application, on the page SCE writes there;
  the command layer states every acceptance it records as the application's own
  (`direct`), so a tool that called it for a client would state, as the owner's
  press, what the owner never pressed, and none does. An acceptance that holds is
  the owner having said yes to THIS design, and `works_read` withdraws the
  instruction to write a new draft; one that lapsed is told as what it is, and the
  owner accepts again themselves. The product not answering (`sce-*`) is reported
  as `standing: unavailable` and does not stop a work being read.
- The owner's text and answers are the owner's: no tool writes either, and none
  removes a work. The works tools are refused to a remote caller, like a path:
  the folder is the machine this server runs on.

`tests/test_the_works_folder_is_reached_through_the_applications_own_command.py`
drives them against the real `sce-work` and generator, including a text of
three-byte scalars across both pipes; the authoring lane builds `sce-work` and
refuses to run without it, so there a skip is a broken build step.

None of these checks whether the SCXML agrees with the prose specification;
the specification owner compares the page and the figures with the prose. The
pack-based `check` and `pseudo` tools below serve the separate
interface-integration workflow.

    python3 -m sce_author check-pack --pack <dir>
    python3 -m sce_author brief     --pack <dir> --prose <file>...
    python3 -m sce_author questions --pack <dir> --prose <file>...
    python3 -m sce_author review    --pack <dir> --prose <file>...
    python3 -m sce_author check     --pack <dir> --binding <file>
    python3 -m sce_author coverage  --pack <dir> --binding <file>...
    python3 -m sce_author pseudo                 --binding <file>

**check-pack** lists everything wrong with a pack in one pass: a key written
twice, a file that does not fit its schema (every place it departs, not the
first), a phrase that is not an expression, a name no input declares, a regular
expression that does not compile, a replacement in `normalise` that names a group
its pattern lacks, a rule that reads an address or protocol the pack does not
declare, a rule that says what a binding's input rule cannot (an unknown key, a
`parameters` that is not an object) or compares an address with a symbol its value
space does not admit, and an examples file that contradicts itself. A rule is held
to the same facts the binding check holds a binding to (the binding schema's
definition of an input rule, and the interface model's value space, asked through
the same helpers), so a rule the pack accepts is not one `check` refuses later. Every other
command refuses at the first problem, so a pack with three mistakes took three
runs to learn about; a pack is prepared by the people who build it and handed to
a specification owner already verified, and they need all of them at once. It
reads the pack through the same loader the other commands use, so each sentence
is the one that loader would have refused with, and the first listed is the one
it refuses with. What could not be checked is printed as `not checked:`: a model
that did not load whole is not used to accuse the rules that read it. It exits 0
only for a pack with nothing wrong and nothing skipped.

**brief** assembles one page for whoever writes the document: the prose, the
addresses and value spaces it touches, the precondition vocabulary, and the
questions already known. It accepts any number of prose files and resolves
names across all of them, because one feature is frequently written across
several documents and a name introduced in one is used in another. `--out` is
where the page is written, and it is required: a brief is a file somebody
keeps open while writing, not something to watch scroll past once. Over MCP
the brief comes back whole only when it fits one tool result (60,000
characters); a larger one comes back as its section index with each
section's size, and `sections: [2, 4]` returns those sections. Section 1 is
the prose itself, which the caller already holds as files. Measured
2026-09-26: the largest specification in one corpus made a 227,761-character
brief, 72% of it section 1, and the client that asked refused a result that
size.

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
| `precondition-not-in-table` | the prose writes a precondition the pack's table has no reading for, so whoever writes the document decides what it means |
| `precondition-assumed` | the prose relies on a precondition the pack reads by assumption; the question says where, how often, and the pack's reason |
| `example-shows-memory` | two cases drive the same inputs and require different results, so the component remembers something the prose never states |
| `depends-on-another-component` | the examples drive addresses another specification in the system writes, so this document is one of several and cannot be judged alone |

The screen shows the count per class. `--out` writes every question itself, as
NDJSON, for a tool that acts on them one at a time.

**review** measures the PACK, which every other command trusts and nothing
else examines. No answer can be more right than the pack is, and the pack is
written by an adapter that reads a platform's own format -- which is not this
tree's format and, when the platform is under an agreement, cannot be
committed here at all. Testing somebody else's converter is not available; the
two ways a pack goes wrong are, and both are computed here: the share of the
document the partition attributes, how many blocks are one or two lines, how
many addresses the prose never writes under any spelling it is given, and how
many addresses the examples drive that the model does not declare -- the only
evidence inside a pack that its model is INCOMPLETE rather than small.

⚠ It reports figures and refuses a verdict. A pack is a claim about a platform
this tree does not have, so "correct" is not something it can be told. What it
does say out loud is the handful of shapes that cannot be right whatever the
platform turns out to be: no outputs, no examples, an address with neither a
value space nor a type, a partition owning none of the document.

⚠⚠ **A low share is not one of them, and the first version of this treated it
as one.** Measured over 129 subject packs: 13 attribute above 90% and 36 below
20%, and the low group is not broken — its median is ONE output in a 49-line
document, against 13 outputs in 655 lines for the high group. Most of a small
specification not being about its one output is what a specification looks
like, and an alarm 36 packs trip is a gate people learn to scroll past. What
cannot be right is owning NOTHING, which is a pack listing no spelling its
prose uses; this tree's own second subject matter did exactly that, and the
figures above come from the instrument that found it.

**scaffold** writes the half of a binding the interface model already decides
-- every position's address, field and value space -- and nothing that reads
the specification; see "Starting a binding from the model" below.

**check** judges a written document against the same model: every input it
declares must exist, every output it writes must be a real field, and every
literal it compares against must be in that field's value space.

**coverage** is the only command handed more than one binding, and therefore
the only one that can say anything about the SET. Every other command here is
given one document and is right about that document — so a conversion that
needed five components and produced three reports green. The three that exist
check, run and pass; the two nobody wrote are missing from no list, because
there was no list. A missing document has no binding, so no command runs
against it, and the failure looks exactly like success.

⚠ Its two figures are different in kind and the exit status tells them apart.
A position no document writes is a STATUS — unfinished work looks precisely
like that, and an alarm on it would fire on every conversion in progress. A
position TWO documents write cannot be right whatever the platform turns out
to be: one field would receive two answers, and which one stood would be
settled by whichever component happened to run last. Only that raises the
status.

⚠⚠ It is a third axis rather than a third spelling of one already here.
`review` reports the model positions no EXAMPLE expects, and `verify` reports
the positions one BINDING writes that no case expects. Both ask whether the
testing is complete; this asks whether the decomposition is.

**verify** RUNS the document against the pack's examples and says which cases
it fails and where. It is the only command that says whether a document
*behaves*: a document can satisfy `check` completely and compute the wrong
answer at every address, because `check` never executes anything. Until it
existed, the last step of the workflow was "and hope".

It needs nothing new. The examples carry values, so they are a test suite; the
binding says which address feeds which input and receives which output, so the
marshalling needs no subject knowledge; and the product already generates
runnable code from the document. The generator builds one document per run, so
`verify` builds every document the root imports beside it, and every import of
those -- the generated code expects each `<sce:import>` as a sibling. ⚠ It refuses rather than skipping -- a kind
whose generated shape it cannot drive, an input rule it cannot evaluate, an
expected address the binding never writes. A verifier that quietly skips what
it does not understand reports a clean run for a document it never executed.

⚠ A pass also says what it RESTS ON that no case can reach. Every precondition
the pack reads by assumption is printed beside the counts, with the pack's
reason. A precondition read as a constant is not in the document at all, so a
right reading and a wrong one pass identically -- the one kind of wrong a run
cannot catch, which is why the run has to name it.

**gaps** turns the same run around to face the SPECIFICATION. Every place an
author had to decide what the text did not say is already recorded —
`sce:assumed` in the document, `assumed` on a binding rule, `unresolved` where
nothing could even be guessed, and the pack's assumed preconditions — and the
run says, for each recorded guess, which of three things is true: **refuted**
(the product's tests answer it, and not as guessed — the case and both values
are given), **held** (the tests agree; the text should still say it) or
**untested** (no case compares a position resting on it). ⚠ The last is the
reason this is a command of its own: `verify` names a refuted guess beside its
failure, and an untested one reads in a verdict of "all passed" exactly like a
checked one. Every guess on the path to a position is credited, not only the
nearest — a document's value and the binding's symbol for it are two decisions.
⚠ Credit is not blame the same way round: a wrong value at a position several
guesses decide together says at least one of them is wrong, not each, so each
is **implicated** (naming the others) and only a guess that decides a failing
position alone is refuted. Blaming all of them turned one component's report
into thirteen refutations and nothing held.

`--counterfactual` then settles which of them the failures rest on, by
changing each binding guess a failure implicates and running the cases again:
every decision its rule makes that has a known set of alternatives
(`when_absent`, `equals`, `map` entries, `also` and `when` fields) is set to
each other value in turn. A guess no alternative moves — and every one was
tried — is **cleared**: the failures do not rest on it. One alternative that
repairs every failure the guess was blamed in and breaks nothing is the answer,
and the guess is reported refuted with it; two failures repaired by two
different values are not an answer — the document hands one value to two
situations the tests tell apart. A recorded guess left alone at a failure once
every other beside it is cleared is marked the only one left. ⚠ Why a run and
not a reading of the record: a guess about a missing input looks decidable by
whether the input was missing, and measured on thirteen components it never was
in the failing cases — but was in earlier rounds of the same run, in documents
that keep values between rounds, so only running the alternative says whether
that still matters.

A number the absence of which is guessed (`when_absent`) is tried at every
threshold the document compares that input against and one value on each side
— one value per interval, inside the `range` the interface model gives the
address. ⚠ That is EVERY behaviour only when the document does nothing with
the number but compare it with constants; a number that also feeds arithmetic
or `previous()` is tried at the same points as a sample, reported as one, and
never cleared.

A document's own guess is changed to each of its `sce:assumed-candidates` —
the values the decision could take under the specification, the current one
among them, space-separated. ⚠ The product accepts that attribute and reads
nothing from it; here it means the current value sits in exactly one place,
and each other candidate goes there. That place is one of: the whole `expr`
(a DECISION VARIABLE — a `<data>` whose expression is the decided value, which
the logic reads; the shape `brief` asks for, and for a choice between two
shapes of logic a `bool` the logic branches on, candidates `true false`), the
whole `sce:initial`, or one
occurrence inside a larger `expr`. `check` refuses a list whose place it
cannot find — a value a nested expression writes several times cannot be told
apart, and measured 2026-09-28 that was four of nine guesses in one document
— rather than guessing which of them the decision was. Trying every candidate
is exhaustive over the list, and the report says which one repairs the
failures; a list of the current value alone says the decision has no other.
`--max-runs` bounds the runs (one per alternative), and what it did not try is
listed.

⚠ `check` refuses an `sce:assumed` with no `sce:assumed-candidates`. A guess
that says nothing of its alternatives cannot be tried when a case fails on it,
so the report could only name it among the suspects. Asked while writing, one
author in five listed any (2026-09-28); asked after a failure, the author
learns which guesses the tests contradict. Requiring the list of every guess
avoids both. `gaps` carries the request for each guess still without one
(`ask`; `--ask-out` writes the requests alone, one per line), and the request
names the guess and nothing a case holds — so it can go back to the author of
a document written before this rule.

With `--prose` each gap is located in the text by file and line (never
quoted, so the report can travel further than the specification may) and the
text's own open questions are counted; `--out` writes every gap as NDJSON. A
run that could not happen reports nothing: without it every guess would read
as untested, which is a claim about cases nobody ran.

`--backend` picks which lowering is DRIVEN. The product emits six; this drives
the one it can import and refuses the rest, rather than reporting on a program
nobody started. `--codegen` names the generator, defaulting to the one built
in this tree.

**`--explain TEXT` shows what the document did in a case, not only whether it
was right.** A failure says what was expected and what was written at one
position; the reason is in the values between the inputs and that position, and
an author cannot see them from outside the document. Every case whose name
contains `TEXT` (repeat the flag for several) is printed with three parts: the
inputs the binding handed the document, every value it returned that round, and
what it kept afterwards. Measured 2026-10-04 while tracing one failing case by
hand: the specification and the document's expressions side by side did not say
why the wrong event was written, and running the document on that case did --
every event condition was false in that round, so the identifier stayed on the
first branch of a fallback chain. A document that keeps values (it reads
`previous()`) returns every variable it declares; a pure computation shows the
outputs its binding names and says it kept nothing. Asking changes no verdict,
costs nothing for the cases not asked about, and a `TEXT` matching no case shows
nothing and is not an error.

It drives two shapes, and they have nothing in common. A pure computation is
CALLED: one function per output, this round's inputs by name. A **statechart
is DRIVEN** -- the machine is built once, the cases are replayed through it in
the order they happened, and what it produces is read from the `<send>`s it
made. ⚠ Built ONCE is the part most easily got wrong: an examples file is one
run, and what a case observes is partly the result of the cases before it,
which is what states are for. Rebuilding between cases verifies a machine that
forgets, which is a different document. So `ordered` stops being optional
there, and a case that drives nothing this document listens for is reported as
unjudged rather than passed -- it would otherwise read whatever the case
before it left, and call that this case's answer.

⚠⚠ Driving one takes TWO builds, and it is the product's own handshake rather
than a way around one. A `<send type="x">` compiles to a runtime
`error.execution` until the build is told the host serves `x`; with
`--host-processor x` the same site compiles to a dispatch and the manifest's
cause for it disappears. The first build is how the types become known and the
second is how they become reachable. Measured without it: the machine took
every transition correctly, sent nothing anybody could receive, and every case
read the resting value -- a full run, judged, about a document nobody could
hear.

**A document with delays does half its work between the cases**, so virtual
time has to move. `elapsed_ms` is what moves it, read as what it says it is:
the AGE of the situation, and the situation is what the case just drove. So
the reading sits that far after the drive and **nothing is subtracted from
anything**. A delta between two cases would be wrong twice over -- the field
restarts whenever the situation does, and its own schema calls a record that
goes backwards ordinary rather than broken. One advance, however large: the
engine pops due entries one macrostep apart, so a long step does not step over
a deadline the document distinguishes, and choosing a step SIZE is the move
its runtime explicitly warns against.

**Two channels, asked in that order.** A send is preferred, and failing that
the driver reads a variable the document declares `sce:direction="out"` --
which is the document saying that variable is part of its outward surface. The
generator emits a host-facing accessor for every one, and `check` already
refuses a binding that leaves one uncovered, so reading it is reading the
declared interface rather than the insides. ⚠ Failing BOTH, the only place
left is the active configuration, and that stays unreadable: no state is
declared an output anywhere, so an assertion on one would break when a state
is renamed or split while the document went on doing exactly the same thing.
That is an assertion about the insides wearing the clothes of one about
behaviour, and it is refused instead.

**It drives ONE of the six backends the product emits, and the verdict says
which.** Python is the one whose generated form imports into this process and
whose runtime this process can reach, so the document becomes an object and
driving it is calling methods. Every other backend is a build and a separate
process, which needs three things nothing here has: a build step for that
language, a host program that stands the engine up and registers what a
verifier registers, and a wire carrying each case's inputs in and each reading
out. Naming one is refused, and the refusal says that rather than quietly
running Python instead.

⚠ The limitation is printed beside the counts because leaving it unsaid is
what costs. Most of this product ships as C++, and a reader handed `12 passed,
0 failed` finishes the sentence themselves about the thing they are about to
ship. ⚠⚠ The backend parity suite does not close that gap, though it looks as
though it should: it compares what the six emitters WRITE, byte for byte. How
they BEHAVE under these cases is a different claim, and nothing in this tree
makes it.

⚠ Two silences get refused rather than chosen. A reading with no `elapsed_ms`
taken while the machine is still waiting on a delayed act is dated to a moment
no record names. And a case that drove the same addresses to the same values
as the one before it is a real assertion -- the schema is explicit that a
restatement is not nothing happening -- but nothing says whether the age
beside it runs from this assertion or from the one that started the situation.
Those are different moments, and a delayed act can fall between them.

⚠ **A time known only as bounds is written as bounds.** `elapsed_ms:
{min, max}` is for a record whose harness waits "up to" a timeout, or whose
steps last as long as the platform takes: it knows a window, not an instant.
A SETUP step moves the engine to its window's earliest end and carries the
rest as *slack* — how far real time may be ahead of it — through every later
step until nothing is pending. A JUDGED step is read the way a harness that
waits "up to" a timeout reads: at the first moment in the window that meets
the case — the engine visits the window's start and then each deadline
inside it — and, failing every one, at the window's end. A case asserting
that an answer ARRIVES 2 s after the drive is therefore judged, not
withheld: the deadline it asserts is inside its window by design. Where a
deadline may or may not lie inside a window because of slack carried from
earlier steps, whether it fired is not known, so the run is given up from
there exactly as for an event that might have been sent — never judged on
one of two machines.
A setup step that moves nothing still passes its time. A `clock` input is
refused under a window rather than handed one end of it, and `when_absent`
does not apply: a window is a statement, not a silence. Measured on one
component: fourteen cases had no time at all and could not be judged, and a
single number chosen for them would have judged them against a moment no run
is known to have reached.

⚠ A computation reads a `clock` only on a host that runs it as time passes
(`activation: periodic`). On `on-change` the host runs it at the moment an
input changed — where the situation is 0 ms old, every time — so what it
answers later is computed in no round, and `check` and `verify` both refuse
the clock (`check.clock_refusals`); with no activation stated they refuse it
too. `verify` used to read the clock at each case's observation, a host that
computes again as time passes, and passed a document ("255 means 105% after
500 ms") the platform had no round to run that way (2026-09-28). Time on an
`on-change` platform is a statechart's delayed `<send>`, which the host
schedules.

⚠ **It also says what the cases never looked at.** "Every case passed" is a
statement about the cases, and a run that judged two of nine written positions
prints the same count as one that judged nine of nine. So the positions no
case expects are named beside the count, always -- pass or fail. Measured over
127 packs with examples: 3,272 of 3,593 output positions are expected by some
case, 101 packs expect every one of their own, 25 expect some, and **one
expects none**, whose cases pass while judging nothing at all. Without the
figure, that pack's run and a thorough one are the same line of output.

**pseudo** SHOWS the document the way a person reads it, and it is the only
command here whose judge is a human. Everything above answers a question a
machine can answer -- the names are real, the examples pass, the set reaches
every position -- and a document can satisfy all of them and still not be the
thing the specification owner asked for. Nothing in this tool can say so.

That judgement has to be a person's, and a person handed XML does not make
it. So the command asks the product for its review surface, which is total by
construction: every field of the model reaches the page, a value appears as
the author spelled it rather than as a number that happens to equal it, and a
document the surface cannot show in full is refused by name instead of
abbreviated. Approving the page is therefore approving the document and not a
summary of it. ⚠ Call it after `verify` passes -- a page that behaves wrongly
is not worth a reader's time.

    python3 -m sce_author pseudo --binding <file> [--deploy <file>]
                                 [--shape <name>] [--lexicon <name>]

⚠ It takes no `--pack`. Rendering consults the pack for nothing, and a caller
handed a refusal about their pack when they asked to read their document has
been told about the wrong file. It takes the **binding** rather than the
document so the file it shows is the file `check` and `verify` were given; a
page rendered from some other document on disk would read just as well, which
is exactly how an approval goes wrong. With `--deploy`, the lines the
deployment decides are shown too, each marked with a leading `!` -- strike
those and what is left is the undeployed page, byte for byte.

`--shape` picks how lines and nesting are written -- `indent`, the default,
nests by two spaces a level; `endmark` keeps that indentation and also closes
each block with the word that opened it. `--lexicon` picks what the grammar's
own words are called: `en`, the default, or `ko`. ⚠ Neither changes a thing
the document says. A shape may surround a value and may never alter one,
and a lexicon renames only the
words the grammar itself spends -- so a reviewer can read the page in their
own language and their approval is still an approval of the document.

⚠ A page written in any pair but the default begins with a line saying which
one, like `#!sce-pseudo shape=endmark lexicon=ko`. That is what lets an
approved page be filed and handed on: whoever picks it up months later has
the page and nothing else, and the page says how it is read rather than
leaving it to be guessed from how it looks.

⚠ Neither flag has a list of valid names in this tool, on purpose. Which
shapes and lexicons exist is the product's registry to answer, and a copy
here would refuse a name the product accepts the day one is registered. An
unknown name comes back as the product's own refusal, naming the real set.

`--codegen` names the product's code generator. Without it, the one built in
this tree is used -- derived from where this package sits rather than from a
constant, which is a home this package is not entitled to have.

⚠ It does not spawn the generator itself. Exactly one module in this core may
run another program, and a second caller gets a function there rather than a
place on the allowed list -- see "The boundary is tested, not asserted".

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
Each sheet is headed `--- sheet: <name>` and comes in the order the workbook
lists them (not the order of the file names it is stored under), a hidden one is
carried and said to be hidden: which sheet applies to a product is a question for
a person, and a sheet can only be asked about by its name.

**A picture on a cell is placed on its cell.** In a table of marks, whether a
cell holds a picture can be the whole datum -- the text says `-` or `O` and the
picture beside it says which row has an image -- so a reader that opened the cells
and skipped the drawing reported the table complete. A picture is placed
wherever a spreadsheet can put one: anchored on a cell (a drawing laid over the
grid) or held in it (the "place in cell" picture, whose stored text is only a
placeholder error). The cell reads `[picture]`, beside its text if it has any,
and `[N pictures]` when several sit there; the count and the cell count are said
once per workbook. Only WHERE is read. What a picture shows, and what the column
it sits in means, is for whoever can read it -- that is a fact about a product
and belongs to its pack or its owner, not here. What cannot be placed is counted
and said, never dropped: a floating or grouped picture, a chart or a shape, a
rich value that is not a picture, a drawing that cannot be parsed.

**An enclosed file is marked where it was attached.** The rows of an enclosed file
are carried below the body, under its name, which used to leave "the limits are in
the attached sheet" and the sheet it means apart: with several enclosed files
nothing said which one a clause handed its requirement to. Where an object sits
in the text the paragraph now says `[enclosed object: <name> (<kind>)]`, in a
table cell inside its row, by the name the file is carried under below -- so a
reader taking one slice of a long specification can tell which enclosed files
belong to it. An object that names no enclosed file (a picture of an equation, a
reference the document does not declare) is not marked.

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

      - address: Some.Stored.Value
        role: stored                              # read at start AND written back
        names: [WhatTheProseCallsIt]
        type: integer

      - address: Some.Event.Slot
        role: output
        announces_old_off: true                   # moves to a new event by publishing the old one off
        names: [WhatTheProseCallsIt]
        fields:
          ID:   {type: text}
          Stat: {values: {NONE: 0, OFF: 1, ON: 2}}

**`role: stored` and `announces_old_off` are facts of the platform's address, not of
a document.** A value the platform keeps and the component both reads and writes back
was one role too few: declared `input`, its write was an expected position no rule could
be asked to write; declared `output`, the planted value was an address the component was
not said to receive. `stored` is received like an input and written like an output.
`announces_old_off` is a fact of the COMPONENT -- of 244 sources 79 read the cached event
ID back and publish the old event off first -- that neither the specification nor a
document can say; the host reads it from the model.

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
        "mains connected":   # read by ASSUMPTION, with the reason
          expression: "true"
          assumed: "nothing on this platform runs without mains"
      pattern: '(?i)\bwhile (?P<phrase>(?:supply|mains) \w+)\b'

    gate_off:                # question 4, an ordered cascade
      - {when: has_symbol,     symbol: "OFF",  use: "OFF"}   # ⚠ quoted
      - {when: unique_suffix,  suffix: _OFF,   use: matched}
      - {when: binary,                         use: last}
      - {when: always,                         use: first}

`gate_off` is ordered and the first clause that applies wins. It is a cascade
rather than a set of rules because a generator has to choose one value; a rule
set that offers three candidates has not answered anything.

    companion_symbol:        # a symbol one field holds while another is in use
      - address_pattern: '\.Out\.Lamp$'
        field: Stat          # the field that holds the symbol
        companion: Aux       # the other field of the same output
        symbol: SPARE        # what `Stat` holds while `Aux` is in use
        companion_off: "OFF" # `Aux` is in use while it holds anything else
        measured: 125 of 136 cases while in use, 19 of 19 while off

A specification shows the companion where it shows the output and says nothing of
what the first field holds beside it, and a writer fills that silence with the
symbol the field's name suggests. The rule reads: the field holds `symbol` while
the companion holds anything but `companion_off`, and does not otherwise. The
brief says it beside the field as the pack's convention, never as the
specification's; no document is checked against it. The core does hold it to the
interface model, though: a symbol a value space does not admit is one no document
could write, so the pack is refused. A rule about an output that lacks either
field applies to nothing and is not an error.

    field_defaults:          # a value a field holds unless the specification says another
      - address_pattern: '\.Out\.Lamp$'
        fields: {Sound.Kind: REPEAT, Sound.Count: 1}
        measured: kind REPEAT in 1111 of 1329 cases, count 1 in 1109 of 1159

A specification shows an output and the condition that turns it on, and is silent
on a field that is the same nearly everywhere; a writer fills that silence with a
guess. The rule gives, per field, the value it holds unless the specification
states another, and the brief says it beside the field as the pack's convention
with its measured rate, never as the specification's. The exceptions are what the
specification has to say. As with `companion_symbol`, the core holds the rule to
the interface model (a symbol the field does not admit, or a number outside its
range, is refused) and checks no document against it; a rule about an output that
lacks the field applies to nothing.

A phrase whose reading is an assumption about the platform says so, with the
reason, in the object form. ⚠ A reading that names no input -- `true`,
`false` -- MUST: the pack is refused otherwise. Such a reading removes the
condition from the document, so no case can exercise it and a pass is silent
about it by construction. The reason written here is what `questions`, the
brief and `verify` repeat wherever the phrase reaches them. It is often the
right reading -- a condition that holds whenever anything is running has
nothing to observe -- and that is exactly why it has to be written down rather
than left to look like a fact.

A reading is an expression over the inputs the pack declares: names, `!`, `&&`,
`||` and brackets, and `true`/`false`. ⚠ The table used to be read as a bag of
identifiers, so anything else was ignored without a word: a misspelt input read
NO input and the condition the specification states was missing from every
check, and a stray `&&&` or `(` changed nothing at all. The loader now refuses
an expression outside that language, and a name that no `inputs` entry (of any
of the pack's files) declares, naming the phrase and the inputs there are.

A pack file is refused when a mapping writes a key twice. YAML and JSON both
keep the LAST value and say nothing, so `{ACTIVE: 1, ACTIVE: 0}` was `ACTIVE: 0`,
an entry that wrote `role` twice changed role, and a phrase written twice
changed its reading (reproduced for both formats). The refusal names the key and
the lines. The same reader holds a binding and the owner's decision record. The
interface model already refused an ADDRESS declared twice across entries; two
spellings of one phrase in one file are refused for the same reason, while a
later conventions file replacing an earlier one's reading stays the documented
layering. A precondition input's `rule` is held to the pack it is in: an
address the interface model does not declare, a protocol the conventions do not
declare, or a protocol parameter left out or pointing at an undeclared address
is refused when the pack is loaded, the same two facts `check` holds a binding
to.

⚠ What none of this can do is say that the pack is RIGHT. A pack that is
consistent with itself can still describe the platform wrongly; that is for
whoever produces it, and `review` reports figures about it without a verdict.

`pattern` is where this kind of document writes a precondition, with a named
group `phrase`. With it, `questions` looks up every precondition the prose
writes; without it the table cannot be matched against the prose, and
`questions` says so instead of answering empty.

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
          exclusive: [Ladder.Rung0, Ladder.Rung500, Ladder.Rung3500, Ladder.Off700]

Naming a protocol without defining it is still allowed and still means "the
core does not know what this is" — `verify` then declines rather than guessing.
⚠ Every field of `latch` was put there by a case that a shorter definition got
wrong: `cumulative` because reaching a later rung includes the earlier ones and
without it an input asking "has it been on at all" goes false the moment a
longer reading passes; `both` because the round where both move is ordinary and
all three answers occur; `last` because the record's own order says which was
more recent, which is what the idiom is named for; `exclusive` because a reading
that is in exactly one state at a time is taken away from by every other state,
and a latch over two parameters hears only one of them -- on a supply with an on
ladder and two off counters, the input for the on side stayed true when only the
second off counter moved, and the input for that counter stayed true when the
first one moved, five cases of one component.

`host` states what the platform's HOST — the code connecting a generated
document to it — does with a document, once for the platform rather than in
every binding:

    host:
      activation: on-change   # when it runs a document (the binding's `activation`)
      writes: every-round     # every position its rule writes, every round, changed or not

It is written once and every reader takes it from here: `check` and `verify`
use `activation` for a binding that leaves it out and refuse one that says
another; `scaffold` leaves it out of the binding it starts, so there is no copy
to drift; `brief` tells the author what both keys mean (its section 8); and a
host generated for the platform reads the same keys. ⚠ Before, only the
binding could say `activation`, and what a round writes reached the author as
a page written by hand beside the pack — two statements of one platform fact,
one invisible to every tool. The first time they parted, the page's "writes its
outputs at the end of every round" produced a document that announced a
delayed event's old value at 2 ms (2026-09-27).

`verify` models that host, and a case read as its first announcement
(`observed: first`) is judged on it, so such a case is refused where the pack
does not say. The first announcement is the first round from the drive on that
WRITES a position the case expects: a rule that answers when nothing was sent
writes every round, so for most outputs that is the drive's own round; a
`hold_last` rule handed a value its map lacks writes nothing, so its position
announces only in a round the document sends it a mapped value — a delayed act
included. A window nothing writes in is a wait the harness saw go unanswered,
and fails as one.

Where the pack states both — `activation: on-change` and `writes` — a round is
something `verify` knows happened or did not: a case whose drives reach no
input the binding READS (none of them bound, or each restating the value it
held) runs nothing on that host, announces nothing, and fails as a wait that
went unanswered, and a setup step like it moves nothing the document keeps. ⚠
A computation used to be computed for every case whatever it drove, so a
document writing a constant passed a case that changed only an input its
binding did not read — while the platform ran nothing and the test's wait
timed out (measured 2026-09-27). A drive this component does not receive at
all is still withheld rather than failed: its effect may arrive through a
component the document cannot see. Without the host stated, the run is as it
was.

⚠ That is how an output written only at certain moments is stated: in its
rule, not in the host. On one platform's original components about half the
outputs were written in the rounds that delivered an input they read, and the
rest by guards, transitions, timers and the grouping of their code — a timing
that differs output by output, which a single host-wide policy cannot say. A
document that sends such an output only at those moments, bound `hold_last`
with a `when_nothing_sent` its map leaves out, is read at the moment it wrote.
`every-round` is the only host policy listed because it is the only one
modelled; another is added when `verify` can model it and a platform's own
acceptance run agrees with the model.

⚠ `check` refuses the shape that gets this wrong in a statechart: an output
written in every round (its `when_nothing_sent` has a value) that is either
sent with a delay itself, or whose sends ask `In(…)` about — or sit inside — a
state a delayed event enters, following the events that round raises to a
fixpoint. The round that starts the wait announces the value from before it,
first. Measured 2026-09-28: two of three documents written for one component
from the same sentence ("popup on after a 2 s hold") had the shape and failed
the platform's test at 2 ms; the third, bound `hold_last` and sent only when
decided, passed. The difference was no reading of the specification, so it
was no recorded guess — nothing reported it until this rule.

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
      - name: the crossing clears once the train has passed
        before:                  # the setup, driven in order and never judged
          - given: {plant/in/approach: OCCUPIED, plant/in/power: OK}
            drove: [plant/in/approach]
        given:  {plant/in/approach: CLEAR, plant/in/power: OK}
        drove:  [plant/in/approach]
        expect: {plant/out/signal.value: DARK}

⚠ The four properties are separate because they answer separate questions, and
each of them cost a measurement to separate:

| property | what it lets the core do | what happens without it |
|---|---|---|
| values in `given`/`expect` | run the document at all | every check that compares two cases declines |
| `independent_cases` | compare two cases | the memory check declines rather than reading a delta as a whole input |
| `ordered` | replay something that carries state | a document reading `previous()`, protocols, and `previous_of`/`state_of` decline |
| `drove` | know what a case ASSERTED | a reading restated at the same value looks like nothing happening |
| `delivered` | know which drives the platform passed on as changes | the core compares `given` through the value space, which cannot see two raw readings one symbol names |

`delivered` lists which of a step's `drove` the platform delivered as a
change, where the record knows. Under `activation: on-change` it decides
whether a round happened at all, and the core otherwise decides it by
comparing values as the value space names them. ⚠ A platform may decide on
what it CARRIES: a warning signal went from one raw reading to another that
its table maps to the same ON, the platform delivered it and the component
ran, and the core — reading ON then ON — said no round happened (measured
2026-09-27). An address in `delivered` the step did not drive is refused.

A `before` step may carry `repeat: {every_ms, for_ms}` when the platform drove
it as a cycle — the same signals again every `every_ms` for `for_ms`. The step
is then driven `for_ms / every_ms` times (rounded down, at least once), each
round observed exactly `every_ms` after its drive, so a document that averages,
counts or integrates over its rounds is shown as many rounds as the cycle ran.
⚠ It is not one step with a long `elapsed_ms`: a reading restated at the same
value is still a statement that the thing happened again, and a cycle is that
said at a fixed rate. A step that carries both `repeat` and `elapsed_ms` is
refused, because each says when the step was observed.

⚠ `variant` and `elapsed_ms` sit on the case rather than in `given` for the
same reason: neither is a signal. Nothing drives them, they have no address and
no value space. A record tagged with a build and never setting a configuration
signal is ordinary — the tag IS the statement — and until the variant had a
place to live, no binding could read such a case at all.

`elapsed_ms` is a DURATION, not a moment on a timeline — it restarts whenever
the situation does, and a record where it goes backwards is ordinary. Time is
its own category: no address, no value space, nothing drives it, which is why
it sits on the case rather than in `given`.

`observed` says how the record READ its expectation: `any` (the default) when
the expectation held at some moment in `elapsed_ms` — a harness that collects
everything announced — or `first` when it is what the first announcement
after the drive said, a harness that returns at its first notification and
also checks when it came. ⚠ They differ exactly where a document announces
something before the answer a case waits for. Under `first` the reading is
the first round from the drive on that writes a position the case expects, as
the pack's `host` says a round writes (see `host`); an announcement before a
window that opens later fails on arrival as well as on value. Measured 2026-09-27 across thirteen documents run through a
platform's own acceptance tests: with the reading declared per case, `verify`
and the product agreed on 460 of 460 cases both judged, where one case had
disagreed while every case was read as `any`.

`before` is the setup of a case, as the record states it: steps driven in order
BEFORE the case, moving everything a round moves — a machine's state, a latch,
a remembered previous value — and never judged. ⚠ `given` holds only where a
case ENDED, so a record that says "from this, move to that" needs `before` to
say where it moved FROM. Folding the setup into the final values lost exactly
that: a document reading "A becomes B" literally was never shown the A and
failed, while a document reading it as "not B, then B" passed — and the loose
reading was the wrong one. A setup step that cannot be driven leaves its case
unjudged, naming the step.

### binding — the third artefact

Written per document rather than per pack, and named on the command line. It
says which address feeds which of the document's own inputs and receives which
of its outputs. ⚠ This is the artefact that did not exist before: a prose
specification decides, an interface model declares, and neither says which of
the model's addresses the document's names are.

    version: 1
    document: controller.scxml
    activation: on-change
    inputs:
      approaching: {address: plant/in/approach, equals: APPROACHING}
      supplyOn:    {unresolved: "the platform list is not available yet"}
      anyWarning:  {address: plant/in/state, equals_any: [WARN, FAULT]}
      notClear:    {address: plant/in/state, not_equals: CLEAR,
                    note: "a negation stays right when the enum grows"}
      silent:      {address: plant/in/state, absent: true}
      level:       {address: plant/in/level, range: [0, 255], when_absent: 0}
      caption:     {address: plant/in/caption, when_absent: ""}
      onMainline:  {variant_is: [MAINLINE, BRANCH]}
      sinceRise:   {clock: true, when_absent: 0}
      wasDown:     {previous_of: approaching,
                    caller_keeps: "the platform publishes no earlier reading
                                   and this kind cannot hold one"}
      lastShown:   {state_of: signal, initial: 0,
                    caller_keeps: "the loop that calls this already has the
                                   last answer in hand"}
      supplyOn:    {protocol: last-incremented,
                    parameters: {on_counter: plant/count/on,
                                 off_counter: plant/count/off}}
    outputs:
      signal:  {address: plant/out/signal, field: value,
                map: {0: "DARK", 1: "FLASHING"},
                when: {1: {blink: "ON"}}, also: {source: "LOCAL"}}
      reading: {address: plant/out/reading, field: value, passthrough: true}
      held:    {internal: true}

An address with fields is ONE record, and `when` writes some of them per
value. ⚠ Two values that land the same value in the rule's own field — two
ways an event shows ON — must write the same fields: a field one writes and
the other does not keeps, under the second, what the first left there, so the
same field tells two stories depending on which came before. `check` refuses
it and names what each value leaves; give every such value every field, the
platform's neutral value where one does not apply (`assumed` if the
specification does not give it). Measured 2026-09-27: an event's two variants
gave a sound's duration for one and not the other, and seven of the
platform's tests expect it written as 0 there. A value going OFF that writes
the identifier and not the sound is compared only with other ways of going
off: that one is the address's structure, as `hold_last` says it.

**`activation` says when the host runs the document** — once each time its
inputs change (`on-change`), or once per period whatever changed (`periodic`).
It decides what `previous(x)` means, the value one ACTIVATION ago, and an edge
detector means different things under the two, so it is a deployment fact:
the pack's `host.activation` says it for the whole platform, and a binding
that leaves it out takes that one (a binding that says another is refused).
A binding for a document that reads `previous()` is incomplete when neither
says it, and `check` and `verify` both refuse it. `verify` replays one activation per recorded case, which is the
on-change reading, so it also refuses `periodic`, which records of changes
cannot replay. A document that reads none needs no answer.

For a statechart the same key is the **delivery rule**: under `on-change` an
input's event reaches the machine only when the address's value changed, so a
record that rewrites a value it already held -- a setup step restating a
precondition, say -- delivers nothing, in setup and judged rounds alike. The
comparison is through the value space, so `0` and `CLEAR` are one value. A
judged case whose every drive is such a restatement reaches the document not
at all and is not judged. When the records restate a value and the binding
does not say `activation`, `check` reports it and `verify` refuses to replay:
delivering the restatement is a guess about the platform, and on 2026-09-23 it
revived a machine an earlier event had reset and passed a case the product
failed. Records that never restate need no answer.

**Memory belongs in the document.** A transform that needs the round before
says so itself — `previous(x)` is the value field `x` held one activation ago,
and `x` declares the `sce:initial` it holds before the first
(`docs/SCE_ACCEPTED_SUBSET.md` §3.4.1). `verify` then drives the document's own
holder, one activation per round, and the document alone is the component.
`check` refuses a transform whose BINDING reaches back instead, and the refusal
names the move: read `previous(<field>)` where the document read the remembered
input, give the field its `sce:initial`, and drop the input. `previous_of` and
`state_of` stay in this vocabulary only until the bindings that use them have
made that move.

**A rule naming a previous round says who keeps it.** `previous_of` and
`state_of` both require `caller_keeps`, and the string is the reason. Generated
code takes one round's inputs and returns, so a rule reaching back is an
obligation on whoever calls it — inherited by every caller, and until this key
written down nowhere. ⚠ It is required rather than optional because the
alternative is usually available and rarely considered: a document declaring a
memory-bearing kind can hold the value ITSELF, and then no caller owes
anything. The other honest answer is that the platform publishes the earlier
value at an address of its own, in which case the rule should name that address
and stop being a `previous_of` at all. Writing the reason is what makes an
author meet those two before defaulting to the third.

⚠⚠ `verify` reports the same obligation from the other end — it prints the
values the RUN held on the caller's behalf, because a pass that does not
mention them is a pass about a system nobody has agreed to build yet.

**One address to one name, and no form for comparing several at once.** A
platform's own rule format had two keys that group addresses; measured over
499 files of a real corpus they occur 4 times each, both in ONE of 28
hand-written bindings, against 490 uses of the single-address form in 106
files -- and the component that binding serves reaches a full pass without the
shape when its binding is written in this vocabulary instead. So what grouping
buys is brevity in one file, and what it costs is a dictionary that decides
things. Grouping is what the DOCUMENT is for.

#### Starting a binding from the model

Half of a binding is not a decision: which positions exist, their fields and
their value spaces are the interface model's, and copying them by hand is the
one part of the job a machine can do without being wrong. `scaffold` writes
that half, to a file that does not exist yet:

    python3 -m sce_author scaffold --pack <pack> --document controller.scxml \
        --out controller.binding.yaml --activation on-change

It writes `version` and `document`, one output rule per position the model
declares -- its `address`, its `field`, and a `map` keyed by the platform's own
numbers (`passthrough` for a field with no value space) -- and one input rule
per address, with the value space as a comment. For the crossing fixture the
output half is exactly the half of `controller.binding.yaml` above that is not a
decision. `activation` is a fact about the deployment: where the pack's `host`
says it, the binding takes it from there and gets a comment rather than a copy;
otherwise it is written only when it is given.

It writes nothing that reads the specification -- no comparison, no event, no
`when`, no `also` -- and a test holds that line: a tool that decided which
condition gives which value would be the translator this core refuses to be.
What is left is renaming each rule to the document's identifier, deleting what
the document does not use, and writing the decisions; the skeleton is always a
binding `check` can read, so each gap comes back as a refusal naming its rule.
⚠ Measured 2026-09-25: of four models set to write a document and its binding,
the ones that failed failed on the copied half -- a missing `version`, an
output rule with no `field` -- and never reached the half that reads the prose.

`--kind transform` also writes the document the binding names, beside it and
only if it does not exist: the root declaring `sce:kind="transform"` in SCE's
namespace, and one `<data>` per rule, with the rule's identifier, its direction
and a `sce:type` read off the model (`int64` for a value space or an
`integer`, `float64` for a `number`, which does not say it is whole). No
output has an `expr`:
the expression is the specification's reading, and `check` refuses each
transform output that still computes nothing, as the product would. Whether
a component is a transform is itself a reading, so there is no default, and
there is no statechart skeleton -- a statechart's shell is its states and
events, which the model does not know. ⚠ Measured 2026-09-26: a model holding
the binding skeleton wrote the document's shell wrong three rounds running --
no `sce:kind`, the `sce` prefix bound to a made-up namespace, `<if>` straight
inside a state -- and never reached the one condition its specification
states. `check` now also refuses a `sce` prefix bound to any other namespace
(every `sce:` attribute would be silently ignored), and, when a document never
declared its kind and nothing drives it, says it is being read as a statechart
and what a transform would be instead.

Two more answers reached a writer only through `verify`, which a writer who
is judged afterwards does not have. Measured 2026-09-26 on the next document
the same writer produced -- shell right, `check` silent, every case unjudged:

- An output whose `expr` yields only literals (`status == 1 ? 'E1' : ''`, at
  any depth) can produce exactly those, so its `map` must have an entry for
  each; `check` refuses one that does not and names any key that is none of
  them -- that writer had keyed every map by the INPUT's symbol.
- Given the specification (`check --prose <spec>`, or `prose` over MCP), every
  precondition it writes that the pack's table reads (`powered` -> `poweredUp`)
  must be read by the pair: a binding rule or a document input under that
  name. The writer had computed the output from the status alone and dropped
  the supply condition in the same table row. Reading the name is necessary,
  not sufficient; where it is read is `verify`'s to judge.
- A pack may give, beside a precondition input's note, the binding rule that
  reads it (`preconditions.inputs.<name>: {note, rule}`); the brief prints it
  and `check` holds the binding to it. A rule under the name that reads it any
  other way is refused, and the pack's reading under another name satisfies
  it -- so the name stops being the test. The next writer had bound a latch
  under the right name as a plain comparison of one of its counters, which a
  name-only check passed.
- A transform input that no expression reads is refused: it decides nothing,
  and `verify` still refuses a case that leaves it without a value.
- An input read as its address's own value, with no `when_absent`, is refused
  when the pack's cases leave that address unset -- `verify` refuses every
  such case rather than guess, and a writer judged afterwards never sees it.
  Which addresses each case sets is not an answer, so a pack for writers may
  carry its cases with the values (and the names, which can state them)
  withheld; this is asked of those too. Measured 2026-09-26: a 43-input
  document passed `check` with nothing said, and all 196 of its cases were
  then refused over eleven inputs the cases seldom set.

#### A statechart is driven, and answers by sending

Everything above describes a document that is READ: values go in, a value
comes out. A statechart is neither. It is driven by **events**, and what it
produces for anything outside itself leaves as a `<send>` to a host-served
processor — the W3C channel for reaching out of a machine (W3C SCXML 6.2), and
the one a real platform receives. Four keys say so:

    inputs:
      approaching: {address: plant/in/approach, becomes: APPROACHING,
                    event: approach.detected}
    outputs:
      signal: {address: plant/out/signal, field: value,
               sent: {processor: x-sce-host},
               when_nothing_sent: "signal.dark",
               map: {signal.dark: "DARK", signal.flashing: "FLASHING"}}

| key | what it says |
|---|---|
| `event` | the event to send the machine when this rule fires |
| `becomes` | the value the address must take first; omitted, any change drives it |
| `carries` | the field of the event's data that takes the address's VALUE (below) |
| `sent` | this output leaves as a send, optionally narrowed to one `processor`, its value taken from a `param` or the `content` and otherwise being the event name itself |
| `when_nothing_sent` | what the output reads as in a case where no matching send occurred |

⚠⚠ **An output reads the LAST send to its processor in the round, not every
send.** A machine that crosses two states in one round sends twice, and what the
position holds when the round ends is what the record describes, so the verifier
takes the last `<send>` whose `type` is the output's `processor` and reads the
`param` (or the `content`) from that one. A document that sends each value in a
send of its own to the SAME processor —
`<send type="x-sce-host" event="out.hour"><param name="hv"/></send>` followed by
`<send type="x-sce-host" event="out.min"><param name="mv"/></send>` — leaves
`hv` unread: the last send carries `mv` only, and the verifier answers
"reads the param `hv`, which this send carried 0 time(s)". Measured 2026-10-05:
all four documents a writer built under the static data model did this, and
nearly every case was left unjudged. Either put every value the round produces
in ONE send as `<param>`s, or give each group of outputs its own processor
(`x-sce-<name>`, which `generate` declares) and name it in `sent.processor`.

⚠⚠ **A `<parallel>` is never a transition's domain.** W3C SCXML 3.13 takes
the domain from the nearest ancestor that is a compound state or `<scxml>`, so
a transition written on one region — or from one region into another — exits
the whole `<parallel>` and enters it again, and every other region restarts at
its initial state. `check` refuses it and names the way out: `type="internal"`
when only the region should restart (the source is then the domain), the
region's own states otherwise, and the `<parallel>` itself as the target when
all of it is meant to restart. Measured 2026-09-23: a model-written document
reset four latches on any latch's ERROR and failed a shipped case, and the
reference document — which passed every shipped case — reset them on any
change of a fault input.

⚠⚠ **A statechart is handed the event, and a value only through `carries`.**
The driver reads `event`, `address`, `becomes`, `carries`, `absent` and
`when_absent` off an input rule (the last two are for a silent address, below),
so any other key on a statechart's input rule — `equals`, `protocol` — computes
a value no part of the machine receives, and `check` and `verify` both refuse it
in the same words. A `sce:direction="in"`
declaration in a statechart is refused for the same reason: nothing outside the
machine writes its datamodel, and the generated code offers the host a reader
for each variable and a writer for none. Measured 2026-09-22: these were
DROPPED rather than refused, and a machine that flashes above a level of 3,
driven at 5, stayed dark — reported as a correct document failing.

A value reaches a statechart as an event's data (`_event.data`), and
`carries: <field>` is the key that attaches it:

    inputs:
      hours: {address: plant/in/stored-hours,
              event: stored.hours, carries: value}

The document declares the field in an event-schema it imports, one per event,
whose `sce:event-name` is the rule's `event` (`sce-build/tests/fixtures/
static_datamodel/schema_day.scxml` is a complete one), and reads it as
`_event.data.value`. The value is read as the field's declared `sce:type` by the
reading a computation's input gets — a whole number into an `int`/`uint` field,
a number into a `float`, a truth value into a `bool`, a text into a `string` —
and `check` refuses a rule whose event has no such schema or field, or whose
address cannot be read as the field's type, in the words `verify` uses. An
enumeration field is not carried yet. The event is sent whenever the case drove
the address (and, with `becomes`, only at that value), so a stored value that
the platform hands the component every round reaches the machine when it
changes, not every round. Without `carries` a component that compares a level
and keeps nothing is written as a transform; with it, a statechart can count
time AND read a number.

A signal that times out is not a value its address takes: the platform flags it
and the callback runs with nothing to read. A statechart is told of the silence
in one of two ways, and `check` and `verify` read both:

    inputs:
      linkLost: {address: plant/in/link, event: link.lost, absent: true}
      level:    {address: plant/in/level, event: level.set,
                 carries: value, when_absent: 255}

`absent: true` sends the rule's event, bare, when the case leaves the address not
reporting (no value, or a token the conventions list as absence), and not
otherwise. While the binding reads the silence of an address this way, the other
rules on that address that read a value are not told of it: a silent address is
not any value. `when_absent` is for a rule that `carries`: it is the value the
event's data takes when the address is not reporting, read as a transform's
input would be. Without `carries` the event has no data to hold it, and
`absent: true` beside `becomes`, `carries` or `when_absent` has no value to
compare or hand over; `check` refuses both. Measured 2026-10-06: no binding
could satisfy `check` and `verify` for a case that drives a timeout at a
statechart, because `check` refused the key `verify` demanded, and 32 cases of
one component and the timeout case of two more were left unjudged.

An address named with nothing that compares it — `level`, `caption` — hands
the document that address's OWN value, read as the type the document declares
for the input and checked against what the interface model says the address
carries:

| document declares | address carries | the document receives |
|---|---|---|
| a number type (`int32`, `float64`, …) | a number, or an enumeration | the number (an enumeration's through the model) |
| `string` | a text | the text |
| `bool` | a truth value | the truth value |
| `enum:<alias>` | an enumeration, or a number | the number, which must be one of the imported enumeration's variants |

Any other pairing is refused by `check`, and by `verify` in the same words, and
a comparison (`equals`, `absent`, …) into an input not declared `bool` is
refused the same way. ⚠ There is no key saying "read this as a number": the
document and the model already say what it is, and `number: true` — which
said it a third time — fed an `int32` input in all 24 of its uses on one corpus
while nothing compared the two, and left a text input no way to be read at all.
An enumeration meets the platform on the NUMBER, so where both sides name a
value they must give it the same one.

⚠ `becomes` is not a spelling of `equals`. `equals` asks what an address IS
and answers every round; `becomes` asks what it CHANGED TO and answers once. A
machine told the same news every round is not being driven.

⚠ `when_nothing_sent` is REQUIRED beside `sent`, for the reason `when_absent`
is required for a number: there is no safe silent answer. A machine that
should have signalled and did not is the failure most worth catching, and an
output that merely vanishes from the produced set is reported as a position
nobody looked at — which a reader takes for a clean run.

`hold_last: true` on a mapped output: a document value with no entry in `map`
does not write the position, which then keeps the last value the rule wrote —
and before there is one, is not written at all. ⚠ "Keeps" means the slot keeps
whatever was last written there, which is not always this rule's value: a
record may plant a value in an output position itself (a product's test
setting an event's identifier before the round that turns it off), and a round
that writes nothing leaves the planted value, which is what `verify` judges
(`Planted`). A host that re-writes the rule's last value instead overwrites it.
That is the ADDRESS's
behaviour, not the document's: a position the component does not write keeps
what it held. The identifier beside an event's status is the usual case — it
says what is turning off, so it outlives the condition that set it. It lives in
the binding rather than as an extra input fed back into every document that
needs it, because the specification never says it; the address's structure
does. Needs a `map` and ordered examples.

**Every output rule is asked before a run what each case asks of it.** `check`
refuses a rule that neither maps nor passes through, `hold_last` with no map,
and a map that names no entry for a value the document says it can produce:
either case of a `bool` output, and, for a rule reading a send's event name,
every name the document sends to that processor and the `when_nothing_sent`.
A number has no such list, so a value its map lacks is found by the case that
produces it — and under `hold_last` a value with no entry is the one that holds.
These are the same functions `verify` lands values with (`landing.py`), so the
two cannot disagree about which rules are well formed.

`assumed: "<why>"` on any input or output rule: this rule decides something the
specification does not say, like a `when_absent` for a number nobody said the
absence of. The binding's peer of the document's `sce:assumed` — a failure on a
value resting on it is reported as that guess being refuted, not as the
document being wrong.

⚠⚠ **There is no key for reading the datamodel back, and none for reading the
active configuration**, though a statechart answers through both. Neither is
missing vocabulary. `address` plus `map` already says which position a
document value lands at and as what symbol; what changes is where the driver
goes to fetch it, and that is the kind's business rather than the dictionary's.
The configuration is a further step: a state is not an output, and binding one
would make a verification break on a rename that left the behaviour alone.

`check` reads the document for both sides of this. An output bound to a send
is answered by the document containing one — asking the datamodel question of
a statechart reported `the document does not compute it` for an output it
plainly writes, so a correct binding was refused for being correct. And an
`event` no transition listens for is refused, because the case would send it,
the machine would ignore it, and every later reading would be of a machine
that was never driven.

### What a host owes a bound document

A binding says where values go; the HOST — whatever code connects the
generated machine to a platform — decides how. `verify` models the host a
binding describes, so a host that does something else fails cases `verify`
passed. Every item below was such a disagreement, found by running the same
documents through a real platform's own acceptance tests; the document was
right each time and the host was not.

- **A lapsed input is an absence, however the platform says it.** A platform
  may flag a timed-out signal beside its value rather than change the value.
  A host that compares values only sees nothing happen, runs no round, and a
  document reading `absent` is never told. Deliver the flag as the absence
  the binding reads.
- **A delayed act is due when the machine says, not on a polling interval.**
  A document whose generated code reports `needs_event_scheduler` is driven
  with `tick()`: after every round, arm a one-shot timer for
  `timeUntilNextScheduled()` and treat its expiry as a round of its own.
  A host that never ticks has a machine whose delays never fire.
- **`hold_last` means the slot is not written.** Re-writing the last value
  instead overwrites whatever else wrote the slot since — including a
  value the platform's own test planted there.
- **A group of fields is announced once, after every field is written.** A
  host that announces when the first field of a group changes publishes
  the group without the fields later rules write in the same round.
- ⚠ **When to write an unchanged value is not yet a binding decision.** An
  original component writes an output whenever the logic computing it runs,
  changed or not, and acceptance tests wait for that write; the same tests
  also fail a write that arrives before the one they wait for. Writing every
  output every round and writing only changes were both measured wrong on
  one platform. Which outputs are which is per output, and nothing in the
  document or binding says it yet.

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
exist. `verify` runs, never on a value nobody supplied, and judges only what
cannot depend on the gap:

| open rule | what `verify` withholds |
|---|---|
| a boolean input of a computation | the positions that come out different when the case is run under both values |
| an event input of a statechart | every case from the first configuration in which an active state could act on that event |
| an output | every expected position no named rule writes that ends in one of its fields |
| a number input, or one a later round remembers | the whole run, which is refused: there are no two values to try, or the unknown would travel into rounds that never read it |

A case with any withheld position is not passed, and while any rule is open
the command's status is non-zero however the counts read. When the list
arrives, only this file changes.

⚠⚠ This exists because ABSENCE HAS TO BE WRITABLE OR IT DOES NOT GET WRITTEN.
Asked for a complete binding with no list to hand, an author — human or model
— produces a complete-looking one, and a plausible wrong address is invisible
in a way a missing one never is. The document's `sce:unresolved` has stopped
exactly this for VALUES since before this package existed; this is its peer for
addresses.

⚠ `when_absent` is required for an address handed over as it is — a number, a
text, a truth value — whose address a case may not drive, or may drive with a
token the conventions list as absence: a symbol comparison needs no such
declaration, because an address that is not reporting is not any symbol and
that is already the answer — but a value read as it is has no such fallback,
and folding absence into zero made seven cases on one corpus look as though the
specification had been misread.

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
