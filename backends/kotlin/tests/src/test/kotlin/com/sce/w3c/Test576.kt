// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
package com.sce.w3c

import com.sce.generated.test576.Test576Event
import com.sce.generated.test576.Test576State
import com.sce.generated.test576.Test576StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, the SCXML Processor MUST enter the states specified by the 'initial' attribute, if it is present.
@DisplayName("Test 576 -- W3C SCXML 3.2")
class Test576 : W3CTestBase<Test576State, Test576Event>() {
    override fun createStateMachine() = Test576StateMachine()
    override val expectedPassState: Test576State = Test576State.Pass
}
