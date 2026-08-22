// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 35c8283af859855fefb53b36dbcc38c1c549511d8a5bf7a3250f4215fef24b75
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test244.scxml:1
package com.sce.w3c

import com.sce.generated.test244.Test244Event
import com.sce.generated.test244.Test244State
import com.sce.generated.test244.Test244StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked process is of type http://www.w3.org/TR/scxml/ and the key of namelist item in the invoke matches the 'id' of a data element in the top-level data declarations of the invoked session, the SCXML Processor MUST use the corresponding value as the initial value of the corresponding data element.
@DisplayName("Test 244 -- W3C SCXML 6.4")
class Test244 : W3CTestBase<Test244State, Test244Event>() {
    override fun createStateMachine() = Test244StateMachine(createEngine())
    override val expectedPassState: Test244State = Test244State.Pass
}
