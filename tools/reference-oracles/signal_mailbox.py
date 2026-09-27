"""Expected values for sce:std/sync/mailbox_assign, mailbox_insert,
mailbox_page and mailbox_ack, computed by running Signal-Server's own
per-device queue scripts (service/src/main/resources/lua/insert_item.lua,
get_items.lua, remove_item_by_guid.lua; AGPL-3.0 — executed, not copied)
in a throwaway Redis.

A seeded sequence of offers, pages and acknowledgements is played against
the scripts. Before each step the queue is read back (ZRANGE WITHSCORES),
so every case's arguments are the reference's own state, and its expected
value is what the script did. Where the script is wrong the documents fix
it; those cases say "deviation:" and record what the script answered:
- insert_item.lua answers a new message with SPUBLISH's result (whether a
  subscriber listened) and a repeated one with its id; the document always
  answers the id. The id of a new message is read back from the guid map.
- remove_item_by_guid.lua deletes the metadata, counter included, when the
  queue drains, so the next id is 1 again; the document's counter is the
  host's and continues. After that step this script re-scores the entry the
  reference numbered from 1 to the document's id, and writes the host's
  counter back, so the steps after it compare like with like; every case
  that step touches says so.

Usage (see README.md):
    python3 tools/reference-oracles/signal_mailbox.py <signal-server-checkout> > cases.json
Requires docker and the redis:7-alpine image.
"""

import json
import random
import subprocess
import sys
import uuid

CONTAINER = f"sce-oracle-redis-{uuid.uuid4().hex[:8]}"
Q, M, L, CH = "user_queue::{a::1}", "user_queue_metadata::{a::1}", "user_queue_persisting::{a::1}", "ch::{a::1}"


def redis(*args: str) -> list[str]:
    out = subprocess.run(
        ["docker", "exec", CONTAINER, "redis-cli", "--raw", *args],
        check=True, capture_output=True, text=True,
    )
    # --raw prints an empty array as one blank line; no member here is blank.
    return [line for line in out.stdout.splitlines() if line]


def halves(guid: str) -> tuple[int, int]:
    b = uuid.UUID(guid).bytes
    return int.from_bytes(b[:8], "big"), int.from_bytes(b[8:], "big")


def slot(i: int, guid: str) -> dict:
    hi, lo = halves(guid)
    return {"id": i, "hi": hi, "lo": lo}


def queue() -> list[dict]:
    """The reference's queue: its sorted set, each member the guid string
    (the envelope bytes here are the guid itself), in score order."""
    flat = redis("ZRANGE", Q, "0", "-1", "WITHSCORES")
    return [slot(int(flat[k + 1]), flat[k]) for k in range(0, len(flat), 2)]


def counter() -> int:
    got = redis("HGET", M, "counter")
    return int(got[0]) if got and got[0] else 0


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: signal_mailbox.py <path to signalapp/Signal-Server checkout>")
    lua = f"{sys.argv[1]}/service/src/main/resources/lua"
    scripts = {n: open(f"{lua}/{n}.lua").read() for n in ("insert_item", "get_items", "remove_item_by_guid")}
    subprocess.run(["docker", "run", "-d", "--rm", "--name", CONTAINER, "redis:7-alpine"],
                   check=True, capture_output=True)
    try:
        for _ in range(50):
            if redis("ping") == ["PONG"]:
                break
        rng = random.Random(0x5a17b0c5)
        guids = [str(uuid.UUID(int=rng.getrandbits(128), version=4)) for _ in range(12)]
        assign, insert, page, ack = [], [], [], []
        host_counter = 0  # the document's counter: the host's, never reset

        def offer(g: str, note: str) -> None:
            nonlocal host_counter
            before, ref_counter = queue(), counter()
            got = redis("EVAL", scripts["insert_item"], "3", Q, M, CH, g, g, "payload")
            reply = got[0] if got else "nil"
            after = queue()
            hi, lo = halves(g)
            ref_id = int(redis("HGET", M, g)[0])
            held = any(s["hi"] == hi and s["lo"] == lo for s in before)
            if ref_counter == host_counter:
                assign.append({
                    "args": [before, host_counter, hi, lo], "expected": ref_id,
                    "note": note if held else f"deviation: {note} — the script answered {reply} (SPUBLISH found no subscriber), not the id {ref_id}",
                })
                insert.append({"args": [before, host_counter, hi, lo], "expected": after, "note": note})
            else:
                # The script's counter restarted when the queue drained.
                mine = next((s["id"] for s in before if s["hi"] == hi and s["lo"] == lo), host_counter + 1)
                expected = before if held else before + [slot(mine, g)]
                assign.append({
                    "args": [before, host_counter, hi, lo], "expected": mine,
                    "note": f"deviation: {note} — the queue drained earlier and the script's counter restarted at {ref_counter}; it gave id {ref_id}",
                })
                insert.append({
                    "args": [before, host_counter, hi, lo], "expected": expected,
                    "note": f"deviation: {note} — the script stored it under id {ref_id} of its restarted counter",
                })
                # Keep the reference's state in the document's numbering for
                # the steps after: re-score the new entry.
                if not held:
                    redis("ZADD", Q, "XX", str(mine), g)
                    redis("HSET", M, g, str(mine))
            if not held:
                host_counter += 1
            redis("HSET", M, "counter", str(host_counter))

        def read(after_id: int, limit: int, note: str) -> None:
            before = queue()
            flat = redis("EVAL", scripts["get_items"], "2", Q, L, str(limit), str(after_id), "false")
            got = [slot(int(flat[k + 1]), flat[k]) for k in range(0, len(flat), 2)]
            page.append({"args": [before, after_id, limit], "expected": got, "note": note})

        def acknowledge(gs: list[str], note: str) -> None:
            before = queue()
            redis("EVAL", scripts["remove_item_by_guid"], "2", Q, M, *gs)
            acked = [dict(zip(("hi", "lo"), halves(g))) for g in gs]
            ack.append({"args": [before, acked], "expected": queue(), "note": note})

        offer(guids[0], "a first message")
        offer(guids[1], "a second message")
        offer(guids[0], "the first message offered again")
        offer(guids[2], "a third message")
        read(-1, 2, "the first page")
        read(2, 2, "the page after id 2")
        read(3, 2, "past the last id")
        acknowledge([guids[1]], "the middle message acknowledged")
        read(-1, 5, "a page with a gap in its ids")
        acknowledge([guids[1], guids[5]], "an acknowledgement repeated, and one for a message never queued")
        for k in range(3, 9):
            offer(guids[k], f"message {k + 1}")
        read(4, 3, "a page from the middle")
        acknowledge([guids[k] for k in (0, 2, 3, 4, 5, 6, 7, 8)], "every message acknowledged: the queue drains")
        offer(guids[9], "a message after the queue drained")
        offer(guids[10], "the message after it")
        read(-1, 10, "a page of the messages after the drain")
        acknowledge([], "an empty acknowledgement")
        offer(guids[9], "a message offered again after the drain")
    finally:
        subprocess.run(["docker", "rm", "-f", CONTAINER], capture_output=True)
    print(json.dumps({
        "reference": "signalapp/Signal-Server lua/insert_item.lua, get_items.lua, remove_item_by_guid.lua",
        "mailbox_assign": assign, "mailbox_insert": insert, "mailbox_page": page, "mailbox_ack": ack,
    }, indent=1))


if __name__ == "__main__":
    main()
