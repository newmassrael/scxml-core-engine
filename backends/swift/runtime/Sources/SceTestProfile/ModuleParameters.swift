// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8; ETSI ES 201 873-1 §8.2):
// the value a suite gives each, which every component that declares a module parameter of that id
// reads into its variable before its machine starts.
//
// A value is held as the one JSON value model a saved state is made of (`JSONValue`). The generated
// suite writes each default through `SavedValues.of` of its declared type, and the generated component
// reads it back through the `SavedValues` reader of the type it declares, which are the reads a
// restored variable is held by: a value of another type, or past its bound, is refused by name rather
// than converted. The runtime knows the type of no module parameter, so it is written once for every
// suite.
//
// Cross-language siblings: `sce_rust_runtime::test::ModuleParameters` (Rust),
// `SCE::Test::ModuleParameters` (C++), `com.sce.runtime.test.ModuleParameters` (Kotlin),
// `testprofile.ModuleParameters` (Go), `sce_runtime.testprofile.ModuleParameters` (Python),
// `sce_test_module_parameters_t` (C11).

import SceRuntime

/// A module parameter's value: the saved form of a value of its declared type.
public typealias ModuleParameterValue = JSONValue

/// The module parameters a run hands its components, by id.
///
/// Generated from a `test-suite` with each parameter at its default, and given other values by the
/// caller of a run where it overrides one (-1 §8.2.1). A value, so the set a run is given is its own:
/// a later change to the caller's leaves the run as it was built, which the other runtimes copy the
/// set for.
public struct ModuleParameters: Equatable, Sendable {
    private var byID: [String: ModuleParameterValue] = [:]

    public init() {}

    /// Gives the module parameter `id` the value `value`, in place of any it had.
    public mutating func set(_ id: String, _ value: ModuleParameterValue) {
        byID[id] = value
    }

    /// Whether the module parameter `id` has a value; `null` is a value.
    public func has(_ id: String) -> Bool {
        byID[id] != nil
    }

    /// The value of the module parameter `id`.
    ///
    /// - Throws: `StateRefusal` naming it, when it has none, since no value is made up in its place.
    public func of(_ id: String) throws -> ModuleParameterValue {
        guard let value = byID[id] else {
            throw StateRefusal("no value is given for the module parameter '\(id)'")
        }
        return value
    }

    /// Every module parameter that has a value, with its value, in ascending order of id.
    public var values: [(id: String, value: ModuleParameterValue)] {
        byID.sorted { $0.key < $1.key }.map { (id: $0.key, value: $0.value) }
    }
}
