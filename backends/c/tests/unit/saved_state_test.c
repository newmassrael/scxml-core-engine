// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The members of a saved session on the C11 backend (docs/adr/0045): `sce/saved_state.h`.
// The C half of the session cases of `tests/saved/SavedStateTest.cpp`, held to the same words:
//
//   * every shared saved text reads as a session of its format, a tree to every depth;
//   * a waiting send, a queued event and a host-run invocation read back as they were
//     written, and a text of another kind is refused in the words every backend uses;
//   * a record's origin is read as its format holds it: a format-3 pair and sender as written and
//     refused unless whole (docs/adr/0090, section 7), a format-2 `origin` by the table;
//   * a machine writes the same members the other backends write, byte for byte;
//   * a state is bound to the shape of its document and to the revision each schema was
//     saved at, and a wall-clock moment converts to the engine's clock and back;
//   * a state records the configuration of a product line it was taken in, a sorted set,
//     and a state of another configuration is refused by the features that differ.
//
// Nothing here allocates.

// `opendir` is POSIX, and the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include <sce/saved_state.h>

#include <dirent.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_SAVED_FIXTURES_DIR
#error "SCE_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/static_datamodel/saved; the build defines it"
#endif

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

enum { TEXT_CAP = 16384 };

// The text of `path` without its closing newline; false when it cannot be read.
static bool read_trimmed(const char *path, char *out, size_t cap, size_t *len) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return false;
    }
    const size_t n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    size_t end = n;
    while (end > 0u && (out[end - 1u] == '\n' || out[end - 1u] == '\r')) {
        --end;
    }
    out[end] = '\0';
    *len = end;
    return true;
}

// How many files of `directory` read as a session of `format`.
static int check_directory(const char *directory, int format) {
    DIR *dir = opendir(directory);
    int seen = 0;
    if (dir == NULL) {
        CHECK(false, "cannot open %s", directory);
        return 0;
    }
    for (struct dirent *entry = readdir(dir); entry != NULL; entry = readdir(dir)) {
        const size_t n = strlen(entry->d_name);
        if (n < 6u || strcmp(entry->d_name + n - 5u, ".json") != 0) {
            continue;
        }
        char path[1024];
        static char text[TEXT_CAP];
        size_t len = 0u;
        sce_saved_error_t err;
        sce_saved_session_t session;
        (void)snprintf(path, sizeof(path), "%s/%s", directory, entry->d_name);
        if (!read_trimmed(path, text, sizeof(text), &len)) {
            CHECK(false, "cannot read %s", path);
            continue;
        }
        const bool ok = sce_saved_read(text, len, &session, &err);
        CHECK(ok, "%s: %s", entry->d_name, ok ? "" : err.message);
        if (ok) {
            CHECK(session.format == format, "%s is format %d", entry->d_name, session.format);
        }
        ++seen;
    }
    (void)closedir(dir);
    return seen;
}

static void every_shared_text_reads_as_a_session_of_its_format(void) {
    char format1[1024];
    char format2[1024];
    char read_as[1024];
    (void)snprintf(format1, sizeof(format1), "%s/format1", SCE_SAVED_FIXTURES_DIR);
    (void)snprintf(format2, sizeof(format2), "%s/format2", SCE_SAVED_FIXTURES_DIR);
    (void)snprintf(read_as, sizeof(read_as), "%s/format2/read_as", SCE_SAVED_FIXTURES_DIR);
    CHECK(check_directory(SCE_SAVED_FIXTURES_DIR, SCE_SAVED_FORMAT_3) >= 10, "the format-3 texts");
    CHECK(check_directory(format1, SCE_SAVED_FORMAT_1) >= 10, "the format-1 texts");
    // The format-2 reading fixtures (docs/adr/0090, section 7), and the format-3 text each is read as.
    CHECK(check_directory(format2, SCE_SAVED_FORMAT_2) >= 4, "the format-2 reading texts");
    CHECK(check_directory(read_as, SCE_SAVED_FORMAT_3) >= 4, "the texts the format-2 reading texts are read as");
}

// `text` read as a state, the refusal it gets in `err`.
static bool refused(const char *text, sce_saved_error_t *err) {
    sce_saved_session_t session;
    return !sce_saved_read(text, strlen(text), &session, err);
}

static void a_text_that_is_not_a_session_is_refused_in_the_words_every_backend_uses(void) {
    sce_saved_error_t err;
    CHECK(refused("{", &err) && strstr(err.message, "the saved state is ") == err.message, "%s", err.message);
    CHECK(refused("[]", &err) && strcmp(err.message, "the saved state is not an object") == 0, "%s", err.message);
    CHECK(refused("{}", &err) && strcmp(err.message, "the saved state has no 'format'") == 0, "%s", err.message);
    CHECK(refused("{\"format\":4}", &err) &&
              strcmp(err.message, "the saved state is format 4, and this runtime reads formats 1, 2 and 3") == 0,
          "%s", err.message);
    CHECK(refused("{\"format\":\"2\"}", &err) &&
              strcmp(err.message, "the saved state is format \"2\", and this runtime reads formats 1, 2 and 3") == 0,
          "%s", err.message);
    // A format-3 text is judged by its format first: it is read as a session of that format.
    CHECK(refused("{\"format\":3}", &err) && strcmp(err.message, "the saved state has no 'configuration'") == 0, "%s",
          err.message);
    CHECK(refused("{\"format\":1}", &err) && strcmp(err.message, "the saved state has no 'configuration'") == 0, "%s",
          err.message);
    CHECK(refused("{\"format\":1,\"configuration\":{}}", &err) &&
              strcmp(err.message, "'configuration' is not an array") == 0,
          "%s", err.message);
    CHECK(refused("{\"format\":1,\"configuration\":[1]}", &err) &&
              strcmp(err.message, "'configuration' is not a text") == 0,
          "%s", err.message);
    CHECK(refused("{\"format\":1,\"configuration\":[],\"variables\":[]}", &err) &&
              strcmp(err.message, "'variables' is not an object") == 0,
          "%s", err.message);
}

