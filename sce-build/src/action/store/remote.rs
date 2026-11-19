// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The fleet's action cache (ADR 0097, answer 3): a plain REAPI v2 client of
//! one instance of a site's server, with SHA-256 digests and no dependency on
//! bx. bx keeps the server (RFC 0049, section 4.11); this is the other side.
//!
//! The calls are the five a cache needs: `FindMissingBlobs`,
//! `BatchReadBlobs` and `BatchUpdateBlobs` of the `ContentAddressableStorage`
//! service, `GetActionResult` and `UpdateActionResult` of the `ActionCache`
//! service, and the `ByteStream` `Read` and `Write` for a blob too large for
//! one batch call. Each is made through `tonic::client::Grpc` with the method
//! path of its `.proto`, the messages being those of [`crate::action::reapi`].
//!
//! A workstation reads the fleet and never writes it: only the site's build
//! machines and the landing integrator write, through the worker port. So a
//! store is opened read-only unless its caller says it may write, and a write
//! to a read-only store is refused before it reaches the server.

use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::uri::PathAndQuery;
use prost::Message;
use tonic::client::Grpc;
use tonic::codec::ProstCodec;
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request, Status};

use super::ContentStore;
use crate::action::reapi::{
    ActionResult, BatchReadBlobsRequest, BatchReadBlobsResponse, BatchUpdateBlobsRequest,
    BatchUpdateBlobsResponse, ByteStreamReadRequest, ByteStreamReadResponse,
    ByteStreamWriteRequest, ByteStreamWriteResponse, Digest, FindMissingBlobsRequest,
    FindMissingBlobsResponse, GetActionResultRequest, UpdateActionResultRequest, UpdateBlob,
    DIGEST_FUNCTION_SHA256,
};

/// The variable naming the fleet's server: a URI such as
/// `http://127.0.0.1:8980` (the forward `bx-fleet site-tunnel <site>` opens)
/// or the worker port of a site.
pub const ENDPOINT_VARIABLE: &str = "SCE_FLEET_CACHE";

/// The variable naming the project's instance on that server (the site
/// registry's name for it).
pub const INSTANCE_VARIABLE: &str = "SCE_FLEET_INSTANCE";

/// The largest blob sent or read in one batch call. gRPC's default message
/// limit is 4 MiB and a batch carries its digests beside the bytes, so a
/// blob above this goes through `ByteStream`.
const BATCH_BYTES: i64 = 3 * 1024 * 1024;

/// The size of each chunk of a `ByteStream` write.
const CHUNK_BYTES: usize = 1024 * 1024;

const FIND_MISSING: &str =
    "/build.bazel.remote.execution.v2.ContentAddressableStorage/FindMissingBlobs";
const BATCH_READ: &str =
    "/build.bazel.remote.execution.v2.ContentAddressableStorage/BatchReadBlobs";
const BATCH_UPDATE: &str =
    "/build.bazel.remote.execution.v2.ContentAddressableStorage/BatchUpdateBlobs";
const GET_RESULT: &str = "/build.bazel.remote.execution.v2.ActionCache/GetActionResult";
const UPDATE_RESULT: &str = "/build.bazel.remote.execution.v2.ActionCache/UpdateActionResult";
const BYTESTREAM_READ: &str = "/google.bytestream.ByteStream/Read";
const BYTESTREAM_WRITE: &str = "/google.bytestream.ByteStream/Write";

/// An instance of the fleet's REAPI cache.
pub struct RemoteStore {
    runtime: tokio::runtime::Runtime,
    channel: Channel,
    instance: String,
    writable: bool,
}

