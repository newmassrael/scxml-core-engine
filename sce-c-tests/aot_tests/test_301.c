// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML test301 — C11 AOT runner.
//
// W3C SCXML 5.8: <script src="..."/> at the document root with an
// unloadable URI must reject the document. The processor refuses to
// run the SM at all — test301's only state s0 transitions
// unconditionally to fail, so any execution would fail; correct W3C
// behaviour is to never instantiate the SM.
//
// The codegen pipeline detects the unloadable src in
// `parse_global_scripts` and routes through sce_codegen.rs's
// document_rejected branch, which emits a stub `_sm.h` containing
// `#define SCE_DOCUMENT_REJECTED 1` and an empty `_sm.c` (no SM
// struct, no datamodel). This runner mirrors the cpp
// `RejectedDocumentTest` macro: when the stub is in effect, the
// compiler sees the SCE_DOCUMENT_REJECTED define and the runner
// short-circuits to PASS without calling any SM API. The else
// branch is unreachable for test301 but kept so a regression that
// silently emits a real SM header would surface as a link error
// rather than as a green-but-meaningless test.

#include <stdio.h>

#include "test301_sm.h"

int main(void) {
#ifdef SCE_DOCUMENT_REJECTED
    /* W3C SCXML 5.8: document was rejected at codegen time — correct
       per spec. The processor refused the document, no execution
       occurs, verdict is PASS. */
    return 0;
#else
    (void)fprintf(stderr,
        "test301: FAIL — expected SCE_DOCUMENT_REJECTED to be defined "
        "(unloadable <script src> should reject the document at codegen time)\n");
    return 1;
#endif
}
