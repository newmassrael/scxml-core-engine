"""The MCP server over HTTP, for a caller on another machine.

stdio reaches only a client that starts this server itself, on the machine
it runs on. An author working through a hosted assistant, or a program
calling a model's API, holds the specification and the drafts somewhere
else -- and the tools used to take nothing but paths on this machine, so
that caller had no way in at all. Over HTTP the same tools answer, and every
file is handed over as text (`mcp._Staging`): a path from a remote caller
names this machine's files, which it neither sees nor should reach, so it is
refused.

The transport is MCP's Streamable HTTP in its plainest form: a POST to the
endpoint carries one JSON-RPC message or a batch, and the answer is JSON --
202 with no body when nothing in it wanted a reply. There is no server-sent
stream, because no tool here sends anything unasked; a GET is 405, which the
protocol reads as exactly that.

⚠ Three guards, each for a way this is reached that stdio never is:

  a token       required whenever the server listens beyond loopback, and
                sent as `Authorization: Bearer <token>`. An authoring server
                runs the code generator on whatever it is handed; open to a
                network, it is a service anyone there can use.
  Origin        a request carrying a browser's Origin that is not this
                server's own is refused, so a page in the owner's browser
                cannot drive a loopback server (DNS rebinding).
  a size cap    a body over `MAX_BODY` is refused before it is read.

⚠ And a fourth, for the one thing a token does not answer: who is trusted with
the host. A caller with the token may hand over a DESIGN, and playing a design
runs code nobody has read under the isolation `process.isolation_level` names
(a child process under the kernel's limits), which stops a runaway and not a
design that reads a file. So a design from a remote caller is read and checked
and not played, until the operator says `--run-designs-under LEVEL`: the weakest
isolation they accept. The host has to give at least that or the server does not
start, and a caller that asks to play a design before then is told why, in the
answer, with how to change it.
"""

from __future__ import annotations

import hmac
import json
import posixpath
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

from . import mcp, process

# The endpoint's one path segment: requests go to `http://HOST:PORT/mcp`.
ENDPOINT = "mcp"

# A document set as text, several figures' worth of SVG, and headroom. A
# body past this is not an authoring request.
MAX_BODY = 16 * 1024 * 1024

_LOOPBACK = {"127.0.0.1", "::1", "localhost"}


class _Handler(BaseHTTPRequestHandler):
    server: "_Server"
    protocol_version = "HTTP/1.1"

    def do_POST(self) -> None:  # noqa: N802 - the stdlib's name
        segments = [s for s in urlsplit(self.path).path.split(posixpath.sep) if s]
        if segments != [ENDPOINT]:
            return self._plain(404, "not found: the endpoint is /" + ENDPOINT)
        refusal = self._refusal()
        if refusal is not None:
            return self._plain(*refusal)
        length = self.headers.get("Content-Length")
        if length is None or not length.isdigit():
            return self._plain(411, "a Content-Length is required")
        if int(length) > MAX_BODY:
            return self._plain(413, f"a request body is at most {MAX_BODY} bytes")
        body = self.rfile.read(int(length))
        try:
            message = json.loads(body)
        except (json.JSONDecodeError, UnicodeDecodeError) as exc:
            return self._json(400, {"jsonrpc": "2.0", "id": None,
                                    "error": {"code": -32700,
                                              "message": f"parse error: {exc}"}})
        batch = isinstance(message, list)
        replies = [reply for reply in map(self._one, message if batch else [message])
                   if reply is not None]
        if not replies:
            # Notifications, or responses: nothing to say back.
            self.send_response(202)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return None
        return self._json(200, replies if batch else replies[0])

    def do_GET(self) -> None:  # noqa: N802
        self._plain(405, "this server sends nothing unasked; POST to /" + ENDPOINT)

    def do_DELETE(self) -> None:  # noqa: N802
        self._plain(405, "this server keeps no session to end")

    def _one(self, message):
        # The same guard the stdio loop keeps: one bad message is answered,
        # and the server stays up for the next.
        try:
            return mcp.handle(message, remote=True,
                              designs_withheld=self.server.designs_withheld)
        except Exception as exc:  # noqa: BLE001 - staying up outranks the bug
            ident = message.get("id") if isinstance(message, dict) else None
            return {"jsonrpc": "2.0", "id": ident,
                    "error": {"code": -32603,
                              "message": f"internal error: {type(exc).__name__}: {exc}"}}

    def _refusal(self) -> tuple[int, str] | None:
        token = self.server.token
        if token is not None:
            sent = self.headers.get("Authorization", "")
            if not hmac.compare_digest(sent, f"Bearer {token}"):
                return 401, "a bearer token is required"
        origin = self.headers.get("Origin")
        if origin is not None and origin not in self.server.origins:
            return 403, f"requests from origin {origin!r} are not served"
        return None

    def _json(self, status: int, payload) -> None:
        data = (json.dumps(payload, ensure_ascii=False) + "\n").encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _plain(self, status: int, text: str) -> None:
        data = (text + "\n").encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, format, *args) -> None:  # noqa: A002 - stdlib signature
        # Requests carry specifications; the access log is not the place
        # for their shape. Errors still reach stderr through the stdlib.
        return None


class _Server(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, host: str, port: int, token: str | None,
                 run_designs_under: str | None = None):
        super().__init__((host, port), _Handler)
        self.token = token
        self.run_designs_under = run_designs_under
        # None is the operator's statement that designs may be played; the
        # absence of that statement is the sentence the caller reads instead.
        self.designs_withheld = None if run_designs_under else mcp.DESIGNS_WITHHELD
        bound = self.server_address[1]
        self.origins = {f"http://{name}:{bound}" for name in (host, *_LOOPBACK)}


def make_server(host: str, port: int, token: str | None,
                run_designs_under: str | None = None) -> _Server:
    """The server, bound and not yet serving -- what a test drives.

    Refuses a non-loopback address without a token rather than serving one
    open, and a host that isolates less than the operator asked for rather than
    playing designs under it: see the module note."""
    if host not in _LOOPBACK and not token:
        raise SystemExit(
            f"refusing to listen on {host} without a token: anyone who can "
            f"reach it could run the code generator. Pass --token-file.")
    if token is not None and not token:
        raise SystemExit("the token file is empty")
    if run_designs_under is not None:
        if run_designs_under not in process.ISOLATION_LEVELS:
            raise SystemExit(f"{run_designs_under!r} is not an isolation level this server "
                             f"knows: {', '.join(process.ISOLATION_LEVELS)}")
        given = process.isolation_level()
        if process.isolation_rank(given) < process.isolation_rank(run_designs_under):
            raise SystemExit(
                f"refusing to start: asked to play designs under {run_designs_under!r}, and "
                f"this host isolates them as {given!r}. Name the level this host gives, or "
                f"serve from a host that gives more.")
    return _Server(host, port, token, run_designs_under)


def serve_http(host: str, port: int, token: str | None,
               run_designs_under: str | None = None) -> int:
    server = make_server(host, port, token, run_designs_under)
    bound_host, bound_port = server.server_address[:2]
    url = f"http://{bound_host}:{bound_port}{posixpath.sep}{ENDPOINT}"
    print(f"sce-author MCP over HTTP at {url}", flush=True)
    if run_designs_under:
        print(f"designs from callers are played, under {process.isolation_level()!r} "
              f"(asked for at least {run_designs_under!r})", flush=True)
    else:
        print("designs from callers are read and checked, not played "
              "(--run-designs-under LEVEL allows it)", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0
