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
(`~/.local/share/sce-workbench/works` on Linux). The person's own settings are not in it: they
are in `SCE_SETTINGS_DIR` if set, else the per-user configuration directory
(`~/.config/sce-workbench/settings` on Linux), and the browser shell reads them with
`--settings DIR` and never changes them. A revision is the SHA-256 of
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
folder.

`read_judgment` is what SCE says of revisions that were read: it takes the `basis` (the
revisions of the text, the model, the list and the answers, as the snapshot gave them) and
the revision of the acceptance when there is one, and answers whether that acceptance holds
for exactly those revisions (`acceptance`, `null` when none was named) and SCE's measure of
that design against that list (`report`: the outcomes and the page of `requirements_report`, in
its words). A revision's file is never rewritten, so the answer is of the revisions named and of
no other, whatever has been saved since; one the work does not keep is `not-found`.

The answer is a function of the revisions' content and carries nothing of which text the model
and the list were written for. The same bytes can be kept again for a text that came later: the
revision is the same and so is what SCE says of it, while that claim moves, and the store answers
a revision with the latest claim made of it. So where the design stands to the text (`current`,
`behind`, `unstated`) is said by the snapshot, in the state the revisions were read in, and the
screen puts it beside SCE's measure; asked again of the same revisions after the design was kept
for another text, the answer is the same. (`requirements_report`, which judges the work as it
stands, still says it.) Each of the two is
`{"said": ...}`, or `{"refused": ...}` when SCE did not answer: a design SCE cannot draw is
still the design, and the screen has it from the snapshot. The core answers `basis` as it
read it, and the screen refuses an answer that names other revisions than it asked about
(`Api.readJudgment`), as a broken contract and not as a verdict to show.

That is what keeps a verdict beside the design it is of. The screen reads the work as one
snapshot and then asks SCE about that snapshot's revisions by name, so it does not need the
core to judge the work "as it stands" (`read_acceptance`, `requirements_report`, which still
do) and then compare what came back with what it shows: those are asked a moment apart, and a
save lands between them. The snapshot needs no SCE, so the model is shown without waiting for
it.

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
A model that is the same revision is not drawn again. A model, a list or an acceptance that
moved is read again as the work: the snapshot is read and SCE is asked about its revisions,
so the design, the list and the verdict beside them are of one state. Answers that were read
again are asked about too, since they are part of what an acceptance is of.

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
does, in words the owner can act on (`no AI client to write models with (...): install Claude
Code, or set SCE_CLAUDE to its path`). The screen reads it (`read_host_status`) and, when no AI is
connected, says why under the generate button. A message on the standard error of a program
started from a menu is one nobody reads, and an installed application has no terminal. A shell
that stopped reporting is not counted: what it last said would be about nothing.

**Either client is enough where connections are kept.** A shell that has the person's settings
runs a request for the connection it was made for, and a connection says which client and which
credential, so a computer with Codex and no Claude Code hosts too (`SCE_CODEX` names the program,
or `codex` is beside the program or on the search path). A shell without the settings runs the
requests nobody chose a connection for, and Codex has no credential to run those on, so it still
needs Claude Code. The executor reports itself as Claude Code when that is there and as Codex when
it is the only one. This build supports Codex CLI **0.159.0 on Linux**, recorded in
`app-core/data/codex_support.json`. Other versions and operating systems wait with a reason until
they have been verified with the real client and the current execution contract.

**A model server is found when the person registers one.** It is no program to look for: a
connection names it (`adapter` `local`), and the executor then needs the authoring server and
nothing else, so a computer with no client at all hosts once a server is registered (looked for
again as the clients are, so one registered with the window open is run for within ten seconds),
and the executor reports itself as `local`. With no client and no server it says where it looked
for each and that a server is a way out. The application is the one that talks to the authoring
server for such a run, as a client of its protocol, and offers the model the same nine tools the
other clients are offered and no others (a model that reads of a tool tries it: one was seen
calling a tool that saves, so the model is told which it has). It talks to the model server in the
OpenAI chat protocol with tool calls (`POST {address}/chat/completions`), carries each call and its
result between the two, and takes the model's last message as its draft. The requirement list is
the one the tool that builds it returned, not a copy the model types out (measured against a real
server: a copy of four thousand characters can be wrong), and the form of the answer is given as an
example to fill in, because a model that is given a JSON Schema in its conversation says it back. A
draft the core does not accept is told back to the model, which writes again. So is a tool call the
server could not read: when the arguments a model wrote for a call are not JSON, Ollama answers
500 (`error parsing tool call`) and there is no message to give back, so the model is told why and
makes the call again, twice at most, counted apart from drafts that are not one. Any other server
error ends the run.

