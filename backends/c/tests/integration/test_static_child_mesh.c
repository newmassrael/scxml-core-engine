// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-21.3 mesh-21.4 mesh-21.5
//
// SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043) under generated C: a child
// session that talks to a Mesh peer is given a router of its own by the machine
// that starts it, asked of the factory kept in the registry the machine is started
// with, and never the machine's own router.
//
// `static_child_mesh` starts `list`, which greets `#server` as it is entered and
// ends when the greeting is refused; the machine counts that end in `refusals`.
// Its hybrid `peer` starts `static_mesh_relay`, whose child `inner` answers
// `inner.ping`. The Rust half is
// `a_child_session_is_given_its_mesh_router_by_its_parent.rs`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_child_mesh_sm.h"

#define LOG_MAX 8
#define ROUTERS_MAX 8

/* What the routers were told, oldest first, each as one line of text. */
static struct {
    char lines[LOG_MAX][64];
    int count;
} told;

/* What the machine's own router was handed: no child's send may reach it. */
static int parent_sends;

static void tell(const char *line) {
    if (told.count < LOG_MAX) {
        (void)snprintf(told.lines[told.count], sizeof(told.lines[0]), "%s", line);
    }
    told.count++;
}

static void forget(void) {
    memset(&told, 0, sizeof(told));
}

typedef struct {
    char name[48];
    sce_mesh_call_t inbox[2];
    int queued;
} recorder_t;

static recorder_t recorders[ROUTERS_MAX];
static int recorders_open;

static void parent_router(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    (void)user_data;
    (void)request;
    (void)out;
    parent_sends++;
}

static void recorder_send(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    const recorder_t *r = (const recorder_t *)user_data;
    char line[64];
    (void)out;
    (void)snprintf(line, sizeof(line), "%.20s sent %.32s", r->name, request->event_name);
    tell(line);
}

static bool recorder_take(void *user_data, sce_mesh_call_t *out) {
    recorder_t *r = (recorder_t *)user_data;
    if (r->queued == 0) {
        return false;
    }
    *out = r->inbox[0];
    r->inbox[0] = r->inbox[1];
    r->queued--;
    return true;
}

static void recorder_detach(void *user_data) {
    const recorder_t *r = (const recorder_t *)user_data;
    char line[64];
    (void)snprintf(line, sizeof(line), "%.40s detached", r->name);
    tell(line);
}

static bool recorder_open(const char *name, sce_child_mesh_router_t *out);

static bool recorder_child(void *user_data, const char *invoke_id, const char *document, sce_child_mesh_router_t *out) {
    const recorder_t *r = (const recorder_t *)user_data;
    char line[64];
    char name[48];
    (void)snprintf(line, sizeof(line), "%.12s asked for %.12s of '%.20s'", r->name, invoke_id, document);
    tell(line);
    (void)snprintf(name, sizeof(name), "%.20s/%.20s", r->name, invoke_id);
    return recorder_open(name, out);
}

static bool recorder_open(const char *name, sce_child_mesh_router_t *out) {
    if (recorders_open >= ROUTERS_MAX) {
        return false;
    }
    recorder_t *r = &recorders[recorders_open++];
    memset(r, 0, sizeof(*r));
    (void)snprintf(r->name, sizeof(r->name), "%s", name);
    out->user_data = r;
    out->send = recorder_send;
    out->invoke = NULL;
    out->take_call = recorder_take;
    out->detach = recorder_detach;
    out->child_router = recorder_child;
    return true;
}

static bool factory(void *user_data, const char *invoke_id, const char *document, sce_child_mesh_router_t *out) {
    char line[64];
    (void)user_data;
    (void)snprintf(line, sizeof(line), "asked for %.20s of '%.20s'", invoke_id, document);
    tell(line);
    return recorder_open(invoke_id, out);
}

static bool refusing_factory(void *user_data, const char *invoke_id, const char *document,
                             sce_child_mesh_router_t *out) {
    (void)user_data;
    (void)invoke_id;
    (void)document;
    (void)out;
    return false;
}

static void settle(static_child_mesh_t *sm) {
    for (int round = 0; round < 5; ++round) {
        static_child_mesh_step(sm);
    }
}

