// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A [`ReferenceHost`] as the invoker of every standard type of an engine.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HostInvokeEvent, HostInvokeResponse, StatePolicy};

use crate::{Outcome, Params, ReferenceHost, Transport, TYPES};

/// A host the engine's invokers and its embedding both hold.
pub type Shared<T> = Arc<Mutex<ReferenceHost<T>>>;

/// Register `host` as the invoker of every standard type on `engine`.
///
/// A start is answered with its completion (a failure is a completion whose
/// `error` is a code) or its refusal, or not at all when the host holds it. The
/// cancel the engine delivers when a state exits or a deadline passes reaches
/// the host too.
pub fn serve<P: StatePolicy, T: Transport + Send + 'static>(
    engine: &mut Engine<P>,
    host: &Shared<T>,
) {
    for operation in TYPES {
        let host = Arc::clone(host);
        engine.register_invoker(operation, move |event| match event {
            HostInvokeEvent::Start(request) => {
                let params: Params = request
                    .params
                    .iter()
                    .filter_map(|(name, values)| Some((name.clone(), values.first()?.clone())))
                    .collect();
                let outcome = host.lock().expect("the host is not left mid-update").start(
                    operation,
                    &request.invoke_id,
                    &params,
                );
                match outcome? {
                    Outcome::Done(data) => Some(HostInvokeResponse {
                        done_data: Some(data),
                        refusal: None,
                    }),
                    Outcome::Refuse(reason) => Some(HostInvokeResponse {
                        done_data: None,
                        refusal: Some(reason.to_string()),
                    }),
                    Outcome::Hold => None,
                }
            }
            HostInvokeEvent::Cancel(cancel) => {
                host.lock()
                    .expect("the host is not left mid-update")
                    .cancel(&cancel.invoke_id);
                None
            }
        });
    }
}
