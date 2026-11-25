#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The gate table as the run of a gate reads it: one entry per gate, each a
# file of its own under `tools/git-hooks/gates/`, read by its slug, and the
# derivations the runner asks of the gates it runs (their dependency closure
# and order, where each runs, the part a shard runs, the submodules each
# needs).
#
# ── One file per gate ─────────────────────────────────────────────
#
# bx keeps a unit's verdict under the trace of what its run read (bx RFC 0084),
# so a file the run reads whole is a file whose every change starts the unit
# again. Until this file existed the runner answered each of those questions
# by loading `gate_registry.py` whole: the closure and the order, the
# placement and the submodule preflight each ran it, and every unit read
# 9257 lines of selection, plan, self-test and cost bookkeeping to learn a few
# fields of its own gate (traced 2026-11-25 with Python's audit hook, three
# gates: every step of every unit opened the registry and listed its
# directory or the root). bx measured the registry changing in 20 of 231
# landings, so no unit could keep a verdict across more than 91% of them
# whatever its gate reads; 97 of the 121 commits that touched the registry
# from 2026-10-25 to 2026-11-25 changed an entry, nearly always one.
#
# So the entries are data, apart from the code. A gate's entry is
# `tools/git-hooks/gates/<slug>.py`: one Python literal, a dict, with the
# reasons for its values as comments beside them. It is read by
# `read_entry`, which evaluates literals and the three names in
# `ENTRY_NAMES` and refuses anything else, so an entry runs no code and
# names nothing this file does not define. A unit's run reads the runner,
# this file and the entries of the gates it runs, and `trigger_globs.py`
# only when its preflight needs a gate's trigger globs and what its builds
# read (a gate keyed on them that needs a submodule); `a-gate-run-reads-its-own-entries` in the
# registry's self-test traces it doing so. The selection of the gates a change
# reaches, the plan, the self-test and the measured costs' bookkeeping stay in
# `gate_registry.py`, which the hooks, the plan and the hosted lanes run,
# and which loads every entry through `load_table`.
#
# The runner asks (`scripts/gate`, `scripts/lib/require_submodules.sh`):
#   gate_table.py order <slug>...        the named gates and what they depend
#                                        on, in run order
#   gate_table.py placement <slug>...    where each gate of a run in that
#                                        order runs, 'host <slug>' or
#                                        'container <slug> <containerfile>'
#   gate_table.py shard-env <slug>       the variable the gate reads its part
#                                        from, or nothing
#   gate_table.py submodules [--which] <slug>...
#                                        the gates that need submodules, and
#                                        which (see require_submodules.sh)
#
# Run with `python3 -I`: no directory goes on `sys.path`, so the
# interpreter lists neither this directory nor the working one to import the
# standard library, and the run reads the files named above and no others.

from __future__ import annotations

import ast
import re
import subprocess
import sys
import types
from collections.abc import Mapping
from pathlib import Path

# One module under one name, however it was started: as the runner's
# program (`__main__`), or loaded by the registry. `trigger_globs.py`
# imports from it by this name, and finds it here without reading a file.
sys.modules.setdefault("gate_table", sys.modules[__name__])

HERE = Path(__file__).resolve().parent

# Where the entries are, and the same directory as a repository path: the
# input a unit's key names for each gate it runs (`entry_input`).
ENTRIES = HERE / "gates"
ENTRIES_INPUT = "tools/git-hooks/gates"

# The shape of a slug, which is the name of the entry's file and of the
# gate's script. A name of any other shape names no entry, so a slug can
# never reach outside the entries' directory.
SLUG = re.compile(r"^[a-z0-9][a-z0-9-]*$")


