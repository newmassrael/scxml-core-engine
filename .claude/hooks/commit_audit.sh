#!/usr/bin/env bash
# Commit gate library — the checks both git hooks share.
#
# Reads a JSON payload on stdin carrying a `git commit` command and the
# repository path, and blocks (exit 2) on a violation. Two callers, each
# feeding it the half it can see:
#
#   tools/git-hooks/commit-msg  — the final message, so COMMIT_FORMAT.md
#                                 and forward-reference checks run.
#   tools/git-hooks/pre-commit  — a message-less payload, so the
#                                 staged-diff gates (architecture
#                                 violation, unconsumed template filter,
#                                 memory lifecycle) run.
#
# History: this began as a Claude Code PreToolUse hook, which inspected
# the `git commit` command string before the harness ran it. That layer
# was removed on 2026-08-10 — it could not see a commit made outside the
# harness, and its self-audit answer file lived under `.git/`, a
# protected path no `permissions.allow` rule can pre-approve, so it
# prompted on every commit. git runs both callers above unconditionally.
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
import re, shlex, sys
cmd = sys.stdin.read()
# Heredoc bodies are the commit MESSAGE, not shell syntax, and they
# routinely contain an apostrophe (possessive s, contractions) or a
# stray quote that shlex reads as an unbalanced quotation — raising
# ValueError and (previously) making a real `git commit` look like
# not-a-commit, silently bypassing every audit/format check. Strip
# `<<TAG ... TAG` blocks before tokenising so the message content
# cannot break detection. (No literal quote chars in this -c body:
# the regex uses \x27/\x22 so the surrounding single-quote survives.)
cmd_detect = re.sub(
    r"<<-?\s*[\x27\x22]?([A-Za-z_][A-Za-z0-9_]*)[\x27\x22]?\s*\n.*?\n\s*\1\s*(?:\n|$)",
    " ", cmd, flags=re.DOTALL)
try:
    tokens = shlex.split(cmd_detect, comments=False, posix=True)
except ValueError:
    # Still unbalanced after stripping heredocs: do NOT fail open.
    # Fall back to a quote-insensitive scan — a `git ... commit` shape
    # anywhere classifies as a commit so the audit still fires.
    print("yes" if re.search(r"\bgit\b[^\n;|&]*\bcommit\b", cmd) else "no")
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
# A non-blank line directly after the subject (no intervening blank)
# makes git fold subject+body into one oversized subject line — the
# bullets never register as a body. COMMIT_FORMAT.md mandates the
# blank line; require it explicitly so a folded message is rejected.
if rest and rest[0].strip():
    violations.append(("body-no-blank-after-subject",
        "No blank line after the subject: git folds the subject and "
        "body into one oversized subject line, so the bullets never "
        "register. Add a blank line after the subject, then 1-3 bullets."))
# Strip leading blanks (the mandatory blank line after subject).
while rest and not rest[0].strip():
    rest.pop(0)

if rest:
    # Parse bullets. A bullet starts with "- " at column 0 and is one
    # line, max 72 chars (incl the "- " prefix) — no continuation/wrap
    # lines. Bullets must be contiguous: a blank line between bullets
    # is rejected (not a soft separator). Any other non-blank line is a
    # prose-paragraph violation. (COMMIT_FORMAT.md §Body.)
    bullets = []
    saw_nonbullet = False
    for line in rest:
        if line.startswith("- "):
            bullets.append(line)
            if len(line) > 72:
                violations.append(("body-bullet-length",
                    f"Bullet is {len(line)} chars; max 72 (incl the "
                    f"`- ` prefix). Rewrite tighter or split into "
                    f"another bullet: {line!r}"))
        elif not line.strip():
            # Interior blank line — bullets must be contiguous.
            violations.append(("body-blank-line",
                "Blank line inside body — bullets must be contiguous "
                "with no blank separator between them. Put every bullet "
                "in a single -m block on consecutive lines."))
        elif line[:1] in (" ", "\t"):
            violations.append(("body-continuation",
                f"Indented continuation of a bullet — one bullet = one "
                f"line, max 72 chars. Rewrite tighter or split: {line!r}"))
            saw_nonbullet = True
        else:
            if not saw_nonbullet:
                violations.append(("body-nonbullet",
                    f"Body line is not a bullet: {line!r}. "
                    f"COMMIT_FORMAT.md requires bullet points "
                    f"(`- ` prefix) only, no prose."))
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
    echo "             \`- \` prefix. No prose paragraphs. Each bullet is"
    echo "             one line, max 72 chars (incl '- '), no wrap. Bullets"
    echo "             must be contiguous — no blank line between them"
    echo "             (put the whole body in a single -m block)."
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