Measured on 2026-10-06 and 07 against Ollama on a machine with one 16 GB GPU, one live run of the
whole chain per line unless it says more, and not a claim about any other computer or run:

| Model | What came of it |
|---|---|
| `qwen3-coder:30b` | accepted by the core, in about five minutes (305 s, and 164 s when asked for through the screen's commands) |
| `gpt-oss:120b` | three runs: ended by a 500 for a call whose arguments were not JSON (before that was told back to the model); not done in 25 minutes, after sixteen turns of putting a design right at a minute and a half each; accepted in 637 s |
| `devstral:24b` | not accepted after 613 s: its first message gave the request's own ids back as if they were the draft, and its last gave a draft with no documents, and it never wrote a design |

A model that does not carry out the work (it is told to call tools, write SCXML, have the product
check it, and answer with the draft) fails in the model and not in the adapter, and the run says
what the model last said of it. Which model is large enough is the person's to find: a limit of
minutes in the live test's environment (`SCE_LOCAL_MINUTES`) is what lets a slow one finish.

**A request ends when it is told to or when its time is up, while it is being sent as well as while
it is answered.** A server may read none of what is sent to it (a model that is busy, a connection
that stalled), and what is sent can be longer than any buffer between the application and the
server: the model server over HTTP is written to in slices of a tenth of a second, as an answer is
waited for, and the authoring server is written to on a thread of its own, so that a caller is not
held by a write that the other end does not take. Measured before it was so: a request given half a
second ended after the system's own timeouts for a write (about fifteen seconds, as a failure to
send), and a call to an authoring server that read nothing waited until that process ended.

**The clients are looked for again while the application runs.** A person installs a client with
the window open, and one that was there goes away. The executor asks for each client again (at most
once in ten seconds) whenever a request needs it, so Claude Code installed after Codex, or Codex
after Claude Code, is run for within that time without closing the application, and a program that
has gone is said to be not found and not to be a client that could not be asked something. What
the executor reports itself as follows what it finds.

**Where a Codex login is kept is one place for a run, the check and the commands.** A run ignores
the person's Codex settings file (`--ignore-user-config`), and the client's `login status` has no
such flag and reads it, so a file that says the login is in the system's keychain made the screen
say somebody was signed in whom a run could not see. A run, the check and the sign-in commands the
screen offers (`codex login -c cli_auth_credentials_store=file`) are all told the same store. What
else a settings file can say about a login (`forced_login_method`, `forced_chatgpt_workspace_id`, a
model provider) cannot be unsaid on the command line, and is part of what a version is verified
against: the name of a verified version covers the arguments a run is started with, so a change to
them is another version.

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

### The ways a person reaches a model

A connection is the settings of one way to reach a model: which client (Claude Code, Codex) or
which server of the person's own, which model, and where the client finds its credentials. It is
kept in the person's settings folder (`SCE_SETTINGS_DIR`, else the per-user configuration
directory), **not** in the works folder: a works folder is shared and moved, and whose account a
person reaches a model with is no part of a work. No secret is in a connection. The credential
is the client's own login, a variable the person chose, or a key the operating system holds, and
a connection says only which. A server address takes no user, query or fragment, and a file
with a field the store does not know is not read.

A revision is the digest of its bytes and an earlier one stays readable, so a request that
pins a connection by id and revision can still say what it was made with after the connection
changed or was deleted. A save names the revision it replaces (`conflict` otherwise), as a save
of a work's text does.

`list_connections`, `read_connection` and `read_auth_policy` read; `save_connection`,
`delete_connection` and `set_default_connection` change. **Only the desktop entrance changes
settings**: the browser shell is a development tool a token reaches over a network, and
`sce-work` is what an AI reaches the works through, and an AI must not be able to change which AI
it runs on. They answer `not-allowed-here`, and an entrance with no settings folder answers
`no-settings`. `describe` says which entrance asked and whether it may change them, and whether
it may start a program (`starts_programs`).