// A format-1 state with `pending` and `external` in place.
static void a_member_is_judged_for_what_it_is(void) {
    static const char head[] = "{\"format\":1,\"configuration\":[],\"variables\":{},\"history\":{},";
    static const char tail[] = "\"invokes\":[],\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\","
                               "\"external\":[],\"shape\":\"s\",\"current\":\"c\"}";
    char text[1024];
    sce_saved_error_t err;
    (void)snprintf(text, sizeof(text), "%s\"pending\":[{\"due\":\"x\"}],%s", head, tail);
    CHECK(refused(text, &err) && strcmp(err.message, "'pending[0].due' (x) is not a whole number of milliseconds") == 0,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s\"pending\":[{\"due\":\"1\",\"act\":\"parent\"}],%s", head, tail);
    CHECK(refused(text, &err) &&
              strcmp(err.message,
                     "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1") == 0,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s\"pending\":[{\"due\":\"1\",\"act\":\"fly\"}],%s", head, tail);
    CHECK(refused(text, &err) &&
              strcmp(err.message,
                     "'pending[0].act' is 'fly', which is not raise, internal or host, the acts of format 1") == 0,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s\"pending\":[{\"due\":\"1\",\"act\":\"raise\"}],%s", head, tail);
    CHECK(refused(text, &err) && strcmp(err.message, "'pending[0]' has no 'event'") == 0, "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s\"pending\":[],%s", head, tail);
    {
        sce_saved_session_t session;
        CHECK(sce_saved_read(text, strlen(text), &session, &err), "%s", err.message);
    }
}

