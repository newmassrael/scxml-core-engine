#!/usr/bin/env node

/**
 * @file standalone_http_server.js
 * @brief Standalone HTTP server for W3C SCXML C.2 BasicHTTP Event I/O Processor tests
 *
 * W3C SCXML Compliance:
 * - C.2: BasicHTTP Event I/O Processor specification
 * - Handles form-encoded, JSON, and plain text content
 * - Returns W3C-compliant HTTP responses
 *
 * Usage:
 *   node standalone_http_server.js [port] [path]
 *
 * Both arguments are optional; omitting one takes the endpoint the harness
 * owns, from `basic_http_test_endpoint.h` via `http_endpoint.js`. The default
 * is NOT written here: this server binds the address the compiled runners
 * publish as their 'location', and a second author of that fact would let the
 * two drift — the machines would post somewhere this never claimed.
 *
 * Port 0 asks the kernel for a free port. The line `[READY] port=<n>` names
 * the port bound, and a caller that asked for 0 points the runners at it
 * through SCE_W3C_HTTP_PORT.
 *
 * `[READY]` is printed only once EVERY address `localhost` resolves to is
 * held by this process, and it is the only proof a caller has that the
 * listener is this server. A connect probe cannot tell: on a build machine
 * shared with other work, another program held 127.0.0.1:8080, this server
 * died of EADDRINUSE, and the probe connected to the other program and called
 * the fixture up (2026-10-20). Thirteen HTTP tests then failed against a
 * server that was not this one. Holding every address matters for the same
 * reason: a runner dials `localhost` and takes whichever address its resolver
 * gives first, so one this server left free is one another program can hold.
 *
 * This server runs as a separate Node.js process to avoid event loop conflicts
 * with WASM test execution. The w3c_test_wrapper.js script manages its lifecycle.
 */

const dns = require('dns');
const http = require('http');
const querystring = require('querystring');
const endpoint = require('./http_endpoint');

// Parse command-line arguments, falling back to the owned endpoint.
const port = process.argv[2] ? parseInt(process.argv[2], 10) : endpoint.endpointPort();
const path = process.argv[3] || endpoint.endpointPath();

// The request handler, shared by the listener on each address.
const handleRequest = (req, res) => {
    // W3C SCXML C.2: Only accept POST requests to configured path
    if (req.method !== 'POST' || req.url !== path) {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
            status: 'error',
            message: `Not found: ${req.method} ${req.url}`
        }));
        return;
    }

    let body = '';
    req.on('data', chunk => {
        body += chunk.toString();
    });

    req.on('end', () => {
        try {
            // W3C SCXML C.2: Parse HTTP request per BasicHTTP Event I/O Processor spec
            const contentType = req.headers['content-type'] || '';
            let eventName = 'event1';  // W3C default event name
            let eventData = '';

            if (contentType.includes('application/x-www-form-urlencoded')) {
                // W3C SCXML C.2: Form-encoded parameters
                const params = querystring.parse(body);

                // W3C SCXML test 518, 531: _scxmleventname has priority
                if (params._scxmleventname) {
                    eventName = params._scxmleventname;
                }

                // W3C SCXML test 518, 519: Map params to _event.data JSON
                const dataObj = {};
                for (const [key, value] of Object.entries(params)) {
                    // Parse numeric values (test 519: param1 should be number)
                    const numValue = Number(value);
                    dataObj[key] = (!isNaN(numValue) && value !== '') ? numValue : value;
                }
                eventData = JSON.stringify(dataObj);

            } else if (body.startsWith('{') || body.startsWith('[')) {
                // W3C SCXML C.2: JSON content
                try {
                    const json = JSON.parse(body);
                    if (json.event) eventName = json.event;
                    eventData = body;
                } catch (e) {
                    eventData = body;
                }
            } else if (body.length > 0) {
                // W3C SCXML test 520: Plain content → HTTP.POST event
                eventName = 'HTTP.POST';
                eventData = body;
            }

            // Log received event (for debugging)
            console.log(`[INFO] HTTP server: Received event "${eventName}" with data: ${eventData}`);

            // W3C SCXML C.2: Parse event data based on content type
            let responseData = {};
            if (eventData) {
                // W3C SCXML test 518, 519: JSON data from form-encoded params
                if (eventData.startsWith('{') || eventData.startsWith('[')) {
                    try {
                        responseData = JSON.parse(eventData);
                    } catch (e) {
                        // W3C SCXML test 520: Plain text content (not JSON)
                        responseData = eventData;
                    }
                } else {
                    // W3C SCXML test 520: Plain text content
                    responseData = eventData;
                }
            }

            // W3C SCXML C.2: Send compliant HTTP response with event data
            const response = {
                status: 'success',
                event: eventName,
                data: responseData,
                sendId: 'standalone_http_' + Date.now(),
                timestamp: Date.now()
            };

            res.writeHead(200, {
                'Content-Type': 'application/json',
                'Cache-Control': 'no-cache',
                'Access-Control-Allow-Origin': '*'
            });
            res.end(JSON.stringify(response));

        } catch (e) {
            console.error('[ERROR] HTTP server: Exception handling request:', e.message);
            res.writeHead(500, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({
                status: 'error',
                message: e.message
            }));
        }
    });
};