impl RemoteStore {
    /// The instance `instance` of the server at `endpoint`, connected now.
    /// `writable` is whether this machine may write the fleet's cache.
    pub fn connect(endpoint: &str, instance: &str, writable: bool) -> io::Result<RemoteStore> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let endpoint = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| failed(format!("{endpoint} is no endpoint: {e}")))?
            .connect_timeout(Duration::from_secs(10));
        let channel = runtime
            .block_on(endpoint.connect())
            .map_err(|e| failed(format!("the fleet's cache is not reachable: {e}")))?;
        Ok(RemoteStore {
            runtime,
            channel,
            instance: instance.to_string(),
            writable,
        })
    }

    /// The store [`ENDPOINT_VARIABLE`] and [`INSTANCE_VARIABLE`] name, read
    /// only; `None` when no endpoint is named, which is a machine that does
    /// not reach the fleet.
    pub fn from_environment() -> Option<io::Result<RemoteStore>> {
        let endpoint = std::env::var(ENDPOINT_VARIABLE)
            .ok()
            .filter(|v| !v.is_empty())?;
        let instance = std::env::var(INSTANCE_VARIABLE).unwrap_or_default();
        Some(RemoteStore::connect(&endpoint, &instance, false))
    }

    fn call<Q, R>(&self, path: &'static str, request: Q) -> Result<R, Status>
    where
        Q: Message + Send + Sync + 'static,
        R: Message + Default + Send + Sync + 'static,
    {
        let mut grpc = Grpc::new(self.channel.clone());
        self.runtime.block_on(async move {
            grpc.ready()
                .await
                .map_err(|e| Status::unavailable(e.to_string()))?;
            grpc.unary(
                Request::new(request),
                PathAndQuery::from_static(path),
                ProstCodec::<Q, R>::default(),
            )
            .await
            .map(tonic::Response::into_inner)
        })
    }

    fn refuse_write(&self) -> io::Result<()> {
        if self.writable {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "this machine reads the fleet's action cache and does not write it: only the \
             site's build machines and the landing integrator write (ADR 0097, answer 3)",
        ))
    }

    fn read_streamed(&self, digest: &Digest) -> io::Result<Option<Vec<u8>>> {
        let request = ByteStreamReadRequest {
            resource_name: resource(
                &self.instance,
                &format!("blobs/{}/{}", digest.hash, digest.size_bytes),
            ),
            read_offset: 0,
            read_limit: 0,
        };
        let mut grpc = Grpc::new(self.channel.clone());
        let read: Result<Vec<u8>, Status> = self.runtime.block_on(async move {
            grpc.ready()
                .await
                .map_err(|e| Status::unavailable(e.to_string()))?;
            let mut stream = grpc
                .server_streaming(
                    Request::new(request),
                    PathAndQuery::from_static(BYTESTREAM_READ),
                    ProstCodec::<ByteStreamReadRequest, ByteStreamReadResponse>::default(),
                )
                .await?
                .into_inner();
            let mut bytes = Vec::new();
            while let Some(chunk) = stream.message().await? {
                bytes.extend(chunk.data);
            }
            Ok(bytes)
        });
        match read {
            Ok(bytes) => Ok(Some(bytes)),
            Err(status) if status.code() == Code::NotFound => Ok(None),
            Err(status) => Err(from_status(status)),
        }
    }

    fn write_streamed(&self, digest: &Digest, bytes: &[u8]) -> io::Result<()> {
        let name = resource(
            &self.instance,
            &format!(
                "uploads/{}/blobs/{}/{}",
                upload_id(),
                digest.hash,
                digest.size_bytes
            ),
        );
        let requests = write_requests(&name, bytes);
        let mut grpc = Grpc::new(self.channel.clone());
        let written: Result<ByteStreamWriteResponse, Status> = self.runtime.block_on(async move {
            grpc.ready()
                .await
                .map_err(|e| Status::unavailable(e.to_string()))?;
            grpc.client_streaming(
                Request::new(tokio_stream::iter(requests)),
                PathAndQuery::from_static(BYTESTREAM_WRITE),
                ProstCodec::<ByteStreamWriteRequest, ByteStreamWriteResponse>::default(),
            )
            .await
            .map(tonic::Response::into_inner)
        });
        let committed = written.map_err(from_status)?.committed_size;
        // A server that holds the blob already may end the write early and
        // say so with the blob's whole size; anything else is a short write.
        if committed != digest.size_bytes {
            return Err(failed(format!(
                "the fleet committed {committed} of the {} bytes of {digest}",
                digest.size_bytes
            )));
        }
        Ok(())
    }
}

