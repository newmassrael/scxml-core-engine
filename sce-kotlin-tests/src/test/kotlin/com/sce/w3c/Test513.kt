// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test513.scxml:1
package com.sce.w3c

import com.sce.generated.test513.Test513Event
import com.sce.generated.test513.Test513State
import com.sce.generated.test513.Test513StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: After it adds the received message to the appropriate event queue, the SCXML Processor MUST then indicate the result to the external component via a success response code 2XX. (Automated validation: HTTP event received successfully validates 200 OK response)
@DisplayName("Test 513 -- W3C SCXML C.2")
class Test513 : W3CHttpTestBase<Test513State, Test513Event>() {
    override fun createStateMachine() = Test513StateMachine(createEngine())
    override val expectedPassState: Test513State = Test513State.Pass
}
