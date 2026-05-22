#!/usr/bin/env bash
# Commit self-audit hook for Claude Code.
#
# Fires as a PreToolUse hook on every Bash call. For `git commit`
# invocations, blocks the first attempt against the current HEAD,
# prints three audit questions (textbook / hack / YAGNI) to stderr
# so Claude sees them, and records the HEAD sha in a marker file.
# Claude answers the questions in-conversation, re-runs the same
# commit command; the second attempt finds the marker matches and
# passes through silently.
#
# Marker is per-HEAD: once a new commit lands (HEAD moves), the next
# `git commit` is audited again. This gives one audit per commit, no
# infinite-loop risk.
#
# Non-commit bash calls and internal git invocations (rev-parse,
# status, diff) exit 0 immediately.

set -euo pipefail

# ── Fail-closed on any internal hook error ──────────────────────
#
# Claude Code treats a hook exit of 1 as "hook errored, allow the
# tool call". That is the wrong default for an AUDIT hook — silent
# bypass is exactly what lets self-inflicted regressions (e.g. an
# over-broad Edit that deletes the `CMD=...` assignment below and
# trips `set -u`) slip commits through unaudited. This trap converts
# any uncaught failure into exit 2 (block), so a broken hook fails
# closed. The two explicit `exit 0` / `exit 2` paths below stay
# intact — they run before the trap would fire.
trap 'rc=$?; echo "commit_audit.sh internal failure at line ${LINENO:-?} (exit ${rc})" >&2; exit 2' ERR

PAYLOAD="$(cat)"

read_field() {
  local key="$1"
  printf '%s' "$PAYLOAD" | python3 -c '
import json, sys
key = sys.argv[1]
payload = json.load(sys.stdin)
cur = payload
for part in key.split("."):
    if isinstance(cur, dict):
        cur = cur.get(part, "")
    else:
        cur = ""
        break
print(cur if isinstance(cur, str) else "")
' "$key"
}

CMD="$(read_field tool_input.command)"
CWD="$(read_field cwd)"

# Belt-and-suspenders self-integrity check. The ERR trap above catches
# most uncaught failures, but bash's `${VAR?msg}` parameter-substitution
# exit bypasses ERR and falls out as exit 1 — exactly the silent-allow
# status we are trying to avoid. Explicit `exit 2` keeps the fail-closed
# contract. Empty string is valid (Bash tool sometimes ships commands
# with empty cwd fields); unset is not.
if [ -z "${CMD+x}" ]; then
  echo "commit_audit.sh: CMD not initialized — hook script broken at the CMD extraction line" >&2
  exit 2
fi
if [ -z "${CWD+x}" ]; then
  echo "commit_audit.sh: CWD not initialized — hook script broken at the CWD extraction line" >&2
  exit 2
fi

# Detect `git commit` robustly.
#
# The previous classifier regex-split on &&/||/;/| *before* shlex.
# That destroyed shell-quoting context: a `;` inside a `-m` body
# (or inside `$(cat <<'EOF' ... ; ... EOF)`) was treated as a command
# separator, leaving fragment segments with mismatched quotes that
# shlex then rejected with ValueError — which the loop silently
# skipped, causing the entire commit to be classified "not a commit"
# and the audit to be bypassed.
#
# Correct approach: shlex.split the *entire* command once. Posix
# mode preserves quoted strings as single tokens and emits shell
# operator tokens (`&&`, `||`, `;`, `|`, `&`) as their own tokens
# only when unquoted. We then walk the token stream, marking each
# operator as a segment boundary, and check each segment for the
# `git [flags...] commit` shape.
IS_COMMIT="$(printf '%s' "$CMD" | python3 -c '
import shlex, sys
cmd = sys.stdin.read()
try:
    tokens = shlex.split(cmd, comments=False, posix=True)
except ValueError:
    # Genuinely malformed input (mismatched quotes at the top
    # level). Fail-closed is tempting but noisy; classify as
    # not-a-commit so the hook does not spam on every broken
    # pipeline. A broken commit would fail git itself anyway.
    print("no")
    sys.exit(0)

