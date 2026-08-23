// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE C11 runtime — which reading §scxml-B-2-8-1 gave an event payload.
//
// The clause hands a payload three readings: interpretable as an XML document
// becomes a DOM, interpretable as structured data becomes the corresponding
// value, and anything else becomes a space-normalized string. The third is a
// fallback, and a fallback is where information leaves the system quietly: the
// document that reads `_event.data.field` afterwards sees nothing, assigns
// nothing, and takes the transition it would have taken had the host sent a
// payload with that field missing. The clause mandates the fallback and says
// nothing about telling anyone it happened.
//
// This type is what the engines say instead of nothing. It lives beside the
// other cross-fixture declarations rather than inside the generated machine
// because the RULE below has to be one rule: seven backends deciding
// separately what "looks like structure" means is how a wire contract grows
// six answers to one question, which this repository has measured happening.

#ifndef SCE_PAYLOAD_READING_H
#define SCE_PAYLOAD_READING_H

/* For `SCE_C_UNUSED`: a generated machine that never receives a payload still
   includes this header, and a `static inline` nobody calls is a warning in a
   build that treats warnings as errors. */
#include <stddef.h>

#include "sce/types.h"

/* The rungs of §scxml-B-2-8-1, plus the distinction the clause does not draw.
   `SCE_PAYLOAD_TEXT` and `SCE_PAYLOAD_UNDECODABLE` are the SAME rung of the
   clause — both are the space-normalized string — separated by whether the
   payload looked like it wanted a different one. */
typedef enum sce_payload_reading_e {
    /* No payload accompanied the event. Not a failure: most events carry
       none, and a diagnostic that fires on every one of those is noise. */
    SCE_PAYLOAD_ABSENT = 0,
    /* First rung: the content parsed as an XML document. */
    SCE_PAYLOAD_DOM,
    /* Second rung: the content parsed as structured data. */
    SCE_PAYLOAD_STRUCTURED,
    /* Third rung, working as intended: prose. W3C test 562 sends exactly
       this and requires it to arrive as a string. */
    SCE_PAYLOAD_TEXT,
    /* Third rung, reached by something that asked for the second: the content
       announced structure and the datamodel refused it. Same value in
       `_event.data`, entirely different thing to have happened. */
    SCE_PAYLOAD_UNDECODABLE
} sce_payload_reading_t;

/* Which of the two text readings applies, given content that did not parse.
   The mirror of cpp `SCE::payloadReadingOfText`, rust `PayloadReading::of_text`,
   go `sce.PayloadReadingOfText`, python `payload_reading_of_text` and kotlin
   `PayloadReading.ofText`.

   Only `{` and `[` announce structure. A number, a bare word or a quoted
   string is what an AUTHOR writes in a `<content>` element and the clause
   requires those to arrive as text without complaint; an object or an array
   is what a HOST constructs, and nobody constructs one by accident. Erring
   the other way would make the count fire on documents that are working,
   and a diagnostic that fires when nothing is wrong is one nobody reads. */
SCE_C_UNUSED static inline sce_payload_reading_t sce_payload_reading_of_text(const char *payload) {
    if (payload == NULL) {
        return SCE_PAYLOAD_ABSENT;
    }
    const char *scan = payload;
    while (*scan == ' ' || *scan == '\t' || *scan == '\n' || *scan == '\r' || *scan == '\f' || *scan == '\v') {
        scan++;
    }
    if (*scan == '\0') {
        return SCE_PAYLOAD_ABSENT;
    }
    return (*scan == '{' || *scan == '[') ? SCE_PAYLOAD_UNDECODABLE : SCE_PAYLOAD_TEXT;
}

#endif  // SCE_PAYLOAD_READING_H