// The tree: a child session is judged to any depth, and refused with the path to it.
static void a_tree_is_judged_to_every_depth(void) {
    static char text[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    sce_saved_error_t err;
    sce_saved_session_t root;
    sce_jiter_t it;
    sce_jv_t item;
    sce_saved_child_t child;
    sce_saved_session_t session;
    (void)snprintf(path, sizeof(path), "%s/static_invoke_tree_working.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the tree text");
    CHECK(sce_saved_read(text, len, &root, &err), "%s", err.message);
    CHECK(sce_jv_string_is(root.session_id, "fixture-root"), "the root's id");
    CHECK(sce_saved_invoke_start(&root, "worker") == 2, "the root has started the worker twice");
    CHECK(sce_saved_invoke_start(&root, "nobody") == 0, "and nothing else");
    CHECK(sce_jv_iter(root.invokes, &it) && sce_jiter_element(&it, &item), "the root holds a child");
    CHECK(sce_saved_child_read(item, 0u, root.format, &child, &err), "%s", err.message);
    CHECK(sce_jv_string_is(child.id, "worker") && child.start == 2 &&
              sce_jv_string_is(child.invoke_id, "working.2.worker") &&
              sce_jv_string_is(child.document, "static_invoke_tree__sce_synth_invoke__worker"),
          "the child's words");
    CHECK(child.format == SCE_SAVED_FORMAT_3, "the child is read in the format of its tree: %d", child.format);
    CHECK(sce_saved_session_read(child.session, child.format, &session, &err), "%s", err.message);
    CHECK(sce_jv_string_is(session.session_id, "fixture-root.working.2.worker"), "the child's id");

    // A grandchild's refusal names the path from the root.
    {
        char broken[TEXT_CAP];
        const char *mark = strstr(text, "\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]}}]");
        CHECK(mark != NULL, "the grandchild's tail is in the text");
        if (mark != NULL) {
            const size_t head = (size_t)(mark - text);
            (void)snprintf(broken, sizeof(broken), "%.*s\"hostinvoketoken\":\"x\"%s", (int)head, text,
                           mark + strlen("\"hostinvoketoken\":\"0\""));
            CHECK(refused(broken, &err) &&
                      strstr(err.message, "the session at invokes[0].session.invokes[0].session: ") == err.message,
                  "%s", err.message);
        }
    }
}

static void a_waiting_send_a_queued_event_and_a_host_invocation_read_back_as_they_were(void) {
    static char text[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    char field[64];
    sce_saved_error_t err;
    sce_saved_session_t timers;
    sce_jiter_t it;
    sce_jv_t item;
    sce_jv_t address;
    sce_saved_send_t send;
    (void)snprintf(path, sizeof(path), "%s/static_timers_midway.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the timers text");
    CHECK(sce_saved_read(text, len, &timers, &err), "%s", err.message);
    CHECK(sce_jv_count(timers.pending) == 3u, "three sends wait");
    CHECK(sce_jv_iter(timers.pending, &it) && sce_jiter_element(&it, &item), "the first");
    CHECK(sce_saved_send_read(item, "pending[0]", timers.format, &send, &err), "%s", err.message);
    CHECK(send.due == 1700000002000 && send.act == SCE_SAVED_ACT_RAISE, "its moment and act");
    CHECK(sce_jv_string(send.event, field, sizeof(field), NULL) && strcmp(field, "beat") == 0, "its event");
    CHECK(sce_jv_string(send.send_id, field, sizeof(field), NULL) && strcmp(field, "__send_0") == 0, "its id");
    CHECK(sce_jv_kind(send.origin) == SCE_JSON_OBJECT && sce_jv_get(send.origin, "address", &address) &&
              sce_jv_string_is(address, "sce://scxml/fixture-root"),
          "its origin, this session's pair");
    CHECK(sce_jv_string(send.sender, field, sizeof(field), NULL) && strcmp(field, "fixture-root") == 0, "its sender");

    // The tree's root holds an event, and its child a host-run invocation.
    (void)snprintf(path, sizeof(path), "%s/static_invoke_tree_working.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the tree text");
    {
        sce_saved_session_t root;
        sce_saved_event_t event;
        sce_saved_child_t child;
        sce_saved_session_t inner;
        sce_saved_host_invoke_t host;
        CHECK(sce_saved_read(text, len, &root, &err), "%s", err.message);
        CHECK(sce_jv_iter(root.external, &it) && sce_jiter_element(&it, &item), "an event waits");
        CHECK(sce_saved_event_read(item, "external[0]", root.format, &event, &err), "%s", err.message);
        CHECK(sce_jv_string_is(event.name, "answer") && sce_jv_string_is(event.invoke_id, "worker") &&
                  !event.has_host_invoke_token,
              "the event's words");
        CHECK(strcmp(event.origin.processor, SCE_SCXML_EVENT_PROCESSOR_TYPE) == 0 &&
                  strcmp(event.origin.address, "sce://scxml/fixture-root.working.2.worker") == 0 &&
                  event.from_session == 1u,
              "the event's pair, sent by the child: '%s' '%s' %u", event.origin.processor, event.origin.address,
              (unsigned)event.from_session);
        CHECK(sce_jv_iter(root.invokes, &it) && sce_jiter_element(&it, &item), "a child runs");
        CHECK(sce_saved_child_read(item, 0u, root.format, &child, &err), "%s", err.message);
        CHECK(sce_saved_session_read(child.session, child.format, &inner, &err), "%s", err.message);
        CHECK(inner.host_invoke_token == 1, "the child has handed out one token");
        CHECK(sce_jv_iter(inner.host_invokes, &it) && sce_jiter_element(&it, &item), "a host run waits");
        CHECK(sce_saved_host_invoke_read(item, "hostinvokes[0]", &host, &err), "%s", err.message);
        CHECK(sce_jv_string_is(host.type, "sce-host-clock-now") && sce_jv_string_is(host.id, "clock") && !host.has_due,
              "the invocation's words");
    }
}

// ── The origin a record holds (docs/adr/0090, section 7) ──────────────────────────────────────

#define SCXML_TYPE "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

// A tree of `format` holding `pending` and `external`, the texts of two JSON arrays, read into
// `out`: false, with the reason in `err`, when it is refused as it is read. The text is held until
// the next call, so `out` reads it in place.
static bool tree_holding(int format, const char *pending, const char *external, sce_saved_session_t *out,
                         sce_saved_error_t *err) {
    static char text[4096];
    (void)snprintf(text, sizeof(text),
                   "{\"format\":%d,\"shape\":\"s\",\"sessionid\":\"x\",\"configuration\":[],\"current\":\"c\","
                   "\"variables\":{},\"history\":{},\"pending\":%s,\"invokes\":[],\"invokestarts\":{},"
                   "\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":%s}",
                   format, pending, external);
    return sce_saved_read(text, strlen(text), out, err);
}

// A queue of one event whose origin members are `members`, the text between `sendid` and
// `invokeid`.
static void queue_of(const char *members, char *out, size_t cap) {
    (void)snprintf(out, cap,
                   "[{\"name\":\"a\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",%s,\"invokeid\":\"\","
                   "\"hostinvoketoken\":null}]",
                   members);
}

// The first event of the queue `session` holds, read as the format of its text says.
static bool first_event(const sce_saved_session_t *session, sce_saved_event_t *out, sce_saved_error_t *err) {
    sce_jiter_t it;
    sce_jv_t item;
    return sce_jv_iter(session->external, &it) && sce_jiter_element(&it, &item) &&
           sce_saved_event_read(item, "external[0]", session->format, out, err);
}

static void a_format_three_origin_is_read_as_it_is_written(void) {
    char queue[1024];
    char sender[64];
    sce_saved_session_t session;
    sce_saved_event_t event;
    sce_saved_error_t err;
    memset(&err, 0, sizeof(err));

    // A session's send: the pair, and the id its location names, which is held once.
    queue_of("\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/peer%3A1\"},\"sender\":\"peer:1\"",
             queue, sizeof(queue));
    CHECK(tree_holding(3, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(strcmp(event.origin.processor, SCXML_TYPE) == 0 && strcmp(event.origin.address, "sce://scxml/peer%3A1") == 0,
          "the pair as written: '%s' '%s'", event.origin.processor, event.origin.address);
    CHECK(event.from_session == 1u &&
              sce_event_origin_sender(&event.origin, event.from_session, sender, sizeof(sender)) &&
              strcmp(sender, "peer:1") == 0,
          "sent by the session it names: '%s'", sender);

    // A host's pair, sent by no session, though it may spell a session's location.
    queue_of("\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"motor\"},\"sender\":null", queue, sizeof(queue));
    CHECK(tree_holding(3, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(strcmp(event.origin.processor, "x-sce-host") == 0 && strcmp(event.origin.address, "motor") == 0 &&
              event.from_session == 0u,
          "a host's pair: '%s' '%s' %u", event.origin.processor, event.origin.address, (unsigned)event.from_session);
    queue_of("\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/kid\"},\"sender\":null", queue,
             sizeof(queue));
    CHECK(tree_holding(3, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(strcmp(event.origin.address, "sce://scxml/kid") == 0 && event.from_session == 0u,
          "a sender is not derived from the pair: %u", (unsigned)event.from_session);

    // No origin.
    queue_of("\"origin\":null,\"sender\":null", queue, sizeof(queue));
    CHECK(tree_holding(3, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(sce_event_origin_is_none(&event.origin) && event.from_session == 0u, "none");

    // A format-2 text is read by the table of section 7: an unencoded location is the pair of the
    // session it names, written again encoded.
    queue_of("\"origin\":\"sce://scxml/peer:1\",\"origintype\":\"\"", queue, sizeof(queue));
    CHECK(tree_holding(2, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(strcmp(event.origin.processor, SCXML_TYPE) == 0 &&
              strcmp(event.origin.address, "sce://scxml/peer%3A1") == 0 && event.from_session == 1u,
          "the format-2 location: '%s' '%s' %u", event.origin.processor, event.origin.address,
          (unsigned)event.from_session);
    // The SCXML prefix alone names no session: a queued event that stores it is not refused, and is
    // read as the pair as saved, sent by no session.
    queue_of("\"origin\":\"sce://scxml/\",\"origintype\":\"\"", queue, sizeof(queue));
    CHECK(tree_holding(2, "[]", queue, &session, &err) && first_event(&session, &event, &err), "%s", err.message);
    CHECK(strcmp(event.origin.processor, SCXML_TYPE) == 0 && strcmp(event.origin.address, "sce://scxml/") == 0 &&
              event.from_session == 0u,
          "the prefix alone: '%s' '%s' %u", event.origin.processor, event.origin.address, (unsigned)event.from_session);
    // And a format-2 text holds its origin as a text, not as a pair.
    queue_of("\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"motor\"},\"origintype\":\"\"", queue,
             sizeof(queue));
    CHECK(!tree_holding(2, "[]", queue, &session, &err) &&
              strcmp(err.message, "'external[0].origin' is not a text") == 0,
          "%s", err.message);
}

static void a_format_three_origin_that_is_not_whole_is_refused(void) {
    typedef struct {
        const char *members;
        const char *reason;
    } refusal_t;

    static const refusal_t cases[] = {
        {"\"origin\":\"motor\",\"sender\":null",
         "'external[0].origin' is neither null nor a pair of a processor and an address"},
        {"\"origin\":[],\"sender\":null",
         "'external[0].origin' is neither null nor a pair of a processor and an address"},
        {"\"origin\":{\"processor\":\"x-sce-host\"},\"sender\":null", "'external[0].origin' has no 'address'"},
        {"\"origin\":{\"address\":\"motor\"},\"sender\":null", "'external[0].origin' has no 'processor'"},
        {"\"origin\":{\"processor\":\"\",\"address\":\"motor\"},\"sender\":null",
         "'external[0].origin.processor' is empty"},
        {"\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"\"},\"sender\":null",
         "'external[0].origin.address' is empty"},
        {"\"origin\":{\"processor\":1,\"address\":\"motor\"},\"sender\":null",
         "'external[0].origin.processor' is not a text"},
        {"\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"motor\",\"port\":\"1\"},\"sender\":null",
         "'external[0].origin' holds a member other than 'processor' and 'address'"},
        {"\"origin\":null,\"sender\":7", "'external[0].sender' is neither null nor a session id"},
        {"\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/kid\"},\"sender\":\"a b\"",
         "'external[0].sender' (a b) is not an XML Nmtoken, which a session id is"},
        // A session that sent an event has a reply address: a sender beside a null origin.
        {"\"origin\":null,\"sender\":\"kid\"",
         "'external[0]' names the sender 'kid' and no origin: a session that sent an event has a reply address"},
        // This build holds a sender only as the id of the SCXML location its pair names, byte for
        // byte: a bound of C11, which runs no remote child, not a rule of the format.
        {"\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"sce://scxml/kid\"},\"sender\":\"kid\"",
         "'external[0].origin' is not the SCXML location of its sender 'kid': C11 holds a sender only as the id of "
         "its SCXML location"},
        {"\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/other\"},\"sender\":\"kid\"",
         "'external[0].origin' is not the SCXML location of its sender 'kid': C11 holds a sender only as the id of "
         "its SCXML location"},
        {"\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"mesh://dev/machine/kid\"},\"sender\":\"kid\"",
         "'external[0].origin' is not the SCXML location of its sender 'kid': C11 holds a sender only as the id of "
         "its SCXML location"},
        {"\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/peer:1\"},\"sender\":\"peer:1\"",
         "'external[0].origin' is not the SCXML location of its sender 'peer:1': C11 holds a sender only as the id "
         "of its SCXML location"},
        // Both members are in every record that carries the pair.
        {"\"origin\":null", "'external[0]' has no 'sender'"},
        {"\"sender\":null", "'external[0]' has no 'origin'"},
        // The format-2 text `origin` in a format-3 record, with its `sender` present so that the
        // origin's shape is the one fault.
        {"\"origin\":\"\",\"sender\":null", "'external[0].origin' is neither null nor a pair of a processor and an "
                                            "address"},
    };
    char queue[1024];
    char long_type[SCE_MAX_ID_LEN + 8];
    char members[SCE_MAX_ID_LEN + 128];
    char reason[128];
    sce_saved_session_t session;
    sce_saved_error_t err;
    memset(&err, 0, sizeof(err));
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        queue_of(cases[i].members, queue, sizeof(queue));
        CHECK(!tree_holding(3, "[]", queue, &session, &err) && strcmp(err.message, cases[i].reason) == 0, "%s: %s",
              cases[i].members, err.message);
    }
    // A pair this build cannot hold whole is refused, not cut.
    memset(long_type, 'p', sizeof(long_type) - 1u);
    long_type[sizeof(long_type) - 1u] = '\0';
    (void)snprintf(members, sizeof(members), "\"origin\":{\"processor\":\"%s\",\"address\":\"motor\"},\"sender\":null",
                   long_type);
    (void)snprintf(reason, sizeof(reason),
                   "'external[0].origin.processor' holds %lu UTF-8 bytes, past the %lu it is "
                   "bounded by",
                   (unsigned long)strlen(long_type), (unsigned long)(SCE_MAX_ID_LEN - 1));
    queue_of(members, queue, sizeof(queue));
    CHECK(!tree_holding(3, "[]", queue, &session, &err) && strcmp(err.message, reason) == 0, "%s", err.message);
}

// One waiting send of `act` whose origin members are `members`, as a `pending` array.
static void pending_of(const char *act, const char *members, char *out, size_t cap) {
    (void)snprintf(out, cap, "[{\"due\":\"1\",\"act\":\"%s\",%s\"event\":\"e\",\"data\":\"\",\"sendid\":\"s\",%s}]",
                   act, strcmp(act, "child") == 0 ? "\"invokeid\":\"kid\"," : "", members);
}

static void a_format_three_act_holds_what_its_event_will_carry(void) {
    typedef struct {
        const char *act;
        const char *members;
        const char *reason;  // NULL for a send that is read
    } act_case_t;

    static const act_case_t cases[] = {
        // A session's act is its own send, and it names its sender.
        {"raise", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":\"x\"", NULL},
        {"parent", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":\"x\"",
         NULL},
        {"child", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":\"x\"", NULL},
        {"raise", "\"origin\":null,\"sender\":null",
         "'pending[0]' is a raise act, this session's own send, and it names no sender"},
        {"parent", "\"origin\":null,\"sender\":null",
         "'pending[0]' is a parent act, this session's own send, and it names no sender"},
        {"child", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":null",
         "'pending[0]' is a child act, this session's own send, and it names no sender"},
        // It carries this session's pair: the SCXML pair of its sender, and its sender is this
        // session (`x`).
        {"raise", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/y\"},\"sender\":\"x\"",
         "'pending[0].origin' is not the SCXML pair of its sender 'x': a raise act is this session's own send, which "
         "carries this session's pair"},
        {"parent", "\"origin\":null,\"sender\":\"x\"",
         "'pending[0].origin' is not the SCXML pair of its sender 'x': a parent act is this session's own send, "
         "which carries this session's pair"},
        {"raise", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/y\"},\"sender\":\"y\"",
         "'pending[0].sender' is 'y', and this session is 'x': a raise act is this session's own send"},
        {"child", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/y\"},\"sender\":\"y\"",
         "'pending[0].sender' is 'y', and this session is 'x': a child act is this session's own send"},
        // An internal act delivers an internal event, which has no origin.
        {"internal", "\"origin\":null,\"sender\":null", NULL},
        {"internal", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":\"x\"",
         "'pending[0]' is an internal act, whose event is internal and has no origin, and it holds an origin or a "
         "sender"},
        {"internal", "\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"motor\"},\"sender\":null",
         "'pending[0]' is an internal act, whose event is internal and has no origin, and it holds an origin or a "
         "sender"},
        // The pair of an act is judged as an event's is.
        {"raise", "\"origin\":\"x\",\"sender\":\"x\"",
         "'pending[0].origin' is neither null nor a pair of a processor and an address"},
        {"raise", "\"origin\":\"x\"", "'pending[0]' has no 'sender'"},
    };
    char pending[1024];
    sce_saved_session_t session;
    sce_saved_send_t send;
    sce_saved_error_t err;
    sce_jiter_t it;
    sce_jv_t item;
    memset(&err, 0, sizeof(err));
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        pending_of(cases[i].act, cases[i].members, pending, sizeof(pending));
        const bool accepted = tree_holding(3, pending, "[]", &session, &err);
        if (cases[i].reason == NULL) {
            CHECK(accepted, "%s %s: %s", cases[i].act, cases[i].members, err.message);
        } else {
            CHECK(!accepted && strcmp(err.message, cases[i].reason) == 0, "%s %s: %s", cases[i].act, cases[i].members,
                  err.message);
        }
    }
    // What a session's act is read as: the two members, as spans of the text.
    pending_of("raise", "\"origin\":{\"processor\":\"" SCXML_TYPE "\",\"address\":\"sce://scxml/x\"},\"sender\":\"x\"",
               pending, sizeof(pending));
    CHECK(tree_holding(3, pending, "[]", &session, &err), "%s", err.message);
    CHECK(sce_jv_iter(session.pending, &it) && sce_jiter_element(&it, &item) &&
              sce_saved_send_read(item, "pending[0]", session.format, &send, &err),
          "%s", err.message);
    CHECK(sce_jv_kind(send.origin) == SCE_JSON_OBJECT && sce_jv_string_is(send.sender, "x"), "the act's members");

    // A format-2 act stores the sending session's id, and an internal act's is not read.
    pending_of("internal", "\"origin\":\"fixture-root\"", pending, sizeof(pending));
    CHECK(tree_holding(2, pending, "[]", &session, &err), "%s", err.message);
    // A format-2 session's act stored with an empty origin is sent by the session that holds it.
    pending_of("raise", "\"origin\":\"\"", pending, sizeof(pending));
    CHECK(tree_holding(2, pending, "[]", &session, &err), "%s", err.message);
    // The SCXML prefix alone names no session, so a session's act that stores it is refused; an
    // internal act's origin is not read.
    pending_of("raise", "\"origin\":\"sce://scxml/\"", pending, sizeof(pending));
    CHECK(!tree_holding(2, pending, "[]", &session, &err) &&
              strcmp(err.message, "'pending[0].origin' is 'sce://scxml/', a location that names no session: a "
                                  "raise act is this session's own send") == 0,
          "%s", err.message);
    pending_of("child", "\"origin\":\"sce://scxml/\"", pending, sizeof(pending));
    CHECK(!tree_holding(2, pending, "[]", &session, &err) &&
              strcmp(err.message, "'pending[0].origin' is 'sce://scxml/', a location that names no session: a "
                                  "child act is this session's own send") == 0,
          "%s", err.message);
    pending_of("internal", "\"origin\":\"sce://scxml/\"", pending, sizeof(pending));
    CHECK(tree_holding(2, pending, "[]", &session, &err), "%s", err.message);
    pending_of("raise", "\"origin\":null", pending, sizeof(pending));
    CHECK(!tree_holding(2, pending, "[]", &session, &err) &&
              strcmp(err.message, "'pending[0].origin' is not a text") == 0,
          "%s", err.message);
}

static void a_count_a_token_and_a_moment_are_whole_numbers_within_what_they_can_hold(void) {
    sce_saved_error_t err;
    sce_jv_t v;
    int64_t out = 0;
    CHECK(sce_json_parse("\"7\"", 3u, &v, &err) && sce_saved_read_count(v, "n", &out, &err) && out == 7, "7");
    CHECK(sce_json_parse("\"0\"", 3u, &v, &err) && !sce_saved_read_count(v, "n", &out, &err) &&
              strcmp(err.message, "'n' (0) is not a count of starts, a whole number from 1") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("\"07\"", 4u, &v, &err) && !sce_saved_read_count(v, "n", &out, &err), "a leading zero");
    CHECK(sce_json_parse("\"0\"", 3u, &v, &err) && sce_saved_read_token(v, "n", &out, &err) && out == 0,
          "a token of 0");
    CHECK(sce_json_parse("\"9223372036854775807\"", 21u, &v, &err) && sce_saved_read_token(v, "n", &out, &err) &&
              out == INT64_MAX,
          "the widest token");
    CHECK(sce_json_parse("\"9223372036854775808\"", 21u, &v, &err) && !sce_saved_read_token(v, "n", &out, &err) &&
              strcmp(err.message, "'n' (9223372036854775808) is not a whole number a token can be") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("\"-1\"", 4u, &v, &err) && !sce_saved_read_moment(v, "t", &out, &err) &&
              strcmp(err.message, "'t' (-1) is not a whole number of milliseconds") == 0,
          "%s", err.message);
}

static void a_machine_writes_the_members_the_other_backends_write(void) {
    char buf[1024];
    sce_jw_t w;
    sce_saved_send_out_t send;
    sce_saved_event_out_t event;
    sce_saved_host_invoke_out_t host;
    sce_event_origin_t own;
    sce_event_origin_t child;
    uint8_t from_session = 0u;
    static const char names[3][8] = {"b", "a", "b"};
    static const char values[3][8] = {"1", "2", "3"};

    // A session's act carries the pair this session's processor stamps, and its id (docs/adr/0090,
    // section 7): the bytes the shared timers hold.
    sce_event_stamp_session(&own, &from_session, "fixture-root");
    CHECK(from_session == 1u, "the session stamps its own send");
    memset(&send, 0, sizeof(send));
    send.due = 1700000002000;
    send.act = SCE_SAVED_ACT_RAISE;
    send.event = "beat";
    send.data = "";
    send.send_id = "__send_0";
    send.origin = &own;
    send.sender = "fixture-root";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"raise\",\"event\":\"beat\",\"data\":\"\","
                          "\"sendid\":\"__send_0\",\"origin\":{\"processor\":"
                          "\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\",\"address\":"
                          "\"sce://scxml/fixture-root\"},\"sender\":\"fixture-root\"}") == 0,
          "%s", buf);

    send.act = SCE_SAVED_ACT_CHILD;
    send.invoke_id = "worker";
    send.event = "nudge";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"child\",\"invokeid\":\"worker\",\"event\":\"nudge\","
                          "\"data\":\"\",\"sendid\":\"__send_0\",\"origin\":{\"processor\":"
                          "\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\",\"address\":"
                          "\"sce://scxml/fixture-root\"},\"sender\":\"fixture-root\"}") == 0,
          "%s", buf);

    // An internal act delivers an internal event, which has no origin: both members are null.
    send.act = SCE_SAVED_ACT_INTERNAL;
    send.invoke_id = "";
    send.event = "inner";
    send.send_id = "__send_2";
    send.origin = NULL;
    send.sender = NULL;
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"internal\",\"event\":\"inner\",\"data\":\"\","
                          "\"sendid\":\"__send_2\",\"origin\":null,\"sender\":null}") == 0,
          "%s", buf);
    send.send_id = "__send_0";
    send.event = "nudge";

    send.act = SCE_SAVED_ACT_HOST;
    send.type = "http";
    send.target = "t";
    send.content = "c";
    send.invoke_id = "";
    send.param_count = 3u;
    send.param_names = names[0];
    send.param_name_stride = sizeof(names[0]);
    send.param_values = values[0];
    send.param_value_stride = sizeof(values[0]);
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"host\",\"type\":\"http\",\"event\":\"nudge\","
                          "\"target\":\"t\",\"content\":\"c\",\"params\":{\"a\":[\"2\"],\"b\":[\"1\",\"3\"]},"
                          "\"sendid\":\"__send_0\",\"data\":\"\",\"invokeid\":\"\"}") == 0,
          "%s", buf);

    // A queued event a session sent: the pair, then the sender, where format 2 held `origin` and
    // `origintype` — the bytes of the shared tree's `answer`.
    memset(&event, 0, sizeof(event));
    sce_event_stamp_session(&child, &from_session, "fixture-root.working.2.worker");
    event.name = "answer";
    event.data = "";
    event.type = "external";
    event.send_id = "";
    event.origin = &child;
    event.sender = "fixture-root.working.2.worker";
    event.invoke_id = "worker";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_event(&w, &event);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"name\":\"answer\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",\"origin\":{"
                          "\"processor\":\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\",\"address\":"
                          "\"sce://scxml/fixture-root.working.2.worker\"},\"sender\":"
                          "\"fixture-root.working.2.worker\",\"invokeid\":\"worker\",\"hostinvoketoken\":null}") == 0,
          "%s", buf);
    // A host's pair is written as the host named it, and the event has no sender.
    CHECK(sce_event_origin_set(&child, "x-sce-host", "motor"), "a host's pair");
    event.sender = NULL;
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_event(&w, &event);
    CHECK(sce_jw_finish(&w) &&
              strstr(buf, "\"origin\":{\"processor\":\"x-sce-host\",\"address\":\"motor\"},\"sender\":null,") != NULL,
          "%s", buf);
    // An event no processor stamped has no origin, and both members are null.
    event.origin = NULL;
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_event(&w, &event);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"name\":\"answer\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",\"origin\":null,"
                          "\"sender\":null,\"invokeid\":\"worker\",\"hostinvoketoken\":null}") == 0,
          "%s", buf);
    event.has_host_invoke_token = true;
    event.host_invoke_token = 4u;
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_event(&w, &event);
    CHECK(sce_jw_finish(&w) && strstr(buf, "\"hostinvoketoken\":\"4\"}") != NULL, "%s", buf);

    memset(&host, 0, sizeof(host));
    host.type = "sce-host-clock-now";
    host.id = "clock";
    host.src = "";
    host.data = "";
    host.content = "";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_host_invoke(&w, &host);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"type\":\"sce-host-clock-now\",\"id\":\"clock\",\"src\":\"\",\"params\":{},\"data\":\"\","
                          "\"content\":\"\",\"due\":null}") == 0,
          "%s", buf);
    host.has_due = true;
    host.due = 1700000009000;
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_host_invoke(&w, &host);
    CHECK(sce_jw_finish(&w) && strstr(buf, "\"due\":\"1700000009000\"}") != NULL, "%s", buf);
}

