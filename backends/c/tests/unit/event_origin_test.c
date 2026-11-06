// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An event's origin on the C11 backend (docs/adr/0090): the reply address its
// processor writes, as one value, and the SCXML processor's address codec.
//
//   * a session id is published at `sce://scxml/` and the id percent-encoded, and
//     the location reads back as the id, a reserved character and a byte past
//     ASCII included (test 6 of the ADR, held to the C++ and Rust codec cases);
//   * a location that names no session, or names one longer than this build
//     holds, is read as naming none, and a bad escape is kept, not dropped;
//   * the pair is written whole or not at all, and the SCXML processor's pair of
//     a session is its type URI at its location, whose id is the sender read
//     back out of it; no other processor's pair names a sender;
//   * a processor alias (`scxml`, `basichttp`) is read as its type URI;
//   * a send target that is a location names the session the decoded id names;
//   * a format-2 `origin` and `origintype` read back as the table of section 7
//     says, never as half a pair, a location this backend once stored
//     unencoded among them.
//
// Header-only code; nothing here allocates.

#include <sce/host_processor.h>
#include <sce/saved_state.h>
#include <sce/send_target.h>
#include <sce/types.h>

#include <stdio.h>
#include <string.h>

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

// The location of `id` must be `want`, and must read back as `id`.
static void round_trips(const char *id, const char *want) {
    char location[SCE_MAX_URI_LEN];
    char back[SCE_MAX_SESSION_ID_LEN];
    CHECK(sce_scxml_location(location, sizeof(location), id), "the location of '%s' fits", id);
    CHECK(strcmp(location, want) == 0, "'%s' is published at '%s', not '%s'", id, location, want);
    CHECK(sce_scxml_location_len(id) == strlen(want), "the length of the location of '%s'", id);
    CHECK(sce_session_id_from_scxml_location(back, sizeof(back), location), "'%s' names a session", location);
    CHECK(strcmp(back, id) == 0, "'%s' reads back as '%s', not '%s'", location, back, id);
}

static void a_session_id_is_published_percent_encoded_and_reads_back(void) {
    round_trips("a b/c#d", "sce://scxml/a%20b%2Fc%23d");
    round_trips("parent.state.1:kid", "sce://scxml/parent.state.1%3Akid");
    round_trips("sess-1_A.~z", "sce://scxml/sess-1_A.~z");
    round_trips("caf\xC3\xA9", "sce://scxml/caf%C3%A9");
}

static void a_location_that_names_no_session_is_read_as_none(void) {
    char id[SCE_MAX_SESSION_ID_LEN];
    CHECK(!sce_session_id_from_scxml_location(id, sizeof(id), "sce://scxml/"), "the prefix alone names no session");
    CHECK(id[0] == '\0', "and leaves the id empty");
    CHECK(!sce_session_id_from_scxml_location(id, sizeof(id), "session-1"), "a bare id is not a location");
    CHECK(!sce_session_id_from_scxml_location(id, sizeof(id), "http://host/x"), "another scheme is not a location");
    CHECK(!sce_session_id_from_scxml_location(id, sizeof(id), NULL), "nothing is not a location");
}

static void a_bad_escape_is_kept_and_any_case_of_hex_is_read(void) {
    char id[SCE_MAX_SESSION_ID_LEN];
    CHECK(sce_session_id_from_scxml_location(id, sizeof(id), "sce://scxml/50%") && strcmp(id, "50%") == 0,
          "a trailing %% is a literal: '%s'", id);
    CHECK(sce_session_id_from_scxml_location(id, sizeof(id), "sce://scxml/a%zzb") && strcmp(id, "a%zzb") == 0,
          "a %% without hex digits is a literal: '%s'", id);
    CHECK(sce_session_id_from_scxml_location(id, sizeof(id), "sce://scxml/a%2fb") && strcmp(id, "a/b") == 0,
          "lower-case hex is an escape: '%s'", id);
}