GIT_FLAGS_WITH_ARG = {"-C", "-c", "--git-dir", "--work-tree",
                     "--namespace", "--exec-path", "--super-prefix"}

# Classify: does any position in the token stream start a `git
# [flags...] commit` sub-command? Earlier versions tracked segment
# boundaries by the shell operators `;` / `&&` / `|` / `&`, but
# shlex treats newlines as whitespace — so a Bash tool invocation
# like `rm -f marker\ngit commit -m ...` tokenises to a single flat
# list where the operator-boundary scan only inspects position 0
# ("rm") and misses the later `git commit`. Scanning every "git"
# token instead catches commits regardless of separator style
# (newlines, operators, sudo / env prefixes) because shlex already
# emits "git" as a bare token only when it is whitespace-bounded —
# `"git commit"` as a quoted literal becomes a single token with a
# space, which `tokens[i] == "git"` does not match.
for i, tok in enumerate(tokens):
    if tok != "git":
        continue
    j = i + 1
    # Skip `-C path`, `-c key=value`, and single-token flags.
    while j < len(tokens) and tokens[j].startswith("-"):
        if tokens[j] in GIT_FLAGS_WITH_ARG and j + 1 < len(tokens):
            j += 2
        else:
            j += 1
    if j < len(tokens) and tokens[j] == "commit":
        print("yes")
        sys.exit(0)
print("no")
')"

if [ "$IS_COMMIT" != "yes" ]; then
  exit 0
fi

# ── Architecture hard-block detector ─────────────────────────────
#
# Scan the staged diff for patterns explicitly forbidden by
# CLAUDE.md's "Guiding Rules" and "Code Comments" sections. These are
# deterministic violations that a human audit cannot waive by simply
# answering the questions below — the author has to remove or rewrite
# the offending line before any commit is allowed. No marker is
# written, so retrying the same `git commit` without editing keeps
# failing; only a clean stage proceeds to the self-audit path.
#
# Restricted to source-code-adjacent extensions so markdown and
# acceptance-doc prose can discuss historical "Phase N" refactors
# without tripping the guard.
ARCH_VIOLATIONS="$(git -C "${CWD:-.}" diff --cached --no-color --unified=0 2>/dev/null |
python3 -c '
import re, sys
diff = sys.stdin.read()

PHASE_RE = re.compile(r"\bPhase \d+\b")
RUST_STUB_RE = re.compile(r"(\btodo!\(\)|\bunimplemented!\(\))")
CODE_EXT_RE = re.compile(r"\.(rs|cpp|cc|cxx|hpp|hxx|h|c|kt|py|go|jinja2?|js|ts|mjs|toml|yaml|yml)$")

current_file = None
violations = []
for line in diff.splitlines():
    if line.startswith("+++ b/"):
        current_file = line[6:]
        continue
    if not line.startswith("+") or line.startswith("+++"):
        continue
    if not current_file or not CODE_EXT_RE.search(current_file):
        continue
    added = line[1:]
    if PHASE_RE.search(added):
        violations.append(("phase-marker", current_file, added.strip()))
    if RUST_STUB_RE.search(added):
        violations.append(("rust-stub", current_file, added.strip()))

for kind, f, body in violations:
    # Cap the printed line so a long template does not drown the banner.
    snippet = body if len(body) <= 160 else body[:157] + "..."
    print(f"  [{kind}] {f}: {snippet}")
')"

