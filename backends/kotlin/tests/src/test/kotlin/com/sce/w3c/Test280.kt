// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test280.scxml:1
package com.sce.w3c

import com.sce.generated.test280.Test280Event
import com.sce.generated.test280.Test280State
import com.sce.generated.test280.Test280StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.3: When 'binding' attribute on the scxml element is assigned the value "late", the SCXML Processor MUST create the data elements at document initialization time, but MUST assign the specified initial value to a given data element only when the state that contains it is entered for the first time, before any onentry markup.
@DisplayName("Test 280 -- W3C SCXML 5.3")
class Test280 : W3CTestBase<Test280State, Test280Event>() {
    override fun createStateMachine() = Test280StateMachine(createEngine())
    override val expectedPassState: Test280State = Test280State.Pass
}
