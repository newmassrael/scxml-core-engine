<!--
  SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
  SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
-->

# SCE Workbench

A place to write a specification in prose, see the model an AI client drew from
it, and accept what you checked. This directory is the application; it knows
nothing about what a specification is about. The text it stores is whatever the
person writes, and what that text means is the authoring tools' business
(`tools/authoring`).

Today it does the first two parts: **works** (a titled piece of specification
text), **immutable revisions** of each, and a **save that refuses to overwrite**
text the caller has not seen; and the **model** of a work, kept beside the text
and shown as the figures **SCE draws of it** and what **SCE says of it** (its check
and the pseudocode page you read against your text): the application draws and
judges nothing, it runs `sce-codegen` and shows what the product writes. On top of
the same folder you **accept a design** against the requirement list your text was
read into, and the application says whether that acceptance still holds.

You can **ask for a model**: the button saves what you typed and registers a request, and an
AI executor takes it. The application hosts one when it finds Claude Code (a request is then
taken, written for and finished with no window of the client open), and an authoring client of
your own can take it instead (`works_begin_generation`). What the executor writes is kept
beside the work and becomes the work's model and requirement list together, as one bundle, only
when SCE accepts the model; the screen says where the request stands, where each answer
stands, which sentence of your text a question is about, and what a new model changed from the
one before.

An authoring client reaches the same folder through the MCP (`works_list`,
`works_read`, `works_save_model`, `works_save_requirements`, and the three that take,
finish and give up a request, in `tools/authoring`): it reads the text you saved and your
answers, writes the model and the requirement list, and saves them back after the product's
own check accepts them and your answers are kept to. They run `sce-work`, so the application
and the MCP cannot disagree about the folder because only one thing writes it. The text and the
answers are yours: no MCP tool writes either, none removes a work, and none accepts a design
for you.

What does not exist yet, so that nothing below is read as done:

- Installers for Windows and macOS. `scripts/package_app.sh` builds the Linux one (a `.deb`)
  and it is the only one tried: it carries the generator, `sce-work` and the authoring server
  (see "What an installer carries" below). The authoring server's launcher is a shell script
  that needs Python 3 with PyYAML and jsonschema; the `.deb` declares all three as dependencies, and a platform
  with no package manager to ask has to carry or install a Python itself, which nothing here
  does yet.
- The screens for the examples' results. They are designs, not code. The model
  screen shows what SCE drew, what SCE says of the model, where it stands to the
  text, a field for your answer under each question the model leaves open, and
  what you accepted; it does not edit the model (an AI client writes it) and it does
  not write the requirement list (an AI client reads your text into one).
- Windows has not been run on a window; the lane `windows` in `app.yml` runs the
  same gate there, and the Linux desktop build and the browser have been run. The hosted
  executor is tested against a stand-in client on Unix only.

## How the parts fit

```
app-core/        (root workspace member; no GUI, no network)
  the works folder, revisions, the JSON command layer, the `sce-work` CLI
app/
  src-tauri/     the desktop application: a window around ui/, one command
  web-shell/     the same commands over HTTP, for a machine with no display
  ui/            the screen (TypeScript, no framework)
```

Everything the screen can do is one call, `call(name, args)`. The window routes
it through Tauri's `invoke` and the browser shell through `POST /api/call`;
`sce-work call <name>` is the same call for a process that is not Rust, which the
authoring MCP uses. All of them end in `sce_app_core::call`, so there is one
definition of what a save is.

The works folder is plain files:

```
<root>/<work-id>/work.json            title, created_at
                 source/<sha256>.txt  one file per revision, named by its digest
                 source.head          the current revision, and which save of
                                      source.log made it current (`log <n>`)
                 source.log           one line per save: revision, parent, time
                 model/<sha256>.scxml the same, for the model
                 model.head           the current model
                 model.log            one line per save, and the source revision
                                      the writer said it read (`written_for`)
                 answers/<sha256>.json  your answers to the model's open questions
                 answers.head         the current answers
                 answers.log          one line per save
                 requirements/<sha256>.json  the requirement list the text was read into
                 requirements.head, requirements.log   (and the text it was read from)
                 acceptances/<sha256>.json   what you accepted, one file per acceptance
                 acceptances.head, acceptances.log
                 removed.json         only for a removed work (see below)
```

`<root>` is `SCE_WORKS_DIR` if set, else the per-user data directory
(`~/.local/share/sce-workbench/works` on Linux). A revision is the SHA-256 of
its exact bytes; a save names the revision it was written from and is refused
with `conflict` if that is no longer current. The text, the model, the answers,
the requirement list and the acceptances are five chains kept by one
implementation (one lock, one compare-and-swap).