impl ContentStore for RemoteStore {
    fn find_missing(&self, digests: &[Digest]) -> io::Result<Vec<Digest>> {
        if digests.is_empty() {
            return Ok(Vec::new());
        }
        let response: FindMissingBlobsResponse = self
            .call(
                FIND_MISSING,
                FindMissingBlobsRequest {
                    instance_name: self.instance.clone(),
                    blob_digests: digests.to_vec(),
                    digest_function: DIGEST_FUNCTION_SHA256,
                },
            )
            .map_err(from_status)?;
        Ok(response.missing_blob_digests)
    }

    fn read_blob(&self, digest: &Digest) -> io::Result<Option<Vec<u8>>> {
        let bytes = if digest.size_bytes > BATCH_BYTES {
            self.read_streamed(digest)?
        } else {
            let response: BatchReadBlobsResponse = self
                .call(
                    BATCH_READ,
                    BatchReadBlobsRequest {
                        instance_name: self.instance.clone(),
                        digests: vec![digest.clone()],
                        digest_function: DIGEST_FUNCTION_SHA256,
                    },
                )
                .map_err(from_status)?;
            let Some(entry) = response.responses.into_iter().next() else {
                return Err(failed(format!("the fleet answered nothing for {digest}")));
            };
            match entry.status.map_or(0, |s| s.code) {
                0 => Some(entry.data),
                code if code == Code::NotFound as i32 => None,
                code => {
                    return Err(failed(format!(
                        "the fleet refused to read {digest} (status {code})"
                    )))
                }
            }
        };
        // A blob is only ever what its name says; a fleet that serves other
        // bytes is a defect of the fleet, said rather than built from.
        match bytes {
            Some(bytes) if Digest::of(&bytes) != *digest => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("the fleet served bytes for {digest} that are not its"),
            )),
            other => Ok(other),
        }
    }

    fn write_blob(&self, bytes: &[u8]) -> io::Result<Digest> {
        self.refuse_write()?;
        let digest = Digest::of(bytes);
        if self.find_missing(std::slice::from_ref(&digest))?.is_empty() {
            return Ok(digest);
        }
        if digest.size_bytes > BATCH_BYTES {
            self.write_streamed(&digest, bytes)?;
            return Ok(digest);
        }
        let response: BatchUpdateBlobsResponse = self
            .call(
                BATCH_UPDATE,
                BatchUpdateBlobsRequest {
                    instance_name: self.instance.clone(),
                    requests: vec![UpdateBlob {
                        digest: Some(digest.clone()),
                        data: bytes.to_vec(),
                    }],
                    digest_function: DIGEST_FUNCTION_SHA256,
                },
            )
            .map_err(from_status)?;
        match response
            .responses
            .first()
            .map(|r| r.status.as_ref().map_or(0, |s| s.code))
        {
            Some(0) => Ok(digest),
            Some(code) => Err(failed(format!(
                "the fleet refused to store {digest} (status {code})"
            ))),
            None => Err(failed(format!("the fleet answered nothing for {digest}"))),
        }
    }

    fn action_result(&self, action: &Digest) -> io::Result<Option<ActionResult>> {
        match self.call::<_, ActionResult>(
            GET_RESULT,
            GetActionResultRequest {
                instance_name: self.instance.clone(),
                action_digest: Some(action.clone()),
                digest_function: DIGEST_FUNCTION_SHA256,
            },
        ) {
            Ok(result) => Ok(Some(result)),
            Err(status) if status.code() == Code::NotFound => Ok(None),
            Err(status) => Err(from_status(status)),
        }
    }

    fn record_action_result(&self, action: &Digest, result: &ActionResult) -> io::Result<()> {
        self.refuse_write()?;
        let named: Vec<Digest> = result
            .output_files
            .iter()
            .filter_map(|f| f.digest.clone())
            .chain(result.stdout_digest.clone())
            .chain(result.stderr_digest.clone())
            .collect();
        let missing = self.find_missing(&named)?;
        if let Some(first) = missing.first() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the result of action {action} names {} blob(s) the fleet does not hold, \
                     the first {first}",
                    missing.len()
                ),
            ));
        }
        self.call::<_, ActionResult>(
            UPDATE_RESULT,
            UpdateActionResultRequest {
                instance_name: self.instance.clone(),
                action_digest: Some(action.clone()),
                action_result: Some(result.clone()),
                digest_function: DIGEST_FUNCTION_SHA256,
            },
        )
        .map_err(from_status)?;
        Ok(())
    }
}