HEAD_SHA="$(git -C "${CWD:-.}" rev-parse --verify HEAD 2>/dev/null || echo none)"

# Discover plan memos that might name YAGNI-adjacent work for this
# project. Memory dirs follow the slug `<leading-slash>path-with-
# dashes`, e.g. /home/coin/scxml-core-engine → -home-coin-scxml-core-
# engine. Missing dir is fine — the questions still stand.
#
# The slug keys off the *main* worktree, not the invoking directory. A
# commit made from a linked worktree has a `cwd` somewhere else
# entirely, which slugs to a directory that has never existed — and
# the whole memory-lifecycle validation below would then skip without
# saying so. Committing from a worktree is a normal thing to do while
# the main tree is busy, and it must not silently disable a gate.
MAIN_WORKTREE="$(git -C "${CWD:-.}" worktree list --porcelain 2>/dev/null \
  | sed -n '1s/^worktree //p')"
PROJECT_SLUG="$(printf '%s' "${MAIN_WORKTREE:-${CWD:-$PWD}}" | sed 's|/|-|g')"
MEMORY_DIR="$HOME/.claude/projects/${PROJECT_SLUG}/memory"
NEXT_MEMOS=""
MEMO_VALIDATION=""
# Initialised out here, not inside the branch below: under `set -u` a
# missing memory dir made the "Missing dir is fine" comment above false
# — the script died on the unbound read a few lines later instead of
# asking its five questions.
MEMO_ERRORS=""
if [ -d "$MEMORY_DIR" ]; then
  # Full memory tree lifecycle validation. Every .md file (except
  # MEMORY.md) must declare frontmatter `status:` from the 8-value
  # enum, satisfy the prefix/suffix/path contract from
  # claudedocs/rfc-memory-sixth-wave.md (refined by seventh-wave RFC),
  # and emit no dangling [[wikilink]] or broken MEMORY.md path link.
  # Surface only open next_*.md in the audit list. Any violation aborts
  # the commit (silently-broken hooks impossible).
  MEMO_VALIDATION="$(MEMORY_DIR="$MEMORY_DIR" python3 -c '
import os, re, sys
from pathlib import Path

MEMORY_DIR = Path(os.environ["MEMORY_DIR"])
VALID = {"open", "active", "feedback",
         "landed", "superseded", "refuted", "retired", "retrospective"}
CLOSED = {"landed", "superseded", "refuted", "retired", "retrospective"}

# Filename suffix → required status. Each suffix encodes its own lifecycle.
SUFFIX_RULES = [
    ("_landed", "landed"),
    ("_done", "landed"),
    ("_complete", "landed"),
    ("_closed", "landed"),
    ("_resolved", "landed"),
    ("_fixed", "landed"),
    ("_removed", "landed"),
    ("_repointed", "landed"),
    ("_superseded", "superseded"),
    ("_absorbed", "superseded"),
    ("_refuted", "refuted"),
    ("_retired", "retired"),
    ("_retrospective", "retrospective"),
]

errors = []
open_memos = []

def read_status(path):
    try:
        content = path.read_text()
    except Exception:
        return None, "read failed"
    lines = content.splitlines()
    if not lines or lines[0] != "---":
        return None, "no frontmatter"
    for i in range(1, len(lines)):
        if lines[i] == "---":
            break
        # Match status: at any indentation so the lifecycle contract is
        # validated by its VALUE, not its YAML nesting: top-level (the
        # original flat form) and under a metadata: block (the harness
        # memory normalizer canonical form) are both accepted.
        m = re.match(r"^\s*status:\s*(\S+)", lines[i])
        if m:
            return m.group(1).strip("\x27\x22"), None
    return None, "no status field"

for f in sorted(MEMORY_DIR.rglob("*.md")):
    if f.name == "MEMORY.md":
        continue
    rel = f.relative_to(MEMORY_DIR).as_posix()
    status, err = read_status(f)
    if err:
        errors.append(f"{rel}: {err}")
        continue
    if status not in VALID:
        errors.append(f"{rel}: invalid status {status!r}")
        continue

    # Path-based contract: archive bucket invariants.
    parts = f.relative_to(MEMORY_DIR).parts
    if parts[0] == "archive":
        if len(parts) == 2:
            # archive/<aggregator>.md → must be active
            if status != "active":
                errors.append(f"{rel}: archive top-level aggregator must be active (got {status!r})")
                continue
        else:
            # archive/closed/**/*.md → must be closed status
            if status not in CLOSED:
                errors.append(f"{rel}: archive/closed/** must be a closed status (got {status!r})")
                continue

    # Prefix contract.
    if f.name.startswith("next_"):
        if status != "open":
            errors.append(f"{rel}: next_*.md must be status:open (got {status!r})")
            continue
        open_memos.append(f.name)
        continue
    if f.name.startswith("feedback_"):
        if status != "feedback":
            errors.append(f"{rel}: feedback_*.md must be status:feedback (got {status!r})")
        continue

    # Suffix contract.
    stem = f.stem
    matched_suffix = None
    for suffix, required in SUFFIX_RULES:
        if stem.endswith(suffix):
            matched_suffix = (suffix, required)
            break
    if matched_suffix is not None:
        suffix, required = matched_suffix
        if status != required:
            errors.append(f"{rel}: filename suffix {suffix!r} requires status:{required} (got {status!r})")

