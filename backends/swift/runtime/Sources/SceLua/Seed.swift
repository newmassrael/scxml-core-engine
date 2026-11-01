// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What a parent's `<invoke>` hands a child for one of the child's variables (§scxml-6.4.1).

/// The value of a parent's `<param>` or namelist entry, in the form it crosses to the child session.
///
/// The clause hands the child the VALUE of the parent's expression and not a rendering of it, and
/// JSON is a rendering that cannot write every value ECMAScript holds: `JSON.stringify(Infinity)` is
/// `"null"`, so a value that went by its JSON reached the child as something else and nothing said
/// so. A scalar therefore crosses as the value itself, and only what has no scalar form (an object,
/// an array) crosses as the JSON that keeps its shape.
public enum LuaSeed: Equatable, Sendable {
    /// A boolean, a number or a string, carried as it is: an integer stays an integer, and a
    /// non-finite number arrives non-finite.
    case scalar(LuaValue)
    /// Anything else: its JSON, which the child parses.
    case json(String)
}
