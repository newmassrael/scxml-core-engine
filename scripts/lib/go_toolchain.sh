# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Go toolchain this repository's Go runs use: the release, and where the
# declared one is.
#
# THE RELEASE IS A PROPERTY OF THE MACHINE, NOT OF THE NETWORK (2026-11-19). The
# Go modules under backends/go declare `go 1.23.0`, and the build machines carry
# the distribution's Go 1.22.2. Until this file, a run there relied on
# GOTOOLCHAIN=auto: the go command fetched go1.23.0 from the network when a
# module asked for it, at run time, in whichever unit got there first. A machine
# that cannot fetch it fails every Go run with "requires go >= 1.23.0 (running go
# 1.22.2)", a sentence about the tree for a property of the machine. So the
# release is `[platform.linux.tools.go]` in bx.toml: `bx-provision <alias>
# --apply` installs it once per build machine (scripts/install_go.sh) at
# ~/.local/share/go/<release>, bx checks it there, and a unit whose command line
# the table's `needed_by` names is placed only where it is.
#
# The release is read from the `go` line of backends/go/forge-runtime/go.mod, as
# the table reads it: the one line go itself reads, and the oldest release the
# module admits, which is the one that holds it to its word. The registry's
# self-test holds every other backends/go/*/go.mod to the same line.
#
# A run that uses it (`sce_go_select`) gets its `bin/` first on PATH and
# GOTOOLCHAIN=local, so no run fetches another toolchain behind the one chosen
# here. Where it is not installed (a developer's machine, a hosted runner that
# set Go up itself) the go on PATH runs, and the line that says so names the
# GOTOOLCHAIN it runs under.
#
# Sourced; sets SCE_GO_VERSION and defines functions only.

# The release, from the forge runtime's go.mod; empty when that file states none,
# which `sce_go_select` says by name.
SCE_GO_VERSION="$(sed -nE 's/^go ([0-9]+\.[0-9]+\.[0-9]+)$/\1/p' \
    "$(dirname "${BASH_SOURCE[0]}")/../../backends/go/forge-runtime/go.mod" 2>/dev/null)"

# Where a declared release is: the `prefix_link` of its table in bx.toml,
# `~/.local/share/go/<release>`, the GOROOT the release's tarball unpacks to.
sce_go_toolchain_dir() {
    printf '%s/.local/share/go/%s\n' "$HOME" "$1"
}

# The bin directory of declared release `$1`, on stdout, when it is there and its
# go names that release; status 1 and nothing printed otherwise. GOTOOLCHAIN=local
# so the answer is that binary's own, never a toolchain it would switch to.
sce_go_toolchain_bin() {
    local bin
    bin="$(sce_go_toolchain_dir "$1")/bin"
    [[ -x "$bin/go" ]] || return 1
    [[ "$(GOTOOLCHAIN=local "$bin/go" version 2>/dev/null)" == "go version go$1 "* ]] || return 1
    printf '%s\n' "$bin"
}

# Puts the declared release first on PATH for the rest of the caller's run, with
# GOTOOLCHAIN=local and no GOROOT inherited from the environment (the toolchain
# finds its own tree beside its binary), and says so on stderr after `$1`, the
# caller's name. Where it is not installed PATH is left as it is, the line says
# which go runs instead and under which GOTOOLCHAIN, and the status is 1.
# Called directly, never in `$(...)`: what it exports must reach the caller.
sce_go_select() {
    local who="$1" bin found
    if [[ -z "$SCE_GO_VERSION" ]]; then
        printf '%s: no "go x.y.z" line read from backends/go/forge-runtime/go.mod, so no declared Go is chosen; the go on PATH runs\n' \
            "$who" >&2
        return 1
    fi
    if bin="$(sce_go_toolchain_bin "$SCE_GO_VERSION")"; then
        PATH="$bin:$PATH"
        GOTOOLCHAIN=local
        export PATH GOTOOLCHAIN
        unset GOROOT
        printf '%s: go%s from %s (GOTOOLCHAIN=local)\n' \
            "$who" "$SCE_GO_VERSION" "$(sce_go_toolchain_dir "$SCE_GO_VERSION")" >&2
        return 0
    fi
    if found="$(GOTOOLCHAIN=local go version 2>/dev/null)"; then
        printf '%s: %s, the go on PATH, under GOTOOLCHAIN=%s: the declared Go %s is not installed at %s (scripts/install_go.sh installs it there)\n' \
            "$who" "$found" "${GOTOOLCHAIN:-<unset>, so go may fetch the release a module asks for}" \
            "$SCE_GO_VERSION" "$(sce_go_toolchain_dir "$SCE_GO_VERSION")" >&2
    else
        printf '%s: no go on PATH and the declared Go %s is not installed at %s; a run of go here fails (scripts/install_go.sh installs it there)\n' \
            "$who" "$SCE_GO_VERSION" "$(sce_go_toolchain_dir "$SCE_GO_VERSION")" >&2
    fi
    return 1
}
