// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test253.scxml:1
package com.sce.w3c

import com.sce.generated.test253.Test253Event
import com.sce.generated.test253.Test253State
import com.sce.generated.test253.Test253StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When the invoked session is of type http://www.w3.org/TR/scxml/, The SCXML Processor MUST support the use of SCXML Event/IO processor (E.1 SCXML Event I/O Processor) to communicate between the invoking and the invoked sessions.
@DisplayName("Test 253 -- W3C SCXML 6.4")
class Test253 : W3CTestBase<Test253State, Test253Event>() {
    override fun createStateMachine() = Test253StateMachine(createEngine())
    override val expectedPassState: Test253State = Test253State.Pass
}
