# Formal models

What a protocol of SCE promises, how it keeps it, and what shows that. Decided by ADR 0115
(`docs/adr/0115-a-protocol-is-specified-by-what-it-promises-refined-to-what-it-does-traced-from-the-code-that-does-it-and-a-gate-runs-all-of-it.md`);
this file says what is here and how to read it.

## The levels, and what is built

| Level (ADR 0115) | Here | Built |
|---|---|---|
| 1. Promise | `AtMostOnceService.tla`, `JournalService.tla`, `PrincipalService.tla`, `InboxService.tla`: what a user may rely on, no mechanism | yes, for five protocols |
| 2. Protocol refinement | `Refines` in `OpenInstances.tla`, `EventJournal.tla`, `Principals.tla`, `BrowserNode.tla` and `EngineInbox.tla` | yes, for five protocols |
| Bounded exploration | every `.cfg`: one size, one part taken out or none | yes |
| Progress and what stays possible (D9) | `progress` and `possible` of `models.json` | yes |
| 3. Theorem for every size (D3) | | no |
| 4. A model generated from a document (D7, D8) | `AppendixD.tla`: Appendix D once, over a document's tables; `W3CTest388.tla`, `W3CTest570.tla`, `W3CTest403c.tla`: three documents of the W3C corpus written as the generator will write them | the semantics and three documents, by hand; no generator, and no trace read against it |
| 5. A run read against a model (D4) | | no |
| 6. A function's contract (D10, D13) | `proofs.json`: the numbers of the `https` client, the Rust engine's drain of its inbox refining `EngineInbox` (ADR 0147, D4), a target's outbound side and the edge's rate limits, by Verus | yes, for four modules |

## The models

| Module | What it is of |
|---|---|
| `SharedListConvergence`, `SharedListConflicts` | ADR 0034, the shared list |
| `BroadcastLog`, `BroadcastLogWrapped` | ADR 0062, the broadcast log of one producer and its readers |
| `OpenInstances` | ADR 0113, open instances: joining, the lease, the fence, requests at most once |
| `EventJournal` | ADR 0114, the event journal: the write, the horizon, a follower and a question as of a moment |
| `Principals` | ADR 0116, a person bound to an incarnation: the proof, the end, the write's condition |
| `BrowserNode` | ADR 0117, the pages of one browser as one node: the lock, the store, a cleared store, a change of person |
| `Freshness` | ADR 0117, a request signed ahead by one who was inside a page |
| `InboxService`, `EngineInbox` | ADR 0084 and ADR 0147, the one door of every runtime: an inbox per session, the drain at the boundary and before an event of the owner's own, the address resolved when posted |
| `AppendixD`, `W3CTest388`, `W3CTest570`, `W3CTest403c` | ADR 0115 D7, the SCXML recommendation's Appendix D over a document's tables, and three documents of the W3C corpus over it: history recorded and restored, the done events of a parallel state's regions and of the state, the removal of conflicting transitions |

`AppendixD` is declared `abstract`: it is the semantics a document's model instantiates, and is run only by those. A
document's module extends it with its tables and promises Appendix D with every part in (`Refines`). Its holding runs
keep that promise by construction; what shows a part is the run that takes it out, which must break an invariant of the
document and the promise both.

Each module opens with what it takes as given ("Taken as given"). The code must be shown to have it, by the test or the
tool that judges that level; the model shows nothing of it.

## Running them

`sce-codegen formal models --manifest docs/formal/models.json` (`--only <module>` for one, `--shard k/N` for a part)
checks every model as `models.json` declares it, with the TLC the judge pins by digest
(`sce-build/src/formal/toolchain.json`, printed by `sce-codegen formal toolchain`). `sce-codegen formal install` fetches
the pinned tools and admits each by its digest; a judge never fetches. Exit 1 (`formal/declaration`,
`formal/verdict`) is a verdict about the models; exit 15 (`formal/tool-unavailable`) is "could not run". The judge is
one command of the code generator (docs/adr/0146), which another repository runs on its own manifest at the revision it
pins, as this one does; `sce-build/tests/fixtures/formal_consumer` is a manifest shaped like a consumer's, judged by the
same gate. The judge's own checks are the unit tests of `sce-build/src/formal`.

A configuration of its own is not enough: `models.json` must declare it, and the judge refuses

- a configuration or a module that is not declared, and a part of a model that no run takes out;
- a property a holding run checks and no run shows to fail: one nothing can break says nothing, however true it is;
- a run that holds with another number of states than is recorded (a model that shrank still reports "no error");
- a run that takes a part out and fails for another reason than it names;
- a model that states no progress and gives no reason, and a progress no run shows to fail without what it stands on;
- a temporal property beside a constraint, a symmetry or a view, under which it may hold for nothing;
- what must stay possible (`AG EF`, decided over the whole graph) and no run that loses it;
- a key it does not know, a tool named in the manifest (the judge pins its tools), and a manifest of another `v`.

The proofs are `proofs.json`'s, judged by `sce-codegen formal proofs --manifest docs/formal/proofs.json` (the gate is
`formal-proofs`): each file verified whole, nothing taken on trust, and each change that breaks what it promises refused.

What a check says (a promise, a property, a contract, what a run must answer) is changed by a commit of its own with its
reason; `pre-push` refuses another (`tools/git-hooks/spec_isolation.py`, ADR 0115 D11).

After a change that moves a count on purpose, `--record` writes the counts TLC found, and the diff is read.

## What all of this stands on

- **TLC**, the release the judge pins, and the Java that runs it.
- **Verus**, the pinned release, its Z3 and the Rust release it runs on; that its reading of Rust is the compiler's.
- **The bounds** of a configuration: an error past them is not found. A theorem (level 3) is what would say so.
- **That the model is of the code.** No model reads the code; levels 4 and 5 are what will. A proof (level 6) is of the
  code it is written on, and the model its preconditions name is read by a person.
- **That the promise is what was meant.** It is read by a person.
- **That `AppendixD.tla` is the recommendation's algorithm.** It is read by a person against the text, until the W3C
  corpus run by the Interpreter is accepted by it (ADR 0115, D8).
- What is judged elsewhere and not here: the memory order (GenMC), the bytes of a wire (the conformance kits), and
  cryptography (assumed).
