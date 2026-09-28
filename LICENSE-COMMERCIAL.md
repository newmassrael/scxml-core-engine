# SCE Commercial License (AGPL-3.0 Alternative)

## Overview

This Commercial License provides an alternative to AGPL-3.0 (with the
SCE Linking Exception) for the SCE (SCXML Core Engine) runtimes and code
generator. It is required when you need to **modify the engine or the
generator privately** or **build a competing product / SDK** on SCE
without releasing it under AGPL-3.0.

**Licensor:** newmassrael  
**License Model:** Negotiated with the licensor  
**License Version:** 5.0 (with AGPL-3.0 + Linking Exception baseline)

---

## When Do You Need This Commercial License?

### ✅ You DON'T Need Commercial License If:

**Using the engine unmodified in your application (any linking mode):**
- Link with the SCE engine without modifications
- Static OR dynamic linking — no obligation to disclose your source or
  the engine's (Linking Exception, section 2 — see
  [LICENSE-EXCEPTION.md](LICENSE-EXCEPTION.md))
- Use in commercial products (closed source OK)
- **License: AGPL-3.0 + Linking Exception (FREE)**

**Using generated code in your application:**
- Code produced by an unmodified `sce-codegen`
- Closed source OK (Linking Exception, section 3)
- **License: AGPL-3.0 + Linking Exception (FREE)**

**Modifying the engine + publishing the modifications:**
- Modify Helper files, StaticExecutionEngine or the generator
- Release the modified engine, and every work based on it, under
  AGPL-3.0, including to users who reach it over a network
- Note: the Linking Exception does NOT apply to modified versions
- **License: AGPL-3.0 (FREE)**

### 💰 You NEED Commercial License If:

**Modifying the engine or the generator + keeping changes private:**
- Modify SendHelper, GuardHelper, StaticExecutionEngine, templates, etc.
- Want to keep modifications proprietary (no source disclosure), in a
  distributed product or in a hosted service

**Creating competing products / SDKs without AGPL-3.0:**
- Build a commercial SDK, library, framework, runtime, development tool
  or hosted service that offers state-machine functionality to others
- Redistribute SCE as a standalone component or library wrapper
  (rebranded or competing state machine product)

---

## Pricing

Pricing and terms are agreed directly with the licensor, for individual
developers and for companies alike. A sponsorship alone does not grant a
Commercial License; the licence is what the licensor confirms in writing.

**Contact:** newmassrael@gmail.com  
**GitHub:** https://github.com/newmassrael

---

## Key Benefits vs AGPL-3.0 + Exception

| Aspect | AGPL-3.0 + Exception (Free) | Commercial (Contact) |
|--------|------------------------------|------------------------------|
| Use unmodified engine | ✅ Free | ✅ Included |
| **Static linking (proprietary app)** | ✅ **Free via Exception** | ✅ Included |
| **Dynamic linking (proprietary app)** | ✅ **Free via Exception** | ✅ Included |
| Generated code in proprietary app | ✅ **Free via Exception** | ✅ Included |
| Modify engine source | ✅ Free (publish under AGPL-3.0) | ✅ Included |
| **Keep modifications private** | ❌ Must publish, including to network users | ✅ **Allowed** |
| **Competing SDK / rebranded product** | ⚠ Only if the whole product is AGPL-3.0 | ✅ **Allowed** |
| Support | Community | **Priority email** |
