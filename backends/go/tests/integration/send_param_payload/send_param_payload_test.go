// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — Go AOT.
//
// Two paths that were fixed at the template layer with no runtime witness,
// because no committed fixture had a machine of the required shape. The
// suites could only show that nothing regressed; that same absence is why
// the defects survived as long as they did.
//
//   engine-less child -> parent   param emission used to be gated on the
//     *machine* needing a script engine rather than on the send needing
//     one, so a `datamodel="null"` child shipped its `<send>` with the
//     params dropped.
//
//   #_internal                    the internal raise took no event data, so
//     params were built and then discarded.
//
// The two reach distinct final states, so a failure names the path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the Rust / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_send_param_payload_go.sh

package send_param_payload

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestSendParamsReachEventDataFromChildAndInternalQueue(t *testing.T) {
	policy := NewSendParamPayloadPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[SendParamPayloadState, SendParamPayloadEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("send_param_payload timed out before reaching a final state — the " +
			"parent never saw `fromChild`, never saw its own `loopback`, or discarded " +
			"a whole `<send>` because one `<param>` would not evaluate (W3C SCXML " +
			"5.7.1 drops the pair, not the message)")
	}

	switch got := engine.GetCurrentState(); got {
	case SendParamPayloadStatePass:
	case SendParamPayloadStateFailChildPayload:
		t.Fatalf("`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` " +
			"child needs no script engine, but its `<send>` still has to carry the " +
			"params it declares. The gate is whether this send folds to literals, " +
			"not whether the machine needs an engine.")
	case SendParamPayloadStateFailInternalPayload:
		t.Fatalf("`loopback` arrived without `_event.data.carried`: a " +
			"`<send target=\"#_internal\">` must raise its params as event data, not " +
			"build them and drop them at the internal-raise boundary.")
	case SendParamPayloadStateFailNumberType:
		t.Fatalf("`typed` arrived with `_event.data.n` unequal to 7: `expr=\"7\"` is the " +
			"Number 7, and a backend that stringifies on the way through delivers " +
			"\"7\", which `===` finds unequal.")
	case SendParamPayloadStateFailStringType:
		t.Fatalf("`typed` arrived with `_event.data.s` unequal to 'kept': a param that " +
			"has to be EVALUATED reaches the runtime serialiser, whose string arm " +
			"must emit the value rather than an engine spelling of it.")
	case SendParamPayloadStateFailDuplicateParams:
		t.Fatalf("`typed` did not carry both values of the repeated name `d` with their " +
			"types: W3C SCXML 6.2 lets a name repeat and every value must be delivered.")
	case SendParamPayloadStateFailNoParamError:
		t.Fatalf("`withBadParam` arrived with no `error.execution` before it: W3C SCXML " +
			"5.7.1 puts that error on the internal queue while the `<send>` is being " +
			"evaluated, so it is dequeued first.")
	case SendParamPayloadStateFailBrokenParamDelivered:
		t.Fatalf("`_event.data.broken` arrived as the empty string: 5.7.1 says ignore " +
			"the name AND the value, so a receiver must find no field at all rather " +
			"than a placeholder under the name.")
	case SendParamPayloadStateFailSiblingParamLost:
		t.Fatalf("`_event.data.kept` did not survive alongside the failed param: one " +
			"`<param>` that will not evaluate costs its own pair and nothing else.")
	default:
		t.Fatalf("send_param_payload settled in %v, which is not a verdict state", got)
	}
}