**A connection names a program only among the ones the application found.** A connection names
a program the application runs, so a command that took any path would be a way to make it run
anything. `find_clients` (the desktop window only, because asking starts the programs) lists the
Claude Code programs the application finds, each with the version it says: those on the search
path, then the folders the official installer uses under the person's home (a window started from
a menu may not have them on its path), then the system's own. Only a program that says it is
Claude Code is listed, and one reached by two names once. `save_connection` keeps an `executable`
only when it is one of those found again at the time of the save (`bad-connection` otherwise,
whatever the file says of itself), and a kind of connection with no program to find takes none.
What the connection names is then what runs for it and what the settings screen asks who is
signed in to; if it is gone or no longer says it is Claude Code when a request is to run, the
request waits and says so.

`read_auth_policy` is the table of ways of signing in that this build uses, each allowed,
conditional, forbidden or unconfirmed, with the decision each gets. What a provider's terms
allow is a fact about a way of reaching its model (a subscription, a key, a cloud provider's
credential), so the table is of routes, and it is a constant of the build: a person cannot widen
it, a release can switch a route off, and a route that is not in it is not used. What each row
rests on (the provider's terms as they read, and the decision the owner took where they are
silent) is said at the row, in `app-core/src/auth_policy.rs`.

**A request is made for a connection.** `request_generation` may name one (`connection`: its id
and the revision the screen read). The core, not the screen, copies from the person's settings
what a run needs to be the same run (`pin`: which client, which model, what it may spend) and
refuses as `moved` when the connection is no longer at that revision; a later change of the
settings moves no request already made. It copies nothing that says where or as whom: no
server address, no name the person gave it, no path of a program, because a works folder is
shared and the executor that needs those reads the connection at the pinned revision from the
settings of whoever runs it. The connection is not an input of the request's key: the same press
sent again is the request it made, with the pin it made it with, even if the connection changed
since.

An executor takes a request by offering the connection it runs for (`claim_request` with
`connection`), and a request is taken by the executor of its own connection and by no other
(`wrong-connection`): a person who chose one AI is never answered by another. A request made
for none (one an AI client asks for through the authoring server) is taken by an executor that
offers none. This is a check against an executor that is honest and mistaken (an old build, a
second adapter), not against a process that lies about what it runs for: the holder's name is a
string the caller chooses.

**The application's own executor runs for connections.** Where the shell has the person's
settings, the executor also takes the requests made for a connection. For each it reads the
connection at the revision the request pinned (settings made on another computer are said to be
missing, not guessed at), asks Claude Code who is signed in with the options a generation runs
with (`--setting-sources ""`, so that what the screen shows is the login a generation uses and
not one a settings file supplies), judges that way of signing in by the build's table, and runs
with the pinned model and limits. It offers that connection when it takes the request. A request
it cannot run stays queued, and the runner says what it left and why (`Runner::waiting`): the
client was not found, nobody is signed in, the way of signing in is not one the build uses, no
model is chosen for a server, a server wants a key this build has no place to keep, the settings
are not on this computer.

**What the runner left, and why, is said where the screen reads it.** The shell's report
(`.sce-hosts/<name>.json`, read by `read_host_status`) carries `waiting`: for each request the
executor could not run, the ids of the work, the request and the connection, and one sentence. The
works folder is shared and moved, so the sentence is the application's and holds no path and
nothing a client printed (the settings screen shows that to the person who is looking; it is not
kept). The screen says it beside the request, and offers to ask again for the connection chosen
now, which replaces the one that waits.

**A request nobody chose a connection for is not run by the application.** An executor that runs
for connections (`Runner::with_connections`) leaves such a request alone, and no default is
assigned to it: which AI it was meant for is the person's to say. It waits for an authoring client
of the person's own, as it always could. A request the application made (`origin` `gui`) for no
connection is one from before the person chose, so the screen says that the application does not
run it and offers the same "generate again with the chosen connection", or opens the AI connection
to choose one. An executor that knows no connections (one generator: a command line tool, a test)
still takes what nobody chose a connection for.