def sibling(name: str) -> types.ModuleType:
    """The module `<name>.py` beside this file, compiled from its source.

    Not imported: an import looks for a cached compilation under
    `__pycache__/`, which no unit's inputs name, and writes one. Each
    module is loaded once per process and kept under its name, so a second
    caller and an `import` of it find the same one.
    """
    if name in sys.modules:
        return sys.modules[name]
    path = HERE / f"{name}.py"
    module = types.ModuleType(name)
    module.__file__ = str(path)
    sys.modules[name] = module
    try:
        exec(compile(path.read_text(encoding="utf-8"), str(path), "exec"),
             module.__dict__)
    except BaseException:
        del sys.modules[name]
        raise
    return module


# `parallelism` is how many processes one run of the gate's OWN script can keep
# busy; a unit asks bx for the largest figure among the gates it runs
# (`parallelism` on a plan line, bx RFC 0004 section 6, named `cores` before bx
# RFC 0081 4.8 renamed it). It is the upper end of the job slots a unit is
# given in its batch: its own jobserver and `BX_GRANT_CORES` (bx RFC 0062,
# 4.5), which `scripts/lib/sce_build_jobs.sh` hands to the builds. Admission
# counts no cores (bx RFC 0081): a busy machine shares its cores by weight, and
# the figure caps what the unit's builds start, not what it may take from
# another run. bx does not learn a unit's figure (bx RFC 0062, 7), so the figure
# here is the whole of what the unit is given. A unit that named none
# was given one slot. MEASURED 2026-11-21: in a landing whose units named
# none, a unit's run was given one core and used 1.3 on average, one unit was
# still running after 7547s, and the landing's judge reached its four-hour
# deadline three times without a verdict.
#
# Every figure is the one of its class, as `peak_gb`'s are until a run is
# read: `PARALLELISM_COMPILE` for a gate that compiles, `PARALLELISM_SCRIPT` for a script
# that runs built tools over the tree. Eight is the core count of the smallest
# build machines the fleet has: a compiling unit is given all of one of those
# and a share of a larger one. A tree build parallelises past eight, but a
# floor above a machine's cores is capped there, and on a busy machine the
# floor is a weight the kernel shares, not cores taken from another run.
PARALLELISM_COMPILE = 8
PARALLELISM_SCRIPT = 1

# A gate whose entry says `"inputs": INPUTS_ARE_TRIGGERS` is keyed on its
# trigger globs (`gate_triggers`): the same derivation that decides whether a
# change selects the gate decides which changes its kept pass outlives (ADR
# 0120, D2). A gate a change does not select is one the change cannot affect,
# so a key over the same globs is that claim restated, and bx tests it: a run
# in the copy that reads outside the globs is "could not measure" (bx RFC 0020
# 5.2), which is also a file the selection was missing. A gate takes the value
# once `bx --checks prepush --learn <slug>` has shown its reads inside its
# triggers; until then it keeps `None` and costs nothing it did not cost.
# A gate that names a shared build (`builds`) is keyed as well on what cargo
# reads for that build without compiling it (`gate_reads`, "What a build
# reads that does not select it"), which changes no verdict and so selects
# nothing.
INPUTS_ARE_TRIGGERS = "triggers"

# The names an entry may use for a value, and nothing else: each stands for
# a figure or a mark whose reason is written once, above.
ENTRY_NAMES = {
    "PARALLELISM_COMPILE": PARALLELISM_COMPILE,
    "PARALLELISM_SCRIPT": PARALLELISM_SCRIPT,
    "INPUTS_ARE_TRIGGERS": INPUTS_ARE_TRIGGERS,
}


class EntryError(ValueError):
    """An entry that is not data: a value other than a literal or a name of
    `ENTRY_NAMES`, or a file that holds no single dict."""


def entry_path(slug: str) -> Path:
    """The file of `slug`'s entry, whether or not it exists."""
    return ENTRIES / f"{slug}.py"


def entry_input(slug: str) -> str:
    """`slug`'s entry as a repository path, the way a unit's inputs name it."""
    return f"{ENTRIES_INPUT}/{slug}.py"