A folder an older build wrote has pointers that are the digest alone, and nothing in it
says which of several saves of one digest the pointer meant: a save that failed leaves
the same log line a save that worked does. This build reads such a folder as it always
read, with one exception. When two saves of a model disagree about the text it was
written for, the model's text is left unstated (the screen's word for it is
`behind`, never `current`) rather than taken from the last of them, and the first save
that works settles it and writes the pointer in the form above. A save into an older
folder names the place first, so a save that fails leaves nothing in the log. What an
older folder cannot give back is the history of two such disagreeing saves: it may hold
the one that failed. `history` and `model_history` say so on that entry, with
`"unconfirmed": true` (a key present only where it is true). The mark is a reading of
the log, not something written into it, and it stays on the entry after a later save has
made the model current: the save of today does not make the one of the past knowable.
The model's screen does not show a model history; a client that does should show the mark.

### Reading a work as one state

A screen that reads a work with a command for each chain (`read_work`, `read_source`,
`read_model`, `read_answers`, `read_requirements`, `read_acceptance`) can be handed a
text of one moment beside a model of another: a save lands between two of the reads.
`read_work_snapshot` reads the work and its five chains as one state. The pointers of
all five are read, then what they name, then the pointers again, and the read starts
over when any of them moved; what is answered is the folder as the second look found it.

It takes no lock, so a screen that reads on every change never makes a save wait. That
holds because a pointer is replaced by one atomic rename and a revision's file is never
rewritten. A folder written to between every look cannot be read that way, and after five
tries the read takes the lock a save takes, so the answer is one state there too.

Each part of the answer is what the command that reads that chain answers, in the same
words: `source`, `model` with `model_standing`, `answers`, `requirements` with
`requirements_standing`, and `acceptance` as it was saved. It does not say whether the
acceptance still holds. That is the product's to say and asking it is not a read of the
folder, so the screen still asks `read_acceptance` for it.

`read_work_heads` is the same read without the texts: the revision at the head of each
chain and, for the model and the requirement list, the source each was written for. The
same model kept again for a later source is the same revision with another `written_for`,
and that is a change a screen has to see. It is read as one state too, and it is cheap
enough to ask every few seconds.

### The screen follows the work

The screen asks `read_work_heads` every two seconds for the work it shows, and compares
the answer with what is ON SCREEN, not with the previous answer: its own save moves a
head and the screen shows the saved text a moment later, and that is not a change from
elsewhere. What differs is read again, and only that: a model or a list an authoring
client saved, a text or answers saved from another window, an acceptance made elsewhere.
A model that is the same revision is not drawn again.

What the person typed is never replaced. A text or answers they are typing are left as
they are, and the conflict a save would meet stays theirs to resolve; when they hold
nothing again (the typing was undone), what was held back is read at the next question.
One question is asked at a time, and the next is scheduled when the last and what it led
to are done. Nothing is marked as seen: a part that was read stops differing from what the
core says, and that is all that stops it being read again. A read that failed (the text, a
model, a list, the answers) leaves the part differing, so the next question asks for it
again, and the failure reaches the wait: a question that fails is asked again after twice
the wait, up to thirty seconds. A part that could not be read is compared as unread, not
left out, because a part that is left out is never read again. A refusal for want of a token
stops the questions until the person signs in. A work taken away from another window is said
so, and the questions stop.

The text and the model are read apart, so the work can be saved between the two reads. A
model read beside a text other than the one in the editor is read again with the text when
the editor holds nothing of the person's, and it is never called current beside a text it
was not read for: the screen says the model was read beside another one until the two agree.

The desktop shell and the browser shell take this same path (`call`), so neither needs a
window permission, a stream or a file watcher the other does not. A change that was missed
is found by the next question instead of being lost with a notice.

### Asking for a model

A work can be asked for a model (`request_generation`): one **request**, kept in the work's
folder as `requests/<id>.json`, with every change of state a line of `requests.log`. The
request is registered against the revisions of the text and the owner's answers the caller
read (`expect`), and refused as `moved` when the work is no longer at them, as `accept` is.
A work has one open request at most: a second is refused as `active-request` unless it says
it replaces the first (`supersede`). The same call sent again (`key`) is the request it
already made, and a key used for other revisions is refused (`key-reused`).

