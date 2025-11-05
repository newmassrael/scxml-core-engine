# Third-Party Licenses

This document lists all third-party libraries used by RSM (Reactive State Machine) and their respective licenses.

## Summary

All dependencies are MIT-compatible with one exception:
- **CRITICAL:** libxml++ (LGPL 2.1) requires special handling for proprietary software

## Dependencies by License Type

### MIT License (Fully Compatible)

The following libraries are MIT licensed and compatible with both open source and commercial use:

#### 1. QuickJS

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

#### 2. spdlog

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

#### 3. cpp-httplib

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

#### 4. pugixml

- **Purpose:** Lightweight XML parser (WebAssembly build)
- **License:** MIT License
- **Copyright:** 2006-2023 Arseny Kapoulkine
- **Website:** https://pugixml.org/
- **Used in:** SCXML parsing (WebAssembly platform only)

**License Text:**
```
Copyright (C) 2006-2023, by Arseny Kapoulkine (arseny.kapoulkine@gmail.com)

Permission is hereby granted, free of charge, to any person obtaining a copy...
```

---

#### 5. nlohmann/json

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

### LGPL 2.1 (Requires Special Handling)

#### 6. libxml++ (XML Parser - Native Platform)

- **Purpose:** C++ XML parsing library (native platform only, not WebAssembly)
- **License:** GNU Lesser General Public License (LGPL) 2.1
- **Copyright:** The libxml++ development team
- **Website:** https://libxmlplusplus.github.io/libxmlplusplus/
- **Used in:** SCXML parsing (native platform only)

**CRITICAL COMPLIANCE NOTES:**

##### For Open Source Projects (MIT/GPL-compatible)
No special action required - LGPL is compatible.

##### For Commercial/Proprietary Projects

LGPL 2.1 requires:

1. **Dynamic Linking (Easiest)** ✅
   - Link libxml++ as shared library (.so/.dll)
   - RSM already does this by default (CMakeLists.txt)
   - No source disclosure required for your code
   - **Status:** Compliant as-is

2. **If Static Linking** ⚠️
   - Must allow users to relink with modified libxml++
   - Provide object files (.o) or libraries (.a)
   - Document relinking procedure
   - **Recommendation:** Use dynamic linking instead

3. **Source Availability**
   - Must provide libxml++ source (or link to official source)
   - Must document which version you used
   - **Status:** Documented here

##### Compliance Checklist for Commercial Use

- [x] Use dynamic linking (default in CMakeLists.txt)
- [x] Document libxml++ version in this file
- [x] Provide link to official libxml++ source
- [ ] Verify libxml++ .so/.dll included in distribution
- [ ] Include LGPL 2.1 license text in distribution

##### Version Information

**Current Version:** libxml++ 3.0.1 or higher (detected by CMake)
**Source:** https://github.com/libxmlplusplus/libxmlplusplus/releases

##### LGPL 2.1 License Text

Full text: https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html

**Key Terms Summary:**
- You can link LGPL library with proprietary code
- Must allow users to replace/modify LGPL library (dynamic linking satisfies this)
- Must provide LGPL library source or offer to provide
- Your proprietary code remains proprietary

---

## Dependency Resolution by Platform

### Native Platform (Linux, macOS, Windows)

| Dependency | License | Usage |
|-----------|---------|-------|
| QuickJS | MIT | ECMAScript expressions |
| spdlog | MIT | Logging |
| cpp-httplib | MIT | HTTP I/O Processor |
| **libxml++** | **LGPL 2.1** | **SCXML parsing** |
| nlohmann/json | MIT | JSON serialization |

### WebAssembly Platform

| Dependency | License | Usage |
|-----------|---------|-------|
| QuickJS | MIT | ECMAScript expressions |
| spdlog | MIT | Logging |
| pugixml | MIT | SCXML parsing (replaces libxml++) |
| nlohmann/json | MIT | JSON serialization |

**Note:** WebAssembly build avoids LGPL entirely by using pugixml instead of libxml++.

---

## How to Verify Dependency Licenses

### Check Installed Versions

```bash
# spdlog
pkg-config --modversion spdlog

# libxml++
pkg-config --modversion libxml++-3.0

# cpp-httplib (header-only, no version check)
grep "CPPHTTPLIB_VERSION" /usr/include/httplib.h
```

### Verify Dynamic Linking (LGPL Compliance)

```bash
# Check that librsm_unified.so links libxml++ dynamically
ldd build/librsm_unified.so | grep libxml

# Expected output:
# libxml++-3.0.so.1 => /usr/lib/x86_64-linux-gnu/libxml++-3.0.so.1
```

---

## License Compatibility Matrix

### RSM MIT License + Dependencies

| Your License | QuickJS | spdlog | cpp-httplib | pugixml | nlohmann/json | libxml++ |
|-------------|---------|--------|-------------|---------|---------------|----------|
| **MIT (Open Source)** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Apache 2.0** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **GPL v2/v3** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **BSD** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Commercial (Proprietary)** | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ (dynamic link only) |

**Legend:**
- ✅ Fully compatible
- ⚠️ Compatible with conditions (see libxml++ section)

---

## Alternative: LGPL-Free Build

To avoid LGPL entirely, compile for WebAssembly:

```bash
cd build_wasm
emcmake cmake ..
emmake make
```

This uses pugixml (MIT) instead of libxml++, resulting in 100% MIT dependencies.

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
- libxml++ (LGPL 2.1) - The libxml++ development team
```

---

## Obtaining Source Code

All dependencies are open source. Sources available at:

- **QuickJS:** https://bellard.org/quickjs/ (or rsm/external/quickjs)
- **spdlog:** https://github.com/gabime/spdlog
- **cpp-httplib:** https://github.com/yhirose/cpp-httplib
- **pugixml:** https://pugixml.org/
- **nlohmann/json:** https://github.com/nlohmann/json
- **libxml++:** https://github.com/libxmlplusplus/libxmlplusplus

---

## Contact for License Compliance Questions

For questions about third-party license compliance:

**Email:** licensing@rsm.dev
**Subject:** Third-Party License Inquiry

We provide compliance assistance as part of our Commercial License support.

---

**Last Updated:** January 4, 2025
**RSM Version:** 1.0
**Verified By:** RSM Development Team
