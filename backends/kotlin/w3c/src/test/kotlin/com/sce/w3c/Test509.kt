// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test509.scxml:1
package com.sce.w3c

import com.sce.generated.test509.Test509Event
import com.sce.generated.test509.Test509State
import com.sce.generated.test509.Test509StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: An SCXML Processor that supports the Basic HTTP Event I/O Processor MUST accept messages at the access URI as HTTP POST requests
@DisplayName("Test 509 -- W3C SCXML C.2")
class Test509 : W3CHttpTestBase<Test509State, Test509Event>() {
    override fun createStateMachine() = Test509StateMachine(createEngine())
    override val expectedPassState: Test509State = Test509State.Pass
}
