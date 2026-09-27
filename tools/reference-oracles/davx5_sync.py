"""Expected values for sce:std/sync/sync_failure and sync_retry_at, computed
by running DAVx5's own exception classifier
(core/src/main/kotlin/at/bitfire/davdroid/sync/SyncExceptionHandler.kt,
classifySyncException; GPL-3.0 — executed, not copied).

The classifier is an Android class's method, so this script lifts its body
and the SyncErrorAction type out of the checkout AT RUN TIME, compiles them
with kotlinc beside stubs for what they name (the Android and dav4jvm
exception types, the string resources, the context), and asks it about one
synthetic exception per case. Nothing of DAVx5 is written into this tree;
the lifted text lives in a temporary directory for the run.

What the stubs decide, stated rather than hidden:
- The dav4jvm exception classes are stand-ins, one per status the
  classifier names (401, 502, 503) under an HttpException for the rest. The
  classifier tests the specific classes before HttpException, so their
  place in the real hierarchy does not change its answer.
- ServiceUnavailableException.getDelayUntil() (dav4jvm, not in the
  checkout) is stubbed as "now + Retry-After"; the 503 delay is therefore
  the stub's, and the 502 delay (15 minutes, computed in the classifier) is
  the reference's.
- The classifier re-throws a cancellation and a dead storage process to the
  Syncer, which treats the first as nothing and the second as a soft error
  (Syncer.kt, read from source): the script maps them so.

sync_retry_at's merge is the document's fix: DAVx5's SyncManager.performSync
overwrites the run's retry time with each collection's (read from source,
one assignment); the cases where that loses a later time say "deviation:".

Usage (see README.md):
    python3 tools/reference-oracles/davx5_sync.py <davx5-checkout> > cases.json
Requires kotlinc and java on PATH.
"""

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

NOW = 1_790_000_000

STUBS = """
import java.time.Instant

annotation class VisibleForTesting
open class RemoteException(message: String? = null) : Exception(message)
class DeadObjectException : RemoteException("dead")
class LocalStorageException(message: String? = null, cause: Throwable? = null) : Exception(message, cause)
open class DavException(message: String, cause: Throwable? = null) : Exception(message, cause)
open class HttpException(val code: Int) : DavException("HTTP $code")
class UnauthorizedException : HttpException(401)
class BadGatewayException : HttpException(502)
class ServiceUnavailableException(private val retryAfter: Long?) : HttpException(503) {
    fun getDelayUntil(): Instant? = retryAfter?.let { Instant.ofEpochSecond(%d + it) }
}
inline fun <reified T : Throwable> Throwable.causedBy(): T? =
    generateSequence(this) { it.cause }.filterIsInstance<T>().firstOrNull()
object R { object string {
    const val sync_error_io = 1
    const val sync_error_authentication_failed = 2
    const val sync_error_http_dav = 3
    const val sync_error_local_storage = 4
} }
object context { fun getString(id: Int, vararg args: Any?): String = "s$id" }
""" % NOW

MAIN = """
import java.io.IOException
import java.security.cert.CertificateException
import java.time.Instant
import java.util.concurrent.CancellationException
import javax.net.ssl.SSLHandshakeException

fun failure(kind: Int, status: Int, retryAfter: Long?): Throwable = when (kind) {
    1 -> IOException("transport")
    2 -> SSLHandshakeException("handshake")
    3 -> SSLHandshakeException("handshake").also { it.initCause(CertificateException("rejected")) }
    4 -> when (status) {
        401 -> UnauthorizedException()
        502 -> BadGatewayException()
        503 -> ServiceUnavailableException(retryAfter)
        else -> HttpException(status)
    }
    5 -> DavException("no sync-token")
    6 -> LocalStorageException("storage")
    7 -> CancellationException("cancelled")
    8 -> LocalStorageException("storage", DeadObjectException())
    else -> IllegalStateException("other")
}

fun main(args: Array<String>) {
    for (line in generateSequence(::readLine)) {
        val (kind, status, retry) = line.split(" ").map { it.toLong() }
        val before = Instant.now()
        val action = classifySyncException(failure(kind.toInt(), status.toInt(), if (retry < 0) null else retry))
        val out = when (action) {
            is SyncErrorAction.LogWarning -> "log"
            is SyncErrorAction.SoftError -> {
                val d = action.delayUntil
                if (d == null) "soft -1"
                else if (kind == 4L && status == 503L) "soft ${d.epochSecond - %d}"
                else "soft ${Math.round((d.toEpochMilli() - before.toEpochMilli()) / 1000.0)}"
            }
            is SyncErrorAction.HardError -> "hard ${action.logMessage}"
            is SyncErrorAction.Rethrow -> "rethrow ${action.throwable::class.simpleName}"
        }
        println(out)
    }
}
""" % NOW


def lift(source: str, start: str) -> str:
    """The declaration beginning at the line containing `start`, through its
    matching closing brace."""
    at = source.index(start)
    line_start = source.rfind("\n", 0, at) + 1
    depth, i, opened = 0, source.index("{", at), False
    while True:
        ch = source[i]
        if ch == "{":
            depth += 1
            opened = True
        elif ch == "}":
            depth -= 1
            if opened and depth == 0:
                return source[line_start:i + 1]
        i += 1


