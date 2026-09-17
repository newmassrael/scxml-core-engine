# elkjs — vendored

`elk.bundled.js` is the Eclipse Layout Kernel compiled to JavaScript. The
visualizer uses it for every diagram it draws: node placement, container
bounds, edge routing and transition-label placement all come from it.

| | |
|---|---|
| Version | 0.9.3 |
| License | EPL-2.0 (see `LICENSE`) |
| Upstream project | https://github.com/kieler/elkjs |
| Upstream (ELK itself) | https://github.com/eclipse-elk/elk |
| Package | https://registry.npmjs.org/elkjs/0.9.3 |
| SHA-256 | `b0745abd7f23cd91690a1587e377edbe19fd7233c783300290936720546216d4` |
| Bytes | 1606238 |

## Why it is vendored rather than fetched

It used to be loaded from `https://unpkg.com/elkjs@0.9.3/lib/elk.bundled.js`
at runtime, which had two consequences worth stating rather than repeating:

1. **A third party could take the visualizer down.** With the CDN
   unreachable the script never runs, `ELK` is undefined, and the page dies
   in `SCXMLVisualizer`'s constructor — the whole visualizer, not just the
   layout.
2. **No offline build or test could use it.** A gate that measures a layout
   cannot depend on the network, and a gate that installs its own copy
   measures a different copy from the one that ships.

## EPL-2.0, and what shipping this file obliges

`elk.bundled.js` is object code: a GWT compilation of ELK's Java sources,
minified. EPL-2.0 section 3 therefore applies to redistributing it.

- **Source Code must be available.** It is, upstream, at the two project
  links above. This file is unmodified, so upstream *is* its source.
- **Notices may not be removed or altered.** The bundle carries its own —
  `Copyright (c) 2017 Kiel University and others`, `Copyright (c) 2021 Kiel
  University and others`, `Copyright 2020 Google LLC` (from a bundled
  dependency) and EPL-2.0 references. It is committed byte for byte; the
  SHA-256 above is what a reviewer checks that against.

⚠ **Section 4 (Commercial Distribution) is an open question, not a settled
one.** It obliges a contributor who "includes the Program in a commercial
product offering" to defend and indemnify every other contributor — and
under EPL-2.0 a distributor *is* a contributor. Claims about intellectual
property infringement are excluded, which bounds it, but the obligation is
real and it attaches when SCE is sold rather than when this file is
committed. Whoever owns the commercial licence needs to have read that
section before SCE ships commercially with this file in it.

⚠ Not vendoring would not have avoided that. A commercial deployment cannot
depend on a public CDN, so the copy reaches the customer either way; the
choice was only whether it reaches them with its licence and notices
attached.

## Refreshing it

```
curl -sS -o elk.bundled.js https://unpkg.com/elkjs@<version>/lib/elk.bundled.js
sha256sum elk.bundled.js
```

Then update the version, the hash and the byte count in the table above, and
the row in `LICENSE-THIRD-PARTY.md`. ⚠ Check the licence when you do:
elkjs 0.9.3 through 0.11.0 declare `EPL-2.0`, and 0.12.0 declares
`EPL-2.0 OR GPL-3.0-or-later`. The dual form still permits EPL-2.0, but it
is not the same statement and the registry must say which one SCE takes.