if [ -n "$ARCH_VIOLATIONS" ]; then
  {
    echo "=== COMMIT BLOCKED: architecture violation in staged diff ==="
    echo ""
    echo "Staged content contains patterns explicitly forbidden by"
    echo "CLAUDE.md 'Guiding Rules' / 'Code Comments'. The self-audit"
    echo "questions do not waive these — no marker is written, so"
    echo "retrying the same command will keep failing. Remove the"
    echo "offending line or replace it with a spec reference, then"
    echo "re-stage."
    echo ""
    echo "$ARCH_VIOLATIONS"
    echo ""
    echo "Rules:"
    echo "  - phase-marker: CLAUDE.md 'Code Comments' — "
    echo "      'No phase markers (\"Phase 1\", \"Phase 2\", etc.) in code or comments.'"
    echo "      Use a W3C SCXML spec clause (e.g. '// W3C SCXML 6.2:')"
    echo "      or an ARCHITECTURE.md section anchor instead."
    echo "  - rust-stub:    CLAUDE.md 'Implementation Completeness' — "
    echo "      'No Incomplete Functions. Every function must work as specified,"
    echo "       not throw \"not implemented\".'"
  } >&2
  exit 2
fi

# ── Unconsumed-filter detector ───────────────────────────────────
#
# Structural guard against the "built but unconsumed" anti-pattern
# (memory `feedback_built_but_unconsumed.md`): a commit that adds
#   env.add_filter("NAME", filter_NAME);
# but leaves zero templates using `| NAME` registers infrastructure
# with no first consumer. The self-audit questions cannot distinguish
# honest "first of N atomic commits" from rationalised "consumer lands
# next session", so we decide structurally against the staged index.
#
# Policy:
#   - Trigger only on `env.add_filter("NAME", ...)` ADDITIONS in the
#     diff. Filter deletions and unrelated `add_filter`-like calls on
#     other objects are ignored.
#   - Verify each added NAME has at least one `| NAME` consumer in the
#     STAGED index (`git grep --cached`), not the working tree. This
#     matches what the commit will actually record: an unstaged
#     consumer must not unlock the registration, and a new staged
#     consumer file is honoured.
#   - No marker write. The author must either add a consumer to the
#     same stage or remove the registration and re-stage.
UNCONSUMED_FILTERS="$(git -C "${CWD:-.}" diff --cached --no-color --unified=0 2>/dev/null |
python3 -c '
import re, subprocess, sys
diff = sys.stdin.read()

# Added-line `env.add_filter("name", ...)` — covers Rust minijinja calls.
# Kotlin/Go/Python registrars route through the same Rust `add_filter`
# site so we do not need per-language patterns.
FILTER_RE = re.compile(r"^\+\s*env\.add_filter\(\s*\"([A-Za-z_][A-Za-z0-9_]*)\"")
added_names = []
for line in diff.splitlines():
    m = FILTER_RE.match(line)
    if m:
        added_names.append(m.group(1))

if not added_names:
    sys.exit(0)

# De-dup while preserving order.
seen = set()
unique_names = [n for n in added_names if not (n in seen or seen.add(n))]

unconsumed = []
for name in unique_names:
    # Pipe-filter syntax in jinja/minijinja: `something | NAME` with
    # optional whitespace. Anchor to a word boundary after NAME so
    # `mesh_rpc` does not match `| mesh_rpc_something`.
    pattern = r"\|\s*" + re.escape(name) + r"\b"
    try:
        # --cached scans the staged index, not the working tree:
        # a consumer must be `git add`-ed to unlock the registration,
        # exactly mirroring what the commit will record. Exit 1 on
        # zero matches is normal; capture without raising.
        r = subprocess.run(
            ["git", "grep", "--cached", "-l", "-E", pattern, "--",
             "*.jinja", "*.jinja2"],
            capture_output=True, text=True, check=False)
    except FileNotFoundError:
        # git missing would have tripped the earlier hook paths first,
        # but keep the conservative fallback for paranoia.
        unconsumed.append((name, "git grep unavailable"))
        continue
    hits = [ln for ln in r.stdout.splitlines() if ln.strip()]
    if not hits:
        unconsumed.append((name, "zero staged `| " + name + "` references in *.jinja*"))

for name, reason in unconsumed:
    print(f"  [unconsumed-filter] {name}: {reason}")
')"