def classify(checkout: str, queries: list[tuple[int, int, int]]) -> list[str]:
    handler = Path(checkout, "core/src/main/kotlin/at/bitfire/davdroid/sync/SyncExceptionHandler.kt").read_text()
    action_type = lift(handler, "internal sealed interface SyncErrorAction")
    classifier = lift(handler, "internal fun classifySyncException")
    imports = "\n".join(line for line in handler.splitlines()
                        if re.match(r"import (java|javax|kotlin)\.", line))
    with tempfile.TemporaryDirectory() as tmp:
        Path(tmp, "Stubs.kt").write_text(STUBS)
        Path(tmp, "Classify.kt").write_text(imports + "\n\n" + action_type + "\n\n" + classifier + "\n")
        Path(tmp, "Main.kt").write_text(MAIN)
        jar = Path(tmp, "classify.jar")
        subprocess.run(["kotlinc", "-nowarn", "Stubs.kt", "Classify.kt", "Main.kt", "-include-runtime", "-d", str(jar)],
                       cwd=tmp, check=True, capture_output=True, text=True)
        stdin = "\n".join(f"{k} {s} {r}" for k, s, r in queries) + "\n"
        out = subprocess.run(["java", "-jar", str(jar)], input=stdin, check=True, capture_output=True, text=True)
    return out.stdout.splitlines()


KIND = {1: "transport I/O", 2: "a TLS handshake", 3: "a rejected certificate", 5: "a DAV protocol error",
        6: "local storage", 7: "a cancellation", 8: "a dead storage process", 9: "an unclassified failure"}


def action(answer: str) -> int:
    if answer == "log":
        return 0
    if answer.startswith("soft"):
        return 1
    if answer == "hard Not authorized anymore":
        return 3
    if answer.startswith("hard"):
        return 2
    if answer == "rethrow CancellationException":
        return 4  # Syncer: nothing failed
    if answer == "rethrow DeadObjectException":
        return 1  # Syncer: a soft error, retried
    raise SystemExit(f"an answer this script cannot map: {answer}")


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: davx5_sync.py <path to bitfireAT/davx5 checkout>")
    statuses = [400, 401, 403, 404, 405, 409, 410, 412, 415, 423, 500, 501, 502, 503, 504, 507]
    queries = [(k, 0, -1) for k in (1, 2, 3, 5, 6, 7, 8, 9)] + [(4, s, -1) for s in statuses] + [(4, 503, 3600)]
    answers = classify(sys.argv[1], queries)
    failure = []
    for (kind, status, retry), answer in zip(queries, answers):
        if retry >= 0:
            continue
        what = f"HTTP {status}" if kind == 4 else KIND[kind]
        failure.append({"args": [kind, status], "expected": action(answer), "note": f"{what} — the classifier answered {answer}"})
    failure += [
        {"args": [0, 0], "fails": "precondition", "note": "no kind of failure"},
        {"args": [4, 200], "fails": "precondition", "note": "a success is not a failure"},
        {"args": [1, 500], "fails": "precondition", "note": "a status on a failure that was not a response"},
    ]
    delay = {(k, s, r): a for (k, s, r), a in zip(queries, answers)}
    after_502 = int(delay[(4, 502, -1)].split()[1])
    after_503 = int(delay[(4, 503, 3600)].split()[1])
    if after_502 != 900:
        raise SystemExit(f"the classifier's 502 delay is {after_502}s, not the 15 minutes the document states")
    retry = []
    for previous, kind, status, retry_after, note in [
        (0, 4, 503, 3600, "a 503 with an hour of Retry-After"),
        (0, 4, 503, 0, "a 503 with no Retry-After asks for no time"),
        (0, 4, 502, 0, "a 502"),
        (0, 4, 404, 0, "a 404 asks for no time"),
        (0, 1, 0, 0, "a transport failure asks for no time"),
        (NOW + 3600, 4, 502, 0, "a 502 after an earlier collection's hour"),
        (NOW + 60, 4, 503, 3600, "a 503's hour after an earlier minute"),
        (NOW + 3600, 4, 404, 0, "a 404 after an earlier collection's hour"),
    ]:
        asked = 0
        if kind == 4 and status == 503 and retry_after > 0:
            asked = NOW + after_503
        elif kind == 4 and status == 502:
            asked = NOW + after_502
        expected = max(previous, asked)
        davx5 = asked if asked else previous
        text = note if davx5 == expected else f"deviation: {note} — DAVx5 overwrites the run's time with {davx5}"
        retry.append({"args": [previous, kind, status, NOW, retry_after], "expected": expected, "note": text})
    retry.append({"args": [-1, 1, 0, NOW, 0], "fails": "precondition", "note": "a negative earlier time"})
    retry.append({"args": [0, 4, 503, 9223372036854775000, 3600], "fails": "overflow", "note": "a time past the clock's range"})
    print(json.dumps({"reference": "DAVx5 SyncExceptionHandler.classifySyncException",
                      "sync_failure": failure, "sync_retry_at": retry}, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
