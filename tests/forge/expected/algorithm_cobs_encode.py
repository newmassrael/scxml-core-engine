# SCE-MAP: algorithm_cobs_encode:32

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.
#
# RFC §synth-5-A: pure synchronous function with bounded loops. Free
# function, no instance state. `bytes` parameters lower to Python's
# native `bytes` (RFC §synth-5-J-5 emitter table); iteration over `bytes`
# yields `int` values 0..255, which line up with the SCXML type-ctx
# contract that `<sce:foreach item>` is `uint8`. Numeric arithmetic
# in Python is arbitrary-precision so unsigned-narrow truncation is
# the body author's responsibility (e.g. `crc & 0xFFFF`).

def algorithm_cobs_encode(data: bytes) -> bytes:
    n: int = len(data)
    out = bytearray()
    p: int = 0
    done: bool = False
    while done == False:
        q: int = p
        while q < n and (q - p) & 0xFFFF < 254 and data[q] != 0:
            q = (q + 1) & 0xFFFF
        run: int = (q - p) & 0xFFFF
        code: int = (run + 1) & 0xFFFF
        out.append(code)
        k: int = p
        while k < q:
            out.append(data[k])
            k = (k + 1) & 0xFFFF
        if q >= n:
            done = True
        else:
            if run < 254:
                p = (q + 1) & 0xFFFF
                if p >= n:
                    last: int = 1
                    out.append(last)
                    done = True
            else:
                p = q
    return bytes(out)