static void a_state_is_bound_to_its_shape_and_to_the_revision_each_schema_was_saved_at(void) {
    // The digest of revision r is the (r - 2)th: revision 2's, then revision 3's the text names.
    static const char *const digests[] = {"2222222222222222222222222222222222222222222222222222222222222222",
                                          "e511eddacf1d4e220c85f78c95988fe26a2a1319cf119cb085f1a84f4f197f8d"};
    static const sce_saved_lineage_t records[] = {{"reading.revised", 3u, digests, 2u, NULL, 0u, NULL, 0u}};
    static const sce_saved_lineage_t enums[] = {{"enum:level", 1u, NULL, 0u, NULL, 0u, NULL, 0u}};
    // The shape the shared revision text names, as a tree (format 2 or 3) does when it names a
    // revision.
    sce_saved_shape_t shape;
    static char text[TEXT_CAP];
    char path[1024];
    size_t len = 0u;
    sce_saved_session_t session;
    sce_saved_error_t err;
    unsigned record_at[1];
    unsigned enum_at[1];

    (void)snprintf(path, sizeof(path), "%s/static_schema_revision.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the revision text");
    CHECK(sce_saved_read(text, len, &session, &err), "%s", err.message);
    memset(&shape, 0, sizeof(shape));
    shape.first = "first";
    shape.format2_first = "first";
    shape.revised = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5";
    shape.format2_revised = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5";
    shape.records = records;
    shape.record_count = 1u;
    shape.enums = enums;
    shape.enum_count = 1u;

    CHECK(sce_saved_check_shape(&session, &shape, record_at, enum_at, &err), "%s", err.message);
    CHECK(record_at[0] == 3u && enum_at[0] == 1u, "saved at revision 3 of the record, 1 of the enum");

    // This machine is on an earlier revision than the state: refused, not read without the fields.
    {
        static const sce_saved_lineage_t older[] = {{"reading.revised", 2u, digests, 1u, NULL, 0u, NULL, 0u}};
        // (an older machine has only revision 2's digest)
        shape.records = older;
        CHECK(!sce_saved_check_shape(&session, &shape, record_at, enum_at, &err) &&
                  strcmp(err.message,
                         "the saved state was saved at revision 3 of 'reading.revised', later than this machine's 2: a "
                         "machine does not restore what it does not know") == 0,
              "%s", err.message);
    }
    // Another document's shape.
    shape.records = records;
    shape.format2_revised = "other";
    CHECK(!sce_saved_check_shape(&session, &shape, record_at, enum_at, &err) &&
              strstr(err.message, "the saved state is of a document of shape cb77") == err.message,
          "%s", err.message);
    // A state that names a revision has the revised shape, not the first.
    shape.format2_revised = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5";
    shape.format2_first = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5";
    CHECK(!sce_saved_check_shape(&session, &shape, record_at, enum_at, &err) &&
              strcmp(err.message, "the saved state names revisions of its record schemas and has the shape of a "
                                  "state that names none") == 0,
          "%s", err.message);
}