An executor (an AI adapter, or the authoring server of a person's own terminal) takes the
request (`claim_request`), keeps it (`heartbeat_request`) and finishes it (`complete_request`,
`fail_request`); the owner can call it off (`cancel_request`). The states are `queued`,
`running`, `completed`, `failed`, `cancelled`, `interrupted` and `superseded`.

- **The claim is a lease and it runs out.** A running request whose lease ran out is read as
  `interrupted`, by the clock, with nothing written: `state` says what the clock reads and
  `stored_state` what was last written. A lease that ran out is a suspicion, not a
  revocation: an executor that slept past it and wakes with the run done still finishes it,
  because nobody else took the request.
- **What revokes a claim is something done.** The owner calls the request off, the text or
  the answers it was asked about are saved (the request is `superseded`, in the same step as
  the save), or somebody claims it again. A request that was let go of is taken again only by a
  caller that says it `resume`s, as the next **attempt**; every word an executor says names
  its attempt, and an attempt that is not the current one is refused (`not-holder`).
- **What the executor itself writes does not end its request.** A model and a requirement list
  move nothing a request was asked about.
- **Nothing is taken again unasked.** A run costs something, and the next one is the owner's
  to ask for.

The heads of a work (`read_work_heads`) say where its latest request stands, by the clock.

#### What a request makes

What an executor writes for a request is a **candidate** (`save_request_candidate`): a model, a
requirement list, or both, kept with the work's other revisions (named by what they hold) and
named in the request (`candidate`). Writing one moves no pointer: the work's model is still the
one it was, and `read_request_candidate` reads what was written. It becomes the work's only by
`complete_request`, which publishes it as one **bundle**.

- **A bundle is one file and one pointer.** `bundles/<digest>.json` names the model, the
  requirement list, the text and answers they were made from, the request and attempt that made
  them, and what was checked of them. `bundles.head` moves to it in one step, so a reader that
  takes the pointer has the model and the list of one generation, never a model of one beside a
  list of another. A work that has a bundle has its model and list from its current bundle
  (`read_model`, `read_requirements`, the snapshot and the heads agree with it), and the heads
  and the snapshot say which bundle (`bundle`).
- **The core checks the model itself.** `complete_request` runs SCE's check of the candidate
  model and records it as `by: core`, with the revision it ran on (`subject`). What the executor
  reports of a check only it can run (`checks` in the call) is kept as `by: client`, as
  reported. A bundle is published only with an accepted core check, and a check that was
  refused, of either kind, stops it (`check-refused`); a check run on a model the request has
  since written over is not a check of this one (`candidate-moved`). A product that does not
  answer is not a refusal: the request stays running and is said again.
- **A request completes by publishing.** Both halves are needed (`no-candidate`), the speaker
  is the holder of the current attempt, and the work is still at the text and answers the
  request was asked about: a save that moved them ended the request in the same step. Said
  again by the same attempt, it is the same bundle. The publication and the request's
  completion are written in one step under the work's lock, and a process that stopped between
  the two leaves a bundle that names its request: the next reader finds the request completed.
- **A work that has a bundle refuses the old saves of one half** (`save_model`,
  `save_requirements` answer `bundled-work`), because one half moved alone is the pair the
  bundle exists to keep from being read. The text and the answers are the owner's and are saved
  as ever. A work that never asked for a generation is read and written as it always was.
- **The bundle keeps what it took over.** The first bundle of a work that had a model or a list
  says which (`previous`), so the model's history continues where the chain stopped, and
  `bundle_history` lists the bundles. `read_bundle` reads one, or the current.
- **The bundle says what it replaced and what its executor worked to.** `replaces` is the bundle
  that was the work's model and list before it (none for the first), and `instructions` is the
  version of the working instructions the executor was given, in the executor's own words
  (`claude-code/<digest>` is the digest of the wording, the form of the answer and the tools the
  client may use; `sce-author-mcp/<digest>` is the digest of the instructions the authoring
  server gives its client), so a result can be told apart from one made to other wording. An
  executor that says nothing of them has none recorded. A selected house-rule pack is recorded
  only where one is applied, and the generation flow applies none.

#### What the screen shows of a request

A section above the model (`generationSection`, with its state in `generation_model.ts`) says
where the work's latest request stands, in the core's words and nothing it made up: no
percentage is invented for an AI that is working, and a request nobody is there to take is not
shown as running.

- **The button** (`Generate pseudocode`, then `Generate again`) saves what is typed first, then
  asks about the text and the answers THIS screen shows (`expect`): a save that conflicts is the
  person's to resolve and nothing is asked in the meantime, and a request the core refuses as
  `moved` (the work changed under the screen) says so and shows what is saved now. A press is
  one request however many times it is sent (`key`), and a second press while the first is on
  its way is not offered.
- **The states** are the core's: waiting for an AI (and, when none is connected, that nobody is
  there to take it, so the person can start an authoring client of their own or cancel), being
  written (who holds it, which attempt), let go of (nothing was published; asking again
  replaces it), failed (with the reason the executor gave), cancelled, ended because the text or
  the answers were saved, and finished. When the model it wrote is published the screen shows
  it without a press, as it shows any model that moves under it.
- **Saving while a request is open asks first**, for the text and for the answers: the core ends
  the request in the same step as the save, so what the AI writes would not be published, and
  that is the person's to decide (`Save and cancel the request`, or `Do not save`).
- **Which AIs are there** (`read_adapter_status`) is read with every question about the work,
  so that "no AI is connected" and "connected: desktop" follow the executor coming and going.
- **Where an answer stands** (`answer_states.ts`) is said under each answer, as four different
  facts and not one "saved": typed and not saved; saved, and not in the model shown; being
  written into a new model (an open request was made about these answers); in the model shown
  (the model was made after this very answer was given, from the bundle's own record of the
  answers it was about, and it no longer asks the question); or the model was made after the
  answer and still asks the question, which is said as it is. A model that no request made says
  nothing of what it was made from, and an answer is then only saved. None of it is a claim
  that the model means what the owner meant: the behaviour it led to is still the owner's to
  read. `Generate again from these answers` saves what is typed and asks about what is saved.

