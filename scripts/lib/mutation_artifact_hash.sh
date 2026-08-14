# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The hash a mutation verdict is drawn from: what an artifact IS, with the
# toolchain's per-invocation identifiers left out.
#
# `scripts/mutate` asks two questions of a test executable — did the mutation
# reach it, and did the restore put it back — and answers both by hashing it.
# That is the right instrument only if the bytes are a function of the
# sources. On this repository's C++ build they are not, and the measurement
# is exact (pc2, gcc 13.3, 2026-08-14): compiling one translation unit twice
# from identical source with identical flags produces objects differing in
# EIGHT bytes, at the `DWO ID` field of the DWARF 5 skeleton unit that
# `-gsplit-dwarf` emits. Everything else — `.text`, `.rodata`, the symbol
# table — is byte-identical.
#
# The linked binary inherits one such field per recompiled translation unit,
# plus `.note.gnu.build-id`, which is a hash OF that content and therefore
# moves with it. A restore that recompiles 19 units moved 159 bytes of a
# 76 MB executable, and the harness read that as "the restore did not
# reproduce the baseline" — a verdict about a mutation, drawn from an
# identifier the compiler picks anew on every invocation.
#
# Three levers were measured against it, and the result is why this file
# exists rather than a build-flag change:
#
#   -frandom-seed=<fixed>   still differs, same byte
#   SOURCE_DATE_EPOCH=0     still differs, same byte
#   no -gsplit-dwarf        IDENTICAL
#
# So the only build-level cure is dropping split DWARF, which the top-level
# CMakeLists chose deliberately and measured at -32% per binary and -33% on
# the build tree. Trading that away to make a test harness's hash stable
# would be the tail wagging the dog. What the harness needs is not the whole
# file but the part of it that decides behaviour, and that is what this
# computes: the artifact with its debugging sections and its build-id note
# removed.
#
# The DWO ID is worth naming precisely, because "ignore debug info" sounds
# like ignoring evidence. It is not a description of the program. It is a
# nonce that ties a skeleton unit to a `.dwo` file sitting beside the object,
# regenerated together with it. Two binaries differing only there are the
# same program built twice.

# Print the sha256 of `$1` as the harness should read it.
#
# ELF only, and detected by its magic rather than by extension: a suite
# registered through ctest can name a shell script as the thing it runs, and
# a script has no sections to normalise — it is hashed as it is.
mutation_artifact_hash() {
    local path="$1" magic
    magic="$(head -c 4 "$path" | od -An -tx1 | tr -d ' \n')"
    if [[ "$magic" == "7f454c46" && -n "${MUTATION_OBJCOPY:-}" ]]; then
        local normalized="${MUTATION_HASH_SCRATCH:-${TMPDIR:-/tmp}}/mutation-normalized.$$"
        if "$MUTATION_OBJCOPY" --strip-debug \
                --remove-section=.note.gnu.build-id \
                "$path" "$normalized" 2>/dev/null; then
            sha256sum "$normalized" | cut -d' ' -f1
            rm -f "$normalized"
            return 0
        fi
        rm -f "$normalized"
        # Not silent: an ELF this cannot normalise is one whose hash carries
        # the identifiers above, and the reader has to know that before a
        # restore check fails for a reason that is not the mutation.
        printf 'mutate: objcopy could not normalise %s — hashing it whole, so a\n' "$path" >&2
        printf '        restore check may fail on debug identifiers rather than on code\n' >&2
    fi
    sha256sum "$path" | cut -d' ' -f1
}
