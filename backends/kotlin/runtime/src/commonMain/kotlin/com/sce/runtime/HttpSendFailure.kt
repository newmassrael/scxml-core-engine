// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Thrown by a BasicHTTP transport ([StateMachineEngine.onHttpSend]) when the
 * request was not delivered: the server could not be reached, or it refused
 * the request with a status outside 2xx.
 *
 * The engine answers it as a send the processor is unable to dispatch:
 * `error.communication`, carrying the send id, on the internal queue. Any
 * other exception the transport throws is not a report of the transport and
 * propagates as it is. The Kotlin twin of `HttpSendFailure` in the Rust and
 * Python runtimes and of the error a Go transport returns.
 */
class HttpSendFailure(reason: String) : Exception(reason)
