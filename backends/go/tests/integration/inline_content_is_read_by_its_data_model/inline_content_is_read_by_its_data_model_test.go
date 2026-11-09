// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: the reading of inline `<content>` text is the data model's —
// Go AOT.
//
// Under `datamodel="ecmascript"` a `<send>`'s `21` is the number 21,
// `{"k": 7}` an object, and text that is no expression the space-normalized
// string, whether or not the sending machine needs a script engine for
// anything else; a `<donedata>`'s XML is a document. The child that sends them
// needs no engine.
//
// Fixture: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_inline_content_is_read_by_its_data_model_go.sh

package inline_content_is_read_by_its_data_model

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestInlineContentIsTheValueItsDataModelReads(t *testing.T) {
	policy := NewInlineContentIsReadByItsDataModelPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[InlineContentIsReadByItsDataModelState, InlineContentIsReadByItsDataModelEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("inline_content_is_read_by_its_data_model timed out before reaching a " +
			"final state: the child's events never reached the parent")
	}

	got, ended := engine.TerminalState()
	if !ended {
		t.Fatalf("inline_content_is_read_by_its_data_model did not end in a top-level "+
			"<final> (parked in %v)", engine.GetCurrentState())
	}
	switch got {
	case InlineContentIsReadByItsDataModelStatePass:
	case InlineContentIsReadByItsDataModelStateFailNumber:
		t.Fatalf("`number` arrived with `_event.data` unequal to 21: under the ECMAScript " +
			"data model `<content>21</content>` is the number 21, finished at build time, " +
			"whether or not the sending machine needs a script engine.")
	case InlineContentIsReadByItsDataModelStateFailText:
		t.Fatalf("`text` arrived with `_event.data` other than 'two words': text that is " +
			"no expression is the space-normalized string it spells.")
	case InlineContentIsReadByItsDataModelStateFailObject:
		t.Fatalf("`object` arrived without `_event.data.k === 7`: `<content>{\"k\": 7}</content>` " +
			"is an object literal, the object it denotes.")
	case InlineContentIsReadByItsDataModelStateFailDocument:
		t.Fatalf("`done.invoke.reporter` arrived without a document whose `book` carries " +
			"title=\"x\": a `<donedata>`'s XML `<content>` travels as its text and the " +
			"receiver builds the DOM; it is not an expression to evaluate.")
	default:
		t.Fatalf("inline_content_is_read_by_its_data_model ended in %v, which is not a verdict state", got)
	}
}
