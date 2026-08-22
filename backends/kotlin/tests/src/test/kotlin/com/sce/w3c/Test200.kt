// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 35c8283af859855fefb53b36dbcc38c1c549511d8a5bf7a3250f4215fef24b75
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test200.scxml:1
package com.sce.w3c

import com.sce.generated.test200.Test200Event
import com.sce.generated.test200.Test200State
import com.sce.generated.test200.Test200StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: SCXML Processors MUST support the type http://www.w3.org/TR/scxml/#SCXMLEventProcessor
@DisplayName("Test 200 -- W3C SCXML 6.2")
class Test200 : W3CTestBase<Test200State, Test200Event>() {
    override fun createStateMachine() = Test200StateMachine()
    override val expectedPassState: Test200State = Test200State.Pass
}
