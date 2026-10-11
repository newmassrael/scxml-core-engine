// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test510.scxml:1
package com.sce.w3c

import com.sce.generated.test510.Test510Event
import com.sce.generated.test510.Test510State
import com.sce.generated.test510.Test510StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: The SCXML Processor MUST validate the message it receives [via the Basic HTTP Event I/O Processor] and then MUST build the appropriate SCXML event and MUST add it to the external event queue
@DisplayName("Test 510 -- W3C SCXML C.2")
class Test510 : W3CHttpTestBase<Test510State, Test510Event>() {
    override fun createStateMachine() = Test510StateMachine(createEngine())
    override val expectedPassState: Test510State = Test510State.Pass
}
