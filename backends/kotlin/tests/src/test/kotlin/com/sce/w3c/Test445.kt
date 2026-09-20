// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7b98e70bedf81ba26d5ed411dff10dd848488064e2842a1bb6ffe1d783a88c2a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test445.scxml:1
package com.sce.w3c

import com.sce.generated.test445.Test445Event
import com.sce.generated.test445.Test445State
import com.sce.generated.test445.Test445StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript datamodel for each data element in the document, if the variable object associated with the element is not assigned at the time indicated by the 'binding' attribute on the scxml element, then the SCXML Processor must assign the variable the default value ECMAScript undefined.
@DisplayName("Test 445 -- W3C SCXML B.2")
class Test445 : W3CTestBase<Test445State, Test445Event>() {
    override fun createStateMachine() = Test445StateMachine(createEngine())
    override val expectedPassState: Test445State = Test445State.Pass
}
