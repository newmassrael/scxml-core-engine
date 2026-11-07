// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import (
	"sort"

	sce "github.com/newmassrael/sce-go-runtime"
)

// The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8;
// ETSI ES 201 873-1 §8.2): the value a suite gives each, which every component
// that declares a module parameter of that id reads into its variable before
// its machine starts.
//
// A value is held as the one JSON value model a saved state is made of (a
// sce.JSONNumber, a string, a bool, a []any, a *sce.JSONObject or nil). The
// generated suite writes each default through the sce.SavedWrite function of
// its declared type, and the generated component reads it back through the
// sce.SavedRead function of the type it declares, which are the reads a
// restored variable is held by: a value of another type, or past its bound, is
// refused by name rather than converted. The runtime knows the type of no
// module parameter, so it is written once for every suite.
//
// Cross-language siblings: `sce_rust_runtime::test::ModuleParameters` (Rust),
// `SCE::Test::ModuleParameters` (C++),
// `com.sce.runtime.test.ModuleParameters` (Kotlin).

// ModuleParameterValue is a module parameter's value: the saved form of a
// value of its declared type.
type ModuleParameterValue = any

// ModuleParameters are the module parameters a run hands its components, by
// id.
//
// Generated from a `test-suite` with each parameter at its default, and given
// other values by the caller of a run where it overrides one (-1 §8.2.1). The
// zero value holds no parameter, and so does a nil *ModuleParameters, which
// every method but Set reads as the empty set.
type ModuleParameters struct {
	byID map[string]ModuleParameterValue
}

// Set gives the module parameter id the value value, in place of any it had.
func (p *ModuleParameters) Set(id string, value ModuleParameterValue) {
	if p.byID == nil {
		p.byID = map[string]ModuleParameterValue{}
	}
	p.byID[id] = value
}

// Has reports whether the module parameter id has a value; nil is a value.
func (p *ModuleParameters) Has(id string) bool {
	if p == nil {
		return false
	}
	_, ok := p.byID[id]
	return ok
}

// Of is the value of the module parameter id, and a *sce.StateRefusal naming
// it when it has none, since no value is made up in its place.
func (p *ModuleParameters) Of(id string) (ModuleParameterValue, error) {
	if !p.Has(id) {
		return nil, sce.Refuse("no value is given for the module parameter '" + id + "'")
	}
	return p.byID[id], nil
}

// IDs are the ids of every module parameter that has a value, in ascending
// order.
func (p *ModuleParameters) IDs() []string {
	if p == nil {
		return nil
	}
	ids := make([]string, 0, len(p.byID))
	for id := range p.byID {
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return ids
}

// Copy is another set of module parameters with the same values, which a
// change to either leaves apart.
func (p *ModuleParameters) Copy() *ModuleParameters {
	copied := &ModuleParameters{}
	for _, id := range p.IDs() {
		copied.Set(id, p.byID[id])
	}
	return copied
}