def _value(node: ast.AST, where: str):
    """The value an entry's expression `node` spells, or `EntryError`."""
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.Name) and node.id in ENTRY_NAMES:
        return ENTRY_NAMES[node.id]
    if isinstance(node, ast.List):
        return [_value(item, where) for item in node.elts]
    if isinstance(node, ast.Tuple):
        return tuple(_value(item, where) for item in node.elts)
    if isinstance(node, ast.Dict) and None not in node.keys:
        return {_value(k, where): _value(v, where)
                for k, v in zip(node.keys, node.values)}
    if (isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub)
            and isinstance(node.operand, ast.Constant)
            and isinstance(node.operand.value, (int, float))):
        return -node.operand.value
    raise EntryError(f"{where}, line {getattr(node, 'lineno', '?')}: "
                     f"{type(node).__name__} is not data; an entry holds "
                     f"literals and the names {sorted(ENTRY_NAMES)}")


def parse_entry(text: str, where: str) -> dict:
    """The entry `text` spells: one dict literal, comments allowed."""
    try:
        tree = ast.parse(text, filename=where, mode="eval")
    except SyntaxError as exc:
        raise EntryError(f"{where}: {exc}") from None
    if not isinstance(tree.body, ast.Dict):
        raise EntryError(f"{where}: an entry is one dict literal")
    entry = _value(tree.body, where)
    if not all(isinstance(key, str) for key in entry):
        raise EntryError(f"{where}: every key of an entry is a string")
    return entry


def read_entry(slug: str) -> dict:
    """The entry of `slug`; `KeyError` when no gate has that name."""
    path = entry_path(slug)
    if not SLUG.match(slug) or not path.is_file():
        raise KeyError(slug)
    return parse_entry(path.read_text(encoding="utf-8"), entry_input(slug))


def entry_source(slug: str) -> list[str]:
    """The lines of `slug`'s entry, for the registry's cases that read the
    reasons written beside the values."""
    return entry_path(slug).read_text(encoding="utf-8").splitlines()


def entry_slugs() -> list[str]:
    """Every gate, by the names of the entries' files, sorted."""
    return sorted(path.stem for path in ENTRIES.glob("*.py"))


def load_table() -> dict[str, dict]:
    """Every entry, keyed by slug and in slug order: the registry's table.

    A directory with no entry is refused, not read as a table of no gates:
    every selection over an empty table is "no gate applies", a pass, so a
    checkout that lost the directory would pass every push it judged.
    """
    slugs = entry_slugs()
    if not slugs:
        raise EntryError(f"no gate entry under {ENTRIES}, so there is no "
                         f"table to select from")
    return {slug: read_entry(slug) for slug in slugs}


class EntryTable(Mapping):
    """The table read an entry at a time: a gate's entry is read when it is
    asked for, and only then. What the runner hands the derivations below,
    so a run reads the entries of the gates it is about and no others.
    Iterating lists every entry, which the runner never does."""

    def __init__(self) -> None:
        self._read: dict[str, dict] = {}

    def __getitem__(self, slug: str) -> dict:
        if slug not in self._read:
            self._read[slug] = read_entry(slug)
        return self._read[slug]

    def __iter__(self):
        return iter(entry_slugs())

    def __len__(self) -> int:
        return len(entry_slugs())


def transitive_deps(slug: str, table) -> set[str]:
    """Every gate `slug` needs, directly or through another gate."""
    seen: set[str] = set()
    stack = list(table[slug].get("deps", []))
    while stack:
        d = stack.pop()
        if d in seen:
            continue
        seen.add(d)
        stack.extend(table[d].get("deps", []))
    return seen


