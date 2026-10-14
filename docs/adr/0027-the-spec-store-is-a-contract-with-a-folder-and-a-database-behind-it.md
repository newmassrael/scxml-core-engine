# ADR 0027 — The spec store is a contract with a folder and a database behind it

- Status: Accepted as the direction (the owner decided it on 2026-10-14); not yet implemented
- Date: 2026-10-14
- Scope: `sce-specs` (the store), the server mode of the workbench, and git as an export
- Related: `docs/adr/0023-the-works-are-sces-and-the-workbench-is-a-repository-of-its-own.md`,
  `docs/adr/0024-a-work-is-called-a-spec.md`,
  `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`

## Context

The store keeps a spec in a folder: revisions are immutable files named by the hash of their bytes, a
head is a small file replaced under a file lock after comparing it with what the writer last saw, logs
are append-only, and the folder is the whole truth about the spec. That suits one person on one machine
and it is easy to put under git.

The owner decided that the server must be supported as well, with several people on shared specs, and
that git and a database must both be possible. They are not alternatives for the same job. Immutable
bytes are content addressed and need no database; the pointers that move (heads), the leases and the
identity of who is asking need atomic updates and are what a database does well; and git is a good way
to hand a spec over, not a way to take concurrent writes.

## Decision

**The store is a contract, and there are two implementations of it.**

The contract is a small set of operations on named things, and every implementation keeps the same
promises about them:

- `put_blob(bytes)` returns the hash and is idempotent; a blob is never changed and never removed
  while a revision names it.
- `read_head(name)` and `compare_and_set_head(name, expected, new)`: of any number of concurrent
  callers that expect the same value, exactly one succeeds and the others are told the value they
  missed (the `moved` refusal).
- `append(log, record)`: records are kept in the order they were accepted, none rewritten or dropped.
- `claim`, `heartbeat`, `release` on a lease, which expires by the store's own clock.
- A write that fails leaves nothing visible: no head names a blob that is not there, no log has half a
  record.

**Implementation one: the folder.** What exists today. The folder is the store, and one machine's file
lock is its atomicity. It remains the default for one person and the form git exports.

**Implementation two: the database.** For the server. Blobs go to an object store by hash; heads,
logs, leases and identities are rows in a database, a compare-and-set is a conditional update in a
transaction, and a lease expires by the database's clock. The choice of engine and object store is not
made here.

**A conformance suite is part of the contract.** One test suite, written once against the contract and
run against every implementation, states each promise above, including the concurrent compare-and-set
and the failed write. An implementation is not allowed in until it passes it. A promise that is not in
the suite is not part of the contract.

**Git is an export, not a backend.** A spec exports to a deterministic tree: its blobs under their
hashes and a file of heads and logs. Importing the tree into either implementation gives the same
hashes. Concurrent writes are never taken through git.

**The principle changes its subject.** "The folder is the only truth" becomes "the store behind the
contract is the only truth". A spec lives in exactly one store at a time; moving it is an export and an
import, and the hashes show it arrived intact.

## Consequences

- Nothing above `sce-specs` learns which implementation it talks to; the command layer, the
  authoring server and the workbench use the contract.
- The folder implementation is first brought under the conformance suite, which also pins its present
  behaviour against change.
- Authentication, authorisation and multi-tenancy of the server are not decided here; the contract only
  needs an identity to attach to leases and to the log records.
- The mutation casefiles that guard the store's promises follow the suite, so that a promise a mutant
  can break without a test failing is found.