if [ -n "$UNCONSUMED_FILTERS" ]; then
  {
    echo "=== COMMIT BLOCKED: unconsumed template filter in staged diff ==="
    echo ""
    echo "The diff registers a new template filter, but no .jinja*"
    echo "file in the staged index invokes it via \`| NAME\`. This is"
    echo "the \"built but unconsumed\" anti-pattern (memory"
    echo "feedback_built_but_unconsumed.md) and is not waivable by"
    echo "the self-audit questions below."
    echo ""
    echo "$UNCONSUMED_FILTERS"
    echo ""
    echo "Resolutions (pick one):"
    echo "  1) Add at least one template consumer that uses \`| NAME\`"
    echo "     in the same commit (textbook — filter and first consumer"
    echo "     land atomically). The consumer file must be \`git add\`-ed."
    echo "  2) Drop the filter registration from staging and commit it"
    echo "     together with the real consumer when ready."
    echo ""
    echo "No marker written — retrying the same command keeps failing."
  } >&2
  exit 2
fi

# ── Forward-reference detector in commit body ────────────────────
#
# Structural guard against forward-reference commit bodies — a body
# that describes what a FUTURE commit will do is a soft TODO: it
# turns the current commit into a promise whose truth depends on
# unlanded work, and it rots silently if the plan changes.
#
# The self-audit asks "is there any hack / TODO?" but the author can
# rationalise forward references as "tracked follow-up" and pass.
# This detector decides structurally against the staged commit
# message (both -m / --message args and heredoc bodies embedded in
# the raw command string).
#
# Conservative pattern list. Each pattern is forward-referring in
# >95% of uses; generic words like "later" or "future" alone are
# too noisy and not included.
FWD_REF_HITS="$(printf '%s' "$CMD" | python3 -c '
import re, shlex, sys
cmd = sys.stdin.read()

# 1. Extract -m / --message argument values via shlex token walk.
messages = []
try:
    tokens = shlex.split(cmd, posix=True)
except ValueError:
    tokens = []
i = 0
while i < len(tokens):
    t = tokens[i]
    if t in ("-m", "--message") and i + 1 < len(tokens):
        messages.append(tokens[i + 1])
        i += 2
        continue
    if t.startswith("--message="):
        messages.append(t.split("=", 1)[1])
    elif t.startswith("-m"):
        # `-mTEXT` glued form (rare but legal).
        messages.append(t[2:])
    i += 1

# 2. Additionally extract heredoc bodies from the raw command. The
#    -m argument for heredoc-style commits is `$(cat <<EOF ... EOF)`
#    which shlex returns verbatim; the actual body sits inside the
#    heredoc delimiters. Parse both quoted and unquoted EOF tags.
#    Re-scans the full CMD in case shlex missed quoted structures.
for m in re.finditer(
    r"<<-?\s*[\x27\x22]?([A-Za-z_][A-Za-z0-9_]*)[\x27\x22]?\s*\n(.*?)\n\s*\1\s*(?:\n|$)",
    cmd, re.DOTALL):
    messages.append(m.group(2))

full_body = "\n".join(messages)
if not full_body.strip():
    sys.exit(0)

# High-confidence forward-reference patterns.
FWD_PATTERNS = [
    (r"\bfollows in\b",                                  "follows in"),
    (r"\bnext (session|commit|turn|PR|pr)\b",            "next session/commit/PR"),
    (r"\bto be (wired|landed|implemented|added|emitted|integrated|consumed)\b",
                                                         "to be X (forward plan)"),
    # Bare numeric stage markers (e.g. `in F3a`, `in step 2`, `phase 4`)
    # are deliberately excluded — they match backward references too
    # ("already landed in F3a") and the forward-direction sibling
    # phrasings are already covered by the verb-anchored patterns
    # below (will follow / consumer follows / to be wired).
    (r"\bupcoming (commit|session|PR|pr|work)\b",        "upcoming commit/session"),
    (r"\bconsumer (follows|comes|arrives|lands|will)\b", "consumer follows/lands"),
    (r"\b(will follow|will be (added|wired|landed))\b",  "will follow/be added"),
    (r"\bplaceholder\b",                                 "placeholder"),
    (r"\bTBD\b",                                         "TBD"),
    (r"\btracked (follow[- ]?up|next session)\b",        "tracked follow-up (rationalisation)"),
]