/// `path` under `instance`, as a `ByteStream` resource names it: the
/// instance's name and a `/` before it when there is one.
fn resource(instance: &str, path: &str) -> String {
    if instance.is_empty() {
        path.to_string()
    } else {
        format!("{}/{path}", instance.trim_end_matches('/'))
    }
}

/// The requests of a `ByteStream` write of `bytes` to `name`: the name on
/// the first only, each chunk at its offset, the last finishing the write.
fn write_requests(name: &str, bytes: &[u8]) -> Vec<ByteStreamWriteRequest> {
    let chunks: Vec<&[u8]> = if bytes.is_empty() {
        vec![&[][..]]
    } else {
        bytes.chunks(CHUNK_BYTES).collect()
    };
    let last = chunks.len() - 1;
    let mut offset = 0i64;
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let request = ByteStreamWriteRequest {
                resource_name: if i == 0 {
                    name.to_string()
                } else {
                    String::new()
                },
                write_offset: offset,
                finish_write: i == last,
                data: chunk.to_vec(),
            };
            offset += i64::try_from(chunk.len()).expect("a chunk is shorter than i64::MAX");
            request
        })
        .collect()
}

/// A name for one upload that no other upload of this or another process
/// shares, in the shape of a UUID, which `ByteStream` asks for.
fn upload_id() -> String {
    static UPLOADS: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let seed = format!(
        "{}:{nanos}:{}",
        std::process::id(),
        UPLOADS.fetch_add(1, Ordering::Relaxed)
    );
    let hex = Digest::of(seed.as_bytes()).hash;
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn failed(detail: String) -> io::Error {
    io::Error::other(detail)
}

fn from_status(status: Status) -> io::Error {
    let kind = match status.code() {
        Code::NotFound => io::ErrorKind::NotFound,
        Code::PermissionDenied | Code::Unauthenticated => io::ErrorKind::PermissionDenied,
        Code::Unavailable => io::ErrorKind::ConnectionRefused,
        Code::DeadlineExceeded => io::ErrorKind::TimedOut,
        Code::InvalidArgument => io::ErrorKind::InvalidInput,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(
        kind,
        format!(
            "the fleet's cache: {:?}: {}",
            status.code(),
            status.message()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resource_is_named_under_its_instance() {
        assert_eq!(resource("", "blobs/h/1"), "blobs/h/1");
        assert_eq!(resource("sce", "blobs/h/1"), "sce/blobs/h/1");
        assert_eq!(
            resource("sites/a/sce/", "blobs/h/1"),
            "sites/a/sce/blobs/h/1"
        );
    }

    /// The name on the first request only, each chunk at the offset the
    /// ones before it end at, and only the last finishing the write; an
    /// empty blob is one request that finishes it.
    #[test]
    fn a_streamed_write_is_named_once_and_finished_once() {
        let bytes = vec![7u8; CHUNK_BYTES * 2 + 5];
        let requests = write_requests("sce/uploads/u/blobs/h/9", &bytes);
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].resource_name, "sce/uploads/u/blobs/h/9");
        assert!(requests[1..].iter().all(|r| r.resource_name.is_empty()));
        let offsets: Vec<i64> = requests.iter().map(|r| r.write_offset).collect();
        let chunk = i64::try_from(CHUNK_BYTES).unwrap();
        assert_eq!(offsets, [0, chunk, 2 * chunk]);
        let finished: Vec<bool> = requests.iter().map(|r| r.finish_write).collect();
        assert_eq!(finished, [false, false, true]);
        assert_eq!(
            requests.iter().map(|r| r.data.len()).sum::<usize>(),
            bytes.len()
        );

        let empty = write_requests("n", b"");
        assert_eq!(empty.len(), 1);
        assert!(empty[0].finish_write && empty[0].data.is_empty());
    }

    #[test]
    fn an_upload_is_named_like_a_uuid_and_never_twice() {
        let (a, b) = (upload_id(), upload_id());
        assert_ne!(a, b);
        let lengths: Vec<usize> = a.split('-').map(str::len).collect();
        assert_eq!(lengths, [8, 4, 4, 4, 12]);
    }
}
