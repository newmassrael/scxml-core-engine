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
```

Every script is deterministic: a fixed-seed generator per group of cases, a
held clock where the reference reads one. Two runs print the same bytes, so a
regenerated case that differs from the committed one is a change in the
reference or in the script, never noise.

## What the scripts check about themselves

- `actual_crdt.mjs` refuses a prune case that drops nothing (a node has at
  most three children, so a careless case would not exercise pruning).
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