**The settings screen.** The AI connection panel (sidebar) shows whether Claude Code is installed
and who is signed in, asked the way a generation runs the client (`read_claude_status`: only the
desktop window may ask, because the command starts the person's program). A way of signing in the
build does not use is shown as the login it is, with the reason, and offers no model. When nobody is
signed in the panel gives the two commands to run in a terminal (`claude auth login` for a
subscription, `claude auth login --console` for billing by use) with a button to copy each, and
asks again when the person says they have signed in: the application starts no sign-in. The model
is chosen among the names the client documents, the client's own default, or an id typed beside the
list; saving writes the connection `claude` on top of the revision read, keeps the limits a person
set elsewhere, and makes it the default, expecting the default that was read, so that two windows
do not overwrite each other unseen. Which Claude Code the connection runs is chosen among the
programs `find_clients` listed (the application's own choice is the first): the one that answered
is shown with where it is, a program the connection names that is not found now stays in the list
and is said to be gone, a save of the model keeps the program the connection already names, and
where nobody is signed in to the program that answered a button keeps another one at once. **That
button keeps the program and does not make the connection the default**: the default is the
person's word for which AI a request is made for, and one that cannot run would leave every later
request waiting (a working default is not replaced by a Codex this build did not verify because the
person chose another program for it). Only the save that is offered when a request made for the
connection would run makes it the default. The status is asked about the connection the panel edits
(`read_claude_status` and `read_codex_status` take it as `connection`), and the program that
answers is the one that connection names, whether or not it is the default: asked about the
default, a program kept for a connection that is not one would be ignored and the login of another
program shown. The status is asked again after a save, of the program that is named now. The browser shell only
shows what the desktop saved: it may not start a program or change a setting.

**A program chosen and not saved is the one whose login is asked.** Both status commands take
`executable` beside `connection`: a path asks of that program (it must be one the application
found for that client, or the command refuses it as `bad-connection`), `null` asks of the
application's own choice, and leaving it out asks of the program the connection names. Choosing
another program in the list asks again at once, and an answer that comes after a later choice is
dropped, so the login shown beside a program is that program's. Without this, a login that was
verified for one program stayed beside another one chosen after it, and the save offered as the
default was a save the person had not been shown to run.

**The panel is for one way to reach a model at a time: Claude Code, Codex or a server of the
person's own.** A choice of the three is at its top and starts on the one the default connection is
for (Claude Code when there is none). Each is asked the first time it is looked at, and what was
chosen for one is kept while another is looked at. Each keeps its own connection (`claude`, `codex`,
`server`), and a save makes the one that is shown the default: it is the person's word for which AI
a request is made for.

For Codex (`read_codex_status`, which starts the person's program and so is the desktop window's)
the panel shows the program and its version, whether this build verified that version, and who is
signed in by the source that is chosen among three: the official client's own login, a login the
application keeps of its own in a folder of its own, or a key in the environment. Each source is
asked on its own, the way a generation runs it, so the screen can say of the one chosen what a
request would find. A source nobody is signed in by gives the commands that sign in to it (with the
folder to start the client in, for the application's own login) or, for a key, the variable to set
where the application starts; a login the build does not use is shown as the login it is.

Somebody is signed in when the client says so and ends well. Anything else it prints (a folder it
was started in that is not there, a settings file it cannot read) is the client not saying who is
signed in: the source is `unknown`, with what the client said, and a request waits for it to be
asked again. It is never read as a login, which is what showed the application's own login as
signed in when its folder was not there. That folder is made, with only its owner able to enter it,
by whichever asks first, the check of who is signed in or a run, because the client does not start
in a `CODEX_HOME` that is not there and the commands that sign in start it in this one.

One line says whether a request made for the connection would run and, if not, the first thing it
waits for, in the order the things are true in: a program that is not there or is not Codex, a
version this build did not verify (whoever is signed in), then the login. The save is offered only
when a request would run, because a connection that only waits is not one to make the default;
while it would wait, another Codex program can still be chosen among those found, which is how a
person reaches a version this build did verify. Linux Codex 0.159.0 is verified; an updated client
or another operating system still needs verification before generation is offered.

**Codex verification (2026-10-07).** The ignored `app-core/tests/codex_live.rs` tests use the real
CLI, an existing official ChatGPT login, `gpt-6.1-sol`, the authoring MCP and `sce-codegen`.
Both a synthetic indicator specification and a version containing instructions to read a private
canary, write an unrelated file and use forbidden tools completed generation. The tests assert
actual MCP calls for source reading, kind selection, validation and requirement checking, core
publication and pseudocode rendering. They also check the Workbench status response and assert
that nothing happened in a run but the model's words, its plan, its errors and calls to the
authoring server (so no command, web search, image, file change or hand-off to a second run),
that the canary was not disclosed, the unrelated file was not written and the source was not
changed. This checks integration and these attacks; the owner still reviews whether the
generated behavior matches the specification.

The adapter passes its developer instructions explicitly, uses strict structured output, ignores
personal configuration, rule files and project instructions, and switches off unrelated features.
Code Mode's host stays enabled because this CLI needs it to call MCP tools. Its presence does not
enable shell tools: `shell_tool` is disabled. Some legacy flags (including `unified_exec`) still
appear enabled after `--disable`, so live event traces, rather than feature names alone, are checked.
**`unified_exec` cannot be switched off in this CLI**: with 0.159.0, `--disable unified_exec` and
`-c features.unified_exec=false` both leave it on, so no setting makes a command impossible. What
stands between a specification that asks for one and the machine is the read-only sandbox (which
allows no writes, though a command can still read what the person's account can read) and the
model not asking. The live tests show a model that did not ask, which is a fact about that model
and that version and not a guarantee: another model, or a new version, is verified again.
The reviewed metadata/UI features introduce no additional authoring server or executable tool.
Only the listed read/check tools receive unattended approval overrides, using Codex's
[per-tool MCP configuration](https://learn.chatgpt.com/docs/config-file/config-reference).
The authoring server additionally enforces `SCE_AUTHOR_WORK`: it advertises only those tools,
refuses mutations and reads of other works, and accepts draft documents and their imports only
as inline text under plain file names. Every document is read before it reaches the product,
whatever its file is called, and is refused when an attribute that names a file (`src`, `href`,
on any element) is not the plain name of a companion staged beside it, when it changes the base
such names resolve against (`xml:base`), declares a document type, or cannot be read as XML.
`companions_text` carries imported documents to the individual-document checkers, including
`scxml_requirements`. The tools it advertises are the ones the application approves
(`AUTHOR_TOOLS`), and a test holds the two lists equal.

After building `sce-codegen`, reproduce the shipped configuration with a signed-in Codex:

```sh
cargo build -p sce-build --features cli --bin sce-codegen
SCE_LIVE_MODEL=gpt-6.1-sol cargo test -p sce-app-core --features cli --test codex_live -- --ignored --nocapture --test-threads=1
```

These tests run a paid model. Traces and the rendered pseudocode contain only synthetic test
material and remain in the build's temporary folder. To evaluate an unregistered version,
`SCE_CODEX_VERIFY=1` creates an in-memory candidate entry **only in this ignored test**; it never
changes the application's support list. Register a version only after both tests pass. Changes
to the prompt, schema, arguments or feature lists invalidate the recorded contract digest;
the offline support test catches a stale entry.

For a server of the person's own (Ollama, LM Studio, llama.cpp, vLLM, anything that speaks the
OpenAI chat protocol with tool calls) nothing is installed or signed in to, so the panel asks the
server. The person gives it a name, an address (a button fills each of the usual ones) and a model,
and presses check: `read_server_status`, which calls an address and so is the desktop window's, asks
the server for its models (`GET /models`, with no key and nothing of a work) and says which of five
things came of it: a list (offered for the model field, where an id that is not listed can still be
typed), a server that is not there, a certificate that was refused, a server that wants a key, or an
answer that is not a list of models (the address is usually the server's OpenAI-compatible root,
such as `http://127.0.0.1:11434/v1`). What was checked belongs to the address it was checked at: a
changed address is not said to be what the old one was.

**Where the specification goes is said beside the address.** That it is sent to the server is said
whatever the answer; it is said louder when the address is another computer over plain http (what
nothing encrypts is read by the network in between) and quieter over https. An address on this
computer is not a promise that it stays there, because a tunnel to another computer looks the same,
so a connection to a server carries the name the person gave it and the screen shows that beside
the generate button. An address over https is held to the roots of the web (`webpki-roots`, with
`rustls`, so a build needs no TLS library of the system's) and to the name in the address: a server
that fails either is sent nothing, and a certificate that a private authority made is not trusted
yet, so such a server is reached through a tunnel or a proxy of the person's own over http. A
server that wants a key cannot be used by this build, which has no place to keep one: the panel
says so and offers no save for it, and a connection that asks for one waits and says why. The save
writes the connection `server` (or the connection to a server that is already the default,
whatever it was named, because it may have been saved by something else), without a key, keeps the
limits a person set elsewhere, and is not offered until a name, an address and a model are given.

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
- **The measure, the standing and the sentences are of the design beside them.** The screen
  reads the work as one snapshot (`read_work_snapshot`): the design, the requirement list and
  the acceptance record are of one state. It then asks SCE (`read_judgment`) about that
  snapshot's revisions by name, so the outcomes, whether the acceptance holds and the basis an
  accept names are of the design and the list on screen, and the sentences the screen quotes
  are the list's own. Nothing is compared to find that out. What the screens before this one
  did was read the list, the measure and the standing with a command each, each of the work as
  it stood when it was asked, and compare them: a save landing between two of the reads
  handed them a sentence of one list beside the outcomes of another, then a "holds" beside a
  design that had changed, then a "lapsed" beside one that was put back, and each comparison
  closed the case before it.
- **A panel is shown only beside the snapshot it is of.** When the work is read again the panel
  stays while SCE is asked if it is of that snapshot (the same revisions and the same
  acceptance), so a reread after a save does not flash. When it is not, it is replaced by
  "reading" at once: the old verdict is not left beside the new design for as long as SCE takes
  to answer, and a verdict that arrives for a read that has since been replaced is dropped.
  SCE is asked after the work is read, so the design is shown without waiting for it. A panel of
  the same revisions takes where the design and the list stand to the text from the new snapshot
  at once, while SCE's words, which are of the bytes, stay: the same bytes kept again for another
  text move that claim without moving a revision, and the accept follows the snapshot and not the
  claim of the read before, however long SCE takes.
- **What the screen cannot read for the person is still compared.** The text and the answers
  are the person's: they are typed over and are read apart from the design, so what the panel
  is of can differ from what is on screen. The accept button waits for them (`differs`,
  `unread`), and whether the acceptance holds is not said of a text or answers that are not the
  ones it was judged of: it is shown as not known yet, neither held nor lapsed. What the panel
  is of is also compared with the core's heads on every question, because the acceptance
  record keeps its revision while the work moves and the answers it is of may be the part the
  screen cannot read; a panel of a work the core has left is asked for again.
- **SCE not answering is an answer, and the screen asks again only when asking again could
  change it.** `read_judgment` can answer that SCE did not measure the design, or did not say
  whether the acceptance holds (`refused`, with the kind and SCE's words). The panel keeps the
  reason and shows what it did get: a measure when only the verdict failed, and the acceptance
  as saved when only the measure failed. A run that timed out, failed, or could not be started
  (`sce-timeout`, `sce-failed`, `sce-unavailable`) is not a fact about the design and passes:
  the saved work is what it was, so nothing else would make the screen ask, and the panel is
  compared as unread and asked again at the pace of the watch (twice as long after each
  failure, up to thirty seconds), with no press of "Read again". A design SCE refused
  (`sce-refused`) says the same of the same revisions, so the reason is shown and it is not
  asked again until the work has moved. While SCE has not said whether the acceptance holds,
  whether this design is accepted already is not known and the accept button waits
  (`unjudged`).

## Seeing the screen

### In a browser, on a machine with no display (and from a phone)

```bash
cd app/ui && npm ci && npm run build
cd .. && cargo run -p sce-web-shell -- --ui ui/dist --listen 127.0.0.1:5174
```

It prints an address ending in `#token=…`; open that. The token is generated
per run (or set `SCE_WEB_TOKEN`, 16 characters or more), is sent as a bearer
header, and is removed from the address bar once read.

A browser may read the person's settings and may not change them or start a program, so it
cannot choose an AI connection, and the application's executor does not run a request that
nobody chose one for. To ask for a model from the browser, either save a connection in the
desktop application first (the same settings folder), or start the shell with
`--claude-connection`: that is the operator's word and not a browser's, and it saves a
connection to Claude Code (the model `SCE_CLAUDE_MODEL` names, when set) as the default only
when nobody has a default. A default somebody chose is never replaced.

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
