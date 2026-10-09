# ADR 0011 — A work keeps its requirement lineage with its requirement list

- Status: Accepted for stages 1 to 3 (implemented and tested, see "What stage 1 / 2 / 3 measured");
  D1 decided (c) on 2026-10-09 and stage 4a is under way; D2 is the owner's: see "Open decisions"
- Date: 2026-10-08
- Scope: `app-core` (the requirement list's record, `save_requirements`, `read_requirements`,
  `save_request_candidate`, bundles), the `sce-work` command layer, the authoring MCP's
  `works_*` tools, and a pure `between` / `extends` in `tools/authoring/sce_author/requirement_lineage.py`.
  Step 6 of the chain begun in ADR 0006
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`,
  `docs/adr/0008-an-acceptance-says-what-moved-since-it-was-taken.md`,
  `docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`,
  `app-core/src/store.rs`, `app-core/src/requirements.rs`, `app-core/src/acceptance.rs`

## Context

The lineage (ADR 0006) is what makes an id mean the same requirement from one revision of a
specification to the next: it remembers which id was issued for which requirement, so that an
id is issued once and never again. It is a file the CALLER keeps and hands back
(`lineage_text`) to the next build.

A work in the workbench is the application's own unit: its text, its model, the owner's
answers, its requirement list and its acceptances, five chains in one folder that ONE
implementation writes (`app-core/src/store.rs`). The folder has no place for the lineage. A
client that revises a work reads `manifest_text` and `sidecar_text` from `works_read`, and
those are all it can give back: `previous_manifest` and `previous_sidecar`, the path ADR 0006
calls adoption.

Adoption is the weakest guarantee the chain has, and the workbench is where revision happens.
Measured on 2026-10-08 with three revisions of a three-sentence specification, the second
dropping the last sentence and the third adding a new one:

| What the next build was given back | ids of revision 3 |
|---|---|
| the lineage of the previous build | `R1`, `R2`, **`R4`** |
| the previous manifest and sidecar only (what a work can hand back today) | `R1`, `R2`, **`R3`** |

`R3` was the sentence dropped in revision 2. A design accepted for revision 1 cites `R3` for
that sentence; read against revision 3, the same id now names the new requirement and nothing
says so. This is the failure ADR 0006 exists to prevent, reached by the path owners will use.
Adoption also loses the previous specification's digest and so the sentence delta, and a
client that gives back nothing at all is told "this is the first list as far as this call can
tell" and restarts at `R1`.

### What the folder already offers

- A requirement list is ONE record in the requirements chain: `{format, v, manifest, sidecar}`,
  a save is a compare-and-swap on the revision the writer read, taken under the folder's lock,
  and every revision is kept and can be read by name (`read_requirements(id, revision)`).
- A work is read as one state (`read_work_snapshot`), so the text, the model, the list and the
  acceptance a screen shows belong together.
- An acceptance names the revisions it was taken from (`Basis.requirements`).
- A request for a model is answered through a candidate that carries the list as a stored text,
  and `complete_request` publishes model and list together as a bundle; a work that keeps
  bundles refuses `save_requirements` with `bundled-work`. The request flow is the workbench's
  main path, and any place the lineage lives has to be on it.

## Decision

**The lineage is part of the requirement list's record: the same chain, the same revision, the
same compare-and-swap.**

1. **One record, not a sixth chain and not a file beside the folder.**
   - A sixth chain would be saved in a second step, so a list and its lineage could be saved
     apart and disagree: a list whose ids the lineage never issued, or a lineage that issued
     ids no list uses. They are one fact, "which ids this list uses and which were ever
     issued", and one save has to make it.
   - A file beside the folder is a second place two processes write, which `store.rs` names as
     the way a folder and its copy come to disagree.
   - In the record, the candidate and the bundle carry it without a line of their own: both
     hold the list as a stored text. And an acceptance names the requirements revision it was
     taken from, so the lineage of an acceptance is the lineage in that revision's record,
     exactly, not reconstructed.
2. **Format.** The record gains an optional `lineage`: the text exactly as
   `scxml_requirement_set` returned it, byte for byte, as the manifest and the sidecar are. The
   core checks its shape and not its meaning: a JSON object whose `lineage` member is
   `sce-requirement-lineage`, so a swapped file is refused and what a lineage says remains the
   authoring package's to define. A record with no lineage is written as `v: 1`, byte for byte
   as before, so every digest already kept and every acceptance pinning one is undisturbed. A
   record with a lineage is `v: 2`. A build that predates this refuses a `v: 2` record loudly
   (`deny_unknown_fields`, and a version it does not know) and does not drop the lineage
   silently. The command layer's version moves, as it has for each change of an answer.
3. **A save that would lose the lineage is refused.** Under the folder's lock, a save without a
   lineage onto a head that has one is refused (`lineage-dropped`, naming what to give). There
   is no override: starting over is a new work. A client that forgot the field is told, and
   the ids do not restart.
4. **A save has to extend the lineage it follows.** `extends(previous, next)` in
   `requirement_lineage.py` refuses a lineage of another `doc_id`, a lower `next`, an id of the
   previous that is missing or whose quotes are not a prefix, a retired id that is live again,
   and a revision list that is not a continuation. The authoring tool runs it after reading the
   head and before saving; the compare-and-swap on `base` guarantees the head it checked is the
   head that is replaced. The core does not run it (see "What this does not claim").
5. **The tools carry it.** `works_read` returns `requirements.lineage_text`;
   `works_save_requirements` takes `lineage_text` (and the candidate command takes `lineage`);
   the instructions say to build with `lineage` and `previous_sidecar` from `works_read`.
6. **The words delta of a work is derived from two states of its own chain, not carried.**
   `between(older, newer)` is a pure function of two lineages: an id live in `older` and live in
   `newer` with the same latest quote is `carried`, with another is `changed`, and live in
   `older` and retired in `newer` is `retired`; an id live in `newer` and absent from `older` is
   `new`; one issued and retired in between is not listed (no design could cite it). The two
   states are the requirements revision an acceptance names and the head. Consequences:
   - ADR 0009's refusal of a delta that is not the record's own cannot arise for a work, because
     both sides are the work's;
   - any two revisions can be compared, so the limit ADR 0009 states (a delta of a later step is
     refused and steps are not composed) does not bind a work;
   - the lineage of ONE state plus a revision number would not do: a specification that did not
     change can still be re-quoted, and two quotes of one id then carry the same `rev`.
7. **Works that already have a list.** The first build against a work whose head has no lineage
   passes its manifest and sidecar as the previous list (adoption), and the lineage it makes is
   saved with the list. Ids are kept; the previous specification's digest and the sentence delta
   of that one step are unknown, as ADR 0006 says. From the next revision on the guarantee is
   the full one.

## Open decisions (the owner's)

- **D1. Where the join lives once two surfaces need it.** ADR 0009 put the join of words and
  evidence in Python because it needed no product change, and the workbench's report needs the
  same join while `app-core` is Rust. A second table written in Rust would be two definitions
  of what a revision finding is, and they would drift. The choices: (a) keep one in Python and
  have the application reach it through the authoring package, (b) port it to `app-core`, (c)
  move it into the product as one command (`revision-join`, with its schema registered in
  `SCE_WIRE_CONTRACTS.md`) that the Python tools and `sce-work` both call. **Recommendation: (c).**
  It changes where ADR 0009's table lives and adds a wire surface, so it is not done by this
  ADR. Stages 1 to 3 do not depend on it.

  ⚠ **(c) is larger than the paragraph above says (2026-10-09).** The join table is not the only
  part a screen would need: `between`, `extends` and `belongs_to_manifest`, which derive the
  words of a revision from two lineages and say whether a lineage continues another, are Python
  too, so a product command that took the join alone would leave the application calling Python
  for the words. (c) done as the long-term answer is the product READING lineages and judging
  from them: it takes the two lineages and the record, compares them, and joins. What stays on
  the authoring side is MAKING a lineage (the similarity of a quote to the words an id last had,
  adoption, `continues`), because deciding that two sentences are one requirement is deciding
  what the requirements ARE, and the product measures a design against a list it did not write.
  The cost this leaves, stated: the lineage's format is then known to two implementations (the
  authoring package makes and checks it, the product reads and compares it), held together by the
  one schema file (`tools/authoring/schema/requirement-lineage.v1.schema.json`) and by the
  same cases run through both; the tests written for the Python version (the twenty pairs of the
  table, the neighbour rule, `between`, `extends`) become the cases the product's must pass.

  **Decided (c), 2026-10-09, by the owner**, who asked for the answer that is right in the long
  run whatever it costs, was told it is larger than the table alone and that making a lineage
  stays in Python, and said to confirm it and go on. Stage 4a below builds it: the product reads
  two lineages and the acceptance record and judges, and the Python tools and `sce-work` call it.
  The order is the safe one: first the cases both implementations must pass (4a-1), then the
  product's command (4a-2), then the callers (4a-3), so the Python version stays the reference
  until a second implementation has been held to the same cases.
- **D2. A way to start the lineage over.** Recommendation: none. A specification written again
  from nothing is a new work, and an id that may be issued twice is the defect.

## What this does not claim

- **The lineage is as good as its similarity.** ADR 0006's limits stand: a wrong succession is
  listed `changed`, never `same`, and a very short identifier may be retired and issued again.
- **The core does not understand a lineage.** A direct `sce-work save_requirements` can store
  any lineage object that has the right marker. Only `lineage-dropped` is structural enough
  for the core to enforce; `extends` is the authoring package's, because it owns what a lineage
  means, and two implementations of that would be two definitions.
- **Nothing new leaves the folder.** The lineage holds hashes and no word of the specification;
  the sidecar, which holds sentences, is already kept there.
- **An acceptance still pins the manifest's bytes** and not the lineage; the product never
  reads it, and staging for the product (`spec/requirements.manifest.json`) is unchanged.
- **Not retroactive.** A list saved before this has no lineage until the next build against it
  (decision 7).
- **Two clients revising one work** are serialised by the compare-and-swap on `base`: the second
  save is refused as a `conflict`, reads again and builds again, and the lineage `next` cannot
  be issued twice.

## Stages, and what each is done when

Criteria set before code. Each stage ends with a push; none needs the next.

1. **The record and the commands (`app-core`).** Done when: a list saved with a lineage reads
   back byte for byte, and one saved without is the same bytes (the same digest) as today; a save
   without a lineage onto a head that has one is refused `lineage-dropped`; a candidate carries
   the lineage and `complete_request` publishes it with the list; a core that predates the field
   refuses a `v: 2` record and does not read it as a list without one; the command layer's
   version has moved; the breaks (the field not written, not read, not carried by the candidate,
   the drop not refused) are each caught. **Done**: see "What stage 1 measured".
2. **The tools (`tools/authoring`).** Done when: through the real `sce-work`, three revisions of a
   work that hands the client back only what `works_read` gave keep their ids with no
   `lineage_text` passed by hand, and the measurement above gives `R4`, not `R3`; a save whose
   lineage does not extend the head's is refused with the reason; `extends` has a refusal for each
   rule of decision 4 and a break that removes it. **Done** for the tools: see "What stage 2
   measured"; the application's own generation is "Stage 2b" there, and is not done.
3. **The derived delta and a check by work.** `between`, and a `works_revision_check` that joins
   it with the product's evidence over the staged work (`app-core/src/acceptance.rs` already
   stages a work for the product, `Snapshot::stage`). Done when: the check of a work agrees with
   `scxml_revision_check` given the same two lineages, and a delta of a later step needs no
   composition. The join's home is D1.
4. **The workbench report.** The screen shows the revision report of a work whose acceptance
   lapsed because its text was revised. Done in the acceptance panel (`revision_model.ts`,
   `App.loadRevision`): beside the acceptance it asks `read_revision_report`, shows the
   verdict, the counts and the page as the product wrote them, and shows the core's own
   sentence when it cannot compare yet (`revision-not-current`, `revision-not-judged`). The
   screen compares nothing; a report of other revisions than the screen is showing is not
   shown as this work's.
   - **4a. The product judges (D1 (c)).** Three steps, each pushed:
     1. *The cases.* `sce-build/tests/fixtures/revision_judgment/cases.json`, generated from the
        Python implementation by `tools/authoring/eval/revision_judgment_cases.py` and guarded
        against drift: the join (all twenty pairs of the table, the neighbour rule, the refusals of
        a delta that is not the right shape), the page, `belongs_to`, `between`, `extends` and
        `belongs_to_manifest`, each with the exact result or the exact refusal sentence. Done when
        the Python reproduces the file and a test fails for any case that is left out of it.
     2. *The command.* `sce-codegen revision-judge`: two lineages, the acceptance record's
        manifest pin and the `acceptance-delta` lines in, the judgment (and the page) out; its
        schema registered in `SCE_WIRE_CONTRACTS.md`. Done when it reproduces every case of 1 and
        the breaks of its rules are each caught.
     3. *The callers.* `works_revision_check/report` and `read_acceptance_delta` call it; the
        Python join stays as the reference the cases are generated from, or is removed, whichever
        the cases show to be safe.

## What stage 3 measured

Implemented as `requirement_lineage.between` (the words of a revision, from two lineages),
`works.read_requirements_at` and `works.read_acceptance_delta`, the tools `works_revision_check`
and `works_revision_report` (`mcp._work_revision_join`), and, in `app-core`, a new command
`read_acceptance_delta` (command set 20) over a new method of the acceptor,
`delta_acceptance`, which stages the work as the product is always shown one and runs
`acceptance-delta`. The acceptance and the design and list it is compared with are read as ONE
state of the work, and the manifest the record pinned is passed on beside the lines. The join
stays where ADR 0009 put it (D1 is not decided by this stage). Held by
`app-core/tests/what_moved_in_a_work_since_it_was_accepted.rs` (6 tests), the stand-in and the
real generator in `tests/acceptance_product.rs`, the contract file, the screen's guard
(`parseReadAcceptanceDelta`, 2 tests; the screen does not show the report yet),
`tests/test_a_lineage_is_only_ever_appended_to.py` (10 tests of `between`) and
`tests/test_a_revision_of_a_work_is_checked_against_what_its_owner_accepted.py` (12 tests
against the real `sce-work` and generator):

1. **Met.** A work nothing was done to carries every requirement over; a sentence dropped with
   its citation taken off is `retired-cleanly`; dropped and still cited is `outside-reach`
   (`retired-still-cited`); a design moved where the words did not is `moved-without-reason`,
   with the places the product names.
2. **Met.** Two revisions after the acceptance are one step: the dropped sentence's id is
   `retired-cleanly` and the new one `implemented-new`, with the id the lineage issued (`R4`, not
   the dropped `R3` again). `between` lists neither an id issued and retired in between nor a
   requirement reworded and reworded back as changed.
3. **Met.** An acceptance of a list that had no lineage is compared by adopting its manifest
   and sidecar; one with neither is refused in words; a work nobody accepted is refused
   (`nothing was accepted`); a remote caller is not offered either tool.
4. **Met, held by a test that alters the answer.** The manifest the acceptance pinned is
   compared with the digest of the list the words start from. By construction the two agree on
   every real path (a lineage is saved only beside the manifest it names), so the check cannot
   fire there; it is held by patching the command's answer, as only a wrong staging could.
5. **Met.** The product's own words reach the tool: a record it cannot compare (taken before it
   kept the rows, or under another rule) is refused in its sentence, not read as "every
   requirement is new".

⚠ Where each refusal is held. The command's own tests use the in-process stand-in product, which
cannot be made to refuse a record through the commands (a work always has a design to compare),
so a test written there for "a record the product cannot compare" held nothing beyond the
timeout and was removed rather than left claiming more. That refusal is held where a refusal can
be made: `acceptance_product.rs` has a script stand-in that refuses a record without evidence and
says nothing for a silent failure, as the real product does.

## What stage 2 measured

Implemented in `tools/authoring`: `requirement_lineage.extends` and `belongs_to_manifest` (the two
judgements the core leaves to this package), `works.read_requirements` and `works.save_requirements`
(the lineage in and out), and `mcp._lineage_refusal` with `works_save_requirements` taking
`lineage_text` on both its paths, the direct save and a generation's candidate, and `works_read`
giving it back. Held by `tests/test_a_lineage_is_only_ever_appended_to.py` (22 tests, no binary
needed) and `tests/test_a_work_keeps_the_ids_of_its_requirements_across_its_revisions.py` (11
tests, against the real `sce-work` and generator):

1. **Met.** Three revisions of a work, each built from what `works_read` gave with nothing handed
   over by hand, keep `R1` and `R2` and issue the new requirement `R4`; the dropped sentence's `R3`
   is not issued again. The control, built from the manifest and the sidecar alone, gives `R3`:
   the measurement of the Context, now reproduced through the tools and kept as a test so that the
   lineage cannot stop mattering unnoticed.
2. **Met.** A list that had no lineage is adopted when the next one is built from its manifest and
   sidecar, the lineage is saved with the next list, and the revision after it is built from the
   lineage alone and keeps the ids.
3. **Met.** A list without the lineage the work holds is refused `lineage-dropped` and the work
   keeps its list; a generation's list without it is told at once, with nothing written, and is
   written again with it. A lineage that is another list's (the digest of the manifest differs), or
   of another history though well formed (a fresh start for the same text), is refused with the
   reason. A save from a stale base is a conflict first, not a complaint about the lineage.
4. **Met.** The same flow through a generation, the application's own path, publishes the lineage
   with the list and reads back the same ids.
5. **Met.** `extends` has a refusal for each rule of decision 4, and a case that is accepted for
   each thing it must allow (a step after a step, a step after one that is not the last, the same
   text read into another list, a requirement re-quoted within the last revision).

Twenty-three ways of breaking it (each rule of `extends` and of `belongs_to_manifest` switched off,
the tool not asking either question, the tool judging a stale base, a generation not told, the
lineage not passed on by either save path, `works_read` not giving it back, the works layer not
sending it) were put to those tests and each was caught.

### Stage 2b: the application's own generation

The application asks a client to write a model and a list (`client_run`, `runner`), and a list
that arrives without the lineage a work holds is refused at publication (`lineage-dropped`), so
the gap is loud and a lineage cannot be lost silently. What is done and what is not:

- **Done, and held by tests.** The draft reads a `lineage_text` when an answer has one
  (`client_run::draft_from`), and the local model path takes the lineage from the very call of
  `scxml_requirement_set` that gave the manifest (`local::Listed`), so a model is never asked to
  copy one and one from another call is never stored beside this list. The client is told what to
  do by the tools' own words (`works_read`'s `next`), which are not part of the application's
  instructions. Three breaks (the draft not reading the lineage, the tool's answer not read for
  one, the list not written back with it) were each caught by the tests meant for them.
- **Closed.** The form a Claude Code or Codex client answers in asks for `lineage_text` (a string
  or null, as a strict form lists every property as required), and the task tells the client
  to build the list against `requirements.lineage_text` and `requirements.sidecar_text` that
  `works_read` gave, and to answer with the `lineage_text` the tool returned. A generation on a
  work that holds a lineage therefore no longer ends in `lineage-dropped` for want of one.
- **Why the Codex entry changed.** Changing the shared task or form is a new execution contract
  for Codex: `tests/codex_support.rs` holds that the verification the application ships names
  the current contract, because Codex has no switch that turns its built-in tools off and a
  version is run only when a person verified it, with the real client, against a specification
  written to attack it (`tests/codex_live.rs`). With the owner's approval both live tests (the
  plain run and the attack run) were run with `SCE_CODEX_VERIFY=1` against Codex 0.159.0 on
  2026-10-09 and passed, and the shipped entry names the new contract, `codex/7465c0fb9357`.
  Of `app-core`'s tests the change moved exactly two: the schema's list of required properties
  in `tests/claude_code.rs` and the Codex contract.
- **What the live run did not measure.** It used the synthetic indicator specification, a
  work that holds no lineage, so it verifies the sandbox, the tools and the answer form under
  the new wording, not that a model builds a revision against a held lineage. That is a run of
  its own, with a work that holds one.

## What stage 1 measured

Implemented in `app-core/src/requirements.rs` (the record), `app-core/src/store.rs` and
`store/bundle_store.rs` (`refuse_a_dropped_lineage`, called by a direct save and by a bundle's
publication, under the work's lock), and `app-core/src/commands.rs` (`lineage` of
`save_requirements` and `save_request_candidate`, `lineage` in what `read_requirements` and the
snapshot return, `COMMAND_SET_VERSION` 19 with the screen's `SUPPORTED_COMMAND_SET_VERSION` and
`contract/replies.json` moved with it). Held by
`app-core/tests/a_work_keeps_the_lineage_of_its_requirement_list.rs` (12 tests), the record's own
unit tests (10) and `tests/contract.rs`:

1. **Met.** A list with a lineage reads back with it byte for byte as a `v` 2 record, and a list
   without one is the bytes it was before lineages: the test writes those bytes out field by
   field instead of comparing the code with itself (the first version of that test did, and held
   nothing).
2. **Met.** A save without a lineage onto a list that has one is refused `lineage-dropped`, with
   the revision it would have replaced, and nothing is written. It is judged after the stale-base
   check, so a writer that has not read the list is told to read again first. A first list, a
   list onto one without a lineage, and a list with another lineage are saved as ever.
3. **Met.** A candidate that loses the lineage is not published, the request stays the
   executor's, and the same request is published once the list is written again with a lineage.
   A work whose lists never had one publishes as ever.
4. **Met, emulated.** A build that predates lineages refuses a `v` 2 list (the older shape is
   written out in the test, strict about unknown fields as the reader was; it is not run) and
   so does not read it as a list without one.
5. **Met.** What is not a lineage (not JSON, not an object, not saying it is a
   `sce-requirement-lineage`) is refused as an invalid list, and a lineage given without a
   manifest is a bad request.
6. **Met.** `lineage` is absent from the reply for a list that has none, and not `null`, so the
   reply of every list that never had one is the reply it always was. The contract file gained
   three entries and a refusal (`save_requirements_lineage`, `read_requirements_lineage`,
   `lineage-dropped`) and its version; it is byte-identical to what the test regenerates.

Seven ways of breaking it were put to those tests and each was caught by the test meant for it:
the refusal removed from a direct save, the refusal removed from a publication, the reply
carrying a `null` lineage, the lineage's kind not checked, the save command ignoring `lineage`,
the candidate command ignoring it, and the record always written as `v` 1. The screen's tests
(17 files, 546 tests) pass with the new version, run with node 22 as CI does; `app-core`'s tests
pass and clippy with warnings denied is clean. One test of the MCP client
(`a_server_that_closes_its_input_and_lives_on_is_said_to_be_gone_and_not_waited_for`) timed out
once in the whole-crate run on a loaded build machine and passes alone; it does not touch lists.

Not done in this stage, by design: nothing in `tools/authoring` gives or reads a lineage through
`works_*` yet (stage 2), and the in-application generation (`client_run`, `runner`) does not hand
a client the previous list's lineage, so a generation in the application still builds its ids
without one. That is the larger half of the main path and is named here so that stage 2 is not
read as closing it.

## Review of 2026-10-09: two findings, both reproduced

A review of the tip after stage 2 made two findings. Both were reproduced before anything was
changed, and each is a case of a judgment that was made in one place and not in the other.

1. **Another specification's list of the same shape was taken for the accepted one.** The
   manifest's digest names the shape of a list (an id, a section, a modality) and no word of it,
   so two specifications with the same ids and sections have one manifest text. Measured with
   two different two-sentence specifications: one manifest text, and the delta of one belonged to
   the acceptance of the other (`revision.belongs_to` passed). **Repaid:** the lineage keeps the
   sidecar's digest beside the manifest's on each revision (`sidecar_sha256`, `pin_list`), the
   delta names it (`from_sidecar_sha256`), the acceptance record pins it (`manifest.sidecar_sha256`,
   `accept --sidecar`, only the digest), and `belongs_to` compares it. The check on a list and its
   lineage, `belongs_to_manifest`, became `belongs_to_list` and takes the sidecar too; a list saved
   with a lineage has to come with the sidecar that lineage names. The shared cases were
   regenerated with the new refusals (the same shape over other words, no digest of the words on
   either side, a record taken without them). Records taken before keep their bytes and are refused
   for a revision with the reason: accept again with the sidecar. ADR 0009 item 6 has the rest.

2. **A lineage that does not continue the work's was saved.** The core refuses a list that
   LOSES the lineage; it did not ask whether a lineage it was given CONTINUES the one it holds,
   because that was left to the authoring tool (`mcp._lineage_refusal`), and the application's own
   paths do not go through that tool: a model that builds its list without passing the work's
   lineage gets a fresh lineage, which the core sees as "a lineage" and keeps, and the retired
   id is issued again to another requirement. Measured by saving, from a work holding the
   `previous` lineage of each `extends` case of the shared cases, the `following` one: the six
   that continue were saved, and so were all twelve that do not (a retired id made live again, an
   id numbered twice, a history rewritten, another specification). **Repaid:** the decision is
   the one already taken (D1 (c)), the product judges. The judgment is a crate of its own,
   `sce-revision` (a member of the root workspace, which both the store and the generator can
   depend on without the one depending on the other): `parse`, `extends`, `belongs_to_list` and
   `between`, in the Python's own sentences, held to it by the shared cases
   (`tests/the_cases_the_python_gives.rs`: every case, the sentence compared and not only the
   fact of a refusal). The store calls it when a list is saved or published
   (`refuse_a_lineage_not_kept`) and refuses four things in the order a person would want to be
   told: `lineage-dropped` (the work's list has a lineage and this one has none),
   `lineage-unusable` (it is not a lineage), `lineage-of-another-list` (it is not THIS list's,
   by the digests of its manifest and sidecar, so a list saved with a lineage has to come with
   the sidecar the lineage names) and `lineage-not-continued` (it does not continue the work's).
   The record stays as tolerant as it was (a lineage names itself), so that what the store holds
   stays readable whatever it says; the judgment is at the door. The test that held "another
   lineage replaces it" was the design this reverses and is replaced: the eighteen `extends`
   cases of the shared cases are saved from a work holding their `previous`, the cases that
   continue are saved and the others are refused, and the review's own case (a list built
   without the lineage it was given, which starts at `R1` again) is refused by a direct save and
   by a published candidate, after which the executor writes the list again and publishes it.
   The command set moved to 21 for the new refusals.

   The rest of D1 (c) followed, and closes it for a work: `sce-revision` also holds `belongs_to`,
   `join`, `render`, the lines of the product's `acceptance-delta` gathered into the object the
   join reads (`delta_object`), and the lineage of a list that predates lineages (`of_list`,
   which adopts it from its sidecar), each held to the Python by the same cases sentence for
   sentence. The application's command layer asks it as one command, `read_revision_report`
   (command set 22): the acceptance and the design and list it is compared with are read as ONE
   state of the work, the list that was accepted is read at the revision the acceptance names,
   and the answer is the verdict, the rows, the revisions the two states are about and the page,
   or `revision-not-judged` in the sentence a person is told (a list that keeps no lineage and
   no sidecar says nothing of its words; an acceptance that pinned another manifest than the
   list it was taken of is not about it), or `revision-not-current` when the text was changed
   after the model or the list was written for it (the list the work holds is then the accepted
   one, and comparing it with itself would call a revision nobody made "all carried over"; only
   a part known to be written for an earlier text is held back, and the refusal names which).
   `works_revision_check` and `works_revision_report` are
   now that command and nothing more, so the screen and the tools judge a revision with one
   implementation. The Python `revision.py` and `requirement_lineage.py` stay as the reference
   the cases are written from, and as the judgment of the tools that are handed a delta and a
   record by hand (`scxml_revision_check`, `scxml_revision_report`), which no work is behind.
