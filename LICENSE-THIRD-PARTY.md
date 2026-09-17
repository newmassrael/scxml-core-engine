# Third-Party Licenses

This document lists all third-party libraries used by SCE (SCXML Core Engine) and their respective licenses.

## Summary

The engine's own dependencies are MIT licensed. The **browser visualizer**
ships two further libraries that are not, and they are listed here with the
rest: d3 (ISC, permissive) and elkjs (**EPL-2.0**, a weak copyleft).

⚠ This summary used to read "All dependencies are MIT licensed", and that
was not true when it was written: d3 has been vendored under ISC since the
visualizer existed. The claim held only because this document counted the
C++ engine's dependencies and nothing counted the web assets — see
[Scope](#scope) below.

⚠⚠ **EPL-2.0 section 4 obliges a commercial distributor**, and SCE is
offered commercially. See the elkjs entry.

## Dependencies

Entries 1-6 are the engine's, all MIT. Entries 7-8 are the visualizer's, and
are not MIT.

### 1. QuickJS

- **Purpose:** JavaScript engine for ECMAScript expression evaluation (W3C SCXML 5.3)
- **License:** MIT License
- **Copyright:** 2017-2021 Fabrice Bellard, Charlie Gordon
- **Website:** https://bellard.org/quickjs/
- **Used in:** Static Hybrid AOT tests, Interpreter engine

**License Text:**
```
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction...
```

---

### 2. spdlog

- **Purpose:** Fast C++ logging library
- **License:** MIT License
- **Copyright:** 2016 Gabi Melman
- **Website:** https://github.com/gabime/spdlog
- **Used in:** All components (Debug/Info/Error logging)

**License Text:**
```
Copyright (c) 2016 Gabi Melman.

Permission is hereby granted, free of charge, to any person obtaining a copy...
```

---

### 3. cpp-httplib

- **Purpose:** HTTP client/server library for BasicHTTP Event I/O Processor (W3C SCXML C.2)
- **License:** MIT License
- **Copyright:** 2024 Yuji Hirose
- **Website:** https://github.com/yhirose/cpp-httplib
- **Used in:** W3C HTTP tests (native platform only, not WebAssembly)

**License Text:**
```
Copyright (c) 2024 Yuji Hirose.

Permission is hereby granted, free of charge, to any person obtaining a copy...
```

---

### 4. pugixml

- **Purpose:** Lightweight XML parser for SCXML files
- **License:** MIT License
- **Copyright:** 2006-2023 Arseny Kapoulkine
- **Website:** https://pugixml.org/
- **Used in:** SCXML parsing (all platforms - native and WebAssembly)

**License Text:**
```
Copyright (C) 2006-2023, by Arseny Kapoulkine (arseny.kapoulkine@gmail.com)

Permission is hereby granted, free of charge, to any person obtaining a copy...
```

**Note:** SCE previously used libxml++ (LGPL 2.1) on native platforms, but has migrated to pugixml for all platforms to eliminate LGPL dependencies entirely.

---

### 5. Lua 5.4

- **Purpose:** Lua scripting engine for ECMAScript-compatible expression evaluation (W3C SCXML 5.3)
- **License:** MIT License
- **Copyright:** 1994-2024 Lua.org, PUC-Rio
- **Website:** https://www.lua.org/
- **Used in:** Static Hybrid AOT tests, Interpreter engine (alternative to QuickJS)

**License Text:**
```
Copyright (C) 1994-2024 Lua.org, PUC-Rio.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction...
```

---

### 6. nlohmann/json

- **Purpose:** JSON for Modern C++ (event data, HTTP payloads)
- **License:** MIT License
- **Copyright:** 2013-2022 Niels Lohmann
- **Website:** https://github.com/nlohmann/json
- **Used in:** Event data serialization, HTTP content

**License Text:**
```
Copyright (c) 2013-2022 Niels Lohmann

Permission is hereby granted, free of charge, to any person obtaining a copy...
```

---

### 7. d3

- **Purpose:** SVG rendering and interaction for the browser visualizer
- **License:** ISC License (permissive, MIT-equivalent in effect)
- **Copyright:** 2010-2023 Mike Bostock
- **Website:** https://d3js.org/
- **Used in:** `web/visualizer/vendor/d3/d3.v7.min.js` — the visualizer only;
  the engine and its language runtimes do not link it
- **Version:** 7.9.0
- **Full text and provenance:** `web/visualizer/vendor/d3/LICENSE` and
  `web/visualizer/vendor/d3/README.md` (version, SHA-256, upstream source)

---

### 8. elkjs (Eclipse Layout Kernel)

- **Purpose:** Every diagram's geometry in the browser visualizer — node
  placement, container bounds, edge routing, transition-label placement
- **License:** **EPL-2.0** (Eclipse Public License 2.0) — **not MIT**
- **Copyright:** 2017, 2021 Kiel University and others; the bundle also
  carries `Copyright 2020 Google LLC` from a bundled dependency
- **Website:** https://github.com/kieler/elkjs
- **Used in:** `web/visualizer/vendor/elkjs/elk.bundled.js` — the visualizer
  only; the engine and its language runtimes do not link it
- **Version:** 0.9.3
- **Full text and provenance:** `web/visualizer/vendor/elkjs/LICENSE` and
  `web/visualizer/vendor/elkjs/README.md` (version, SHA-256, upstream source)

**What EPL-2.0 requires of SCE.** EPL-2.0 is a *file-level* weak copyleft:
it attaches to `elk.bundled.js` and does not reach SCE's own code, unlike
the GPL. Redistributing it obliges two things:

- **Section 3** — the file is object code (ELK's Java, compiled by GWT and
  minified), so Source Code must be available. It is, upstream, at the link
  above; the vendored copy is byte-for-byte unmodified, with its SHA-256
  recorded, so upstream *is* its source. Notices "may not be removed or
  altered" and none have been.
- **Section 4 (Commercial Distribution)** — a contributor who "includes the
  Program in a commercial product offering" agrees to "defend and indemnify
  every other Contributor", and under EPL-2.0 a distributor is a
  contributor. Claims "relating to any actual or alleged intellectual
  property infringement" are excluded, which bounds it.

⚠ **Section 4 is an open item, not a settled one.** It attaches when SCE is
sold with the visualizer in it, not when the file is committed. Whoever owns
the commercial licence should read section 4 before that happens. This
document records the obligation; it does not discharge it.

⚠ Vendoring did not create this. The visualizer previously loaded the same
file from `unpkg.com` at runtime, so it already executed in every viewer's
browser — and a commercial deployment cannot depend on a public CDN, so the
copy reaches a customer either way. What vendoring changed is that it now
travels with its licence and its notices.

**Version note:** elkjs 0.9.3 through 0.11.0 declare `EPL-2.0`; 0.12.0
declares `EPL-2.0 OR GPL-3.0-or-later`. SCE takes EPL-2.0. An upgrade past
0.11.0 must restate that choice here.

---

## Scope

⚠ **What checks this document, and what does not.**
`scripts/gates/license-ssot.sh` (mirrored by `license-verify.yml`) guards
`sce/sce_licenses.cmake` against the `third_party/` tree — the C++
distribution. **It does not look at `web/`.** That is why d3 sat vendored
and unlisted, and why this document could say "all dependencies are MIT"
without anything contradicting it.

`scripts/gates/web-vendor-licenses.sh` now covers that gap: every directory
under `web/visualizer/vendor/` must have an entry here and a `LICENSE` file
beside it.

---

## Dependency Resolution by Platform

### All Platforms (Linux, macOS, Windows, WebAssembly)

| Dependency | License | Usage |
|-----------|---------|-------|
| QuickJS | MIT | ECMAScript expressions |
| Lua 5.4 | MIT | ECMAScript-compatible expressions (alternative to QuickJS) |
| spdlog | MIT | Logging |
| pugixml | MIT | SCXML parsing |
| nlohmann/json | MIT | JSON serialization |

### Native Platforms Only (Linux, macOS, Windows)

| Dependency | License | Usage |
|-----------|---------|-------|
| cpp-httplib | MIT | HTTP I/O Processor |

### Browser Visualizer Only

Neither library is linked by the engine or by any generated code. They ship
with `web/visualizer/` and run in the viewer's browser.

| Dependency | License | Usage |
|-----------|---------|-------|
| d3 | ISC | SVG rendering and interaction |
| elkjs | **EPL-2.0** | Diagram layout (see entry 8 — section 4 applies commercially) |

**WebAssembly note:** cpp-httplib is excluded from WebAssembly builds (HTTP support uses browser Fetch API instead).

---

## How to Verify Dependency Licenses

### Check Installed Versions

```bash
# spdlog
pkg-config --modversion spdlog

# pugixml (header-only, version in source)
grep "PUGIXML_VERSION" third_party/pugixml/src/pugixml.hpp

# cpp-httplib (header-only, no version check)
grep "CPPHTTPLIB_VERSION" /usr/include/httplib.h
```

---

## License Compatibility Matrix

### The engine: SCE Dual License (LGPL-2.1/Commercial) + MIT Dependencies

⚠ This matrix covers the six ENGINE dependencies only. It says nothing about
the visualizer's two, which is the next table.

| Your License | QuickJS | Lua 5.4 | spdlog | cpp-httplib | pugixml | nlohmann/json |
|-------------|---------|---------|--------|-------------|---------|---------------|
| **MIT (Open Source)** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Apache 2.0** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **GPL v2/v3** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **BSD** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Commercial (Proprietary)** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

**Legend:**
- ✅ Fully compatible (all dependencies are MIT)

**Key Benefit:** Every ENGINE dependency is MIT licensed, providing maximum
flexibility for both open source and commercial use. No LGPL or GPL
dependencies means no dynamic linking requirements or source disclosure
obligations for the engine's dependencies.

### The visualizer: d3 (ISC) + elkjs (EPL-2.0)

Shipped as `web/visualizer/`, executed in a browser. Nothing in the engine
or in generated code links either one, so a product that embeds SCE without
the visualizer takes on neither row.

| Your License | d3 (ISC) | elkjs (EPL-2.0) |
|-------------|----------|-----------------|
| **MIT (Open Source)** | ✅ | ✅ keep notices, name the source |
| **Apache 2.0** | ✅ | ✅ keep notices, name the source |
| **GPL v2/v3** | ✅ | ⚠ EPL-2.0 and GPLv2 are not generally compatible; elkjs 0.12.0+ offers a GPL-3.0-or-later alternative for exactly this |
| **BSD** | ✅ | ✅ keep notices, name the source |
| **Commercial (Proprietary)** | ✅ | ⚠ **section 4 applies** — defend and indemnify other contributors, IP-infringement claims excluded |

**Legend:**
- ✅ Compatible
- ⚠ Compatible with an obligation that has to be met, named in the cell

⚠ EPL-2.0 does not make SCE's own code EPL. It is a file-level weak
copyleft: it attaches to `elk.bundled.js` and stops there.

---

## Attribution Requirements

### For Binary Distributions

Include this file (LICENSE-THIRD-PARTY.md) or equivalent attribution in:
- Product documentation
- "About" dialog or credits screen
- README or NOTICE file

### Minimum Attribution Text

```
SCE uses the following third-party libraries:
- QuickJS (MIT) - Copyright Fabrice Bellard, Charlie Gordon
- Lua 5.4 (MIT) - Copyright Lua.org, PUC-Rio
- spdlog (MIT) - Copyright Gabi Melman
- cpp-httplib (MIT) - Copyright Yuji Hirose
- pugixml (MIT) - Copyright Arseny Kapoulkine
- nlohmann/json (MIT) - Copyright Niels Lohmann
```

---

## Obtaining Source Code

All dependencies are open source. Sources available at:

- **QuickJS:** https://bellard.org/quickjs/ (or third_party/quickjs)
- **Lua 5.4:** https://www.lua.org/ (or third_party/lua)
- **spdlog:** https://github.com/gabime/spdlog (or third_party/spdlog)
- **cpp-httplib:** https://github.com/yhirose/cpp-httplib (or third_party/cpp-httplib)
- **pugixml:** https://pugixml.org/ (or third_party/pugixml)
- **nlohmann/json:** https://github.com/nlohmann/json (or third_party/nlohmann_json)

---

## Contact for License Compliance Questions

For questions about third-party license compliance:

**Email:** newmassrael@gmail.com
**Subject:** Third-Party License Inquiry

We provide compliance assistance as part of our Commercial License support.

---

**Last Updated:** March 31, 2026
**Verified By:** SCE Development Team
