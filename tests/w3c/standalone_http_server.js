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
 * Example:
 *   node standalone_http_server.js 8080 /test
 *
 * This server runs as a separate Node.js process to avoid event loop conflicts
 * with WASM test execution. The w3c_test_wrapper.js script manages its lifecycle.
 */

const http = require('http');
const querystring = require('querystring');

// Parse command-line arguments
const port = parseInt(process.argv[2]) || 8080;
const path = process.argv[3] || '/test';

// Create HTTP server
const server = http.createServer((req, res) => {
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
});

// Start server
server.listen(port, 'localhost', () => {
    console.log(`[INFO] Standalone HTTP server started on localhost:${port}${path}`);
    console.log(`[INFO] W3C SCXML C.2 BasicHTTP Event I/O Processor ready`);
    console.log('[READY]');  // Signal for wrapper script
});

server.on('error', (err) => {
    console.error('[ERROR] HTTP server failed to start:', err.message);
    process.exit(1);
});

// Graceful shutdown
process.on('SIGTERM', () => {
    console.log('[INFO] Received SIGTERM, shutting down...');
    server.close(() => {
        console.log('[INFO] HTTP server stopped');
        process.exit(0);
    });
});

process.on('SIGINT', () => {
    console.log('[INFO] Received SIGINT, shutting down...');
    server.close(() => {
        console.log('[INFO] HTTP server stopped');
        process.exit(0);
    });
});
