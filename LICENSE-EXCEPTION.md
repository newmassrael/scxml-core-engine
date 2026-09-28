# SCE Linking Exception

**Version 2.0** — Copyright (c) 2025-2026 newmassrael

As additional permissions under section 7 of the GNU Affero General
Public License, version 3 ("AGPL-3.0"), the copyright holder of the SCXML
Core Engine grants the permissions below. They apply to every file that
carries the SPDX expression given at the end of this document.

Version 1.0 of this Exception supplemented the GNU Lesser General Public
License, version 2.1. Copies of SCE obtained under that version keep the
terms they were obtained under; this version governs releases that carry
the AGPL-3.0 expression.

---

## 1. DEFINITIONS

- **"Library"** means the SCXML Core Engine: its runtimes for every
  target language, its code generator (`sce-codegen`) and the generator's
  templates, and its authoring tools (`tools/authoring`, including its
  MCP server), as published by the copyright holder.

- **"Tool"** means `sce-codegen` or one of the Library's authoring tools.

- **"Unmodified"** means identical to a version of the Library published
  by the copyright holder. Selecting a documented build option, feature
  flag or configuration value is not a modification. Instantiating a
  template, including a header, or calling a public interface of the
  Library is not a modification.

- **"Application"** means a work that uses the Library only through its
  documented interfaces: by linking, by including its headers, by
  instantiating its templates, by importing its packages, or by
  incorporating Generated Output.

- **"Generated Output"** means what a Tool produces from an input
  document: the files `sce-codegen` writes in any target language, and
  the pseudocode, scaffolds, reports and other responses an authoring
  tool returns, whether through its command line or its MCP interface,
  and whether or not the output carries a licence header.

- **"Competing Product"** means software whose purpose, in whole or in
  substantial part, is to provide state-machine execution, SCXML
  processing, state-machine code generation or state-machine authoring to
  third parties who build their own software on it. This includes an SDK,
  a library, a framework, a runtime, a development tool, or a hosted
  service that exposes such functionality to its users. An Application
  that uses state machines internally to implement its own features is
  not a Competing Product.

---

## 2. PERMISSION TO LINK THE UNMODIFIED LIBRARY

You may combine an Application with the Unmodified Library, and convey
the combination in object code or source form, under terms of your
choice for the Application, provided that:

1. the Library is Unmodified;
2. the Application is not a Competing Product;
3. you preserve the Library's copyright notices, this Exception and a
   reference to AGPL-3.0 in the Application's documentation or "About"
   screen; and
4. you do not remove or obscure copyright notices in Library source
   files you redistribute (for example the `embed/` package).

When these conditions hold:

- the Application is not a work based on the Library by reason of that
  combination, and the terms of AGPL-3.0 do not extend to it;
- you are not required to provide the Corresponding Source of the
  Application or of the Library, because the Unmodified Library remains
  available in source form from the copyright holder; and
- users interacting with the Application over a network do not acquire,
  through AGPL-3.0 section 13, any right to the source of the Application
  or the Library.

---

## 3. PERMISSION FOR GENERATED OUTPUT

Generated Output combines material derived from your input document with
material copied from the Library's templates.

- The material derived from your input document is yours. This Exception
  claims nothing in it.

- For the material copied from the Library's templates, you may use,
  modify and convey Generated Output, and works that contain it, under
  terms of your choice, provided that:
  1. the Generated Output was produced by an Unmodified Tool, and, for
     `sce-codegen`, from Unmodified templates; and
  2. neither the Generated Output nor the work containing it is a
     Competing Product.

A modification you make to Generated Output after it is produced does
not end this permission. The permission covers the work you build with
the output; it does not cover using the output as a template library, a
runtime or a generator for others.

---

## 4. PERMISSION TO COMBINE WITH THIRD-PARTY COMPONENTS

You may combine the Library with the third-party components listed in
[LICENSE-THIRD-PARTY.md](LICENSE-THIRD-PARTY.md), and convey the
combination, with each component remaining under its own licence, even
where that licence is not compatible with AGPL-3.0. This covers in
particular elkjs (Eclipse Public License 2.0), which the browser
visualizer loads. The Library's own files stay under AGPL-3.0 and this
Exception.

---

## 5. WHAT THIS EXCEPTION DOES NOT COVER

- **A modified Library.** A modified Library, and any work based on it,
  is governed by AGPL-3.0 in full, including section 13, unless you hold
  an SCE Commercial License.
- **A Competing Product.** A Competing Product that contains the Library
  or Generated Output is governed by AGPL-3.0 in full, unless you hold an
  SCE Commercial License.
- **Output of a modified Tool.** Generated Output produced by a
  modified Tool or from modified templates is governed by
  AGPL-3.0 in full, unless you hold an SCE Commercial License.

When a condition of section 2 or 3 does not hold, that permission does
not apply to your work, and AGPL-3.0 governs it. The permissions stay
available to everyone else.

---

## 6. REMOVING THESE PERMISSIONS

As AGPL-3.0 section 7 allows, when you convey a copy of the Library, or
a modified version of it, you may remove any of the permissions above
from that copy. Only the copyright holder may add to them.

---

## 7. RELATIONSHIP TO SCE LICENCES

| Your situation | Terms that apply | Cost |
|---|---|---|
| Application using the Unmodified Library | AGPL-3.0 + this Exception, section 2 | Free |
| Using Generated Output in an Application | AGPL-3.0 + this Exception, section 3 | Free |
| Modified Library, source published under AGPL-3.0 | AGPL-3.0 | Free |
| Modified Library kept private | SCE Commercial License | Paid |
| Competing Product | AGPL-3.0 in full, or SCE Commercial License | Free / Paid |

See [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md) and
[LICENSE-GENERATED.md](LICENSE-GENERATED.md).

---

## SPDX IDENTIFIER

Files covered by AGPL-3.0 with this Exception, and offered alternatively
under the SCE Commercial License, carry:

    SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial

---

## Contact

- **Email:** newmassrael@gmail.com
- **GitHub:** https://github.com/newmassrael/scxml-core-engine
