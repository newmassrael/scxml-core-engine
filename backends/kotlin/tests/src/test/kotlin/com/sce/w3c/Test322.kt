// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
package com.sce.w3c

import com.sce.generated.test322.Test322Event
import com.sce.generated.test322.Test322State
import com.sce.generated.test322.Test322StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The Processor MUST keep the _sessionid variable bound to the system-generated id until the session terminates.
@DisplayName("Test 322 -- W3C SCXML 5.10")
class Test322 : W3CTestBase<Test322State, Test322Event>() {
    override fun createStateMachine() = Test322StateMachine(createEngine())
    override val expectedPassState: Test322State = Test322State.Pass
}