hits = []
for pat, label in FWD_PATTERNS:
    for m in re.finditer(pat, full_body, re.IGNORECASE):
        matched = m.group(0)
        # Deduplicate by label + matched text.
        key = (label, matched.lower())
        if key not in {(h[0], h[1].lower()) for h in hits}:
            hits.append((label, matched))

for label, matched in hits:
    print(f"  [{label}] \"{matched}\"")
')"

if [ -n "$FWD_REF_HITS" ]; then
  {
    echo "=== COMMIT BLOCKED: forward-reference in commit body ==="
    echo ""
    echo "The commit body contains forward references — phrasing that"
    echo "depends on future commits. A textbook commit message describes"
    echo "what THIS diff achieves and does not promise follow-up work."
    echo "Forward references act as soft TODOs and rot silently if the"
    echo "plan changes."
    echo ""
    echo "$FWD_REF_HITS"
    echo ""
    echo "Resolutions (pick one):"
    echo "  1) If the commit has no standalone meaning without the future"
    echo "     work, bundle the consumer into this commit for an atomic"
    echo "     landing."
    echo "  2) If the commit IS a standalone chore/refactor, rewrite the"
    echo "     body to describe only what this diff achieves, and move"
    echo "     plan hand-offs into a memory / plan note (out of the"
    echo "     commit message)."
    echo ""
    echo "No marker written — retrying the same command keeps failing."
  } >&2
  exit 2
fi

# ── COMMIT_FORMAT.md structural detector ─────────────────────────
#
# Hard-block commits whose message violates COMMIT_FORMAT.md. These
# are deterministic rules (type prefix, bullet body, 1-3 items, no
# Co-Authored-By / emoji / Claude Code attribution), not waivable by
# the self-audit questions. No marker written, so the command must be
# rewritten — not re-answered — to pass.
#
# Scope: only runs when the command provides a message via -m /
# --message / heredoc. `--amend --no-edit` and editor-based commits
# (no -m) are skipped because there is no inline message to inspect.
FORMAT_VIOLATIONS="$(printf '%s' "$CMD" | python3 -c '
import re, shlex, sys
cmd = sys.stdin.read()

# Extract -m / --message values plus --amend --no-edit marker.
messages = []
amend_no_edit = False
saw_m = False
try:
    tokens = shlex.split(cmd, posix=True)
except ValueError:
    tokens = []
i = 0
while i < len(tokens):
    t = tokens[i]
    if t in ("-m", "--message") and i + 1 < len(tokens):
        messages.append(tokens[i + 1])
        saw_m = True
        i += 2
        continue
    if t.startswith("--message="):
        messages.append(t.split("=", 1)[1])
        saw_m = True
    elif t.startswith("-m") and len(t) > 2:
        # Glued `-mTEXT` form (legal but rare).
        messages.append(t[2:])
        saw_m = True
    elif t == "--no-edit":
        amend_no_edit = True
    i += 1

# Heredoc-embedded bodies — shlex preserves `$(cat <<EOF ... EOF)` as
# an opaque token, so the real message lives inside the heredoc tag.
for m in re.finditer(
    r"<<-?\s*[\x27\x22]?([A-Za-z_][A-Za-z0-9_]*)[\x27\x22]?\s*\n(.*?)\n\s*\1\s*(?:\n|$)",
    cmd, re.DOTALL):
    messages.append(m.group(2))
    saw_m = True

# --amend --no-edit reuses the existing (already-audited) message; no
# new text to check. Commits without -m / heredoc (editor-based) are
# not inspectable here — git will open $EDITOR and the user authors
# directly, which is out of scope.
if amend_no_edit or not saw_m:
    sys.exit(0)

