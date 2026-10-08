// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// §scxml-C-2: the `<param>`s of a BasicHTTP `<send>` of a `datamodel="sce-static"`
// machine are read from its own fields when the send runs, and cross as the text
// a form carries (docs/adr/0005, decision 4) — Go path. The request is observed
// where the engine hands it to its transport, so no listener is involved.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_static_datamodel_go.sh

package static_datamodel

import (
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_send_http"
)

// httpMachine is a machine whose transport keeps what it was handed.
func httpMachine(t *testing.T) (*static_send_http.StaticSendHttpPolicy, func(name string), *[]sce.HttpSendRequest) {
	t.Helper()
	policy := static_send_http.NewStaticSendHttpPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_send_http.StaticSendHttpState, static_send_http.StaticSendHttpEvent](&policy)
	var posted []sce.HttpSendRequest
	engine.SetHTTPSendCallback(func(request sce.HttpSendRequest) *sce.HttpSendResponse {
		posted = append(posted, request)
		return nil
	})
	engine.Initialize()
	raise := func(name string) {
		engine.RaiseExternalByName(name, "")
		engine.Step()
	}
	return &policy, raise, &posted
}

func httpPairs(list ...[2]string) map[string][]string {
	pairs := map[string][]string{}
	for _, pair := range list {
		pairs[pair[0]] = []string{pair[1]}
	}
	return pairs
}

func TestASendOverHTTPCarriesTheTextTheFieldsHoldWhenItRuns(t *testing.T) {
	_, raise, posted := httpMachine(t)
	raise("bump")
	raise("go")

	if len(*posted) != 1 {
		t.Fatalf("%d requests were handed to the transport, want one", len(*posted))
	}
	request := (*posted)[0]
	if request.Target != "http://example.invalid/hook" || request.EventName != "note" {
		t.Errorf("the request names target %q event %q", request.Target, request.EventName)
	}
	if request.Content != "" {
		t.Errorf("no <content>, so the body is the pairs: content = %q", request.Content)
	}
	want := httpPairs(
		[2]string{"count", "4"},
		[2]string{"ready", "true"},
		[2]string{"label", "busy"},
		[2]string{"twice", "8"},
		[2]string{"delta", "-5"},
		[2]string{"ratio", "1.5"},
	)
	if !reflect.DeepEqual(request.Params, want) {
		t.Errorf("params = %v, want %v: each value is the text it spells", request.Params, want)
	}
}

func TestThePairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp(t *testing.T) {
	_, raise, posted := httpMachine(t)
	raise("go")

	if len(*posted) != 1 {
		t.Fatalf("%d requests were handed to the transport, want one", len(*posted))
	}
	want := httpPairs(
		[2]string{"count", "3"},
		[2]string{"ready", "false"},
		[2]string{"label", "idle"},
		[2]string{"twice", "6"},
		[2]string{"delta", "-5"},
		[2]string{"ratio", "1.5"},
	)
	if got := (*posted)[0].Params; !reflect.DeepEqual(got, want) {
		t.Errorf("params = %v, want %v: without `bump` the fields hold their initial values, "+
			"and the request is read when the send runs", got, want)
	}
}

func TestAParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes(t *testing.T) {
	policy, raise, posted := httpMachine(t)
	raise("bump")
	raise("boom")

	if len(*posted) != 1 {
		t.Fatalf("%d requests were handed to the transport, want one: the request goes with the pair that could be read", len(*posted))
	}
	want := httpPairs([2]string{"count", "4"})
	if got := (*posted)[0].Params; !reflect.DeepEqual(got, want) {
		t.Errorf("params = %v, want %v: `big` is `count * 2000000000`, which a 32-bit field "+
			"cannot hold, so its pair is left out and not carried as a zero", got, want)
	}
	if got := policy.Errors(); got != 1 {
		t.Errorf("errors = %d, want 1: §scxml-5.7.1, the failed pair is reported as error.execution once", got)
	}
}
