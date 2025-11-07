// XMLHttpRequest polyfill for Node.js environment
if (typeof module !== 'undefined' && typeof require !== 'undefined') {
    try {
        global.XMLHttpRequest = require('xmlhttprequest').XMLHttpRequest;
        console.log('[INFO] XMLHttpRequest polyfill loaded for Node.js');
    } catch (e) {
        console.warn('[WARN] xmlhttprequest polyfill not found. Install: npm install xmlhttprequest');
    }

    const { spawn } = require('child_process');
    const path = require('path');

    // W3C SCXML C.2: HTTP test IDs requiring external HTTP server
    const HTTP_TEST_IDS = new Set(['201', '509', '510', '513', '518', '519', '520', '522', '531', '532', '534', '567', '577']);

    // Detect if HTTP tests will be executed
    const args = process.argv.slice(2);
    const testArgs = args.filter(arg => !arg.startsWith('--'));  // Filter out flags (--interpreter-only, etc.)
    const needsHttpServer = testArgs.length === 0 || // Run all tests (no test numbers specified)
        testArgs.some(arg => {
            // Exact match
            if (HTTP_TEST_IDS.has(arg)) return true;

            // Range format: "500~600", "~600", or "500~"
            if (arg.includes('~')) {
                const parts = arg.split('~');
                const start = parseInt(parts[0]) || 1;    // Empty string → 1 (start from test 1)
                const end = parseInt(parts[1]) || 999;     // Empty string → 999 (end at max test)
                return Array.from(HTTP_TEST_IDS).some(id => {
                    const num = parseInt(id);
                    return num >= start && num <= end;
                });
            }

            return false;
        });

    let httpServerProcess = null;
    let httpServerReady = false;

    // Module initialization: Ensure Module and preRun are defined
    if (!Module) var Module = {};
    if (!Module.preRun) Module.preRun = [];

    // NODEFS mounting: Execute in preRun (after FS initialization, before main())
    Module.preRun.push(function() {
        try {
            // Mount project root to WASM /project
            // process.cwd() is build_wasm/tests, so ../.. is project root
            const projectRoot = path.resolve(process.cwd(), "../..");
            FS.mkdir("/project");
            FS.mount(NODEFS, { root: projectRoot }, "/project");
            console.log("[INFO] NODEFS mounted project root at /project:", projectRoot);
        } catch (e) {
            console.error("[ERROR] Failed to mount NODEFS:", e.message);
        }
    });

    // HTTP server management: Start asynchronously (independent of FS)
    (async function() {
        // Start external HTTP server if needed
        if (needsHttpServer) {
            console.log("[INFO] HTTP tests detected - starting external HTTP server...");

            try {
                await startHttpServer();
                process.env.USE_EXTERNAL_HTTP_SERVER = '1';
                console.log("[INFO] External HTTP server ready - w3c_test_cli will use it");
            } catch (err) {
                console.error("[ERROR] Failed to start HTTP server:", err.message);
                console.error("[ERROR] HTTP tests will fail without server");
                // Continue anyway - tests will fail gracefully
            }
        }
    })();  // Immediately invoke async function

    // Force clean exit after main() completes (EXIT_RUNTIME=0 keeps event loop alive)
    Module['onExit'] = function(status) {
        // Kill HTTP server when runtime exits
        if (httpServerProcess && !httpServerProcess.killed) {
            httpServerProcess.kill('SIGKILL');
        }
        // Force process exit
        process.exit(status || 0);
    };

    // Start HTTP server and wait for ready signal
    async function startHttpServer() {
        const serverScript = path.resolve(__dirname, 'standalone_http_server.js');
        const serverPort = 8080;
        const serverPath = '/test';

        httpServerProcess = spawn('node', [serverScript, serverPort.toString(), serverPath], {
            stdio: ['ignore', 'pipe', 'pipe'],
            detached: false  // Keep attached for proper cleanup
        });

        // Do NOT unref() - we need to wait for server to be ready
        // httpServerProcess.unref();  // Removed: causes race condition with server startup

        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                if (httpServerProcess) {
                    httpServerProcess.kill();
                }
                reject(new Error(`HTTP server startup timeout (10000ms)`));
            }, 10000);  // Increase timeout to 10 seconds for WASM environment

            httpServerProcess.stdout.on('data', (data) => {
                const output = data.toString();
                process.stdout.write(output);

                if (output.includes('[READY]')) {
                    clearTimeout(timeout);
                    resolve();
                }
            });

            httpServerProcess.stderr.on('data', (data) => {
                process.stderr.write(`[Server Error] ${data.toString()}`);
            });

            httpServerProcess.on('error', (err) => {
                clearTimeout(timeout);
                reject(new Error(`Failed to spawn HTTP server: ${err.message}`));
            });

            httpServerProcess.on('exit', (code, signal) => {
                clearTimeout(timeout);
                if (code !== null && code !== 0) {
                    reject(new Error(`HTTP server exited prematurely (code: ${code}, signal: ${signal})`));
                }
            });
        });
    }

    // Cleanup handler - stop HTTP server on process exit
    function stopHttpServer() {
        if (httpServerProcess && !httpServerProcess.killed) {
            console.log('[INFO] Stopping HTTP server...');
            httpServerProcess.kill('SIGTERM');
            // Immediate force kill as fallback (no setTimeout to avoid event loop hang)
            httpServerProcess.kill('SIGKILL');
        }
    }

    // Register cleanup handlers
    process.on('exit', () => {
        if (httpServerProcess && !httpServerProcess.killed) {
            // Synchronous cleanup only (no async operations in exit handler)
            httpServerProcess.kill('SIGKILL');
        }
    });
}
