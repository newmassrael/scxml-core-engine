# Reference oracles

Expected values for standard documents (`stdlib/`) whose behaviour an
existing implementation already defines. Each script **runs that reference's
own code** — it never re-implements it — and prints the cases that
`tests/forge/conformance/numerical_reference.json` holds for the document.
The conformance harness then holds all six backends to them.

Where a reference is wrong, the document fixes it, and the script marks the
case `deviation:`: its expected value is the document's definition, and its
note records what the reference does with the same input. A fix is thereby
evidenced by the reference's own answer, not asserted.

| Script | Reference (pinned) | Runs | Documents |
|---|---|---|---|
| `actual_crdt.mjs` | actualbudget/actual `packages/crdt` @ `bfa850ca618ca78c556ded61274602f1fca7a4a4` (MIT), with `murmurhash@2.0.1` as its `yarn.lock` pins | `merkle.ts` `insert`/`prune`/`diff`, `timestamp.ts` `recv`, `hash` and `toString`, `murmurhash.v3` | `sce:std/hash/murmur3_32`, `sce:std/merge/merkle_insert`, `merkle_prune`, `merkle_diff`, `hlc_within_drift`, `hlc_text`, `lww_classify` (see below) |
| `signal_rate.py` | signalapp/Signal-Server @ `bdf3e1aea1b83e6ce14530ba515501c15bade3ad` (AGPL-3.0; executed, not copied) | `service/src/main/resources/lua/validate_rate_limit.lua` via `EVAL` in `redis:7-alpine` | `sce:std/rate/gcra_admit` |
| `sabre_changes.php` | sabre-io/dav @ `1ce51f845f778b6fab8e9289670c8ad82e43a5c2` (BSD-3-Clause) | `Sabre\CalDAV\Backend\PDO::getChangesForCalendar` over SQLite built from `examples/sql/sqlite.calendars.sql`, in `php:8.4-cli-alpine` | `sce:std/sync/changes_since` |
| `sabre_precondition.php` | sabre-io/dav, as above, with its composer dependencies | `Sabre\DAV\Server::checkPreconditions` on real requests | `sce:std/http/precondition` |
| `signal_mailbox.py` | signalapp/Signal-Server, as above | `lua/insert_item.lua`, `get_items.lua`, `remove_item_by_guid.lua` via `EVAL`, a seeded sequence of offers, pages and acknowledgements | `sce:std/sync/mailbox_assign`, `mailbox_insert`, `mailbox_page`, `mailbox_ack` |
| `davx5_sync.py` | bitfireAT/davx5 (GPL-3.0; executed, not copied — the classifier is lifted from the checkout into a temporary directory for the run) | `SyncExceptionHandler.classifySyncException`, compiled with kotlinc against stubs for the Android and dav4jvm types it names | `sce:std/sync/sync_failure`, `sync_retry_at` (see below) |
| `batch_order.py` | none — see below | the document's own definition, cut at every point | `sce:std/sync/changes_apply` |
| `sabre_acl.php` | sabre-io/dav, as above, with its composer dependencies and its test suite's mocks (`tests/Sabre/DAVACL/MockPrincipal.php`, `MockACLNode.php`, `tests/Sabre/DAV/Auth/Backend/Mock.php`) | `Sabre\DAVACL\Plugin::getPrincipalMembership`, `getFlatPrivilegeSet`, `getCurrentUserPrivilegeSet` | `sce:std/acl/acl_membership`, `acl_granted` |

## Running

The references are not part of this tree. Clone each at the pinned commit,
then:

```bash
# Actual: its crdt package needs murmurhash and uuid, installed OUTSIDE the
# yarn workspace (npm refuses its `workspace:` protocol) and linked in.
npm install --prefix <deps> murmurhash@2.0.1 uuid@14.0.2
ln -s <deps>/node_modules <actual>/packages/crdt/node_modules
node tools/reference-oracles/actual_crdt.mjs <actual>            # Node >= 22.18 runs the .ts sources

python3 tools/reference-oracles/signal_rate.py <signal-server>   # starts and removes its own redis container

docker run --rm -v <sabre-dav>:/sabre:ro -v $PWD/tools/reference-oracles:/oracle:ro \
  php:8.4-cli-alpine php /oracle/sabre_changes.php

# checkPreconditions needs sabre/dav's composer dependencies (sabre/http, …):
#   composer install --no-dev in a copy of <sabre-dav>/composer.json → <vendor>
docker run --rm -v <sabre-dav>:/sabre:ro -v <vendor>:/vendor:ro -v $PWD/tools/reference-oracles:/oracle:ro \
  php:8.4-cli-alpine php /oracle/sabre_precondition.php

python3 tools/reference-oracles/signal_mailbox.py <signal-server>  # its own redis container, as signal_rate.py

python3 tools/reference-oracles/davx5_sync.py <davx5>             # kotlinc and java on PATH

python3 tools/reference-oracles/batch_order.py                    # no reference: see below

docker run --rm -v <sabre-dav>:/sabre:ro -v <vendor>:/vendor:ro -v $PWD/tools/reference-oracles:/oracle:ro \
  php:8.4-cli-alpine php /oracle/sabre_acl.php
```

Every script is deterministic: a fixed-seed generator per group of cases, a
held clock where the reference reads one. Two runs print the same bytes, so a
regenerated case that differs from the committed one is a change in the
reference or in the script, never noise.

## What the scripts check about themselves

- `actual_crdt.mjs` refuses a prune case that drops nothing (a node has at
  most three children, so a careless case would not exercise pruning).
- ⚠ `davx5_sync.py` runs DAVx5's classifier against stand-ins for the
  dav4jvm exception classes it names. The classifier tests each specific
  class before the generic `HttpException`, so where they sit in the real
  hierarchy does not change its answer; but a 503's delay comes from the
  stubbed `getDelayUntil` (dav4jvm is not in the checkout), and only the
  502's 15 minutes is the reference's own. `sync_delete_outcome` and
  `sync_upload_outcome` are NOT run at all — their catch clauses live in
  `SyncManager`, which needs Android — and their cases say they were
  written from the source as read.
- ⚠ `batch_order.py` runs no reference: the two clients measured
  (Tutanota's missed-update cache, Proton Calendar for iOS's event
  processor) both apply a batch out of log order, which is what
  `changes_apply` exists to fix, and neither runs outside its app. Its
  expected values are the document's definition; what it proves is the
  property — it refuses a case that any cut of the log answers differently,
  and it prints each random batch's second half applied to the answer to
  its first half as a case of its own, so every backend is held to the same
  property. The two clients' orders are recorded in the counterexample
  notes as read from their source, not as run.
- ⚠ `lww_classify` is the one document whose cases are NOT the reference's
  function run: `compareMessages` (`loot-core/src/server/sync/index.ts`)
  reads its answer from the replica's database. The script applies the rule
  that function states after its query — duplicate, then later, then apply —
  to stamps ordered by the reference's own `Timestamp.toString()`, so the
  ORDER is the reference's and the three-way rule is transcribed. Running the
  function itself needs its SQLite schema seeded per case.
- `signal_rate.py` asserts, for every case it takes from the script, that the
  script's answer and the arrival-time limiter's agree — the equivalence the
  document claims, measured on each input.
- `sabre_precondition.php` names, for every deviation, which of the
  reference's known departures from RFC 9110 caused it, and marks one it
  cannot name `UNEXPLAINED`.
