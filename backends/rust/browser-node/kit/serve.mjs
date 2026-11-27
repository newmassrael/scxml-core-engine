// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The origin the kit runs at: one loopback server that serves the page, the shell, the kit and
// the module, and passes `/sce-mesh/` to the binding's server (`https_echo`), which it starts,
// so that the page's exchanges are of the origin that served it (docs/adr/0117, D9). The request
// goes on with the `Host` it came with, which is the target URI the page signed.
//
//   POST /kit/proxy?open=0|1   fail every exchange at the transport from now on, or stop
//   GET  /kit/server           what the server printed when it started
//   POST /kit/result           the page's report, which `result` resolves with

import http from "node:http";
import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
};

/** Start `echo` (the server's binary) and the origin in front of it; `glue` is the web glue. */
export async function serveKit({ echo, glue = null }) {
  const server = spawn(echo, [], { stdio: ["ignore", "pipe", "inherit"] });
  const exited = new Promise((resolve) => server.on("exit", (code) => resolve(code)));
  const printed = await Promise.race([
    new Promise((resolve) => {
      readline.createInterface({ input: server.stdout }).once("line", resolve);
    }),
    exited.then((code) => {
      throw new Error(`the server exited (${code}) before it printed where it listens`);
    }),
  ]);
  const started = JSON.parse(printed);

  let open = true;
  let received;
  const result = new Promise((resolve) => {
    received = resolve;
  });
  // Only these directories, and only a file directly in one, by name.
  const served = {
    "/js/": path.join(here, "..", "js"),
    "/kit/": here,
    ...(glue ? { "/pkg/": glue } : {}),
  };

  const origin = http.createServer((request, response) => {
    const { pathname, searchParams } = new URL(request.url, "http://127.0.0.1");
    if (pathname.startsWith("/sce-mesh/")) {
      if (!open) {
        request.socket.destroy();
        return;
      }
      const upstream = http.request(
        {
          host: "127.0.0.1",
          port: started.port,
          method: request.method,
          path: request.url,
          headers: request.headers,
        },
        (answer) => {
          response.writeHead(answer.statusCode, answer.headers);
          answer.pipe(response);
        },
      );
      upstream.on("error", () => request.socket.destroy());
      response.on("close", () => upstream.destroy());
      request.pipe(upstream);
      return;
    }
    if (request.method === "POST" && pathname === "/kit/proxy") {
      open = searchParams.get("open") !== "0";
      response.writeHead(204).end();
      return;
    }
    if (request.method === "GET" && pathname === "/kit/server") {
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify(started));
      return;
    }
    if (request.method === "POST" && pathname === "/kit/result") {
      const body = [];
      request.on("data", (chunk) => body.push(chunk));
      request.on("end", () => {
        response.writeHead(204).end();
        received(Buffer.concat(body).toString("utf8"));
      });
      return;
    }
    if (request.method === "GET" && pathname === "/") {
      response.writeHead(200, { "content-type": TYPES[".html"] });
      response.end(readFileSync(path.join(here, "page.html")));
      return;
    }
    for (const [prefix, directory] of Object.entries(served)) {
      const name = pathname.startsWith(prefix) ? pathname.slice(prefix.length) : null;
      if (request.method === "GET" && name && name === path.basename(name)) {
        const file = path.join(directory, name);
        if (existsSync(file)) {
          response.writeHead(200, {
            "content-type": TYPES[path.extname(file)] ?? "application/octet-stream",
          });
          response.end(readFileSync(file));
          return;
        }
      }
    }
    response.writeHead(404).end();
  });
  await new Promise((resolve) => origin.listen(0, "127.0.0.1", resolve));

  return {
    origin: `http://127.0.0.1:${origin.address().port}`,
    server: started,
    result,
    setOpen(value) {
      open = value;
    },
    async close() {
      origin.closeAllConnections?.();
      await new Promise((resolve) => origin.close(resolve));
      server.kill();
      await exited;
    },
  };
}
