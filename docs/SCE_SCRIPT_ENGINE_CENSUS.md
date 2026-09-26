# Script-engine cause census

ADR 0003 decided that SCE compiles one portable document into each
backend's own language, and that a runtime script engine is the fallback
for what cannot be decided at build time. This document is the
measurement that decision is worked against: **how often, and why, does a
document still need an engine?**

It exists before the programme deliberately. Ordering the work by guess
is how a long tail gets paid off first; the table below says which three
causes carry three quarters of the population.

## How the number is derived

`sce-build/tests/script_engine_census.rs` walks every tracked `.scxml`,
parses and analyzes each one, and holds the result to the ceilings below.
It is the same `SCXMLParser::parse_file` followed by `analyzer::analyze`
that the binary's own `scxml_host_requirement_facts` performs, which is
what makes the test and `sce-codegen check` unable to disagree about a
document's causes.

It runs in `tree-hygiene`, is registered in `UNFILTERABLE_GATES`, and
takes **0.65s** for the whole corpus.

### Three populations, not two

The distinction below is the one this document previously got wrong, in
both of its earlier instruments, so it is stated first.

| | 2026-09-16 | What it means |
|---|---|---|
| walked | 736 | every tracked `.scxml` |
| judged | 475 | documents a parse resolves, and therefore the denominator every count below is taken over |
| *(retired)* manifest-emitting | 650 | what the previous gate reported as "judged" |

The retired gate ran one `sce-codegen check` per document and counted a
document as judged whenever a manifest came back. But the facts function
behind that manifest parses the document itself and **returns nothing when
it cannot**, skipping it silently. So a document could be counted in that
denominator while contributing no cause and never having been asked --
the exact false-clean the gate existed to prevent, one level below where
it was looking. 475 is the population the causes actually come from.

That the two instruments report **identical counts for all eighteen
kinds** is the evidence they analyze the same documents; only the
denominator moved.

### What the 261 unjudged documents are

| | Count | |
|---|---|---|
| forge documents (root `sce:kind`) | 187 | stateless by construction -- no script engine is reachable from them, so their absence costs the census nothing |
| `sce:template` roots | 28 | not statecharts |
| `not-a-template` / deliberately unparseable fixtures | 2 | negative fixtures; the stage that judges them is not this one |
| statecharts a bare parse does not resolve | 44 | negative fixtures, every one |

**The 44 are explained, measured 2026-09-16.** Every one is a deliberate
negative fixture whose purpose is to be refused: 22 under
`tests/parsing/fixtures/`, 13 under `tests/w3c_template_parity/fixtures/`,
13 under `sce-build/tests/fixtures/`, and none outside a fixture directory.
Their diagnostics are the errors they exist to assert -- `<xi:include>` and
`<sce:use>` cycles, missing files, malformed templates, nesting depth
limits, and EventSchema type mismatches and enum overflows. **No real
statechart is silently skipped**, so this was a gap in this document, not a
defect in the instrument's reach.

⚠ **Reproducing it needs one correction, which cost a wrong claim first.**
The probe is one `sce-codegen check` per plain statechart, and it returns
**48** refusals, not 44. `check` is not the same predicate as the census's
`parse_file`: it also runs the validators and the backend, so four
documents that parse cleanly are refused later, each with `Forge codegen
error: ...` (no state nodes, an unknown initial state, two unknown
transition targets). 48 - 4 = 44, and that is what reconciles the probe
with `documents-judged`. The mistake to avoid is reading a `check` refusal
as a parse refusal -- `cmd_check`'s comment says the statechart arm reaches
the expander THROUGH `parse_file`, which makes parsing a part of `check`,
not the whole of it.

## Measured 2026-09-16

| | Count | Held as |
|---|---|---|
| Tracked documents walked | 736 | floor 700 -- an empty sweep must not read as a clean one |
| Documents judged | 475 | floor 450 -- documents that stop parsing would lower every count below while reading as progress |
| Documents needing an engine | 233 | ceiling |
| Documents using a native prefix (`cpp:` / `kt:`) | 1 | ceiling, and **not** a number to drive to zero |

The native-prefix column is asked of the parser, not grepped out of the
text: the parser is what decides whether `cpp:` in a `cond` is a native
guard or a string that merely starts that way.

### Causes by kind

Every number the test enforces lives in this one block, bookkeeping keys
included, so nothing it judges is parsed out of prose a later edit would
reword.

