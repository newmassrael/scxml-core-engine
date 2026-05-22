// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
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
