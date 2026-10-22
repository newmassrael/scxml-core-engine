// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The members of a saved session on the C11 backend (docs/adr/0045): `sce/saved_state.h`.
// The C half of the session cases of `tests/saved/SavedStateTest.cpp`, held to the same words:
//
//   * every shared saved text reads as a session of its format, a tree to every depth;
//   * a waiting send, a queued event and a host-run invocation read back as they were
//     written, and a text of another kind is refused in the words every backend uses;
//   * a machine writes the same members the other backends write, byte for byte;
//   * a state is bound to the shape of its document and to the revision each schema was
//     saved at, and a wall-clock moment converts to the engine's clock and back.
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
    (void)snprintf(format1, sizeof(format1), "%s/format1", SCE_SAVED_FIXTURES_DIR);
    CHECK(check_directory(SCE_SAVED_FIXTURES_DIR, SCE_SAVED_FORMAT_2) >= 10, "the format-2 texts");
    CHECK(check_directory(format1, SCE_SAVED_FORMAT_1) >= 10, "the format-1 texts");
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
    CHECK(refused("{\"format\":3}", &err) &&
              strcmp(err.message, "the saved state is format 3, and this runtime reads formats 1 and 2") == 0,
          "%s", err.message);
    CHECK(refused("{\"format\":\"2\"}", &err) &&
              strcmp(err.message, "the saved state is format \"2\", and this runtime reads formats 1 and 2") == 0,
          "%s", err.message);
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
    CHECK(sce_saved_child_read(item, 0u, &child, &err), "%s", err.message);
    CHECK(sce_jv_string_is(child.id, "worker") && child.start == 2 &&
              sce_jv_string_is(child.invoke_id, "working.2.worker") &&
              sce_jv_string_is(child.document, "static_invoke_tree__sce_synth_invoke__worker"),
          "the child's words");
    CHECK(sce_saved_session_read(child.session, SCE_SAVED_FORMAT_2, &session, &err), "%s", err.message);
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
    sce_saved_send_t send;
    (void)snprintf(path, sizeof(path), "%s/static_timers_midway.json", SCE_SAVED_FIXTURES_DIR);
    CHECK(read_trimmed(path, text, sizeof(text), &len), "the timers text");
    CHECK(sce_saved_read(text, len, &timers, &err), "%s", err.message);
    CHECK(sce_jv_count(timers.pending) == 3u, "three sends wait");
    CHECK(sce_jv_iter(timers.pending, &it) && sce_jiter_element(&it, &item), "the first");
    CHECK(sce_saved_send_read(item, "pending[0]", SCE_SAVED_FORMAT_2, &send, &err), "%s", err.message);
    CHECK(send.due == 1700000002000 && send.act == SCE_SAVED_ACT_RAISE, "its moment and act");
    CHECK(sce_jv_string(send.event, field, sizeof(field), NULL) && strcmp(field, "beat") == 0, "its event");
    CHECK(sce_jv_string(send.send_id, field, sizeof(field), NULL) && strcmp(field, "__send_0") == 0, "its id");
    CHECK(sce_jv_string(send.origin, field, sizeof(field), NULL) && strcmp(field, "fixture-root") == 0, "its origin");

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
        CHECK(sce_saved_event_read(item, "external[0]", &event, &err), "%s", err.message);
        CHECK(sce_jv_string_is(event.name, "answer") && sce_jv_string_is(event.invoke_id, "worker") &&
                  !event.has_host_invoke_token,
              "the event's words");
        CHECK(sce_jv_iter(root.invokes, &it) && sce_jiter_element(&it, &item), "a child runs");
        CHECK(sce_saved_child_read(item, 0u, &child, &err), "%s", err.message);
        CHECK(sce_saved_session_read(child.session, SCE_SAVED_FORMAT_2, &inner, &err), "%s", err.message);
        CHECK(inner.host_invoke_token == 1, "the child has handed out one token");
        CHECK(sce_jv_iter(inner.host_invokes, &it) && sce_jiter_element(&it, &item), "a host run waits");
        CHECK(sce_saved_host_invoke_read(item, "hostinvokes[0]", &host, &err), "%s", err.message);
        CHECK(sce_jv_string_is(host.type, "sce-host-clock-now") && sce_jv_string_is(host.id, "clock") && !host.has_due,
              "the invocation's words");
    }
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
    static const char names[3][8] = {"b", "a", "b"};
    static const char values[3][8] = {"1", "2", "3"};

    memset(&send, 0, sizeof(send));
    send.due = 1700000002000;
    send.act = SCE_SAVED_ACT_RAISE;
    send.event = "beat";
    send.data = "";
    send.send_id = "__send_0";
    send.origin = "fixture-root";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"raise\",\"event\":\"beat\",\"data\":\"\","
                          "\"sendid\":\"__send_0\",\"origin\":\"fixture-root\"}") == 0,
          "%s", buf);

    send.act = SCE_SAVED_ACT_CHILD;
    send.invoke_id = "worker";
    send.event = "nudge";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_send(&w, &send);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"due\":\"1700000002000\",\"act\":\"child\",\"invokeid\":\"worker\",\"event\":\"nudge\","
                          "\"data\":\"\",\"sendid\":\"__send_0\",\"origin\":\"fixture-root\"}") == 0,
          "%s", buf);

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

    memset(&event, 0, sizeof(event));
    event.name = "answer";
    event.data = "";
    event.type = "external";
    event.send_id = "";
    event.origin = "";
    event.origin_type = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";
    event.invoke_id = "worker";
    sce_jw_init(&w, buf, sizeof(buf));
    sce_saved_write_event(&w, &event);
    CHECK(sce_jw_finish(&w) &&
              strcmp(buf, "{\"name\":\"answer\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",\"origin\":\"\","
                          "\"origintype\":\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\","
                          "\"invokeid\":\"worker\",\"hostinvoketoken\":null}") == 0,
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
    // The shape the shared revision text names, as format 2 does when it names a revision.
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
        "{\"format\":2,\"shape\":\"S\",\"sessionid\":\"x\",\"configuration\":[],\"current\":\"a\","
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
    a_count_a_token_and_a_moment_are_whole_numbers_within_what_they_can_hold();
    a_machine_writes_the_members_the_other_backends_write();
    a_state_is_bound_to_its_shape_and_to_the_revision_each_schema_was_saved_at();
    a_value_of_a_variant_a_later_revision_added_is_refused();
    a_wall_clock_moment_converts_to_the_engines_clock_and_back();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("saved state: ok");
    return 0;
}
