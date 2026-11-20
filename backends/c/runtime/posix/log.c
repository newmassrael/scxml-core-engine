// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The host's definition of the log seams (`sce/log.h`), where a machine's
// `<log>` and the runtime's diagnostics go.
//
// Each line goes to `stderr` in one write, spelled as the machine wrote it
// before the seam existed: `<label>: <text>`, the label alone or the text alone
// when the other is absent, and a newline after. A real is written with `%g`,
// as a `<log>` of a real was. The tests that read a machine's log read these
// lines, so the spelling is a contract of this file and not of the machine.
//
// The level is not written: every line went to the one stream before, and a
// host that wants them apart defines the seams itself.

#include <stdio.h>

#include "sce/log.h"

void _sce_log_write(sce_log_level_t level, const char *label, size_t label_len, const char *text, size_t text_len) {
    (void)level;
    if (label != NULL && text != NULL) {
        (void)fprintf(stderr, "%.*s: %.*s\n", (int)label_len, label, (int)text_len, text);
    } else if (label != NULL) {
        (void)fprintf(stderr, "%.*s\n", (int)label_len, label);
    } else if (text != NULL) {
        (void)fprintf(stderr, "%.*s\n", (int)text_len, text);
    } else {
        (void)fputc('\n', stderr);
    }
}

void _sce_log_real(sce_log_level_t level, const char *label, size_t label_len, double value) {
    (void)level;
    if (label != NULL) {
        (void)fprintf(stderr, "%.*s: %g\n", (int)label_len, label, value);
    } else {
        (void)fprintf(stderr, "%g\n", value);
    }
}
