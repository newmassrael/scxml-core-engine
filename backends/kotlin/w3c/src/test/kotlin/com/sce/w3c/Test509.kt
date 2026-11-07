// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
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
