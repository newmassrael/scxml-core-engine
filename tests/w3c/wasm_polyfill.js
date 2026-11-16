// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file wasm_polyfill.js
 * @brief The Node host of the WASM W3C runner, linked as a `--pre-js`.
 *
 * It gives the program what a Node host has to: the BasicHTTP fixture server a
 * run of `http` fixtures posts to (§scxml-C-2). The host's files, working
 * directory and environment are the program's already: every test program is
 * linked with NODERAWFS (apply_sce_test_policies in tests/CMakeLists.txt), so
 * nothing is mounted or copied here. The engine's own HTTP client needs nothing
 * from here either: under
 * Node it posts through Node's `http` module (EmscriptenFetchClient), and only
 * the browser's path goes through Emscripten's fetch and its XMLHttpRequest.
 *
 * It reads from w3c_registry_pre.js, the `--pre-js` linked before it, which
 * tests/CMakeLists.txt generates: the fixtures the registry runs under its
 * `http` harness, the largest test number, and the directory of the harness's
 * Node modules (this one).
 *
 * It runs on the main thread only. A pthread's worker loads this same program
 * and runs every `--pre-js` again; a host set up there would start a server
 * once per worker.
 *
 * `main()` does not start before the fixture server is ready: the server is a
 * run dependency of the program, so the runner never races its listener.
 */

// `ENVIRONMENT_IS_PTHREAD` is declared only in a program linked with pthreads.
const SCE_W3C_ON_A_PTHREAD = typeof ENVIRONMENT_IS_PTHREAD !== 'undefined' && ENVIRONMENT_IS_PTHREAD;

if (ENVIRONMENT_IS_NODE && !SCE_W3C_ON_A_PTHREAD) {
    const { spawn } = require('child_process');
    const path = require('path');
    // The harness's own modules are resolved from where they are kept, not
    // from beside the linked program.
    const harnessRequire = require('module').createRequire(path.join(SCE_W3C_HARNESS_DIR, 'wasm_polyfill.js'));

    // §scxml-C-2: the fixtures that need the server are the ones the registry
    // runs under its `http` harness; an open range ends at its largest number.
    const HTTP_TEST_IDS = new Set(SCE_W3C_HTTP_TEST_IDS);
    const testArgs = process.argv.slice(2).filter(arg => !arg.startsWith('--'));
    const needsHttpServer = testArgs.length === 0 ||
        testArgs.some(arg => {
            if (HTTP_TEST_IDS.has(arg)) return true;
            // Range format: "500~600", "~600", or "500~"
            if (arg.includes('~')) {
                const parts = arg.split('~');
                const start = parseInt(parts[0]) || 1;
                const end = parseInt(parts[1]) || SCE_W3C_LARGEST_TEST_NUMBER;
                return Array.from(HTTP_TEST_IDS).some(id => {
                    const num = parseInt(id);
                    return num >= start && num <= end;
                });
            }
            return false;
        });

    let httpServerProcess = null;

    // Start the fixture server and resolve with the port it reports on its
    // `[READY] port=<n>` line.
    function startHttpServer() {
        const serverScript = path.join(SCE_W3C_HARNESS_DIR, 'standalone_http_server.js');
        // §scxml-C-2-3: the path the harness owns, read from the header the
        // native runners include, and the port SCE_W3C_HTTP_PORT names or 0 for
        // one the kernel assigns: the endpoint has no default port.
        const endpoint = harnessRequire('./http_endpoint');
        const serverPort = endpoint.endpointPort();
        const serverPath = endpoint.endpointPath();

        httpServerProcess = spawn(process.execPath, [serverScript, serverPort.toString(), serverPath], {
            stdio: ['ignore', 'pipe', 'pipe'],
            detached: false
        });

        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                httpServerProcess.kill();
                reject(new Error('the fixture server did not report ready within 10000ms'));
            }, 10000);

            // Read line by line: a chunk may end inside the `[READY]` line.
            let pending = '';
            httpServerProcess.stdout.on('data', (data) => {
                const output = data.toString();
                process.stdout.write(output);
                pending += output;
                const lines = pending.split('\n');
                pending = lines.pop();
                for (const line of lines) {
                    const bound = endpoint.readyPort(line);
                    if (bound !== null) {
                        clearTimeout(timeout);
                        resolve(bound);
                        return;
                    }
                }
            });
            httpServerProcess.stderr.on('data', (data) => {
                process.stderr.write(`[Server Error] ${data.toString()}`);
            });
            httpServerProcess.on('error', (err) => {
                clearTimeout(timeout);
                reject(new Error(`could not spawn the fixture server: ${err.message}`));
            });
            httpServerProcess.on('exit', (code, signal) => {
                clearTimeout(timeout);
                if (code !== null && code !== 0) {
                    reject(new Error(`the fixture server exited (code: ${code}, signal: ${signal})`));
                }
            });
        });
    }

    Module.preRun = Module.preRun || [];
    Module.preRun.push(function() {
        if (needsHttpServer) {
            addRunDependency('w3c-fixture-server');
            startHttpServer().then((bound) => {
                // The program reads the host's environment as its own
                // (NODERAWFS), from main() on, so the port the server bound
                // reaches basic_http_test_endpoint.h's out-of-process reader
                // through the variable it reads.
                process.env.SCE_W3C_HTTP_PORT = String(bound);
                process.env.USE_EXTERNAL_HTTP_SERVER = '1';
                removeRunDependency('w3c-fixture-server');
            }, (err) => {
                // A run of `http` fixtures without their server is not a
                // conformance run: stop before main() rather than report each
                // fixture as failed.
                console.error(`[ERROR] ${err.message}`);
                process.exit(1);
            });
        }
    });

    // The program's exit is taken here, with the server, so the listener is
    // never left running behind it.
    Module['onExit'] = function(status) {
        if (httpServerProcess && !httpServerProcess.killed) {
            httpServerProcess.kill('SIGKILL');
        }
        process.exit(status || 0);
    };
    process.on('exit', () => {
        if (httpServerProcess && !httpServerProcess.killed) {
            httpServerProcess.kill('SIGKILL');
        }
    });
}
