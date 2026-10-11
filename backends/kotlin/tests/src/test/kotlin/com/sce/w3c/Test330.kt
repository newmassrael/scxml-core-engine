// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test330.scxml:1
package com.sce.w3c

import com.sce.generated.test330.Test330Event
import com.sce.generated.test330.Test330State
import com.sce.generated.test330.Test330StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST insure that the following fields (name, type, sendid, origin, origintype, invokeid, data) are present in all events (_event variable), whether internal or external.
@DisplayName("Test 330 -- W3C SCXML 5.10")
class Test330 : W3CTestBase<Test330State, Test330Event>() {
    override fun createStateMachine() = Test330StateMachine()
    override val expectedPassState: Test330State = Test330State.Pass
}
