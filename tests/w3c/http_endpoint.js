// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @file http_endpoint.js
 * @brief JavaScript's reader of the W3C BasicHTTP fixture endpoint.
 *
 * W3C SCXML C.2.3: the endpoint is owned by `basic_http_test_endpoint.h`, a C
 * header because the C11 AOT runners must include it. This module does not
 * restate the port; it READS it from that header, so a Node fixture server and
 * a compiled runner cannot come to disagree about where the listener answers.
 *
 * `SCE_W3C_HTTP_PORT` in the environment wins, which is how a second checkout
 * is given a port of its own — the listener is machine-global and only one
 * process per host can hold it.
 *
 * Throws rather than guessing. A server that quietly bound the default after
 * being told otherwise would take the port another tree is using, and the
 * collision would surface as a test failure in whichever tree lost it.
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
    const portMatch = text.match(/^#define\s+SCE_W3C_HTTP_DEFAULT_PORT\s+(\d+)/m);
    const pathMatch = text.match(/^#define\s+SCE_W3C_HTTP_TEST_PATH\s+"([^"]+)"/m);
    if (!portMatch || !pathMatch) {
        throw new Error(
            `${HEADER} no longer declares SCE_W3C_HTTP_DEFAULT_PORT and ` +
            'SCE_W3C_HTTP_TEST_PATH — the endpoint owner moved or was renamed');
    }
    return {port: parseInt(portMatch[1], 10), path: pathMatch[1]};
}

/** The port the fixture listener binds and the runners address. */
function endpointPort() {
    const raw = process.env.SCE_W3C_HTTP_PORT;
    if (raw !== undefined && raw !== '') {
        const value = Number(raw);
        if (!Number.isInteger(value) || value < 1 || value > 65535) {
            throw new Error(`SCE_W3C_HTTP_PORT="${raw}" is not a TCP port`);
        }
        return value;
    }
    return readHeader().port;
}

/** The path the fixture listener answers on. */
function endpointPath() {
    return readHeader().path;
}

module.exports = {endpointPort, endpointPath};