# Drop messages that are shell-expansion placeholders (e.g.
# "$(cat <<EOF ... EOF)" literal from an unresolved substitution);
# the real content comes from the heredoc body extracted above.
def is_expansion_placeholder(s):
    s = s.strip()
    return s.startswith("$(") or s.startswith("${") or s.startswith("`")

real_messages = [m for m in messages if m.strip() and not is_expansion_placeholder(m)]
if not real_messages:
    sys.exit(0)

# Git -m semantics: first value is subject, each subsequent -m is a
# body paragraph separated by a blank line. Heredoc bodies already
# carry their own newlines; keep them intact.
if len(real_messages) == 1:
    full_msg = real_messages[0]
else:
    full_msg = "\n\n".join(m.strip() for m in real_messages)

lines = full_msg.splitlines()
# Strip trailing empty lines so bullet counting is not skewed by
# trailing newlines that heredoc patterns add.
while lines and not lines[-1].strip():
    lines.pop()
if not lines:
    sys.exit(0)

subject = lines[0]
rest = lines[1:]

violations = []

# ── Subject rules (COMMIT_FORMAT.md §"Subject Line") ──
SUBJECT_RE = re.compile(r"^(feat|refactor|fix|docs|test|chore)(?:\([^)]+\))?: \S.*$")
if not SUBJECT_RE.match(subject):
    violations.append(("subject-type",
        f"Subject must match `<type>: <text>` where type ∈ "
        f"{{feat, refactor, fix, docs, test, chore}} (optional scope "
        f"`(name)`). Got: {subject!r}"))
if len(subject) > 72:
    violations.append(("subject-length",
        f"Subject is {len(subject)} chars; max 72. Got: {subject!r}"))
if subject.rstrip().endswith("."):
    violations.append(("subject-period",
        f"Subject must not end with a period. Got: {subject!r}"))

# ── Body rules (COMMIT_FORMAT.md §"Body") ──
# Strip leading blanks (the mandatory blank line after subject).
while rest and not rest[0].strip():
    rest.pop(0)

if rest:
    # Parse bullets. A bullet starts with "- " at column 0. Lines
    # that begin with whitespace are treated as wrap continuations
    # of the preceding bullet. Any other non-blank line is a violation.
    bullets = []
    saw_nonbullet = False
    current_bullet = None
    for line in rest:
        if line.startswith("- "):
            current_bullet = line
            bullets.append(current_bullet)
        elif not line.strip():
            current_bullet = None
        elif line[:1] in (" ", "\t") and current_bullet is not None:
            # Wrap continuation — OK.
            continue
        else:
            if not saw_nonbullet:
                violations.append(("body-nonbullet",
                    f"Body line is not a bullet and not a wrap "
                    f"continuation: {line!r}. COMMIT_FORMAT.md requires "
                    f"bullet points (`- ` prefix) only."))
                saw_nonbullet = True

    n = len(bullets)
    if n == 0 and not saw_nonbullet:
        violations.append(("body-no-bullets",
            "Body has no bullet points. COMMIT_FORMAT.md requires "
            "1-3 bullets after the subject line."))
    elif n > 3:
        violations.append(("body-too-many",
            f"Body has {n} bullets. COMMIT_FORMAT.md requires "
            f"1-3 items (fewer is better)."))

# ── Forbidden style (COMMIT_FORMAT.md §"Style") ──
if re.search(r"Co-?Authored-?By\s*:", full_msg, re.IGNORECASE):
    violations.append(("coauthor-tag",
        "`Co-Authored-By` tag is forbidden by COMMIT_FORMAT.md §Style. "
        "Project rule overrides any global template."))
if re.search(r"Generated with \[?Claude Code", full_msg, re.IGNORECASE):
    violations.append(("claude-attribution",
        "`Generated with Claude Code` attribution is forbidden by "
        "COMMIT_FORMAT.md §Style."))