static void a_value_of_a_variant_a_later_revision_added_is_refused(void) {
    static const sce_saved_added_variant_t added[] = {{"high", 2u}};
    static const sce_saved_enum_holder_t holders[] = {{"v", NULL, false}, {"r", "level", false}, {"l", "level", true}};
    sce_saved_lineage_t enums[1];
    sce_saved_shape_t shape;
    sce_saved_session_t session;
    sce_saved_error_t err;
    unsigned enum_at[1];
    char text[1024];
    static const char head[] =
        "{\"format\":3,\"shape\":\"S\",\"sessionid\":\"x\",\"configuration\":[],\"current\":\"a\","
        "\"variables\":";
    static const char tail[] = ",\"history\":{},\"pending\":[],\"invokes\":[],\"invokestarts\":{},\"hostinvokes\":[],"
                               "\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]}";
    memset(enums, 0, sizeof(enums));
    enums[0].key = "enum:level";
    enums[0].revision = 2u;
    enums[0].added = added;
    enums[0].added_count = 1u;
    enums[0].holders = holders;
    enums[0].holder_count = 3u;
    memset(&shape, 0, sizeof(shape));
    shape.first = shape.format2_first = "S";
    shape.enums = enums;
    shape.enum_count = 1u;

    (void)snprintf(text, sizeof(text), "%s{\"v\":\"high\",\"r\":{},\"l\":[]}%s", head, tail);
    CHECK(sce_saved_read(text, strlen(text), &session, &err), "%s", err.message);
    CHECK(!sce_saved_check_shape(&session, &shape, NULL, enum_at, &err) &&
              strcmp(err.message,
                     "'v' holds 'high', which revision 1 of 'enum:level' does not declare: revision 2 added it") == 0,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s{\"v\":\"low\",\"r\":{\"level\":\"high\"},\"l\":[]}%s", head, tail);
    CHECK(sce_saved_read(text, strlen(text), &session, &err), "%s", err.message);
    CHECK(!sce_saved_check_shape(&session, &shape, NULL, enum_at, &err) &&
              strstr(err.message, "'r.level' holds 'high'") == err.message,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s{\"v\":\"low\",\"r\":{},\"l\":[{\"level\":\"low\"},{\"level\":\"high\"}]}%s",
                   head, tail);
    CHECK(sce_saved_read(text, strlen(text), &session, &err), "%s", err.message);
    CHECK(!sce_saved_check_shape(&session, &shape, NULL, enum_at, &err) &&
              strstr(err.message, "'l[1].level' holds 'high'") == err.message,
          "%s", err.message);
    (void)snprintf(text, sizeof(text), "%s{\"v\":\"low\",\"r\":{},\"l\":[]}%s", head, tail);
    CHECK(sce_saved_read(text, strlen(text), &session, &err), "%s", err.message);
    CHECK(sce_saved_check_shape(&session, &shape, NULL, enum_at, &err), "%s", err.message);
}

// ── The configuration a state was taken in (docs/adr/0057, section 7) ──────────────────────────

// A format-1 state with every member in place, taken with the features `features` (the text of a
// JSON value), or recording none when it is NULL, read into `out`: false, with the reason in `err`,
// when it is refused as it is read. `text` holds the state, which `out` reads in place.
static bool taken(const char *features, char *text, size_t cap, sce_saved_session_t *out, sce_saved_error_t *err) {
    static const char head[] =
        "{\"format\":1,\"shape\":\"s\",\"configuration\":[\"s\"],\"current\":\"s\",\"variables\":{},\"history\":{},"
        "\"pending\":[],\"invokes\":[],\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]";
    if (features == NULL) {
        (void)snprintf(text, cap, "%s}", head);
    } else {
        (void)snprintf(text, cap, "%s,\"features\":%s}", head, features);
    }
    return sce_saved_read(text, strlen(text), out, err);
}

// `features` as the member a session writes, alone in an object, into `out`.
static void written(const char *const *features, char *out, size_t cap) {
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    sce_jw_begin_object(&w);
    sce_saved_write_features(&w, features);
    sce_jw_end_object(&w);
    CHECK(sce_jw_finish(&w), "the member fits");
}

static void a_configuration_is_read_back_as_it_was_written(void) {
    static const char *const built[] = {"session-unicast", "tls", NULL};
    static const char *const none_on[] = {NULL};
    static const char lead[] = "{\"features\":";
    char member[256];
    char array[256];
    char text[1024];
    size_t len;
    sce_saved_session_t session;
    sce_saved_error_t err;
    sce_jiter_t it;
    sce_jv_t item;
    memset(&err, 0, sizeof(err));

    written(built, member, sizeof(member));
    CHECK(strcmp(member, "{\"features\":[\"session-unicast\",\"tls\"]}") == 0, "%s", member);
    // The array the member holds, read back as a state's.
    len = strlen(member);
    (void)snprintf(array, sizeof(array), "%.*s", (int)(len - strlen(lead) - 1u), member + strlen(lead));
    CHECK(taken(array, text, sizeof(text), &session, &err), "%s", err.message);
    CHECK(session.has_features && sce_jv_count(session.features) == 2u, "two features");
    CHECK(sce_jv_iter(session.features, &it) && sce_jiter_element(&it, &item) &&
              sce_jv_string_is(item, "session-unicast") && sce_jiter_element(&it, &item) &&
              sce_jv_string_is(item, "tls"),
          "the features, in the order they were written");
    CHECK(sce_saved_check_features(&session, built, &err), "%s", err.message);

    // The configuration that turns none on is a configuration: an empty array, not no member.
    written(none_on, member, sizeof(member));
    CHECK(strcmp(member, "{\"features\":[]}") == 0, "%s", member);
    CHECK(taken("[]", text, sizeof(text), &session, &err), "%s", err.message);
    CHECK(session.has_features && sce_jv_count(session.features) == 0u, "the empty configuration");

    // A machine of no product line records no configuration, and its text is the one it was
    // before configurations were recorded.
    written(NULL, member, sizeof(member));
    CHECK(strcmp(member, "{}") == 0, "%s", member);
    CHECK(taken(NULL, text, sizeof(text), &session, &err), "%s", err.message);
    CHECK(!session.has_features && strstr(text, "features") == NULL, "%s", text);
}

// A restore judges the configuration a state was taken in before its shape, and names the features
// that differ.
static void a_state_of_another_configuration_is_refused_by_the_features_that_differ(void) {
    static const char *const none_on[] = {NULL};
    static const char *const a[] = {"a", NULL};
    static const char *const a_b[] = {"a", "b", NULL};
    static const char *const a_c[] = {"a", "c", NULL};
    char text[1024];
    sce_saved_session_t session;
    sce_saved_error_t err;
    sce_saved_shape_t shape;
    unsigned record_at[1];
    unsigned enum_at[1];
    memset(&err, 0, sizeof(err));

    CHECK(taken(NULL, text, sizeof(text), &session, &err) && sce_saved_check_features(&session, NULL, &err), "%s",
          err.message);
    CHECK(taken("[]", text, sizeof(text), &session, &err) && sce_saved_check_features(&session, none_on, &err), "%s",
          err.message);
    CHECK(taken("[\"a\",\"b\"]", text, sizeof(text), &session, &err) && sce_saved_check_features(&session, a_b, &err),
          "%s", err.message);

    CHECK(taken("[\"a\",\"b\"]", text, sizeof(text), &session, &err) &&
              !sce_saved_check_features(&session, a_c, &err) &&
              strcmp(err.message, "the saved state was taken with the features [a, b] on, and this machine is built "
                                  "with [a, c]: [b] on only where it was saved, [c] on only in this build") == 0,
          "%s", err.message);
    CHECK(taken("[\"a\"]", text, sizeof(text), &session, &err) && !sce_saved_check_features(&session, none_on, &err) &&
              strcmp(err.message, "the saved state was taken with the features [a] on, and this machine is built "
                                  "with []: [a] on only where it was saved") == 0,
          "%s", err.message);
    CHECK(taken("[\"a\"]", text, sizeof(text), &session, &err) && !sce_saved_check_features(&session, a_c, &err) &&
              strcmp(err.message, "the saved state was taken with the features [a] on, and this machine is built "
                                  "with [a, c]: [c] on only in this build") == 0,
          "%s", err.message);
    CHECK(taken(NULL, text, sizeof(text), &session, &err) && !sce_saved_check_features(&session, a, &err) &&
              strcmp(err.message, "the saved state records no features, and this machine is built with the "
                                  "features [a] on") == 0,
          "%s", err.message);
    CHECK(taken("[]", text, sizeof(text), &session, &err) && !sce_saved_check_features(&session, NULL, &err) &&
              strcmp(err.message, "the saved state was taken with the features [] on, and this machine is built "
                                  "from a document of no feature model") == 0,
          "%s", err.message);

    // Judged before the shape: a state of another build is refused for its features even where its
    // shape differs too.
    memset(&shape, 0, sizeof(shape));
    shape.first = shape.revised = "other";
    shape.format2_first = shape.format2_revised = "other";
    shape.features = a;
    CHECK(taken("[\"b\"]", text, sizeof(text), &session, &err) &&
              !sce_saved_check_shape(&session, &shape, record_at, enum_at, &err) &&
              strstr(err.message, "the saved state was taken with the features [b] on") == err.message,
          "%s", err.message);
    // And a state of this build is judged by its shape after.
    CHECK(taken("[\"a\"]", text, sizeof(text), &session, &err) &&
              !sce_saved_check_shape(&session, &shape, record_at, enum_at, &err) &&
              strstr(err.message, "the saved state is of a document of shape s") == err.message,
          "%s", err.message);
}

// `features` is a set, written sorted by the bytes of each name: a text that repeats a feature or
// names them out of order is refused as it is read.
static void the_features_of_a_saved_state_are_a_sorted_set(void) {
    static const char *const unordered[] = {"[\"b\",\"a\"]", "[\"a\",\"a\"]", "[\"ab\",\"a\"]", "[\"\\u00e9\",\"z\"]"};
    char text[1024];
    sce_saved_session_t session;
    sce_saved_error_t err;
    size_t i;
    memset(&err, 0, sizeof(err));

    CHECK(taken("[\"a\",\"b\"]", text, sizeof(text), &session, &err) && sce_jv_count(session.features) == 2u, "%s",
          err.message);
    // By bytes: an upper-case letter before a lower-case one, a name before a longer one it begins,
    // and a byte past ASCII after every ASCII one.
    CHECK(taken("[\"B\",\"a\",\"ab\",\"\\u00e9\"]", text, sizeof(text), &session, &err), "%s", err.message);
    for (i = 0u; i < sizeof(unordered) / sizeof(unordered[0]); ++i) {
        CHECK(!taken(unordered[i], text, sizeof(text), &session, &err) &&
                  strcmp(err.message, "'features' is not in order or names a feature twice; it is a set, sorted") == 0,
              "%s: %s", unordered[i], err.message);
    }
    CHECK(!taken("\"a\"", text, sizeof(text), &session, &err) && strcmp(err.message, "'features' is not an array") == 0,
          "%s", err.message);
    // Every element is a text before any two are compared.
    CHECK(!taken("[\"b\",\"a\",1]", text, sizeof(text), &session, &err) &&
              strcmp(err.message, "'features' is not a text") == 0,
          "%s", err.message);
}

static void a_wall_clock_moment_converts_to_the_engines_clock_and_back(void) {
    CHECK(sce_saved_due_on(1000, 50u, 80u) == 1030, "30 ms from now");
    CHECK(sce_saved_due_on(1000, 50u, 20u) == 1000, "none earlier than now, for one already past");
    CHECK(sce_saved_due_on(INT64_MAX - 5, 0u, 100u) == INT64_MAX, "held to what a signed count says");
    CHECK(sce_saved_ready_at(1000, 50u, 1030) == 80u, "30 ms after the engine's now");
    CHECK(sce_saved_ready_at(1000, 50u, 900) == 50u, "one that came due while the machine was away is due now");
    CHECK(sce_saved_ready_at(0, UINT64_MAX - 1u, 100) == UINT64_MAX, "held to what an unsigned count says");
}

int main(void) {
    every_shared_text_reads_as_a_session_of_its_format();
    a_text_that_is_not_a_session_is_refused_in_the_words_every_backend_uses();
    a_member_is_judged_for_what_it_is();
    a_tree_is_judged_to_every_depth();
    a_waiting_send_a_queued_event_and_a_host_invocation_read_back_as_they_were();
    a_format_three_origin_is_read_as_it_is_written();
    a_format_three_origin_that_is_not_whole_is_refused();
    a_format_three_act_holds_what_its_event_will_carry();
    a_count_a_token_and_a_moment_are_whole_numbers_within_what_they_can_hold();
    a_machine_writes_the_members_the_other_backends_write();
    a_state_is_bound_to_its_shape_and_to_the_revision_each_schema_was_saved_at();
    a_value_of_a_variant_a_later_revision_added_is_refused();
    a_configuration_is_read_back_as_it_was_written();
    a_state_of_another_configuration_is_refused_by_the_features_that_differ();
    the_features_of_a_saved_state_are_a_sorted_set();
    a_wall_clock_moment_converts_to_the_engines_clock_and_back();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("saved state: ok");
    return 0;
}