static void a_location_or_an_id_that_does_not_fit_is_refused_not_cut(void) {
    char small[16];
    char id[4];
    CHECK(!sce_scxml_location(small, sizeof(small), "x y z"), "a location longer than its buffer is refused");
    CHECK(small[0] == '\0', "and leaves the buffer empty");
    CHECK(sce_scxml_location(small, sizeof(small), "abc"), "one that fits is written: '%s'", small);
    CHECK(!sce_session_id_from_scxml_location(id, sizeof(id), "sce://scxml/abcd"), "an id longer than its buffer");
    CHECK(id[0] == '\0', "is refused, leaving the id empty");
}

static void the_pair_is_written_whole_or_not_at_all(void) {
    sce_event_origin_t origin;
    char long_address[SCE_MAX_URI_LEN + 8];
    memset(long_address, 'a', sizeof(long_address) - 1u);
    long_address[sizeof(long_address) - 1u] = '\0';

    CHECK(sce_event_origin_set(&origin, "x-sce-host", "motor"), "a pair that fits is written");
    CHECK(strcmp(sce_event_origin_processor(&origin), "x-sce-host") == 0, "its processor");
    CHECK(strcmp(sce_event_origin_address(&origin), "motor") == 0, "its address, as named, with no scheme added");
    CHECK(!sce_event_origin_is_none(&origin), "it is an origin");

    CHECK(!sce_event_origin_set(&origin, "x-sce-host", long_address), "an address that does not fit is refused");
    CHECK(sce_event_origin_is_none(&origin), "and the processor is not kept without it");
    CHECK(strcmp(sce_event_origin_address(&origin), "") == 0 && strcmp(sce_event_origin_processor(&origin), "") == 0,
          "none reads as two empty strings");

    memset(&origin, 0, sizeof(origin));
    CHECK(sce_event_origin_is_none(&origin), "a zeroed event carries no origin");
    CHECK(sce_event_origin_is_none(NULL), "nothing is none");
}

static void a_session_sends_with_its_type_at_its_location(void) {
    sce_event_origin_t origin;
    uint8_t from_session = 0u;
    char sender[SCE_MAX_SESSION_ID_LEN];
    sce_event_stamp_session(&origin, &from_session, "a b");
    CHECK(from_session == 1u, "the SCXML processor's stamp marks a session's send");
    CHECK(strcmp(origin.processor, SCE_SCXML_EVENT_PROCESSOR_TYPE) == 0, "is the processor's URI: '%s'",
          origin.processor);
    CHECK(strcmp(origin.address, "sce://scxml/a%20b") == 0, "at the location it publishes: '%s'", origin.address);
    CHECK(sce_event_origin_sender(&origin, from_session, sender, sizeof(sender)) && strcmp(sender, "a b") == 0,
          "and its sender is the session that location names: '%s'", sender);

    sce_event_stamp_session(&origin, &from_session, "");
    CHECK(sce_event_origin_is_none(&origin) && from_session == 0u,
          "a sender that holds no id has no address to publish, so the event carries none and is no send");
    CHECK(!sce_event_origin_sender(&origin, from_session, sender, sizeof(sender)) && sender[0] == '\0',
          "and an event with no origin has no sender");
}

// On C11, which runs no remote child, only the SCXML processor's stamp makes a sender, whatever
// pair another names.
static void an_event_no_session_sent_has_no_sender(void) {
    sce_event_origin_t origin;
    char sender[SCE_MAX_SESSION_ID_LEN];
    // docs/adr/0090, section 6: a host that spells a session's own pair is still no session.
    CHECK(sce_event_origin_scxml(&origin, "kid_session_3"), "a host writes a session's pair itself");
    CHECK(!sce_event_origin_sender(&origin, 0u, sender, sizeof(sender)) && sender[0] == '\0',
          "and the event it posts, unmarked, names no sender");
    CHECK(sce_event_origin_set(&origin, "x-sce-host", "sce://scxml/kid"), "a host's pair at an SCXML-shaped address");
    CHECK(!sce_event_origin_sender(&origin, 1u, sender, sizeof(sender)),
          "names no session even marked: the processor is the host's");
    CHECK(sce_event_origin_mesh_peer(&origin, "hmi"), "a Mesh peer's pair");
    CHECK(!sce_event_origin_sender(&origin, 1u, sender, sizeof(sender)), "names no session: its address is mesh://");
    CHECK(sce_event_origin_scxml(&origin, "kid_session_3"), "a session's pair");
    CHECK(!sce_event_origin_sender(&origin, 1u, sender, 4u) && sender[0] == '\0',
          "whose id does not fit the room asked for is refused, not cut");
    CHECK(!sce_event_origin_sender(NULL, 1u, sender, sizeof(sender)), "nothing has no sender");
}

