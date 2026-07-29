// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367096
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test534.scxml:1
package com.sce.w3c

import com.sce.generated.test534.Test534Event
import com.sce.generated.test534.Test534State
import com.sce.generated.test534.Test534StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If the 'event' parameter of send is defined, the SCXML Processor MUST use its value as the value of the HTTP POST parameter _scxmleventname
@DisplayName("Test 534 -- W3C SCXML C.2")
class Test534 : W3CHttpTestBase<Test534State, Test534Event>() {
    override fun createStateMachine() = Test534StateMachine(createEngine())
    override val expectedPassState: Test534State = Test534State.Pass
}