- **The sentence a question or a requirement is about** (`grounding_model.ts`) is not guessed from
  words. The product says where in the design each requirement is carried (`node_paths`, in SCE's
  path syntax), the list keeps the sentence each requirement quotes (its sidecar), and a question
  says where in the design it was asked (the same syntax). A question asked inside a part a
  requirement is carried by is about that requirement's sentence, and the most specific such part
  names it. The sentence is shown with the question and `Show in the text` selects it in the
  editor (white space is not told apart, so a sentence the text broke across lines is found); when
  the text no longer holds it, the screen says the requirement quotes an earlier text. Nothing is
  shown for a work with no list, or for a place no requirement carries.
- **From a requirement to the text and the pseudocode.** Each row of the requirement table can
  select its sentence in the editor, and `Mark in the pseudocode` lights the lines of the page
  that name the states the requirement is carried by. The page is the product's rendering and
  carries no address of what a line is for, so a state's name is what ties a line to a place, and
  the screen says that is what it did (and says so when no line names them).

- **What a new model changed from the one before** (`change_model.ts`, `line_diff.ts`) is the
  difference of the two models' pseudocode pages, line by line, with a few lines around each
  change. The earlier model is the one before this in the core's model history (a work whose
  models were made by requests lists them as bundles, after the ones it had before; a model
  published again unchanged replaced nothing, so the one before it is the one compared). It says
  nothing of what a change MEANS: a condition, a signal, a value or a time that moved is in the
  lines that hold it, and the owner reads them against the text. A first model has nothing to be
  compared with; a page SCE did not write is said not to be there.
- **An answer can be put into the text, by the owner and nobody else.** `Add to the text` (offered
  for an answer the core holds) adds its words to the end of the editor and saves nothing: they
  put it where it belongs and save, or do not. A specification is theirs, and an answer that is
  not in it stays a decision kept beside it.

#### The executor the application hosts

