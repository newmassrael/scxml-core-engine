#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The stack frame of a codec's lent `decode_into` on an MCU target, measured on
# a stable toolchain (backends/rust/probes/nostd-codec-frame).
#
# A bare-metal client measured a put's `decode_into` frame at 1,416 bytes on
# thumbv7em: the decode read a group's lists into locals and built the arm
# from them, so the frame held a copy of each inline list and more for each
# move (docs/adr/0065 §6, amendment of 2026-11-05). The decode now builds each
# list where the arm holds it. This script reads what that claim rests on.
#
# It stages the committed codec goldens into the probe, builds the probe's
# staticlib for thumbv7em-none-eabihf at the client's profile (opt-level s,
# lto), disassembles it, and reads each entry point's frame from its prologue
# (`push`, `vpush`, `sub sp`) with `tools/mcu_stack/worst_case_stack.py`, the
# reader the MCU footprint gate uses. For each of the three entry points it
# prints its own frame, the worst-case stack of the chain below it, and every
# frame on that chain, and it refuses any of those frames that is not shorter
# than one inline list of the codec. The length of a list is the probe's own
# number: each `SCE_CODEC_FRAME_<SHAPE>_LIST` symbol is that many bytes, read
# with `nm --print-size`.
#
# The disassembler is binutils for ARM (`arm-none-eabi-objdump`) when the
# machine has it, else LLVM's `llvm-objdump` from the rustup component
# `llvm-tools`, and the symbol reader the `nm` beside it (`arm-none-eabi-nm`,
# `llvm-nm`), else the host's `nm`. The script prints which it used.
#
# Usage (from the repository root):
#   scripts/codec_frame_probe.sh            measure the goldens of the tree, and
#                                           refuse a frame that holds a list
#   scripts/codec_frame_probe.sh --at REV   measure the goldens committed at REV
#                                           and print the frames without refusing
#                                           (the shape an earlier commit took)
#
# The runtime is the tree's in both, which builds an earlier commit's goldens:
# what the runtime gained for the in-place decode only adds to it.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

PROBE="backends/rust/probes/nostd-codec-frame"
TARGET="thumbv7em-none-eabihf"
LIB="$PROBE/target/$TARGET/release/libsce_nostd_codec_frame_probe.a"
# The goldens the probe's modules name: the three codecs it decodes and every
# codec they hold.
CODECS=(
    codec_chain_has_envelope
    codec_chain_has_marker
    codec_chain_has_marker_slice
    codec_chain_has_slice
    codec_repeat_elem
    codec_repeat_unconditional_count
    codec_zenoh_ext_entry
    codec_zenoh_ext_unit
    codec_zenoh_ext_zbuf
    codec_zenoh_ext_zint
)
SHAPES=(chain placed paired)