// How many free ports to try when the kernel's first choice is free on one
// address of `localhost` and held on another. Only port 0 retries: a port the
// caller named is the one the runners will dial, so it is held or refused.
const EPHEMERAL_ATTEMPTS = 20;

// An address no socket of this host can bind (`::1` named by /etc/hosts on a
// host without IPv6) is one no other program can hold and no runner can
// reach, so it is left out rather than refused.
const UNBINDABLE = new Set(['EADDRNOTAVAIL', 'EAFNOSUPPORT']);

let servers = [];

function fail(message) {
    console.error(`[ERROR] HTTP server failed to start: ${message}`);
    process.exit(1);
}

function closeAll() {
    for (const s of servers) {
        s.close();
    }
    servers = [];
}

// Listen on each of `addresses` at one port: the first at `wanted` (0 lets the
// kernel choose), the rest at the port the first was given. `done(err, port)`.
function listenOnEach(addresses, wanted, done) {
    const next = (i, at) => {
        if (i === addresses.length) {
            done(null, at);
            return;
        }
        const s = http.createServer(handleRequest);
        s.once('error', (err) => {
            if (UNBINDABLE.has(err.code)) {
                next(i + 1, at);
                return;
            }
            err.address = addresses[i].address;
            err.wantedPort = at;
            done(err);
        });
        s.listen(at, addresses[i].address, () => {
            servers.push(s);
            next(i + 1, s.address().port);
        });
    };
    next(0, wanted);
}

function start(addresses, attempt) {
    listenOnEach(addresses, port, (err, bound) => {
        if (err) {
            closeAll();
            if (err.code === 'EADDRINUSE' && port === 0 && attempt < EPHEMERAL_ATTEMPTS) {
                start(addresses, attempt + 1);
                return;
            }
            const held = err.code === 'EADDRINUSE'
                ? ' — another process holds it, and a runner dialing localhost could reach that process instead'
                : '';
            fail(`${err.address}:${err.wantedPort} (${err.code || err.message})${held}`);
            return;
        }
        if (servers.length === 0) {
            fail(`no address of localhost (${addresses.map((a) => a.address).join(', ')}) can be bound on this host`);
        }
        const where = servers.map((s) => s.address().address).join(', ');
        console.log(`[INFO] Standalone HTTP server started on localhost:${bound}${path} (${where})`);
        console.log(`[INFO] W3C SCXML C.2 BasicHTTP Event I/O Processor ready`);
        console.log(`[READY] port=${bound}`);  // Signal for wrapper script
    });
}

if (!Number.isInteger(port) || port < 0 || port > 65535) {
    fail(`"${process.argv[2]}" is not a TCP port (0 asks the kernel for a free one)`);
}

// The runners dial the NAME `localhost`, so what it resolves to here is what
// must be held — resolved through the same resolver Node's own clients use.
dns.lookup('localhost', {all: true}, (err, addresses) => {
    if (err) {
        fail(`localhost does not resolve (${err.message})`);
    }
    start(addresses, 1);
});

// Graceful shutdown
function shutdown(signal) {
    console.log(`[INFO] Received ${signal}, shutting down...`);
    let open = servers.length;
    if (open === 0) {
        process.exit(0);
    }
    for (const s of servers) {
        s.close(() => {
            open -= 1;
            if (open === 0) {
                console.log('[INFO] HTTP server stopped');
                process.exit(0);
            }
        });
    }
}

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));
