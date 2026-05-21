// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
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