# Build slug set for dangling [[wikilink]] gate.
slugs = set()
for f in MEMORY_DIR.rglob("*.md"):
    slugs.add(f.stem.replace("_", "-").lower())

def strip_code(content):
    content = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
    content = re.sub(r"`[^`\n]*`", "", content)
    return content

# Dangling [[wikilink]] gate.
for f in MEMORY_DIR.rglob("*.md"):
    try:
        cleaned = strip_code(f.read_text())
    except Exception:
        continue
    rel = f.relative_to(MEMORY_DIR).as_posix()
    for m in re.finditer(r"\[\[([a-zA-Z0-9-]+)\]\]", cleaned):
        slug = m.group(1).lower()
        if slug not in slugs:
            errors.append(f"{rel}: dangling wikilink [[{m.group(1)}]]")

# MEMORY.md path-link existence gate.
memory_md = MEMORY_DIR / "MEMORY.md"
if memory_md.exists():
    try:
        content = memory_md.read_text()
    except Exception:
        content = ""
    for m in re.finditer(r"\]\(([^)]+\.md)\)", content):
        path = m.group(1).strip()
        if path.startswith("http"):
            continue
        target = (memory_md.parent / path).resolve()
        if not target.exists():
            errors.append(f"MEMORY.md: broken link to {path}")

# Output: errors first, then open-memos list (one per line, prefixed).
print("--ERRORS--")
for e in errors:
    print(e)
print("--OPEN-MEMOS--")
for n in sorted(open_memos):
    print(n)
')"

  # Parse python output into MEMO_ERRORS + NEXT_MEMOS.
  in_errors=1
  in_open=0
  MEMO_ERRORS=""
  while IFS= read -r line; do
    case "$line" in
      "--ERRORS--") in_errors=1; in_open=0; continue ;;
      "--OPEN-MEMOS--") in_errors=0; in_open=1; continue ;;
    esac
    if [ "$in_errors" -eq 1 ] && [ -n "$line" ]; then
      MEMO_ERRORS="${MEMO_ERRORS}
       - $line"
    fi
    if [ "$in_open" -eq 1 ] && [ -n "$line" ]; then
      NEXT_MEMOS="${NEXT_MEMOS}
       - $line"
    fi
  done <<< "$MEMO_VALIDATION"
fi

# Fail loud on schema or contract violations — re-validate every retry
# until fixed. No marker write happens here so retries cannot pass silently.
if [ -n "$MEMO_ERRORS" ]; then
  {
    echo "=== COMMIT BLOCKED: memory lifecycle contract violations ==="
    echo ""
    echo "Files in $MEMORY_DIR"
    echo "must comply with claudedocs/rfc-memory-sixth-wave.md +"
    echo "claudedocs/rfc-memory-seventh-wave.md:"
    echo "  - status: from {open, active, feedback,"
    echo "    landed, superseded, refuted, retired, retrospective}"
    echo "  - next_*.md → status:open"
    echo "  - feedback_*.md → status:feedback"
    echo "  - *_landed.md / _done.md / _complete.md / _closed.md /"
    echo "    _resolved.md / _fixed.md / _removed.md / _repointed.md → status:landed"
    echo "  - *_superseded.md / _absorbed.md → status:superseded"
    echo "  - *_refuted.md → status:refuted"
    echo "  - *_retired.md → status:retired"
    echo "  - *_retrospective.md → status:retrospective"
    echo "  - archive/<aggregator>.md → status:active"
    echo "  - archive/closed/**/*.md → closed status"
    echo "  - every [[wikilink]] outside backticks resolves to an"
    echo "    existing file's slug"
    echo "  - every MEMORY.md markdown link path resolves to an"
    echo "    existing file"
    echo ""
    echo "Violations:${MEMO_ERRORS}"
    echo ""
    echo "Fix each file's frontmatter status, link, or path, then re-run."
  } >&2
  exit 2
fi

# All gates passed. The message-based checks above run from the
# `commit-msg` git hook and the diff-based ones from `pre-commit`,
# so every check is git-guaranteed rather than harness-dependent.
exit 0