def run_order(slugs, table) -> list[str]:
    """Order the given gates: dependencies first, then cheapest first.

    Two properties, in that priority. A gate never runs before something it
    executes the output of, and among gates that are free to move, the one
    that can fail soonest goes first. Nothing about the order is written
    down as an order — it falls out of `deps` and `cost_s`, so adding a gate
    cannot silently push a cheap check behind an expensive one the way the
    hand-placed sequence did.

    A gate with no measured cost sorts last among the gates available at
    that moment rather than being guessed at: an unmeasured gate should not
    claim a cheap slot it has not earned.

    One gate is taken at a time, not one dependency level at a time. Level
    order looks equivalent and is not: it puts every dependent behind every
    independent gate, so a 1s gate that only needs the 1s generator build
    would run after the 399s vendor smoke. Taking the cheapest currently
    available gate lets a dependency unlock cheap work that then goes
    straight to the front.
    """
    remaining = set(slugs)
    ordered: list[str] = []
    while remaining:
        ready = [s for s in remaining
                 if all(d not in remaining for d in table[s].get("deps", []))]
        if not ready:
            # A dependency cycle is a registry bug, not a runtime condition.
            raise ValueError(f"dependency cycle among {sorted(remaining)}")
        pick = min(ready, key=lambda s: (table[s].get("cost_s") is None,
                                         table[s].get("cost_s") or 0.0,
                                         s))
        ordered.append(pick)
        remaining.discard(pick)
    return ordered


def closure_order(slugs, table) -> list[str]:
    """The named gates and every gate they depend on, in run order.

    What a run of named gates runs: asking for `forge-cpp` alone must not
    run it against a generator that was never built. `KeyError` for a
    slug no gate has.
    """
    want = set(slugs)
    for slug in list(want):
        want |= transitive_deps(slug, table)
    return run_order(want, table)


def container_placement(order, table) -> list[tuple[str, str, str]]:
    """Where each gate of a run in `order` runs: `(where, slug, containerfile)`.

    `where` is "host" (the machine the runner is on) or "container" (the
    image `containerfile` builds, where the runner runs the gate with its
    dependencies). A dependency that only gates in an image need is left out:
    it runs inside, with them. One that a gate on the host also needs stays,
    since that gate executes its output here. The order of `order` is kept.
    """
    in_image = [s for s in order if table[s].get("container")]
    inside: set[str] = set()
    for slug in in_image:
        inside |= transitive_deps(slug, table)
    host = {s for s in order if s not in in_image and s not in inside}
    for slug in list(host):
        host |= transitive_deps(slug, table) & set(order)
    placed = []
    for slug in order:
        if slug in in_image:
            placed.append(("container", slug, table[slug]["container"]))
        elif slug in host:
            placed.append(("host", slug, ""))
    return placed


def gate_inputs(slug, table, triggers=None, reads=None):
    """What `slug`'s OWN script reads, as its key names it: `None` for the
    whole tracked tree, or a sorted list of patterns. A declared list is
    itself; `INPUTS_ARE_TRIGGERS` is the gate's trigger globs from
    `triggers` (`gate_triggers`), which a caller computes once per plan,
    the listing patterns in its `lists` (a directory the script lists,
    whose names bx keys and whose contents it does not: bx RFC 0020, 4.2),
    and what the builds it names read without selecting it, from `reads`
    (`gate_reads`). Neither a listing nor such a read is a trigger: a name
    new in such a directory is selected by the glob that matches it, or
    reads nothing the verdict depends on.

    A gate keyed on its triggers that names a build and is given no `reads`
    is refused, as one given no `triggers` is: its key would miss what its
    build reads."""
    own = table[slug].get("inputs")
    if own != INPUTS_ARE_TRIGGERS:
        return own
    if triggers is None:
        raise ValueError(f"{slug} is keyed on its triggers and none were given")
    if table[slug].get("builds") and (reads is None or slug not in reads):
        raise ValueError(f"{slug} is keyed on its triggers and builds "
                         f"{table[slug]['builds']}, and what those read was "
                         f"not given")
    return sorted(dict.fromkeys(list(triggers[slug])
                                + list(table[slug].get("lists", []))
                                + list((reads or {}).get(slug, []))))