AT=""
while (( $# > 0 )); do
    case "$1" in
        --at)
            [[ $# -ge 2 ]] || { echo "codec_frame_probe: --at needs a revision" >&2; exit 2; }
            AT="$2"
            shift 2
            ;;
        -h|--help)
            sed -n '2,/^set -euo/p' "$0" | sed '$d'
            exit 0
            ;;
        *)
            echo "codec_frame_probe: unknown argument $1" >&2
            exit 2
            ;;
    esac
done

# The goldens, from the tree or from the revision asked for.
for codec in "${CODECS[@]}"; do
    if [[ -n "$AT" ]]; then
        git show "$AT:tests/forge/expected/$codec.rs" >"$PROBE/src/$codec.rs" \
            || { echo "codec_frame_probe: $codec.rs is not committed at $AT" >&2; exit 2; }
    else
        cp "tests/forge/expected/$codec.rs" "$PROBE/src/$codec.rs"
    fi
done

rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo build --release --manifest-path "$PROBE/Cargo.toml" --target "$TARGET" >&2 \
    || { echo "codec_frame_probe: the probe did not build" >&2; exit 1; }
[[ -f "$LIB" ]] || { echo "codec_frame_probe: no staticlib at $LIB" >&2; exit 1; }

# A tool for ARM objects: binutils for ARM when present, else LLVM's from the
# rustup component, else (for the symbol reader only) the host's.
arm_tool() {
    local gnu="$1" llvm="$2" host="${3:-}"
    if command -v "$gnu" >/dev/null 2>&1; then
        command -v "$gnu"
        return
    fi
    rustup component add llvm-tools >/dev/null 2>&1 || true
    local candidate
    candidate="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/$llvm"
    if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return
    fi
    if [[ -n "$host" ]] && command -v "$host" >/dev/null 2>&1; then
        command -v "$host"
    fi
}

OBJDUMP="$(arm_tool arm-none-eabi-objdump llvm-objdump)"
NM="$(arm_tool arm-none-eabi-nm llvm-nm nm)"
[[ -n "$OBJDUMP" ]] \
    || { echo "codec_frame_probe: no ARM disassembler: arm-none-eabi-objdump, or the rustup component llvm-tools" >&2; exit 1; }
[[ -n "$NM" ]] || { echo "codec_frame_probe: no symbol reader" >&2; exit 1; }
echo "codec_frame_probe: $(rustc --version); disassembler $OBJDUMP; symbols $NM"
if [[ -n "$AT" ]]; then
    echo "codec_frame_probe: goldens committed at $AT (recorded, not refused)"
else
    echo "codec_frame_probe: goldens of the tree"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# The length of one inline list of each shape, from the probe's own symbols.
"$NM" --print-size "$LIB" >"$WORK/symbols" 2>/dev/null \
    || { echo "codec_frame_probe: $NM could not read $LIB" >&2; exit 1; }
list_bytes() {
    local symbol="$1" size
    size="$(awk -v name="$symbol" '$NF == name && NF >= 4 { print $2; exit }' "$WORK/symbols")"
    [[ -n "$size" ]] || return 1
    printf '%d\n' "$((16#$size))"
}

"$OBJDUMP" -d -r -C --no-show-raw-insn "$LIB" >"$WORK/disassembly" 2>/dev/null \
    || { echo "codec_frame_probe: $OBJDUMP could not disassemble $LIB" >&2; exit 1; }

refused=0
for shape in "${SHAPES[@]}"; do
    root="sce_codec_frame_${shape}_decode_into"
    marker="SCE_CODEC_FRAME_$(tr '[:lower:]' '[:upper:]' <<<"$shape")_LIST"
    if ! ceiling="$(list_bytes "$marker")"; then
        echo "codec_frame_probe: no symbol $marker in $LIB: the instrument is not reading its own list" >&2
        exit 1
    fi
    if ! python3 tools/mcu_stack/worst_case_stack.py "$WORK/disassembly" \
        --root "$root" --min-functions 3 --report >"$WORK/$shape.report" 2>"$WORK/$shape.err"; then
        echo "codec_frame_probe: $root: the stack could not be read:" >&2
        cat "$WORK/$shape.err" >&2
        exit 1
    fi
    # The report's lines: `<bytes>  <root>`, then the chain one function a line,
    # each `<name> [<frame>]`, then the `worst-case-stack` summary.
    chain_bytes="$(awk -v root="$root" '$2 == root { print $1; exit }' "$WORK/$shape.report")"
    own="$(awk -v root="$root" 'index($0, root " [") { sub(/.*\[/, ""); sub(/\].*/, ""); print; exit }' "$WORK/$shape.report")"
    [[ "$chain_bytes" =~ ^[0-9]+$ && "$own" =~ ^[0-9]+$ ]] \
        || { echo "codec_frame_probe: $root: no frame in the report" >&2; cat "$WORK/$shape.report" >&2; exit 1; }
    printf '  %-6s %-36s own frame %5d B, worst-case stack %5d B, one inline list %5d B\n' \
        "$shape" "$root" "$own" "$chain_bytes" "$ceiling"
    # Every frame on the chain, the entry point's first: none may hold a list.
    while IFS= read -r line; do
        [[ "$line" =~ ^[[:space:]]+(.*)\ \[([0-9]+)(\ \+\ [0-9]+\ assumed)?\]$ ]] || continue
        name="${BASH_REMATCH[1]}"
        frame="${BASH_REMATCH[2]}"
        printf '         %6d B  %s\n' "$frame" "$name"
        if (( frame >= ceiling )); then
            printf '  [codec-frame] %s: %s takes %d B, not below one inline list (%d B)\n' \
                "$shape" "$name" "$frame" "$ceiling" >&2
            refused=1
        fi
    done < <(sed -n '2,$p' "$WORK/$shape.report" | sed '/^worst-case-stack /,$d' | sed '/^ *[0-9][0-9]*  /,$d')
done

if (( refused )); then
    if [[ -n "$AT" ]]; then
        echo "codec_frame_probe: a frame at $AT holds a whole list (recorded)"
        exit 0
    fi
    echo "codec_frame_probe: a decode's frame holds a whole list; it is built in a local and moved" >&2
    exit 1
fi
echo "codec_frame_probe: no frame of the three decodes holds a whole list"