`runner.rs` is the host of an executor that is a process of the application: it finds the
requests nobody has taken (`list_open_requests`, every work's open requests oldest first),
takes the oldest one that is `queued`, and has a `Generator` (whatever writes a model from a
specification: an AI client, a script in a test) write for it. It is the same executor an
authoring client in a person's own terminal is, with the same words to the same folder.

- **It takes what was asked and nothing else.** A request a lease let go of is the owner's to
  ask again for; the runner never takes an `interrupted` one.
- **It keeps what it takes.** A generator can work for longer than a lease and never says it
  is still there, so the runner renews the claim, and reports the adapter, from a thread of
  its own. A renewal refused because the request ended (the owner called it off, or the text it
  was asked about was saved) stops the generator at its next look (`Cancel`), and nothing is
  renewed for the request again.
- **What it writes is a candidate, and the core decides.** The draft is saved for the request
  and completed through the step `complete_request` takes, so the core checks the model
  itself. A model the core refuses comes back with the product's own records (`records` in the
  refusal's `detail`), the generator is given them and writes again (twice by default), and a
  request that never gets a model the core accepts fails with what the core last said.
- **A request that ended under it is not failed.** What the owner or the text did to it is not
  the generator's failure. A runner told to stop leaves its request to run out the lease, which
  is how the owner reads that the executor went away.

**Claude Code as the generator** (`claude_code.rs`). One run of the headless client per draft,
in a folder of its own that is removed when the run is over, and given:

- **no built-in tool at all** (`--tools ""`): no shell, no files, no web. What it can do is what
  the SCE authoring server offers, and of that only reading the work it was started for
  (`works_read`, which also gives the owner's answers and the decision record they make) and
  checking what it writes (`validate_scxml`, `decisions`, `scxml_requirement_set` and the
  like). It has no tool that saves, takes a request or accepts: the application saves what it
  answers. A specification is text the owner may have pasted from anywhere, and a client that
  has read it should not be one that can be talked into writing to the owner's other work;
- **nothing from the machine's settings**: no hook, no `CLAUDE.md`, no other server;
- **an answer in one form** (`--json-schema`): the documents of the model and the requirement
  list exactly as it checked them. What it says of itself goes in on standard input, so it is
  not in a process listing, and a run is bounded by its turns, its time and optionally its cost.

A run that is cancelled is killed; one that stops with a status, says something that is not
JSON, or reports an error fails the request with what it said. A draft that is not a draft (a
manifest that is not JSON, documents that name each other wrongly) is told back with the
reason, as a refusal of the core is. The host says where the authoring server is installed
(`AuthorServer`: a checkout runs it out of the tree, a bundle ships a launcher), because that
is the one thing this side cannot know. `tests/claude_code.rs` holds this side against a
stand-in `claude`; `tests/claude_code_live.rs` (ignored, minutes and money) runs the real
client through the real server and product once, end to end.

**Who hosts it.** Both shells call `host::start`, so that what they host cannot differ for a
reason that lives in a shell: the desktop application (as `desktop`) and the browser shell used
while developing the screen (as `web-shell`). It finds what it needs the way the product's
generator is found (the environment, then what an installer carried, then beside the program,
then the search path) and a shell that cannot find it does not fail: it says what it looked for
and hosts nothing, and the application shows that no AI is connected and works as it always
did. Nobody has to have Claude Code to use the workbench.

| What | Environment | Otherwise |
|---|---|---|
| Claude Code | `SCE_CLAUDE` | `claude` on the search path |
| The authoring server's launcher | `SCE_AUTHOR_MCP` (a checkout has `scripts/sce_author_mcp.sh`) | the installer's `sce-author-mcp`, else beside the program or on the search path |
| `sce-work`, for the authoring server | `SCE_WORK` | the installer's, else beside the program or on the search path |
| The product, for the authoring server | `SCE_CODEGEN` | the installer's, else the product's own discovery |

**It asks the server before it takes a request.** Finding a launcher says nothing about whether
the server will start: with no Python, a Python module it needs missing or no generator it dies
at once, and from outside that is an AI that never answers. `host::start` runs the launcher with
`--check`, in the environment the client will give it, and a server that is not ready is a host
that hosts nothing, with the server's own words as the reason (`the product's generator is not
at ...: set SCE_CODEGEN to it`, `PyYAML is not installed for this Python: pip install pyyaml`).
What the server needs is said by the server (`tools/authoring/sce_author/needs.py` is the one
list of its Python modules), and nothing here copies it.

**What an installer carries.** An installer cannot rely on anything being installed beside the
application, so `scripts/package_app.sh` puts the authoring bundle
(`scripts/package_sce_author.sh`: `bin/sce-codegen`, `bin/sce-work`, `bin/sce-author-mcp`, the
templates and the Python package) in the application's resources (`sce-author/`, which
`tauri.conf.json` names) and has Tauri build the installer around it. The desktop application
looks there (`installed::in_bundle`) and uses what it finds where the environment names
nothing: a developer's variable still wins, and a development build, which carries nothing,
finds them as before. `scripts/verify_installed_app.sh` judges an installer by installing it:
the `.deb` is unpacked into a scratch directory and started under a virtual display with no
`SCE_*` variable set and the system's search path alone, and the application has to report an
executor (`.sce-hosts/desktop.json`); with the bundled generator removed it has to report that it
hosts none and name the generator, which is the control that shows the first answer came from the
installer and not from the machine. It also holds the `.deb`'s `Depends` to that list of Python
modules, since a machine that has a module the `.deb` never asks for would start it and pass it
(`python3-jsonschema` was missing until this check existed). An installer whose application looks for its bundle under
another name fails it (tried by renaming the folder in the shell and rebuilding). CI runs it as
the `installer` gate (`.github/workflows/installer.yml`); its first two runs passed (the first
cold, about twelve minutes). A stand-in row in
`sce-build/tests/ci_supersession_policy.rs` stands for the lane until it has runs of its own to
measure.

**A shell says what it is doing where the owner looks.** `host::start` never fails: that nothing
could be hosted is a state, and the shell reports it the way an adapter does (`.sce-hosts/<name>.json`,
every thirty seconds, counted while recent): that an executor runs and which client, or why none
does, in words the owner can act on (`no Claude Code to write models with (...): install it, or
set SCE_CLAUDE to its path`). The screen reads it (`read_host_status`) and, when no AI is
connected, says why under the generate button. A message on the standard error of a program
started from a menu is one nobody reads, and an installed application has no terminal. A shell
that stopped reporting is not counted: what it last said would be about nothing.

`SCE_EXECUTOR=off` hosts nothing, `SCE_CLAUDE_MODEL` names the model a run uses, and
`SCE_CLAUDE_BUDGET_USD` bounds what one run may cost (anything that is not a positive number is
no bound). Dropping the host, which the desktop application does when it exits, stops the
runner and kills a client at work: closing the window does not leave a run nobody is waiting
for.

An AI adapter says it is there by reporting (`report_adapter`: its name, its kind and what it
can do) and is there for ninety seconds after its last report. `read_adapter_status` lists the
adapters that ever reported with whether each is there now, so the screen can say that no AI is
connected, and offer only what the connected one can do. These records are soft state beside the
works (`.sce-adapters/`), not a fact about any work.

### Removing a work

`remove_work` (the screen's "Remove this work", after it asks) takes a work out
of the list and refuses every later read and save of it as `not-found`. It does
not delete anything: the work's files stay, with a `removed.json` beside them,
because a specification is a person's writing and the one command that can end
it should not be the one that cannot be taken back. Deleting `removed.json`
brings the work back whole, history included; deleting the folder is the removal
that cannot be undone, done by the person who means it. The marker is written
under the work's lock and a save re-checks for it once it holds the lock, so a
save that was waiting when the removal came writes nothing. No MCP tool removes a
work.

### The model, and where it stands

A model is saved for a text revision (`written_for`), so "is this model about the
text I am looking at" is a comparison of two digests, and the core says the answer
in one word every shell shows the same way (`standing`): `current`, `behind` (the
text moved on), or `unstated`. The same model saved again for a later text is a
new entry of its history, not an `unchanged`: the writer read the new text and
kept the model, and that is a fact about the model.

**A model of several documents.** A statechart written the way the authoring tools
ask for one imports an event schema for each event and closes its interface, so the
model is a set of documents that name each other by file name. The store still keeps
ONE text per revision: a set is a JSON object holding every document under its file
name (`model_set.rs`), a model of one document is the document itself exactly as it
always was, and the two are told apart by their first character (`<` or `{`). The
same documents are the same bytes however they were listed, so saving them again is
no change. `save_model` takes `text` (one document) or `documents` and an `entry`
(several), and a model that is read says its `entry` and lists its `documents`, the
entry first. Every file name is one plain name (letters, digits, `_`, `.`, `-`; no
directory) and is checked again when the set is written out for SCE, because it
becomes a path there. SCE is asked about the whole set: the entry under its own file
name and every other document beside it, where an import finds it. With a closed
interface, a set whose schema is not staged is refused by the product
(`import/file-not-found`), which is how the tests know the schema reached it.

`figures` runs the generator: the first of `SCE_CODEGEN`, an `sce-codegen` beside
the running program, or one on `PATH`. It stages the model under the work's own
name (the product names its figures by its document's file), waits for it at most
30 seconds, writes its output to files and not pipes, and reads back only files
inside the folder it was given. What SCE refuses (a figure that does not fit its
page, a character its font table has no width for, a document it does not
accept) reaches the screen as the product's own code and sentence, with the model
still shown as text. The screen asks for the language it is in (`--lexicon`),
shows each sheet as an `<img>` (an image never runs script), at 150% unless the
person chooses another size, and scrolls a wide sheet sideways instead of
shrinking it.

### What SCE says of a model

`review` runs the same generator twice and passes on what it writes:
`sce-codegen check --lint` (the product's verdict on the document, every record it
wrote with its code and line, what an accepted model still leaves open in the
product's own sentences, and the questions the model marks `sce:unresolved`) and
`sce-codegen pseudo` (the page, in the screen's language, kept byte for byte). A
model SCE refuses is an **answer** (`check.verdict` is `refused`, with its
records and no page); SCE not answering at all is a refusal of the same
`sce-*` kinds `figures` has. When SCE accepts a model and will not write its page,
the verdict is kept and the page's own refusal is shown beside it.

⚠ "Accepted" is the product's verdict on the document. The screen says once, next
to it, that it does not say the model agrees with your text, and nothing on it
claims that: you compare the page with your own words.

### Closing the desktop window

A browser tab is asked "leave this page?" by the browser; a window is not. So the
screen tells the shell whether it holds anything the core has not been given (the
editor's text, or answers typed and not saved), and when the window is asked to
close with something unsaved the shell (`src-tauri/src/close_gate.rs`) holds the
close and runs a fixed script that has the SCREEN ask: save and close, discard and
close, or stay. Only the person's choice lets the window go (`sce_close`); a window
with nothing unsaved closes at once. The commands the window may call are
`sce_call`, `sce_unsaved` and `sce_close`, and nothing else.

⚠ A window that cannot be closed is its own failure. If the screen was asked and says
nothing for three seconds (it crashed, or never loaded), the next close request goes
through: a screen that does not answer protects nothing. That is decided from the
times in `close_gate.rs`, and its tests say so.

Checked on a real window (Linux, under a virtual display, with the close request
sent the way a window manager's close button sends it): held with unsaved text;
"discard and close" closes; "save, then close" saves the text and closes; a window
with nothing unsaved closes at once.

### Your answers to what the model leaves open

A model marks what your text did not say (`sce:unresolved="open-guard"`, with the
question in its own words as `sce:unresolved-reason`). The screen puts a field
under each such question; what you write is saved by `save_answers` as ONE map
(the question's id to your words) on top of the revision it was read as, and the
core refuses a revision that is no longer current, so answers saved from another
window are never overwritten unseen. The screen then keeps what you typed and
loads what turned up.

- An answer is stamped with the time its words last CHANGED, not the time of the
  save that carried it; saving what is already saved is no change at all.
- An answer to a question the model no longer asks is kept apart and kept: the
  next draft may ask it again. Clear a field and save to take an answer back.
- The map is the application's own. The decision record the product's checks
  read is made by the authoring package from it (`works_read` hands the client
  `decisions_text`), so the format of THAT record stays where it is read.
  `works_save_model` holds a draft to the answers with the product's own
  `decisions` check: one that leaves an answered question open, or guesses where
  no answer licenses it, is not saved.
- Nothing here changes the model. Your answers reach the authoring client the next
  time it reads the work, and it applies them.

### Accepting a design

An authoring client reads your text into a **requirement list** (`save_requirements`:
two JSON files, the manifest of coordinates and the sidecar of quoted sentences,
kept as written because an acceptance pins their bytes) and writes the design. The
screen then shows what the product finds, and nothing the application worked out:

- `requirements_report` runs `sce-codegen requirements --manifest` and
  `acceptance-report`: each requirement in SCE's own word (`implemented`,
  `missing`, `needs-scenario`, or one a later SCE adds, shown as spelled), where
  the text anchors it and where the design carries it, and the page you read
  before deciding, byte for byte. It also gives the `basis`: the revisions of the
  text, the model, the list and your answers, which is exactly what you were shown.
- `accept` takes that `basis` back as `expect`. If any of it has moved (the text
  saved from another window, the client's next model, an answer saved since) nothing
  is accepted, the refusal is `moved` and names which, and the screen reads what is
  there now before you can press again. A design or a list written for an earlier
  text is refused as `not-current`. Otherwise the core stages the work in a fixed
  layout (`design/`, `spec/`), asks the product to take the record
  (`sce-codegen accept --channel direct`), and keeps it as a new entry of the
  acceptances chain. The record pins every file by path and hash.
- `read_acceptance` asks the product whether the record still holds
  (`acceptance-check`) for the work as it is now. It is the product that says, by the
  bytes of each pinned file: a design put back to what was accepted is the design that
  was accepted. When it no longer holds the screen shows the product's own sentence of
  what moved.

What this does not claim, stated so nothing on the screen is read as more:

- **A design with a gap is yours to accept.** The screen tells you before you press:
  SCE's count marks the requirements it finds unsettled (`missing`, `unresolved`,
  `dangling`, `contradicted`, `scenario-failed`, `needs-scenario`), each of SCE's
  eleven words is shown with what the product means by it, and a word SCE adds later
  is shown as spelled and not guessed at; the matters SCE lists as left open are
  counted beside the button. The record keeps what was open. Accepting closes none of
  them and is not refused for them.
- **The channel is a statement and not a proof.** `direct` is how this application's
  own button states itself; the product records it and cannot verify it. An
  acceptance a client relayed on your word would say `relayed`, and the screen says
  so. No MCP tool records an acceptance, because the command layer states every one
  it records as `direct`.
- **The product's lapse is one sentence.** It joins its lapses with `; ` and one lapse
  can contain one, so the sentence is passed on whole and not split into a list the
  product does not state.
- **Your answers are pinned, not applied.** The product keeps the answers file as a
  pinned input and does not read it, so the record's list of what was open is the
  product's statement about the design and does not shrink when you answer in the
  application. The screen shows your answers beside it.
- **The sidecar is not pinned.** The product pins the manifest and the files the design
  read; the sentences sidecar only feeds the page, so editing it does not lapse an
  acceptance.
- **What is accepted is what is saved.** The button is withheld while text or answers
  are typed and not saved, while SCE has not measured the design, while the design
  or the list was written for an earlier text, and while the text, the design or the
  answers on screen are not the ones the page measured (a text saved from another
  entrance moves under the screen, and the page is then of a newer text than the one
  being read: the screen is read again before it can be accepted). An answer sheet the
  screen has not read yet is not "no answers": the button waits for it and says so
  (`unread`), because a page measured with answers cannot be shown against none.
- **The list is the one that was measured.** The requirement list and SCE's measure of it
  are asked for apart, and a list saved between the two is not the one SCE measured: the
  sentences the screen quotes would be of one list, and the outcomes and the basis an accept
  names of another, so the owner would read a requirement by a sentence of the old list and
  accept the new one. The list read is therefore held to the measure's basis (`requirements`
  is part of what the button compares). When they differ the pair is read again, three
  times at most, and a list that keeps moving shows no sentence and offers no accept until a
  later question reads a pair that agrees.
- **Whether the acceptance holds is said of the work that was measured.** `read_acceptance`
  says whether an acceptance still holds and names, as `now`, the work it judged; the report
  names the work it measured as its `basis`. Both come from the same reading of the work, and
  they are asked for apart, so a model saved between the two leaves a "holds" beside a design it
  was not judged of, or a "lapsed" of the model before it was put back, and nothing about the
  acceptance record changes for a later question to notice. The screen does not work the
  standing out itself; it holds `now` to the basis and reads the pair again, three times at
  most. What still differs is shown as not known yet (neither held nor lapsed), the accept
  waits (`unread`), and the next question reads it again because the part is compared as unread.

## Seeing the screen

### In a browser, on a machine with no display (and from a phone)

```bash
cd app/ui && npm ci && npm run build
cd .. && cargo run -p sce-web-shell -- --ui ui/dist --listen 127.0.0.1:5174
```

It prints an address ending in `#token=…`; open that. The token is generated
per run (or set `SCE_WEB_TOKEN`, 16 characters or more), is sent as a bearer
header, and is removed from the address bar once read.

To open it from a phone: either listen on the machine's Tailscale address
(`--listen 100.x.y.z:5174`) and open the printed address with the Tailscale app
running on the phone, or keep it on `127.0.0.1` and forward the port over SSH
(`ssh -L 5174:127.0.0.1:5174 <machine>`).

The browser shell listens only on loopback and Tailscale addresses
(100.64.0.0/10, fd7a:115c:a1e0::/48); any other address, `0.0.0.0` included, is
refused. It is a separate program from the application, so a release build of the
application cannot contain it. Every wait on a client is bounded (headers, body,
number of connections), so a client that stops partway cannot hold the server.

### While changing the screen

```bash
cargo run -p sce-web-shell            # commands on :5174
cd ui && npm run dev                  # screen on :5173, forwards /api to :5174
```

### The desktop application

Needs Tauri's system libraries (on Debian/Ubuntu: `libwebkit2gtk-4.1-dev
libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`; on Windows the
WebView2 runtime). Then `cargo tauri dev` or `cargo tauri build` from
`app/src-tauri`. The window may call the one registered command and nothing
else: it has no file-system, shell or network permission.

## Tests

| What | Command |
|---|---|
| Works folder and command layer | `cargo test -p sce-app-core --features cli` |
| The model chain and `figures` (a stand-in generator, Unix) | `--test models`, `--test figures` of the same package |
| Removing a work, and a save racing it | `--test removal` of the same package |
| A work read as one state, with a writer saving while it is read | `--test snapshot` (and `--lib`) of the same package |
| Requests: leases, attempts, supersession by a save, callers racing at the lock | `--test requests`, `--test request_commands` (and `--lib` for the state machine) |
| Candidates and bundles: publishing, the core's own check, readers that never see two generations, a stopped publication, the old saves refused | `--test bundles` (and `--lib`) of the same package |
| The runner that hosts a generator: taking, renewing, repairing, ending | `--test runner` of the same package |
| Claude Code as the generator, against a stand-in client (Unix) | `--test claude_code` of the same package |
| A shell that hosts the executor: the settings, what it says when it cannot (including a server that would not start), taking a request, stopping | `--test host` of the same package |
| The programs an installer carries, found in its bundle and named when missing | `--test installed` of the same package |
| The authoring server saying whether it could start (`--check`) | `python3 -m unittest tests.test_the_server_says_whether_it_can_do_its_work_before_it_is_asked_to` (in `tools/authoring`, with `PYTHONPATH=.`) |
| The installer, installed and started (Linux `.deb`) | `scripts/package_app.sh --debug`, then `scripts/verify_installed_app.sh <the .deb it prints>`; CI runs both as the `installer` gate (`installer.yml`) |
| The real client, server and product end to end (a model runs: minutes and money) | `cargo test -p sce-app-core --features cli --test claude_code_live -- --ignored --nocapture` |
| Which AI adapters are there | `--test adapters` of the same package |
| A model of several documents (`model_set.rs`, staging, the command's shapes) | `--lib`, `--test model_sets`, `--test figures` of the same package |
| The owner's answers: the chain, stamps, conflicts | `--test answers` (and `--lib`) of the same package |
| What SCE says of a model (a stand-in generator, and the real one with `SCE_CODEGEN`) | `--test figures` of the same package |
| Requirements and acceptance: the commands, and the product behind them (a stand-in, and the real generator with `SCE_CODEGEN`) | `--test acceptance`, `--test acceptance_product` (and `--lib`) of the same package |
| The MCP's works tools, against the real `sce-work` and generator | `python3 -m unittest tests.test_the_works_folder_is_reached_through_the_applications_own_command` (in `tools/authoring`, with `PYTHONPATH=.`) |
| The same with the real generator | `SCE_CODEGEN=<path to sce-codegen> cargo test -p sce-app-core --test figures` (skipped, and says so, without it) |
| Browser shell: handler, sockets | `cargo test -p sce-web-shell` (from `app/`) |
| Screen: guards, editor model, model panel, acceptance, heads, following the work, transport, words | `npm test` (in `app/ui`) |
| Screen types and build | `npm run build` (in `app/ui`) |

`app-core/contract/replies.json` is every command's reply, written by running
the commands. `app-core/tests/contract.rs` fails when a reply changes without
that file, and `app/ui/test/contract.test.ts` feeds the same file to the
screen's guards, so a screen and a core that disagree fail a test instead of
rendering `undefined`.

## Licence

The application is under the repository's licence. The only code from outside
that reaches a user at run time is `@tauri-apps/api` (MIT OR Apache-2.0) in the
screen's bundle, and Tauri and its Rust dependencies in the desktop binary;
`ui/test/dependencies.test.ts` pins the set of runtime npm packages and their
licences, so a new one has to be added there on purpose.