// docs/adr/0090, section 4: an alias reads as the URI it names; anything else as written.
static void a_processor_alias_is_read_as_its_type_uri(void) {
    CHECK(strcmp(sce_canonical_processor_type("scxml"), SCE_SCXML_EVENT_PROCESSOR_TYPE) == 0, "scxml");
    CHECK(strcmp(sce_canonical_processor_type("basichttp"), SCE_BASIC_HTTP_EVENT_PROCESSOR_TYPE) == 0, "basichttp");
    CHECK(strcmp(sce_canonical_processor_type(""), "") == 0, "the empty type is the default processor's, unchanged");
    CHECK(strcmp(sce_canonical_processor_type("x-example-host"), "x-example-host") == 0, "a host type, unchanged");
    CHECK(strcmp(sce_canonical_processor_type("SCXML"), "SCXML") == 0, "an alias is spelled in lower case");
    CHECK(sce_canonical_processor_type(NULL) == NULL, "nothing stays nothing");
}

static void a_mesh_peer_is_named_at_its_transport_address(void) {
    sce_event_origin_t origin;
    CHECK(sce_event_origin_mesh_peer(&origin, "hmi"), "a peer's event");
    CHECK(strcmp(origin.processor, SCE_SCXML_EVENT_PROCESSOR_TYPE) == 0 && strcmp(origin.address, "mesh://hmi") == 0,
          "is the SCXML processor at mesh://hmi: '%s' '%s'", origin.processor, origin.address);
    CHECK(sce_event_origin_mesh_rpc_reply(&origin, "motor"), "a request's answer");
    CHECK(strcmp(origin.processor, SCE_MESH_RPC_INVOKE_TYPE) == 0 && strcmp(origin.address, "mesh://motor") == 0,
          "is the request's type at mesh://motor: '%s' '%s'", origin.processor, origin.address);
    CHECK(sce_event_origin_mesh_rpc_reply(&origin, "") && sce_event_origin_is_none(&origin),
          "an end no peer answered names none");
}

static void a_location_target_names_the_session_its_id_decodes_to(void) {
    char id_buf[SCE_MAX_SESSION_ID_LEN];
    const char *id = NULL;
    CHECK(sce_classify_send_target("sce://scxml/kid%3A1", "me", id_buf, sizeof(id_buf), &id) == SCE_SEND_TARGET_SESSION,
          "another session's location is a session target");
    CHECK(id != NULL && strcmp(id, "kid:1") == 0, "naming the decoded id: '%s'", id != NULL ? id : "(none)");
    CHECK(sce_classify_send_target("sce://scxml/m%20e", "m e", id_buf, sizeof(id_buf), &id) == SCE_SEND_TARGET_SELF,
          "the sender's own location is the sender");
    CHECK(sce_classify_send_target("#_scxml_kid", "me", id_buf, sizeof(id_buf), &id) == SCE_SEND_TARGET_SESSION &&
              id != NULL && strcmp(id, "kid") == 0,
          "`#_scxml_<id>` names the id as written");
}

