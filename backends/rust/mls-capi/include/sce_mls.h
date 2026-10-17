/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The group key agreement of a shared list (MLS, RFC 9420), for the lanes that are not Rust.
 *
 * A lane holds a device and tells it the membership log as data: the members with their keys, the
 * commit the log names for each epoch, and the epoch the log is in. The group applies a commit only
 * when the log names it (its SHA-256 is in an entry the administrators signed), and refuses a roster
 * that is not the log's. The lane's own verification of the log is what that data must be.
 *
 * Every function returns SCE_MLS_OK (0) or a status, and the text of what went wrong is in
 * sce_mls_last_error() for the thread. A buffer the library returns is given back with
 * sce_mls_buf_free(). A handle is used by one thread at a time.
 *
 * Written by hand and kept equal to the library's exports by a test.
 */

#ifndef SCE_MLS_H
#define SCE_MLS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SCE_MLS_ABI_VERSION 1u

#define SCE_MLS_OK 0
#define SCE_MLS_ERR_LIBRARY 1
#define SCE_MLS_ERR_NO_GROUP 2
#define SCE_MLS_ERR_IN_GROUP 3
#define SCE_MLS_ERR_PENDING_COMMIT 4
#define SCE_MLS_ERR_NO_PENDING_COMMIT 5
#define SCE_MLS_ERR_COMMIT_NOT_NAMED 6
#define SCE_MLS_ERR_NOT_IN_LOG 7
#define SCE_MLS_ERR_STILL_A_MEMBER 8
#define SCE_MLS_ERR_ROSTER_DIFFERS 9
#define SCE_MLS_ERR_WELCOME_NOT_THE_LOGS 10
#define SCE_MLS_ERR_MALFORMED 11
#define SCE_MLS_ERR_REMOVED 12
#define SCE_MLS_ERR_NULL_ARGUMENT 20
#define SCE_MLS_ERR_BAD_TEXT 21
#define SCE_MLS_ERR_PANIC 22

typedef struct SceMlsDevice SceMlsDevice;
typedef struct SceMlsLog SceMlsLog;

/* A buffer the library made; give it back with sce_mls_buf_free. */
typedef struct SceMlsBuf {
    uint8_t *ptr;
    size_t len;
} SceMlsBuf;

uint32_t sce_mls_abi_version(void);
const char *sce_mls_last_error(void);
void sce_mls_buf_free(SceMlsBuf buf);

/* The membership log as data. */
SceMlsLog *sce_mls_log_new(uint32_t epoch);
int32_t sce_mls_log_add_member(SceMlsLog *log, uint64_t node, const uint8_t *key /* 32 bytes */);
int32_t sce_mls_log_name_commit(SceMlsLog *log, uint32_t epoch, const uint8_t *hash /* 32 bytes */);
void sce_mls_log_free(SceMlsLog *log);

/* A device of one list, signing with the key the 32-byte seed makes. NULL on failure. */
SceMlsDevice *sce_mls_device_new(const char *list, uint64_t node, const uint8_t *seed /* 32 bytes */);
SceMlsDevice *sce_mls_device_from_bytes(const char *list, uint64_t node, const uint8_t *seed, const uint8_t *state,
                                        size_t len);
void sce_mls_device_free(SceMlsDevice *device);
/* What the device keeps of its group: secret, for the host's store. */
int32_t sce_mls_device_to_bytes(SceMlsDevice *device, SceMlsBuf *out);

int32_t sce_mls_key_package(SceMlsDevice *device, SceMlsBuf *out);
int32_t sce_mls_create_group(SceMlsDevice *device);

/* The commit that adds the device of one key package, made and not merged. commit_hash is 32 bytes. */
int32_t sce_mls_stage_add(SceMlsDevice *device, const uint8_t *package, size_t package_len, SceMlsLog *log,
                          SceMlsBuf *commit, SceMlsBuf *welcome, uint8_t *commit_hash, uint64_t *epoch);
int32_t sce_mls_stage_remove(SceMlsDevice *device, uint64_t node, SceMlsLog *log, SceMlsBuf *commit,
                             uint8_t *commit_hash, uint64_t *epoch);
int32_t sce_mls_merge_pending(SceMlsDevice *device, SceMlsLog *log, uint64_t *epoch);
int32_t sce_mls_clear_pending(SceMlsDevice *device);

/* Apply a commit another device made, when the log names it. *removed is 1 when it removed this
 * device (no later key is derivable), 0 otherwise; *epoch is the epoch the group is in. */
int32_t sce_mls_apply_commit(SceMlsDevice *device, const uint8_t *commit, size_t commit_len, SceMlsLog *log,
                             int32_t *removed, uint64_t *epoch);
/* Join from a Welcome, with the commit that made its epoch. */
int32_t sce_mls_join(SceMlsDevice *device, const uint8_t *welcome, size_t welcome_len, const uint8_t *commit,
                     size_t commit_len, SceMlsLog *log, uint64_t *epoch);

int32_t sce_mls_epoch(SceMlsDevice *device, uint64_t *epoch);
int32_t sce_mls_is_active(SceMlsDevice *device);
/* The 32-byte key of the current epoch, and of one section of it. */
int32_t sce_mls_epoch_key(SceMlsDevice *device, uint8_t *out /* 32 bytes */);
int32_t sce_mls_section_key(SceMlsDevice *device, const char *name, uint8_t *out /* 32 bytes */);

#ifdef __cplusplus
}
#endif

#endif /* SCE_MLS_H */
