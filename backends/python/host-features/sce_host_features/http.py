# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""HTTP (docs/SCE_HOST_FEATURES.md, section 5): the semantics over a transport the
embedding supplies."""

from __future__ import annotations

import http.client
import re
import urllib.parse
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, List, Optional, Protocol, Tuple

from .records import HOLD, Outcome, failure, record

if TYPE_CHECKING:
    from .host import ReferenceHost

#: The most bytes of response headers and of body a result record holds.
HEADERS_BOUND = 4096
BODY_BOUND = 16384

METHODS = ("GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS")

#: An RFC 9110 token: what a header name is made of.
_TOKEN = re.compile(r"[!#$%&'*+\-.^_`|~0-9A-Za-z]+\Z")


@dataclass
class HttpRequest:
    """What the host hands its transport: the request, its header lines already
    split into name and value."""

    method: str
    url: str
    headers: List[Tuple[str, str]] = field(default_factory=list)
    body: str = ""


@dataclass
class HttpResponse:
    """What a transport answers: the response as it came."""

    status: int
    headers: List[Tuple[str, str]] = field(default_factory=list)
    body: bytes = b""


class TransportFailure(Exception):
    """A transport that got no response: name resolution, connection, TLS, or the
    connection ended."""


class Transport(Protocol):
    """Carries one exchange. A response is the answer; `TransportFailure` is a
    transport that got none; `None` is a transport that has not answered, and the
    host holds the exchange until it is cancelled."""

    def exchange(self, request: HttpRequest) -> Optional[HttpResponse]: ...


class HttpTransport:
    """An exchange over the standard library's HTTP client. It follows no
    redirect and keeps no cookie, and gives a response's fields in the order they
    were received."""

    def __init__(self, timeout_s: float = 30.0) -> None:
        self.timeout_s = timeout_s

    def exchange(self, request: HttpRequest) -> Optional[HttpResponse]:
        parts = urllib.parse.urlsplit(request.url)
        connection_type = (
            http.client.HTTPSConnection if parts.scheme == "https" else http.client.HTTPConnection
        )
        target = parts.path or "/"
        if parts.query:
            target += "?" + parts.query
        try:
            connection = connection_type(parts.hostname, parts.port, timeout=self.timeout_s)
            try:
                connection.putrequest(request.method, target, skip_accept_encoding=True)
                for name, value in request.headers:
                    connection.putheader(name, value)
                payload = request.body.encode("utf-8")
                if payload or request.method in ("POST", "PUT", "PATCH"):
                    connection.putheader("Content-Length", str(len(payload)))
                connection.endheaders(payload if payload else None)
                response = connection.getresponse()
                # One byte past the bound is enough to see it passed.
                body = response.read(BODY_BOUND + 1)
                return HttpResponse(response.status, list(response.getheaders()), body)
            finally:
                connection.close()
        except (OSError, http.client.HTTPException, ValueError) as error:
            raise TransportFailure(str(error)) from error


def _header_lines(text: str) -> Optional[List[Tuple[str, str]]]:
    """The header lines of a request, split into name and value, or `None` for a
    line that is not one. Empty lines are ignored, and the value is without the
    spaces and tabs around it."""
    headers: List[Tuple[str, str]] = []
    for line in text.split("\n"):
        line = line[:-1] if line.endswith("\r") else line
        if line == "":
            continue
        name, colon, value = line.partition(":")
        if not colon or not _TOKEN.match(name):
            return None
        headers.append((name, value.strip(" \t")))
    return headers


def _is_http_url(url: str) -> bool:
    """An absolute `http` or `https` URL with a host, and no ASCII space or control
    character (U+0000 to U+0020 and U+007F) in it."""
    for scheme in ("http://", "https://"):
        if url.startswith(scheme):
            rest = url[len(scheme):]
            break
    else:
        return False
    for index, char in enumerate(rest):
        if char in "/?#":
            authority = rest[:index]
            break
    else:
        authority = rest
    if authority == "" or authority.startswith(":"):
        return False
    return not any(ord(c) <= 0x20 or ord(c) == 0x7F for c in url)


def acceptable(method: str, url: str, headers: str) -> bool:
    """Whether the arguments of an exchange are ones the operation accepts."""
    return method in METHODS and _is_http_url(url) and _header_lines(headers) is not None


def fetch(host: "ReferenceHost", invoke_id: str, method: str, url: str, headers: str, body: str) -> Outcome:
    lines = _header_lines(headers) or []
    try:
        response = host.transport.exchange(HttpRequest(method, url, lines, body))
    except TransportFailure:
        return failure("sce-host-http-fetch", "network")
    if response is None:
        host.held.append(invoke_id)
        return Outcome(HOLD)
    joined = "\n".join(f"{name.lower()}: {value.strip()}" for name, value in response.headers)
    # A response to HEAD has no body, whatever the transport carried.
    payload = b"" if method == "HEAD" else response.body
    if len(joined.encode("utf-8")) > HEADERS_BOUND or len(payload) > BODY_BOUND:
        return failure("sce-host-http-fetch", "response_too_large")
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError:
        return failure("sce-host-http-fetch", "invalid_body")
    return record({"status": response.status, "headers": joined, "body": text})
