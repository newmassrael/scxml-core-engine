/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * Where a C11 machine's log goes: two seams the machine declares and its
 * deployment defines (docs/adr/0110, D2), as `_sce_clock_now_ms` is
 * (`sce/clock.h`).
 *
 * SCXML asks a processor to log a `<log>`'s label and value and leaves where,
 * and in what form, to the platform. A machine's core does no I/O and
 * holds no formatter of real numbers (docs/adr/0110, D1), so it hands the seam
 * what it has: a text it built with `sce/text.h` (a string, a boolean, the
 * digits of an integer), or a real whole, which the seam formats as it likes.
 * The runtime's own diagnostics (a host advancing a clock it does not own, a
 * script that failed) reach the same seam at their own level.
 *
 * The deployment that links the machine defines both, once for the image:
 *
 *   - `sce_c_runtime_posix` (backends/c/runtime/posix/log.c): the host's,
 *     writing each line to `stderr`.
 *   - A bare-metal deployment's: a UART, a ring buffer, or nothing at all.
 *
 * A machine that logs nothing references neither, and states no `log`
 * requirement (`RuntimeRequirement::Log`, sce-build/src/generator.rs), so a
 * deployment learns what it must define from what its machines state and from
 * the link.
 */
#ifndef SCE_LOG_H
#define SCE_LOG_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* How much a line matters. A `<log>` is information the document chose to
 * write; a warning is the runtime saying a host misused it and the machine
 * went on; an error is the runtime saying something failed. */
typedef enum sce_log_level { SCE_LOG_INFO = 0, SCE_LOG_WARNING = 1, SCE_LOG_ERROR = 2 } sce_log_level_t;

/* One line: the `label_len` bytes of `label` and the `text_len` bytes of `text`,
 * neither of which need end in a NUL. `label` is NULL when the line has none,
 * and `text` is NULL when it has no value (a `<log>` with a label and no
 * `expr`); an empty text is not the same as none. The seam adds what it writes
 * between the two and after them. */
void _sce_log_write(sce_log_level_t level, const char *label, size_t label_len, const char *text, size_t text_len);

/* One line whose value is a real number, handed whole: `label` as above. */
void _sce_log_real(sce_log_level_t level, const char *label, size_t label_len, double value);

#ifdef __cplusplus
}
#endif

#endif /* SCE_LOG_H */
