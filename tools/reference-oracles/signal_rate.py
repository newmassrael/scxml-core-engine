"""Expected values for sce:std/rate/gcra_admit, computed by running
Signal-Server's own token-bucket script (lua/validate_rate_limit.lua,
AGPL-3.0 — executed, not copied) in a throwaway Redis.

The script's state is (tokens, last); the document's is one arrival time.
Where the bucket loses nothing — every elapsed time a whole number of
intervals, and 1/interval exact in binary so the script's double product is
exact — the two are the same limiter, and the bucket's state maps to
`tat = last + (size - tokens) * interval`. Those cases are taken from the
script. Outside that, the bucket is wrong and the document fixes it; those
cases are marked "deviation", their expected value is the document's
definition, and the note records what the script answered.

Usage (see README.md):
    python3 tools/reference-oracles/signal_rate.py <signal-server-checkout> > cases.json
Requires docker and the redis:7-alpine image.
"""

import json
import subprocess
import sys
import uuid

CONTAINER = f"sce-oracle-redis-{uuid.uuid4().hex[:8]}"


def redis(*args: str) -> str:
    out = subprocess.run(
        ["docker", "exec", CONTAINER, "redis-cli", "--no-raw", *args],
        check=True, capture_output=True, text=True,
    )
    return out.stdout.strip()


def evaluate(script: str, key: str, size: int, interval: int, now: int, cost: int) -> int:
    rate = repr(1.0 / interval)  # Java's String.valueOf(double) for these exact values
    reply = redis("EVAL", script, "1", key, str(size), rate, str(now), str(cost), "true")
    return int(reply.split()[-1])


def state(key: str, size: int, interval: int, fresh_tat: int) -> int:
    fields = redis("HMGET", key, "s", "t").splitlines()
    values = [f.split(")", 1)[-1].strip().strip('"') for f in fields]
    if values[0] in ("", "(nil)") or values[1] in ("", "(nil)"):
        return fresh_tat
    tokens, last = int(float(values[0])), int(float(values[1]))
    return last + (size - tokens) * interval


def gcra(tat: int, now: int, cost: int, interval: int, burst: int) -> dict:
    arrival = max(tat, now) + cost * interval
    limit = now + burst * interval
    ok = arrival <= limit
    return {"admitted": ok, "tat": arrival if ok else tat, "retryAfter": 0 if ok else arrival - limit}


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: signal_rate.py <path to signalapp/Signal-Server checkout>")
    with open(f"{sys.argv[1]}/service/src/main/resources/lua/validate_rate_limit.lua") as f:
        script = f.read()
    subprocess.run(["docker", "run", "-d", "--rm", "--name", CONTAINER, "redis:7-alpine"],
                   check=True, capture_output=True)
    try:
        for _ in range(50):
            if subprocess.run(["docker", "exec", CONTAINER, "redis-cli", "ping"],
                              capture_output=True, text=True).stdout.strip() == "PONG":
                break
        cases = []
        # (size, interval, [(now, cost), ...]) — every elapsed time a whole
        # number of intervals. The intervals are 2^17..2^20 ms: the script
        # sets a real-time PEXPIRE of (spent permits x interval) ms, which a
        # short interval lets lapse before the state is read back.
        i1, i2, i3 = 2**17, 2**18, 2**20
        t1, t2, t3 = 10 * i1, 20 * i2, 30 * i3
        runs = [
            (3, i1, [(t1, 1), (t1, 1), (t1, 1), (t1, 1), (t1 + i1, 1), (t1 + i1, 2)]),
            (5, i2, [(t2, 5), (t2, 1), (t2 + i2, 2), (t2 + 5 * i2, 3), (t2 + 5 * i2, 3)]),
            (10, i3, [(t3, 4), (t3 + i3, 4), (t3 + i3, 4), (t3 + 11 * i3, 10), (t3 + 11 * i3, 1)]),
        ]
        for size, interval, calls in runs:
            key = f"bucket-{size}-{interval}"
            tat = 0
            for now, cost in calls:
                before = state(key, size, interval, tat)
                deficit = evaluate(script, key, size, interval, now, cost)
                after = state(key, size, interval, before)
                expected = {
                    "admitted": deficit == 0,
                    "tat": after if deficit == 0 else before,
                    "retryAfter": deficit * interval,
                }
                assert expected == gcra(before, now, cost, interval, size), (expected, before, now, cost)
                cases.append({"args": [before, now, cost, interval, size], "expected": expected,
                              "note": f"size {size}, 1 per {interval} ms: {cost} at {now} (script deficit {deficit})"})
                tat = after if deficit == 0 else before
        # Deviation 1: an elapsed time that is not a whole number of intervals.
        size, interval = 2, i1
        t = 40 * i1
        key = "bucket-fraction"
        evaluate(script, key, size, interval, t, 2)                           # empty the bucket at t
        d1 = evaluate(script, key, size, interval, t + interval * 3 // 2, 1)  # 1.5 intervals later: 1 permit, 0.5 lost
        d2 = evaluate(script, key, size, interval, t + 2 * interval, 1)       # 2 intervals after t
        tat = t + 2 * interval
        first = gcra(tat, t + interval * 3 // 2, 1, interval, size)
        second = gcra(first["tat"], t + 2 * interval, 1, interval, size)
        cases.append({"args": [first["tat"], t + 2 * interval, 1, interval, size], "expected": second,
                      "note": f"deviation: 2 intervals after emptying, after one permit spent at 1.5 — "
                              f"the script answered deficit {d1} then {d2}: its floor at 1.5 dropped half a permit, "
                              f"so the second permit, due at 2 intervals, is refused"})
        # A clock behind the stored time (another server's): the script's
        # negative elapsed time takes permits away but also moves `last`
        # back, and the outcome is the arrival time's — measured, so it is
        # an agreement case, not a deviation.
        key = "bucket-backwards"
        t = 50 * i1
        evaluate(script, key, 3, i1, t, 1)
        for now, cost in [(t - 10 * i1, 2), (t - i1, 1), (t, 1)]:
            before = state(key, 3, i1, 0)
            deficit = evaluate(script, key, 3, i1, now, cost)
            after = state(key, 3, i1, before)
            expected = {"admitted": deficit == 0, "tat": after if deficit == 0 else before,
                        "retryAfter": deficit * i1}
            assert expected == gcra(before, now, cost, i1, 3), (expected, before, now, cost)
            cases.append({"args": [before, now, cost, i1, 3], "expected": expected,
                          "note": f"{(t - now) // i1} intervals before the permit spent at t, after a clock "
                                  f"running behind (script deficit {deficit}) — agrees"})
        # Deviation 2: a request larger than the burst can never be admitted.
        key = "bucket-oversized"
        d = evaluate(script, key, 3, i1, 60 * i1, 4)
        cases.append({"args": [0, 60 * i1, 4, i1, 3], "fails": "precondition",
                      "note": f"deviation: a request of 4 against a burst of 3 — the script answers deficit {d}, "
                              f"a finite wait for a request no wait admits"})
        json.dump({"reference": "signalapp/Signal-Server lua/validate_rate_limit.lua", "gcra_admit": cases},
                  sys.stdout, indent=1)
        print()
    finally:
        subprocess.run(["docker", "rm", "-f", CONTAINER], capture_output=True)


if __name__ == "__main__":
    main()
