#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Installs an opt-in pre-commit hook that runs the §6.2.6 drift
# verification suite on every commit that touches a drift-relevant
# path. Mirrors .github/workflows/drift-verify.yml so a regression
# that would fail CI fails locally first.
#
# The hook itself is NOT tracked in the repo — .git/hooks/ is outside
# the working tree by design. Each contributor opts in by running this
# script once. Idempotent: re-running overwrites the hook in place.
#
# Usage (from any working directory inside the repo):
#   tools/dev/install-hooks.sh

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HOOK_DIR="$ROOT/.git/hooks"
HOOK="$HOOK_DIR/pre-commit"

mkdir -p "$HOOK_DIR"

cat > "$HOOK" <<'EOF'
#!/usr/bin/env bash
# Auto-installed by tools/dev/install-hooks.sh.
#
# Mirrors .github/workflows/drift-verify.yml: any drift-touching change
# must pass the §6.2.6 integration suites before the commit is
# accepted. The path filter matches the workflow's `paths:` block so
# commits that do not affect drift hashes skip the hook entirely.

set -euo pipefail

if git diff --cached --name-only --diff-filter=ACM \
    | grep -qE '^(sce-build/|tools/codegen/templates/|sce-rust-tests/(src/generated|fixtures)/|resources/|Cargo\.(lock|toml)$)'; then
    cargo test -p sce-build --features cli --test b9_drift_detection -- --test-threads=1
    cargo test -p sce-build --features cli --test s5_o_atomic_1_sourcemap
fi
EOF

chmod +x "$HOOK"

echo "pre-commit hook installed at $HOOK"
echo "Hook runs §6.2.6 drift + §5.O sourcemap integration suites on every drift-touching commit."
