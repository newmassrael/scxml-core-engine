#!/usr/bin/env bash
# Install SCE git hooks into the active .git/hooks/ directory.
#
# Hooks live in `scripts/hooks/` and are tracked under version control so
# updates stay reviewable. This installer symlinks them into the
# repo-local `.git/hooks/`, which git auto-discovers per-clone.
#
# Why opt-in install (not core.hooksPath) — the CI gate
# (`.github/workflows/fmt-check.yml`) is the authoritative enforcement
# point. Local hooks are a developer fast-fail that some workflows
# (e.g. `--no-verify` for stash-on-WIP, IDE auto-commit) need to bypass.
# Forcing core.hooksPath in repo config would surprise contributors who
# rely on those bypasses.
#
# Usage: run once after cloning.
#
#   scripts/install-hooks.sh
#
# Idempotent: re-running replaces existing symlinks.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
src_dir="${repo_root}/scripts/hooks"
dst_dir="${repo_root}/.git/hooks"

if [ ! -d "${src_dir}" ]; then
    echo "install-hooks: ${src_dir} does not exist." >&2
    exit 1
fi
if [ ! -d "${dst_dir}" ]; then
    echo "install-hooks: ${dst_dir} does not exist — is this a git checkout?" >&2
    exit 1
fi

count=0
for hook in "${src_dir}"/*; do
    [ -e "${hook}" ] || continue
    name="$(basename "${hook}")"
    dst="${dst_dir}/${name}"
    ln -sf "${hook}" "${dst}"
    chmod +x "${hook}"
    echo "install-hooks: linked ${name}"
    count=$((count + 1))
done

if [ "${count}" -eq 0 ]; then
    echo "install-hooks: no hook files found in ${src_dir}." >&2
    exit 1
fi

echo "install-hooks: ${count} hook(s) installed into ${dst_dir}."
echo "install-hooks: run 'git commit --no-verify' to bypass for a single commit."
