# d3 — vendored

`d3.v7.min.js` draws the visualizer: SVG elements, the data joins that bind
states and transitions to them, zoom, and the drag behaviours. It does not
compute layout — that is elkjs, next door.

| | |
|---|---|
| Version | 7.9.0 |
| License | ISC (see `LICENSE`) |
| Upstream | https://github.com/d3/d3 |
| Website | https://d3js.org/ |
| SHA-256 | `f2094bbf6141b359722c4fe454eb6c4b0f0e42cc10cc7af921fc158fceb86539` |
| Bytes | 279706 |

## Why it moved here

It sat at `web/visualizer/d3.v7.min.js` from the beginning, with no licence
file and no entry in `LICENSE-THIRD-PARTY.md` — which stated, for as long as
it was there, that every dependency was MIT. d3 is ISC. The claim was false
and nothing could contradict it, because the licence gate reads
`third_party/` and never looked at `web/`.

⚠ Being permissive is why this was harmless, not why it was acceptable. The
same blind spot carried elkjs, whose licence obliges a commercial
distributor. `scripts/gates/web-vendor-licenses.sh` now reads this directory
and requires each entry here to be registered — but it can only read what is
IN the directory, which is why a vendored file left outside it is invisible
to the gate however correct its licence happens to be.

## Refreshing it

```
curl -sS -o d3.v7.min.js https://cdn.jsdelivr.net/npm/d3@<version>/dist/d3.min.js
sha256sum d3.v7.min.js
```

Then update the version, hash and byte count above, and the row in
`LICENSE-THIRD-PARTY.md`.