```census
documents-floor 700
documents-judged-floor 450
engine-documents 237
native-prefix-documents 2
datamodel-variable-init 284
transition-guard 220
assign-action 233
child-invoke-needs-script-engine 46
log-expr 45
send-param-expr 32
send-dynamic-attr 32
foreach-action 15
static-invoke-namelist 9
donedata-param 9
donedata-content 9
inline-script-action 2
send-namelist 5
if-condition 3
global-script 3
mesh-rpc-srcexpr 2
hybrid-invoke 4
cancel-expr 2
```

Each line is a ceiling: the count may fall freely, and a rise fails the
test. A rise is not forbidden -- it is required to be deliberate, which
means editing this table in the same commit that causes it. A row whose
population reaches zero is removed in the commit that empties it, because
a ceiling nothing can breach is indistinguishable from one whose key was
never spelled correctly.

## What the table says

- **Three causes carry 75%** of the 884 records: `<data>` initialisers,
  transition guards, and `<assign>`. These are the ECMAScript expression
  core, and they are the programme's first three sections. Nothing about
  that ordering was decided by preference.
- **`elseif-condition` and `unresolved-external-script` have no
  population here.** They are real kinds with zero instances in this
  corpus, so this table says nothing about them; a programme section for
  either would be written against no evidence.
- **The escape hatch is essentially unused in SCE's own corpus: 1
  document.** That is the baseline the ADR's escape-hatch column
  ratchets against. ⚠ **2 since 2026-09-22, deliberately:**
  `tests/integration/test_thermostat.scxml` now carries a `cpp:` guard,
  because it is the one document a test EXECUTES one from —
  `examples/smart_light` is generated and never run. The same commit took
  that document off the engine: its five `<script>` calls to undeclared
  functions and its script guard became `<sce:action>`s and the native
  guard, which is what `engine-documents`, `transition-guard` and
  `inline-script-action` fell by. ⚠ **2026-09-26, deliberately:** the host
  invoker fixture's `locating` state (idlocation member paths: two sends, a
  guard, four assigns, five counters) and the new
  `integration_resources/typed_reader_names/` (one engine document: eleven
  typed `<data>` and four assigns) raised `engine-documents`,
  `datamodel-variable-init`, `transition-guard`, `assign-action` and
  `send-dynamic-attr`. ⚠ **2026-09-26 again, deliberately:** the host
  invoker fixture's `timed` state (a host-run invocation's deadline: three
  counters, the two outcome guards, three assigns, and the `<cancel
  sendidexpr="''">` that must not reach a deadline — the first document to
  raise `cancel-expr` past 1) and the new
  `onexit_runs_before_the_state_leaves.scxml` (one engine document) raised
  `engine-documents`, `datamodel-variable-init`, `transition-guard`,
  `assign-action` and `cancel-expr`. Then the fixture's `typed` state (a
  typed host-run completion: three counters, three assigns) raised
  `datamodel-variable-init` and `assign-action` — and NOT
  `transition-guard`: its two guards read the `sce:result` record and lower
  natively, which is the point of the state. Then
  `the_run_ends_by_exiting_every_state.scxml` (one engine document: three
  handler records, four assigns, an `In()` inside an `<if>`) raised
  `engine-documents`, `datamodel-variable-init` and `assign-action`. A consumer pairing `cond="cpp:…"` with
  `datamodel="null"` is a separate population living in its own
  repository, and this number does not see it.
- **49% of judged documents need an engine** (233 of 475). The remaining
  51% already compile without one, which is what makes "the engine is a
  fallback" a description of the tree rather than an aspiration.
  ⚠ This figure was previously stated as 32%, taken over the 736 walked
  documents rather than the 475 the causes were measured over. The
  population did not change; the denominator was wrong.

## What this cost before

The first instrument was a shell gate running one `sce-codegen check` per
tracked document: **2610s**, against a `PUSH_BUDGET_S` of 300. Almost none
of that was the measurement.

- `check`'s contract is that it reaches the verdict `generate` would --
  the same parse, the same validators, **the same backend codegen** -- and
  with no `--language` it sweeps every backend. The census reads two
  fields that a parse and an analyze already produce.
- One process per document paid a 95 MB debug binary's start-up 736 times.
  Putting 50 documents through a single process changed nothing (148s),
  which is how the cost was identified as compute rather than spawn.

Neither finding justified moving the gate somewhere the cost would be less
visible. Parsing and analyzing the whole corpus in one process is already
paid for several times over in the same lane -- `scope_obligation` does
exactly this per tracked document -- and the census now costs 0.65s.