# Emoji: conservative ranges — Miscellaneous Symbols, Dingbats, and
# the Emoji blocks. Latin text + §, ×, ≤ etc. are outside these ranges.
if re.search(r"[\U0001F300-\U0001FAFF\U00002600-\U000027BF\U0001F000-\U0001F02F]",
             full_msg):
    violations.append(("emoji",
        "Emojis are forbidden by COMMIT_FORMAT.md §Style."))

for kind, body in violations:
    print(f"  [{kind}] {body}")
')"

if [ -n "$FORMAT_VIOLATIONS" ]; then
  {
    echo "=== COMMIT BLOCKED: COMMIT_FORMAT.md violation ==="
    echo ""
    echo "The commit message does not match the project's commit rules"
    echo "(COMMIT_FORMAT.md in the repo root). These are deterministic"
    echo "format rules and not waivable by the self-audit questions."
    echo ""
    echo "$FORMAT_VIOLATIONS"
    echo ""
    echo "Rule summary (see COMMIT_FORMAT.md for full spec):"
    echo "  - Subject: \`<type>: <text>\` (type ∈ feat/refactor/fix/docs/test/chore),"
    echo "    max 72 chars, no trailing period."
    echo "  - Body:    one blank line after subject, then 1-3 bullets with"
    echo "             \`- \` prefix. No prose paragraphs."
    echo "  - Style:   no Co-Authored-By tag, no 'Generated with Claude Code',"
    echo "             no emojis. Project rule overrides global templates."
    echo ""
    echo "No marker written — retrying the same command keeps failing."
    echo "Rewrite the commit message and re-run."
  } >&2
  exit 2
fi

