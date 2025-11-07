# Third-Party Licenses

This document lists all third-party libraries used by RSM (Reactive State Machine) and their respective licenses.

## Summary

**All dependencies are MIT licensed** - fully compatible with both open source and commercial use, with no LGPL or GPL dependencies.

## Dependencies

All libraries listed below are MIT licensed and compatible with open source and commercial use:

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

**Note:** RSM previously used libxml++ (LGPL 2.1) on native platforms, but has migrated to pugixml for all platforms to eliminate LGPL dependencies entirely.

---

### 5. nlohmann/json

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

## Dependency Resolution by Platform

### All Platforms (Linux, macOS, Windows, WebAssembly)

| Dependency | License | Usage |
|-----------|---------|-------|
| QuickJS | MIT | ECMAScript expressions |
| spdlog | MIT | Logging |
| pugixml | MIT | SCXML parsing |
| nlohmann/json | MIT | JSON serialization |

### Native Platforms Only (Linux, macOS, Windows)

| Dependency | License | Usage |
|-----------|---------|-------|
| cpp-httplib | MIT | HTTP I/O Processor |

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

### RSM Dual License (LGPL-2.1/Commercial) + MIT Dependencies

| Your License | QuickJS | spdlog | cpp-httplib | pugixml | nlohmann/json |
|-------------|---------|--------|-------------|---------|---------------|
| **MIT (Open Source)** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Apache 2.0** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **GPL v2/v3** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **BSD** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Commercial (Proprietary)** | ✅ | ✅ | ✅ | ✅ | ✅ |

**Legend:**
- ✅ Fully compatible (all dependencies are MIT)

**Key Benefit:** All third-party dependencies are MIT licensed, providing maximum flexibility for both open source and commercial use. No LGPL or GPL dependencies means no dynamic linking requirements or source disclosure obligations for dependencies.

---

## Attribution Requirements

### For Binary Distributions

Include this file (LICENSE-THIRD-PARTY.md) or equivalent attribution in:
- Product documentation
- "About" dialog or credits screen
- README or NOTICE file

### Minimum Attribution Text

```
RSM uses the following third-party libraries:
- QuickJS (MIT) - Copyright Fabrice Bellard, Charlie Gordon
- spdlog (MIT) - Copyright Gabi Melman
- cpp-httplib (MIT) - Copyright Yuji Hirose
- pugixml (MIT) - Copyright Arseny Kapoulkine
- nlohmann/json (MIT) - Copyright Niels Lohmann
```

---

## Obtaining Source Code

All dependencies are open source. Sources available at:

- **QuickJS:** https://bellard.org/quickjs/ (or rsm/external/quickjs)
- **spdlog:** https://github.com/gabime/spdlog
- **cpp-httplib:** https://github.com/yhirose/cpp-httplib
- **pugixml:** https://pugixml.org/ (or third_party/pugixml)
- **nlohmann/json:** https://github.com/nlohmann/json

---

## Contact for License Compliance Questions

For questions about third-party license compliance:

**Email:** newmassrael@gmail.com
**Subject:** Third-Party License Inquiry

We provide compliance assistance as part of our Commercial License support.

---

**Last Updated:** January 7, 2025
**RSM Version:** 1.0
**Verified By:** RSM Development Team