def glob_to_regex(glob: str) -> re.Pattern:
    """Translate a GitHub Actions path glob into an anchored regex.

    Handles the three shapes the workflows actually use — `dir/**`,
    `**/*.ext`, and literal paths — plus the single-segment `*` that
    `backends/*/forge-runtime/**` needs. `**` crosses separators, `*` does
    not, matching Actions' own semantics.
    """
    out = ["^"]
    i = 0
    while i < len(glob):
        c = glob[i]
        if glob.startswith("**/", i):
            out.append("(.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        else:
            out.append(re.escape(c))
            i += 1
    out.append("$")
    return re.compile("".join(out))


def submodule_paths(repo_root: Path) -> list[str]:
    """The paths of the submodules `.gitmodules` records, sorted."""
    done = subprocess.run(
        ["git", "config", "-f", str(repo_root / ".gitmodules"),
         "--get-regexp", r"^submodule\..*\.path$"],
        capture_output=True, text=True, check=False)
    return sorted(line.split(None, 1)[1]
                  for line in done.stdout.splitlines() if " " in line)


def reaches_beneath(pattern: str, path: str) -> bool:
    """Can input pattern `pattern` name a file beneath directory `path`?

    Asked a segment at a time, so `third_party/*/*` reaches
    `third_party/spdlog` and `web/visualizer/vendor/**` does not. A
    pattern that ends in `/` gives a directory's names and nothing they
    hold (bx RFC 0020, section 4.2), so it reaches nothing beneath them.
    """
    if pattern.endswith("/"):
        return False
    parts = path.split("/")
    for i, seg in enumerate(pattern.split("/")):
        if seg == "**" or i == len(parts):
            return True
        if not glob_to_regex(seg).match(parts[i]):
            return False
    return False


def submodules_reached(patterns, paths) -> list[str]:
    """Those of the submodules at `paths` beneath which `patterns` name a file.

    A narrowed unit's copy holds a submodule checked out only when its
    inputs give the submodule's `<path>/.git`, and holds every other one as
    a bare gitlink (bx RFC 0026, section 4.1, 2026-11-23). So the submodule
    preflight asks a narrowed gate for these alone
    (`scripts/lib/require_submodules.sh`), and
    `a-narrowed-gate-holds-the-submodules-it-reads` refuses an entry that
    reaches one without its `.git`.
    """
    return [p for p in paths if any(reaches_beneath(g, p) for g in patterns)]


# ── The submodules a gate needs ──────────────────────────────────
#
# What `scripts/lib/require_submodules.sh` asks before a run: which of the
# gates need submodules checked out, read from the job that runs each in the
# workflows its entry mirrors (that script's header says why per job, and why
# a narrowed gate needs only those its inputs reach).

def declares_submodules(text):
    """True if this YAML really asks `actions/checkout` for submodules.

    Comment lines are stripped FIRST. `tree-hygiene.yml` explains its own
    exemption in the words it is exempt from — "No `submodules: recursive`:
    the gate skips third_party/" — and a substring search reads that comment
    as the opposite of what it says. Measured 2026-08-24: it put both gates
    mirroring that workflow on the wrong side of this answer.
    """
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        code = stripped.split("#", 1)[0]
        if code.startswith("submodules:") and "recursive" in code:
            return True
    return False


def jobs_of(text):
    """`(job name, job body)` for each job in a workflow.

    A job is a two-space key under `jobs:`, which is the whole grammar this
    needs — the answer being derived is one step's `with:` value, and a YAML
    parser is not in the standard library. A file whose shape this cannot
    read yields no jobs, and the caller falls back to the file's answer.
    """
    out, name, body = [], None, []
    in_jobs = False
    for line in text.splitlines():
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        header = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if header:
            if name is not None:
                out.append((name, "\n".join(body)))
            name, body = header.group(1), []
            continue
        body.append(line)
    if name is not None:
        out.append((name, "\n".join(body)))
    return out


def runs_slug(body, slug):
    """Does this job body invoke `scripts/gate <slug>`?

    The trailing boundary is load-bearing: without it `w3c-python` matches
    the job running `scripts/gate w3c-python-bindings`, and the two jobs
    check out differently.
    """
    return re.search(r"\bgate\s+" + re.escape(slug) + r"(?![\w.-])", body) is not None


def submodule_needs(repo_root: Path, slugs, table, which: bool = False):
    """The lines the preflight reads for `slugs`, those that need submodules.

    With `which`, each such slug alone. Otherwise `slug<TAB>scope`, the
    scope being `*` (every submodule) or the paths of those the gate's
    inputs reach (`gate_inputs`, the key's own reading of the entry). A
    slug no entry names is left out, as a gate that needs nothing.
    """
    workflow_dir = repo_root / ".github" / "workflows"
    submodules = None
    for slug in slugs:
        try:
            entry = table[slug]
        except KeyError:
            continue
        texts = []
        for workflow in entry.get("workflows", []):
            path = workflow_dir / workflow
            if path.is_file():
                texts.append(path.read_text(encoding="utf-8"))
        running = [body for text in texts for _, body in jobs_of(text)
                   if runs_slug(body, slug)]
        if running:
            # The job that runs it is the one whose checkout it has to mirror.
            needs = any(declares_submodules(body) for body in running)
        else:
            # No job names it: fall back to the file, which refuses rather
            # than guessing that a gate it cannot place needs nothing.
            needs = any(declares_submodules(text) for text in texts)
        if not needs:
            continue
        if which:
            yield slug
            continue
        triggers = reads = None
        if entry.get("inputs") == INPUTS_ARE_TRIGGERS:
            # A gate keyed on its triggers reaches what they and its builds'
            # reads name, derived for this gate alone: the unit's copy holds
            # its inputs and no other gate's (`trigger_globs.py`, read only
            # here).
            derive = sibling("trigger_globs")
            triggers = {slug: derive.triggers_of(repo_root, entry)}
            reads = {slug: derive.reads_of(repo_root, entry)}
        own = gate_inputs(slug, table, triggers, reads)
        if own is None:
            yield f"{slug}\t*"
            continue
        if submodules is None:
            submodules = submodule_paths(repo_root)
        yield f"{slug}\t" + " ".join(submodules_reached(own, submodules))


def main(argv: list[str]) -> int:
    """The runner's questions; see the header for each."""
    if not argv or argv[0] not in ("order", "placement", "shard-env",
                                   "submodules"):
        print("usage: gate_table.py order|placement|shard-env|submodules "
              "[--which] <slug>...", file=sys.stderr)
        return 2
    question, slugs = argv[0], argv[1:]
    which = question == "submodules" and slugs[:1] == ["--which"]
    if which:
        slugs = slugs[1:]
    table = EntryTable()
    if question == "submodules":
        repo_root = HERE.parent.parent
        for line in submodule_needs(repo_root, slugs, table, which):
            print(line)
        return 0
    unknown = sorted(s for s in set(slugs) if s not in table)
    if unknown:
        # The runner's own words for a name it does not know: a hook, a lane
        # or a person reads them, and the status, before anything runs.
        print("gate: unknown slug(s): " + ", ".join(sorted(unknown)), file=sys.stderr)
        return 2
    if question == "order":
        print("\n".join(closure_order(slugs, table)))
    elif question == "placement":
        for where, slug, containerfile in container_placement(slugs, table):
            print(f"{where} {slug} {containerfile}".rstrip())
    elif len(slugs) != 1:
        print("gate: shard-env takes one slug", file=sys.stderr)
        return 2
    else:
        print(table[slugs[0]].get("shard_env", ""))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))