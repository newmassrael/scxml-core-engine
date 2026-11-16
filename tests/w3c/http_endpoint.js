// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @file http_endpoint.js
 * @brief JavaScript's reader of the W3C BasicHTTP fixture endpoint.
 *
 * W3C SCXML C.2.3: the endpoint is owned by `basic_http_test_endpoint.h`, a C
 * header because the C11 AOT runners must include it. This module does not
 * restate the path; it READS it from that header, so a Node fixture server and
 * a compiled runner cannot come to disagree about where the listener answers.
 *
 * The port has no default. A listener this module starts binds the port
 * `SCE_W3C_HTTP_PORT` names, or 0 for one the kernel assigns, and names the
 * port it bound on its `[READY] port=<n>` line; whoever started it hands that
 * port to the processes that post to it (the header says how).
 *
 * Throws rather than guessing. A server that quietly bound another port after
 * being told one would take the port another tree is using, and the collision
 * would surface as a test failure in whichever tree lost it.
 */

const fs = require('fs');
const path = require('path');

const HEADER = path.join(__dirname, 'basic_http_test_endpoint.h');

function readHeader() {
    let text;
    try {
        text = fs.readFileSync(HEADER, 'utf8');
    } catch (err) {
        throw new Error(
            `the BasicHTTP fixture endpoint header is unreadable: ${HEADER} (${err.message})`);
    }
    const pathMatch = text.match(/^#define\s+SCE_W3C_HTTP_TEST_PATH\s+"([^"]+)"/m);
    if (!pathMatch) {
        throw new Error(
            `${HEADER} no longer declares SCE_W3C_HTTP_TEST_PATH — the endpoint ` +
            'owner moved or was renamed');
    }
    return {path: pathMatch[1]};
}

/**
 * The port a fixture listener is told to bind: the one `SCE_W3C_HTTP_PORT`
 * names, or 0 for one the kernel assigns.
 */
function endpointPort() {
    const raw = process.env.SCE_W3C_HTTP_PORT;
    if (raw !== undefined && raw !== '') {
        const value = Number(raw);
        if (!Number.isInteger(value) || value < 1 || value > 65535) {
            throw new Error(`SCE_W3C_HTTP_PORT="${raw}" is not a TCP port`);
        }
        return value;
    }
    return 0;
}

/** The path the fixture listener answers on. */
function endpointPath() {
    return readHeader().path;
}

/**
 * The port a listener's `[READY] port=<n>` line names, or null when `line` is
 * not that line. The one reading of the line for every Node caller.
 */
function readyPort(line) {
    const match = /^\[READY\] port=(\d{1,5})$/.exec(line.trim());
    return match ? parseInt(match[1], 10) : null;
}

module.exports = {endpointPort, endpointPath, readyPort};