// The pair, and the sender it names, a stored `origin` and `origintype` read back as.
static void reads_as(const char *text, const char *type, const char *processor, const char *address,
                     const char *sender_want) {
    sce_event_origin_t origin;
    uint8_t from_session = 0u;
    char sender[SCE_MAX_SESSION_ID_LEN];
    CHECK(sce_saved_origin_read(text, type, &origin, &from_session), "'%s' of type '%s' is read", text, type);
    (void)sce_event_origin_sender(&origin, from_session, sender, sizeof(sender));
    CHECK(strcmp(origin.processor, processor) == 0, "'%s' of type '%s': processor '%s', not '%s'", text, type,
          origin.processor, processor);
    CHECK(strcmp(origin.address, address) == 0, "'%s' of type '%s': address '%s', not '%s'", text, type, origin.address,
          address);
    CHECK(strcmp(sender, sender_want) == 0, "'%s' of type '%s': sender '%s', not '%s'", text, type, sender,
          sender_want);
}

static void a_stored_origin_reads_back_as_the_table_of_section_7_says(void) {
    const char *scxml = SCE_SCXML_EVENT_PROCESSOR_TYPE;
    reads_as("", "", "", "", "");
    reads_as("motherSession", scxml, scxml, "sce://scxml/motherSession", "motherSession");
    reads_as("a:b", scxml, scxml, "sce://scxml/a%3Ab", "a:b");
    // What this backend stored before the pair: the location, unencoded.
    reads_as("sce://scxml/a:b", scxml, scxml, "sce://scxml/a%3Ab", "a:b");
    reads_as("sce://scxml/a%3Ab", "", scxml, "sce://scxml/a%3Ab", "a:b");
    // A value that starts `sce://scxml/` and names a session is that session's pair whatever type
    // was saved beside it (`ask` of the format-2 reading fixture).
    reads_as("sce://scxml/peer%3A1", "x-sce-host", scxml, "sce://scxml/peer%3A1", "peer:1");
    // The prefix alone names no session: the pair as saved, sent by no session.
    reads_as("sce://scxml/", "", scxml, "sce://scxml/", "");
    reads_as("motor", "x-sce-host", "x-sce-host", "motor", "");
    reads_as("mesh://hmi", scxml, scxml, "mesh://hmi", "");
    // An empty `origin` is none whatever type was saved beside it: format 2 saved the SCXML type
    // beside events no processor stamped, and an empty address answers nothing.
    reads_as("", scxml, "", "", "");
    reads_as("", "x-sce-host", "", "", "");
    // An address saved with an empty `origintype` is the SCXML processor's, what format 2 meant.
    reads_as("bare", "", scxml, "sce://scxml/bare", "bare");
    reads_as("mesh://hmi", "", scxml, "mesh://hmi", "");

    sce_event_origin_t origin;
    uint8_t from_session = 0u;
    CHECK(sce_saved_origin_read("motherSession", scxml, &origin, &from_session) && from_session == 1u,
          "a stored session's id is read back as a session's send");
    CHECK(sce_saved_origin_read("sce://scxml/peer:1", "", &origin, &from_session) && from_session == 1u,
          "a stored location is read back as the send of the session it names");
    CHECK(sce_saved_origin_read("motor", "x-sce-host", &origin, &from_session) && from_session == 0u,
          "a stored host pair is read back as no session's send");
    CHECK(sce_saved_origin_read("mesh://ecu", "", &origin, &from_session) && from_session == 0u,
          "an SCXML pair whose address names no session is no session's send");
    CHECK(sce_saved_origin_read("", scxml, &origin, &from_session) && from_session == 0u &&
              sce_event_origin_is_none(&origin),
          "and none is no session's send");
}

int main(void) {
    a_session_id_is_published_percent_encoded_and_reads_back();
    a_location_that_names_no_session_is_read_as_none();
    a_bad_escape_is_kept_and_any_case_of_hex_is_read();
    a_location_or_an_id_that_does_not_fit_is_refused_not_cut();
    the_pair_is_written_whole_or_not_at_all();
    a_session_sends_with_its_type_at_its_location();
    an_event_no_session_sent_has_no_sender();
    a_processor_alias_is_read_as_its_type_uri();
    a_mesh_peer_is_named_at_its_transport_address();
    a_location_target_names_the_session_its_id_decodes_to();
    a_stored_origin_reads_back_as_the_table_of_section_7_says();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("event origin: ok");
    return 0;
}
