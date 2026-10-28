# Targets — one logical interface, a profile per platform

A document says what a component decides. A binding says which addresses of a platform it reads and
writes. This directory cuts a binding in two and builds from the half that every platform shares.

```
binding.v1 ──split──▶ logical interface   what the specification names: signals, the rules that
                                          turn a signal into what the document reads, the
                                          outputs as the document produces them
                      profile             what only one platform needs: addresses, record
                                          fields, its symbols and counter protocols, the
                                          host's delivery rule
```

The logical interface carries **nothing of a platform**: no address, no record field, no counter
protocol, and no number for a symbol. A closed set of values crosses by name, so a signal lists its
symbols and an output lists the document's own values; a platform's number for a symbol is its own
code, and stays in the interface model that platform's side reads. The schema
(`schema/logical-interface.v1.schema.json`) refuses each of them.

## The tools

They need the authoring core on the module path (`PYTHONPATH=tools/targets:tools/authoring`).

| | |
|---|---|
| `python -m sce_targets.split split <binding> <pack> <spec.md> <out>` | writes `<out>/logical/<C>.interface.yaml` and `<out>/profile/<C>.yaml` |
| `python -m sce_targets.split join <interface> <profile> <binding>` | rebuilds the binding |
| `python -m sce_targets.split roundtrip <binding> <pack> <spec.md> <scratch>` | split, join, and check the core reads the result as the original |
| `python -m sce_targets.replay <interface> <profile> <pack> <script>` | the pack's examples, said again in logical terms, as a replay script |
| `python -m sce_targets.standalone <interface> targets/standalone.yaml <sce-codegen> <out>` | a C++ library and a driver that replays a script on it |

A key the split does not place is refused, never dropped: a dropped key is one the rebuilt binding
would silently lack. The same holds for a key the standalone target does not build.

Where an output's keys go, for a transform and a statechart alike: `hold_last` is the output's own
behaviour (a value or a send outside its `values` is no announcement, and it keeps the last one), so
it is `holds_last` in the interface and the library keeps the value; a statechart's
`when_nothing_sent` may then name a token outside `values`, which says "nothing is announced". Before
there is a last announcement, a record's expectation is counted (`unwritten`), not failed. `also` and
`when` fill the other fields of a platform's record; a library has no record, so they are the
profile's and those positions are counted as platform-only. A record that reads an
event slot as its first announcement (`announces_old_off`) and expects OFF is a reading only a host
has, and is counted as host-announced.

A statechart input that is told a value is placed too: `carries` sends the event with the signal's
number or truth value as its data (the library calls the machine's typed inject for that event), and
a `protocol` names a state in the interface (`logical`) and keeps its counters in the profile. A
statechart input with `absent` or `when_absent`, a closed set carried as data, and a machine with a
delayed act (it needs a clock the library does not have) are refused.

## What is judged

A library built from the interface alone, replaying the pack's examples, has to fail the same cases
`verify` fails (`tests/test_a_library_judges_as_verify_does.py`): a target that passes where `verify`
fails reads the interface differently from the core. The gate (`scripts/gate targets`) builds the
product's generator and a C++ library, and refuses to run without either, because a case that cannot
run skips and a skip reads as a pass.

The tests use two made-up packs: `tests/fixtures/pump`, a transform, and `tests/fixtures/lamp`, the
smallest statechart (a button whose press and release reach the machine as events, a lamp that leaves
as a send). A statechart is judged the same way (`tests/test_a_statechart_is_carried_as_a_transform_is.py`),
and its library also links the runtime sources the generator copies beside it (`RUNTIME_SOURCES` in
`sce_targets/standalone.py`, the Logger among them: docs/adr/0060). Nothing in a test
is a platform's, and `tests/test_the_targets_are_domain_free.py` holds the sources to the authoring
core's own vocabulary guard.

## What is not here

A target for a particular platform is a program that reads the same two files. It lives with that
platform, not here: this directory knows no platform's names, and the profile is the only place a
platform's addresses are written.