static void start(static_child_mesh_t *sm, sce_child_mesh_routers_fn ask) {
    sce_host_processor_registry_t wiring;
    memset(&wiring, 0, sizeof(wiring));
    forget();
    parent_sends = 0;
    recorders_open = 0;
    (void)sce_host_registry_register_mesh_router(&wiring, parent_router, NULL);
    if (ask != NULL) {
        (void)sce_host_registry_register_child_mesh_routers(&wiring, ask, NULL);
    }
    static_child_mesh_init_with_host_processors(sm, &wiring);
    settle(sm);
}

static void send_named(static_child_mesh_t *sm, const char *name) {
    static_child_mesh_event_t event;
    if (!static_child_mesh_resolve_event_by_name(name, &event)) {
        return;
    }
    static_child_mesh_event_with_meta_t meta;
    memset(&meta, 0, sizeof(meta));
    meta.event = event;
    static_child_mesh_raise_external(sm, &meta);
    settle(sm);
}

/* Whether the routers were told exactly `want`, in order; forgets what they were. */
static int told_exactly(const char *const *want, int count, const char *what) {
    int bad = told.count != count;
    for (int i = 0; !bad && i < count; ++i) {
        bad = strcmp(told.lines[i], want[i]) != 0;
    }
    if (bad) {
        (void)fprintf(stderr, "static_child_mesh: FAIL - %s: the routers were told %d line(s):\n", what, told.count);
        for (int i = 0; i < told.count && i < LOG_MAX; ++i) {
            (void)fprintf(stderr, "  %s\n", told.lines[i]);
        }
    }
    forget();
    return bad;
}

static int expect(bool holds, const char *what) {
    if (!holds) {
        (void)fprintf(stderr, "static_child_mesh: FAIL - %s\n", what);
        return 1;
    }
    return 0;
}

int main(void) {
    int bad = 0;
    static_child_mesh_t sm;

    /* A child given no router has its greeting refused and ends; the machine's own
       router is not asked. */
    start(&sm, NULL);
    bad |= expect(static_child_mesh_get_refusals(&sm) == 1u, "a child given no router ended on its refused send");
    bad |= expect(parent_sends == 0, "a child's send fell back to the machine's router");
    static_child_mesh_destroy(&sm);

    /* A factory that answers none leaves the child as one given no router. */
    start(&sm, refusing_factory);
    bad |= expect(static_child_mesh_get_refusals(&sm) == 1u, "a factory that answers none gave the child a router");
    bad |= expect(parent_sends == 0, "a child's send fell back to the machine's router");
    static_child_mesh_destroy(&sm);

    /* Each start asks the factory before the child is entered, and each end
       detaches what it answered. */
    start(&sm, factory);
    static const char *const started[] = {"asked for list of ''", "list sent list.hello"};
    bad |= told_exactly(started, 2, "started");
    bad |= expect(static_child_mesh_get_refusals(&sm) == 0u, "a child given a router had its send refused");

    sce_mesh_call_t *ping = &recorders[0].inbox[recorders[0].queued++];
    memset(ping, 0, sizeof(*ping));
    ping->kind = SCE_MESH_CALL_RAISE;
    (void)snprintf(ping->name, sizeof(ping->name), "%s", "list.ping");
    (void)snprintf(ping->source, sizeof(ping->source), "%s", "server");
    settle(&sm);
    static const char *const pinged[] = {"list sent list.pong"};
    bad |= told_exactly(pinged, 1, "a peer's event reaches the child when the machine drives it");

    send_named(&sm, "open");
    static const char *const opened[] = {"asked for peer of 'static_mesh_relay'", "peer asked for inner of ''"};
    bad |= told_exactly(opened, 2, "the candidate is asked for by its stem, and its child of its router");

    send_named(&sm, "again");
    static const char *const ended[] = {"peer/inner detached", "peer detached", "list detached"};
    bad |= told_exactly(ended, 3, "every session ends detached once, a grandchild before its parent");

    send_named(&sm, "back");
    static const char *const restarted[] = {"asked for list of ''", "list sent list.hello"};
    bad |= told_exactly(restarted, 2, "entered anew, the invoke asks again");
    bad |= expect(parent_sends == 0, "a child's send fell back to the machine's router");
    static_child_mesh_destroy(&sm);

    if (bad == 0) {
        (void)printf("static_child_mesh: PASS\n");
    }
    return bad;
}