GIT_DIR="$(git -C "${CWD:-.}" rev-parse --git-dir 2>/dev/null || echo .git)"
# Normalize to absolute path so the marker lives next to the repo,
# not in whichever directory the hook happened to run from.
case "$GIT_DIR" in
  /*) ;;
  *) GIT_DIR="${CWD:-$PWD}/$GIT_DIR" ;;
esac

MARKER="$GIT_DIR/.claude-commit-audit-sha"
HEAD_SHA="$(git -C "${CWD:-.}" rev-parse --verify HEAD 2>/dev/null || echo none)"

# Marker key binds both the current HEAD and the exact command bytes.
# Storing just HEAD was unsafe: any incidental invocation of this hook
# (diagnostic probes, other shells, manual runs) at the same HEAD
# would write HEAD as the marker and then let the *next* real commit
# pass silently, because HEAD match alone was enough. By hashing in
# the command, a real `git commit -m ...` retry preserves its own
# marker while an unrelated invocation produces a different marker
# that does not unlock anybody else's commit.
MARKER_KEY="$(printf '%s\n%s' "$HEAD_SHA" "$CMD" | sha1sum | awk '{print $1}')"

if [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "$MARKER_KEY" ]; then
  # Already audited at this (HEAD, command). Let the commit through.
  exit 0
fi

# Record the marker *before* blocking so that the retry picks it up.
printf '%s' "$MARKER_KEY" > "$MARKER"

# Discover plan memos that might name YAGNI-adjacent work for this
# project. Memory dirs follow the slug `<leading-slash>path-with-
# dashes`, e.g. /home/coin/scxml-core-engine → -home-coin-scxml-core-
# engine. Missing dir is fine — the questions still stand.
PROJECT_SLUG="$(printf '%s' "${CWD:-$PWD}" | sed 's|/|-|g')"
MEMORY_DIR="$HOME/.claude/projects/${PROJECT_SLUG}/memory"
NEXT_MEMOS=""
if [ -d "$MEMORY_DIR" ]; then
  # Only surface plan memos whose frontmatter declares `status: open`.
  # Closed lifecycle states (superseded/landed/retired/retrospective)
  # are filtered so the audit list reflects active work, not history.
  NEXT_MEMOS="$(
    for f in "$MEMORY_DIR"/next_*.md; do
      [ -f "$f" ] || continue
      if awk '/^---$/{c++; if(c==2)exit} c==1' "$f" | grep -q "^status: open$"; then
        printf '       - %s\n' "$(basename "$f")"
      fi
    done | sort
  )"
fi

{
  echo "=== COMMIT SELF-AUDIT (hook-mandated, answer then retry) ==="
  echo ""
  echo "This commit has not been audited yet (keyed on HEAD + command)."
  echo "Answer the five questions below, then re-run the exact same"
  echo "commit command. The retry passes because of the marker at"
  echo "$MARKER — which is a hash of HEAD + command, so diagnostic"
  echo "probes, other shells, and manual runs cannot unlock it."
  echo ""
  echo "1. Textbook quality: does this diff contain format coupling,"
  echo "   hardcoded magic numbers, invariants without drift guards,"
  echo "   or hallucinated cross-references? If any remain, justify"
  echo "   keeping them in your reply or the commit body."
  echo ""
  echo "2. Hacks: are there workarounds, band-aids, empty stubs, or"
  echo "   TODO comments hiding unfinished implementation? Did you"
  echo "   treat symptoms instead of the root cause? If so, list them"
  echo "   and explain why they stay as they are."
  echo ""
  echo "3. YAGNI / plan alignment: if any plan memo listed below"
  echo "   overlaps this diff's scope and this commit does not handle"
  echo "   it, that is incompleteness, not YAGNI — decide whether to"
  echo "   bundle it into the same commit. If the diff landed in a"
  echo "   different file path / directory / module boundary than the"
  echo "   plan memo specified (new file location, different tier,"
  echo "   split or merged files), state the reason in the commit body"
  echo "   or reply — noting 'we went with A instead of B' is not"
  echo "   enough; the reason WHY B was unsuitable must be included."
  echo ""
  echo "4. Architectural consistency (ARCHITECTURE.md + CLAUDE.md"
  echo "   'Guiding Rules'). Answer only the bullets that apply:"
  echo "   (a) Engine sharing: if only one of Interpreter / AOT was"
  echo "       modified, show why the other is unaffected, or that"
  echo "       the change was routed through a shared helper"
  echo "       (sce/include/core, sce/include/common)."
  echo "       — 'Zero Duplication: Shared Helper functions between engines'"
  echo "   (b) Backend parity: if only one of C++/Kotlin/Rust/Go/Python"
  echo "       changed, explain why the others already have the same"
  echo "       capability or are intentionally skipped."
  echo "   (c) 4-tier boundary: does sce_core (header-only) gain a"
  echo "       .cpp file or an external dependency? If so, justify"
  echo "       the tier reclassification."
  echo "   (d) Template-first: if build/*/generated/* or any produced"
  echo "       artifact was edited directly, explain why it cannot"
  echo "       be expressed as a tools/codegen/templates/ change."
  echo ""
  echo "5. Scope probe: does this commit belong directly to the user's"
  echo "   MOST RECENT EXPLICIT instruction? Check for side-quest"
  echo "   patterns:"
  echo "   (a) Infrastructure (hook, CI, tooling) piggy-backing on a"
  echo "       feature commit: state a structural reason they must"
  echo "       ride together (e.g. validated through the same path)"
  echo "       or split them into separate commits."
  echo "   (b) Environment cleanup (unused-import removal, build"
  echo "       flags, small cleanups) mixed with a feature: if the"
  echo "       cleanup is independent, do not bundle — land it as a"
  echo "       preceding chore."
  echo "   (c) A symbol now misnamed (e.g. a struct whose name no"
  echo "       longer matches its data): state whether the rename is"
  echo "       included in this commit or recorded in a follow-up"
  echo "       memo — silent deferral is not acceptable."
  echo ""
  if [ -n "$NEXT_MEMOS" ]; then
    echo "     Related plan memos:"
    echo "$NEXT_MEMOS"
  else
    echo "     (plan memo directory is empty or absent: $MEMORY_DIR)"
  fi
  echo ""
  echo "Write your audit answers in the reply, then re-run the same"
  echo "git commit command."
} >&2

# Exit 2 = block tool call, pass stderr to Claude as reason
exit 2
