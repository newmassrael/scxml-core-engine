# The reference harness, run by a host program

A program that asks a language model for a value surrounds each call with a loop:
it states what it wants, reads the answer, checks it, and when the answer does not
hold, says why and asks again. SCE writes that loop as a statechart, the reference
harness of [ADR 0087](../../docs/adr/0087-a-statechart-asks-a-model-for-a-value-of-a-schema-and-keeps-or-refuses-its-answer-as-data.md),
section 6. An application copies it and owns its checks, prompts and bounds.

This program runs the harness through the C++ backend, against the documents the
repository's tests replay, read in place. There is one copy of each document:

| Document | What it is |
|---|---|
| `sce-build/tests/fixtures/static_datamodel/static_model_harness.scxml` | the harness statechart |
| `lift_answer.scxml` | the answer schema the harness names (`sce:answer`) and lifts each answer into |
| `harness_ask.scxml`, `harness_repair.scxml` | the two text templates: the prompt it asks with, and the one it asks again with after a refusal |
| `harness_refusal.scxml`, `enum_harness_code.scxml` | the application's refusal record and its numbered code enum, which repeats the thirteen codes of the lift and adds the application's own (`notOffered`) |
| `harness_offer.scxml`, `harness_start.scxml` | the events that offer the harness an id and start it |

## What it does

The host is the reference host of the standard host features
(`sce/include/host/StandardHost.h`) over a scripted provider: the program reaches
no model and needs no key. The provider answers twice. The first answer is a record
whose title is 17 bytes, past the 16 its schema allows, so the lift refuses it as
`tooLong` at `/title`; the harness keeps the refusal, renders the repair prompt from
it and the answer refused, and asks again. The second answer is accepted.

```
--- call 1 (sce-host-model-generate)
Name the item the text between the markers asks for, ...
<<<TEXTPlan my dayTEXT>>>
--- call 2 (sce-host-model-generate)
Your last answer was refused for tooLong, at the place the pointer between the first markers names; ...
<<<TEXT/titleTEXT>>>
...
--- the run ended in `accepted`: outcome `accepted`, 1 attempt(s) after the first
```

The program exits non-zero when the run does not end as the documents say, and the
build registers it as the test `model_harness_example`.

## With a model

A host that has a model gives the reference host a provider of its own
(`SCE::StandardHost::ModelProvider`). Each call hands it the prompt and the id of the
answer schema the build named; the machine's table (`ANSWER_SCHEMAS`) holds that
schema's JSON Schema and its GBNF grammar, so a provider that decodes under a grammar
is given the form it takes, and one that cannot ignores it. Either way the lift is
the gate: an answer is never executed, only read into the schema's type, checked and
used as a value.

## Where the harness is tested

Every lane that runs the harness (Rust, Go, Kotlin, Python, C++, C11, Swift) replays
its five cases against the scripted provider
(`tests/host_features/scenarios/static_model_harness_*.json`) and runs it against the
exhaustive provider for every sequence of outcome classes up to its attempt bound,
requiring every row of its transition table taken, every refusal code reached and
every host outcome reached at every call site (ADR 0087, section 9.4). The
Interpreter refuses the harness by name: its host-run invoke is a model's
generation, whose answer schema the build writes.
