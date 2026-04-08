// GENERATED -- DO NOT EDIT (sce-codegen)
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
