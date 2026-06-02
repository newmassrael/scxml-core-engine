// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958
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
    override fun createStateMachine() = Test509StateMachine()
    override val expectedPassState: Test509State = Test509State.Pass
}
