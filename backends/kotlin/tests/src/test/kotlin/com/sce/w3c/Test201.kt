// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
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
    override fun createStateMachine() = Test201StateMachine(createEngine())
    override val expectedPassState: Test201State = Test201State.Pass
}
