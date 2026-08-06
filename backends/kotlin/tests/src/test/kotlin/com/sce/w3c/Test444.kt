// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7cd07f2c974b616900d2b201907d23253ba7d2b7e90840149b8c3f98eea7706a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test444.scxml:1
package com.sce.w3c

import com.sce.generated.test444.Test444Event
import com.sce.generated.test444.Test444State
import com.sce.generated.test444.Test444StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel, for each data element in the document, the SCXML Processor must create an ECMAScript variable object whose name is the value of the id attribute of the data element.
@DisplayName("Test 444 -- W3C SCXML B.2")
class Test444 : W3CTestBase<Test444State, Test444Event>() {
    override fun createStateMachine() = Test444StateMachine(createEngine())
    override val expectedPassState: Test444State = Test444State.Pass
}
