# Format-3 reading fixtures

These are saved states in format 3 (`schemas/sce-saved-state.v3.schema.json`), kept for the readers of format 4.
Format 3 held no start number of a host-run invocation and no count of those starts. docs/adr/0107, D4, says how
such a text is read: each running host-run invocation is the first start of its `<invoke>`, and the session counts
1 for each one it lists and nothing for any other `<invoke>`.

Every backend restores each text here with a machine of its document, at the wall-clock moment `1700000000000`,
as for the shared instances in `../`. Saved again at once, it writes, byte for byte, the shared format-4 text of
the same name in `../`. `sce-build/tests/saved_state_schema.rs` holds each shared text to that reading, applied to
the fixture here.

| Fixture | What it is | Read as |
|---|---|---|
| `static_invoke_tree_working.json` | The shared tree as every backend wrote it in format 3. Its child runs the host-run invocation `clock`. | `../static_invoke_tree_working.json`, the child's `clock` as start 1 and its count `{"clock":"1"}` |

The host-run invocation fixture of the same kind is
`sce-build/tests/fixtures/host_processor/saved/format3/statechart_static_host_invoke_running.json`, read as
`../statechart_static_host_invoke_running.json` beside it.
