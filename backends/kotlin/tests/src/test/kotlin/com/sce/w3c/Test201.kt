// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371281
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test201.scxml:1
package com.sce.w3c

import com.sce.generated.test201.Test201Event
import com.sce.generated.test201.Test201State
import com.sce.generated.test201.Test201StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: Processors that support HTTP POST must use the value http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor for the "type" attribute
@DisplayName("Test 201 -- W3C SCXML 6.2")
class Test201 : W3CHttpTestBase<Test201State, Test201Event>() {
    override fun createStateMachine() = Test201StateMachine()
    override val expectedPassState: Test201State = Test201State.Pass
}
