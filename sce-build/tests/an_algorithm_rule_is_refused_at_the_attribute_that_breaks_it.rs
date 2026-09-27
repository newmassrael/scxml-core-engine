// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A rule an algorithm document breaks is refused where the author broke
//! it: at the attribute, on the row and column it is written at.
//!
//! # What was wrong
//!
//! The algorithm kind judges some rules only once its body is read — a
//! parameter is read-only, a `bytes` buffer and its `capacity` agree with
//! the signature — and those refusals were placed at `<sce:body>`, the one
//! element the check still held: "the nearest container element the
//! diagnostic can point to without re-threading nodes through the IR".
//! Measured 2026-09-23, every one of them named a row that did not hold its
//! token, so `SCE_ERROR_CONTRACT.md` §3.1.1 — `actual` occurs on the line
//! the record names — failed for each, and a scalar's stray `capacity` was
//! reported as the text `(present)`, which no document contains.
//!
//! Four integer attributes (`capacity`, `max-iter`, `returns-max-size`,
//! `max-count`) were read so that a value outside `u32` became ABSENT. The
//! schema's integer types are unbounded, so `4294967296` passed it and then
//! vanished: a `max-iter` bound was dropped, a buffer's `capacity` was
//! reported missing. And `returns-max-size` on a return that is not `bytes`
//! was accepted and dropped, on the algorithm's signature and on a
//! procedure's `<sce:helper>` alike.
//!
//! # What is held
//!
//! Each case writes the attribute on a row its element does not start on,
//! so a record placed at the element — or at `<sce:body>` — is caught.
//! `actual` must also occur on the row the record names, read off the
//! document rather than off the fixture's claim.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// One document, and where in it the refused attribute is written.
struct Case {
    file: &'static str,
    document: &'static str,
    code: &'static str,
    line: u64,
    col: u64,
    /// The value as the document spells it on `line` — `None` when the
    /// refusal is of something not written at all (a missing attribute).
    actual: Option<&'static str>,
}

/// A parameter assigned inside a nested block, its name on the assign's
/// second row.
const LVALUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_lvalue" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:if cond="x === 0">
      <sce:assign expr="1"
                  target="data"/>
    </sce:if>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A scalar local with a `capacity`, which only a `bytes` buffer takes.
const SCALAR_CAPACITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_scalar_capacity" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"
             capacity="4"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` return with no `returns-max-size` — refused at the
/// `<sce:return>`, which is where the attribute belongs.
const RETURN_WITHOUT_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_return_without_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return
        type="bytes"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="4"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `returns-max-size` on a scalar return, which has no buffer to bound.
const SCALAR_RETURN_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_scalar_return_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"
                returns-max-size="8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="data"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` buffer with no `capacity` — refused at its `<sce:var>`.
const BUFFER_WITHOUT_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_without_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out"
             type="bytes"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A buffer whose `capacity` is not the signature's `returns-max-size`.
const CAPACITY_MISMATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_capacity_mismatch" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes"
             capacity="8"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `capacity` one past `u32::MAX`, which the unbounded schema type admits.
const CAPACITY_OVERFLOW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_capacity_overflow" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes"
             capacity="4294967296"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `max-iter` one past `u32::MAX` — the bound that used to vanish.
const MAX_ITER_OVERFLOW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_max_iter_overflow" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:while cond="x &lt; 3"
               max-iter="4294967296">
      <sce:assign target="x" expr="x + 1"/>
    </sce:while>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A procedure helper returning a scalar, with a `returns-max-size`.
const HELPER_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_helper_cap" initial="start">
  <datamodel>
    <data id="value" sce:type="uint8" sce:direction="in"/>
    <sce:helper name="scale" args="uint8" returns="uint16"
                sce:returns-max-size="8"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A second `bytes` buffer, where v1 takes one — refused at the second.
const EXTRA_BUFFER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_extra_buffer" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="4"/>
    <sce:var type="bytes" capacity="4"
             name="spare"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` buffer in an algorithm that does not return `bytes`.
const BUFFER_NOT_RETURNED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_not_returned" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var type="bytes" capacity="4"
             name="out"/>
    <sce:return expr="data"/>
  </sce:body>
</scxml>
"#;

/// An element of a list of records read by index: a record is read through
/// a name — a foreach item — never as `xs[i]` (SCE_FORGE.md §4.12).
///
/// ⚠ This slot held a `list<T>` PARAMETER, refused in v1 until
/// `f5868fe0a5` admitted one as a read-only view; the case kept expecting a
/// refusal of a document that had become legal.
const RECORD_LIST_INDEX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_list_index" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="xs" type="list&lt;record:Hlc&gt;"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="xs[0].wallTime"/>
  </sce:body>
</scxml>
"#;

/// A foreach item assigned: it is an element of the collection it iterates,
/// read-only as that parameter is.
const FOREACH_ITEM_ASSIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_foreach_item_assign" version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;int32&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:foreach item="x" in="xs">
      <sce:assign expr="0"
                  target="x"/>
    </sce:foreach>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;

/// A list of records appended something that is not a record of its
/// schema by name — there is no record expression.
const RECORD_LIST_APPEND_VALUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_list_append_value" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="xs" type="list&lt;record:Hlc&gt;"/>
    <sce:return type="list&lt;record:Hlc&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;record:Hlc&gt;" capacity="4"/>
    <sce:foreach item="x" in="xs">
      <sce:append target="out"
                  expr="x.wallTime"/>
    </sce:foreach>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `list<string>` return: an element with a length of its own.
const LIST_OF_STRING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_of_string" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return returns-max-size="4"
                type="list&lt;string&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="data"/>
  </sce:body>
</scxml>
"#;

/// A `list<int32>` buffer in an algorithm that returns `list<int64>` —
/// refused at the buffer's name, as a buffer that is not returned is.
const LIST_ELEM_MISMATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_elem_mismatch" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="list&lt;int64&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var type="list&lt;int32&gt;" capacity="4"
             name="out"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `<sce:test-vector>` on a list return, whose `value=` is one scalar.
const LIST_TEST_VECTOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_test_vector" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="list&lt;int64&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;int64&gt;" capacity="4"/>
    <sce:return expr="out"/>
  </sce:body>
  <sce:test-vector hex="00"
                   value="1"/>
</scxml>
"#;

/// A list-returning algorithm the two call cases import. Accepted on its
/// own: a host may call it; another algorithm may not.
const LIST_CALLEE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_callee" version="1.0">
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="list&lt;int64&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;int64&gt;" capacity="4"/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `<sce:call>` of the list-returning algorithm by its alias.
const LIST_CALL_BY_ALIAS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_call_by_alias" version="1.0">
  <sce:import kind="algorithm" src="probe_list_callee.scxml" as="days"/>
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:call args="n"
              target="days"/>
    <sce:return expr="n"/>
  </sce:body>
</scxml>
"#;

/// The list-returning algorithm called inside an expression — refused at
/// the callee, from its host-only signature rather than an arity check
/// against the empty parameter list that signature carries.
const LIST_CALL_IN_EXPRESSION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_call_in_expression" version="1.0">
  <sce:import kind="algorithm" src="probe_list_callee.scxml" as="days"/>
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="int64"
             init="n + days(n)"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// The same call from a validator — the stateless-import route every
/// non-algorithm kind registers its imports through.
const LIST_CALL_FROM_VALIDATOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="validator" version="1.0">
  <sce:import src="probe_list_callee.scxml" kind="algorithm" as="days"/>
  <datamodel>
    <data id="raw" sce:type="int64" sce:direction="in"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="days(raw) &gt; 0"/>
  </datamodel>
</scxml>
"#;

/// The same call spelled `alias.name` — the other route to the same callee.
const LIST_CALL_BY_NAME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_list_call_by_name" version="1.0">
  <sce:import kind="algorithm" src="probe_list_callee.scxml" as="days"/>
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:call args="n"
              target="days.probe_list_callee"/>
    <sce:return expr="n"/>
  </sce:body>
</scxml>
"#;

/// Documents a case imports, written beside the cases but not refusals
/// themselves.
/// A two-field event-schema the record cases type their values by.
const SCHEMA_HLC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe_schema_hlc" sce:event-name="hlc.stamp">
  <datamodel>
    <data id="wallTime" sce:type="int64" sce:direction="in"/>
    <data id="counter" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// An event-schema with a `string` field, which a record cannot carry in v1.
const SCHEMA_NAMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe_schema_named" sce:event-name="named.thing">
  <datamodel>
    <data id="label" sce:type="string" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A record parameter typed by a schema with a `string` field — refused at
/// the `type` that names it (SCE_FORGE.md §4.12).
const RECORD_STRING_FIELD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_string_field" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_named.scxml" as="Named"/>
  <sce:signature>
    <sce:param name="n"
               type="record:Named"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;

/// A record local that leaves a schema field out — refused at its name.
const RECORD_MISSING_FIELD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_missing_field" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="w" type="int64"/>
    <sce:return type="record:Hlc"/>
  </sce:signature>
  <sce:body>
    <sce:var type="record:Hlc"
             name="r">
      <sce:set name="wallTime" expr="w"/>
    </sce:var>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// A `<sce:set>` the schema does not declare — refused at its name.
const RECORD_UNKNOWN_FIELD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_unknown_field" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="w" type="int64"/>
    <sce:return type="record:Hlc"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="record:Hlc">
      <sce:set name="wallTime" expr="w"/>
      <sce:set expr="0"
                 name="counterr"/>
    </sce:var>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// An assignment to a whole record, which v1 updates a field at a time —
/// refused at its target.
const RECORD_WHOLE_ASSIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_whole_assign" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="p" type="record:Hlc"/>
    <sce:return type="record:Hlc"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="record:Hlc">
      <sce:set name="wallTime" expr="p.wallTime"/>
      <sce:set name="counter" expr="p.counter"/>
    </sce:var>
    <sce:assign expr="p"
                target="r"/>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// A schema field for field the same as `probe_schema_hlc` but a different
/// document — so a different record type (SCE_FORGE.md §4.12).
const SCHEMA_TWIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe_schema_twin" sce:event-name="twin.stamp">
  <datamodel>
    <data id="wallTime" sce:type="int64" sce:direction="in"/>
    <data id="counter" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// An algorithm with a record parameter, which another algorithm may call
/// with a record of the same schema.
const RECORD_CALLEE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_callee" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="s" type="record:Hlc"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="s.wallTime"/>
  </sce:body>
</scxml>
"#;

/// The record callee called with a record of the same schema, imported
/// under another alias — accepted: a record type is its schema document,
/// not the alias either side names it by.
const RECORD_CALL_SAME_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_call_same_schema" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Stamp"/>
  <sce:import kind="algorithm" src="probe_record_callee.scxml" as="wall"/>
  <sce:signature>
    <sce:param name="p" type="record:Stamp"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:var name="w" type="int64" init="wall(p)"/>
    <sce:return expr="w"/>
  </sce:body>
</scxml>
"#;

/// The record callee called with a record of the twin schema under the
/// callee's own alias — refused at the argument: the alias matches, the
/// schema does not.
const RECORD_CALL_OTHER_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_call_other_schema" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_twin.scxml" as="Hlc"/>
  <sce:import kind="algorithm" src="probe_record_callee.scxml" as="wall"/>
  <sce:signature>
    <sce:param name="p" type="record:Hlc"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:var name="w" type="int64"
             init="wall(p)"/>
    <sce:return expr="w"/>
  </sce:body>
</scxml>
"#;

/// A foreach over the list buffer the body is building — refused at its
/// source: the buffer is read back by `len` and an index, never iterated,
/// since an append inside the loop would move the end it walks to
/// (SCE_FORGE.md §4.12).
const BUFFER_FOREACH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_foreach" version="1.0">
  <sce:signature>
    <sce:param name="n" type="int64"/>
    <sce:return type="list&lt;int64&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;int64&gt;" capacity="4"/>
    <sce:append target="out" expr="n"/>
    <sce:foreach item="x"
                 in="out">
      <sce:append target="out" expr="x"/>
    </sce:foreach>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// The bytes buffer the body is building, compared as a value — refused at
/// the buffer: it is the algorithm's own storage (C11's result struct,
/// Kotlin's SceByteBuf), read back only by `len` and an index, not a `bytes`
/// value an operator can take (SCE_FORGE.md §4.12).
const BUFFER_AS_VALUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_as_value" version="1.0">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="4"/>
    <sce:var name="same" type="bool"
             init="out === data"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// The record callee called from a validator — refused at the call: only an
/// algorithm's body calls a record algorithm besides a host.
const RECORD_CALL_FROM_VALIDATOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="validator" version="1.0">
  <sce:import src="probe_record_callee.scxml" kind="algorithm" as="wall"/>
  <datamodel>
    <data id="raw" sce:type="int64" sce:direction="in"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="wall(raw) &gt; 0"/>
  </datamodel>
</scxml>
"#;

/// Two records compared whole — refused at the first: a record is passed
/// whole or read a field at a time, never an operand.
const RECORD_AS_OPERAND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_as_operand" version="1.0">
  <sce:import kind="event-schema" src="probe_schema_hlc.scxml" as="Hlc"/>
  <sce:signature>
    <sce:param name="p" type="record:Hlc"/>
    <sce:param name="q" type="record:Hlc"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:var name="same" type="bool"
             init="p === q"/>
    <sce:return expr="same"/>
  </sce:body>
</scxml>
"#;

/// A standard schema named by `sce:std/…` here and by a relative path inside
/// the standard callee (`hlc_compare` imports `hlc_timestamp.scxml`) —
/// accepted: both names reach one document, so one record type.
const RECORD_CALL_STD_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_record_call_std_schema" version="1.0">
  <sce:import kind="event-schema" src="sce:std/merge/hlc_timestamp.scxml" as="Stamp"/>
  <sce:import kind="algorithm" src="sce:std/merge/hlc_compare.scxml" as="order"/>
  <sce:signature>
    <sce:param name="a" type="record:Stamp"/>
    <sce:param name="b" type="record:Stamp"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="c" type="int32" init="order(a, b)"/>
    <sce:return expr="c"/>
  </sce:body>
</scxml>
"#;

const SUPPORT: &[(&str, &str)] = &[
    ("probe_list_callee.scxml", LIST_CALLEE),
    ("probe_schema_hlc.scxml", SCHEMA_HLC),
    ("probe_schema_named.scxml", SCHEMA_NAMED),
    ("probe_schema_twin.scxml", SCHEMA_TWIN),
    ("probe_record_callee.scxml", RECORD_CALLEE),
    (
        "probe_record_call_same_schema.scxml",
        RECORD_CALL_SAME_SCHEMA,
    ),
    ("probe_record_call_std_schema.scxml", RECORD_CALL_STD_SCHEMA),
];

const CASES: &[Case] = &[
    Case {
        file: "probe_lvalue.scxml",
        document: LVALUE,
        code: "algorithm/lvalue-unsupported",
        line: 11,
        col: 27,
        actual: Some("data"),
    },
    Case {
        file: "probe_scalar_capacity.scxml",
        document: SCALAR_CAPACITY,
        code: "validation/attribute-rule-violated",
        line: 9,
        col: 14,
        actual: Some("4"),
    },
    Case {
        file: "probe_return_without_cap.scxml",
        document: RETURN_WITHOUT_CAP,
        code: "validation/missing-attribute",
        line: 5,
        col: 5,
        actual: None,
    },
    Case {
        file: "probe_scalar_return_cap.scxml",
        document: SCALAR_RETURN_CAP,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 17,
        actual: Some("8"),
    },
    Case {
        file: "probe_buffer_without_cap.scxml",
        document: BUFFER_WITHOUT_CAP,
        code: "validation/missing-attribute",
        line: 8,
        col: 5,
        actual: None,
    },
    Case {
        file: "probe_capacity_mismatch.scxml",
        document: CAPACITY_MISMATCH,
        code: "validation/invalid-attribute",
        line: 9,
        col: 24,
        actual: Some("8"),
    },
    Case {
        file: "probe_capacity_overflow.scxml",
        document: CAPACITY_OVERFLOW,
        code: "validation/attribute-rule-violated",
        line: 9,
        col: 14,
        actual: Some("4294967296"),
    },
    Case {
        file: "probe_max_iter_overflow.scxml",
        document: MAX_ITER_OVERFLOW,
        code: "validation/attribute-rule-violated",
        line: 10,
        col: 16,
        actual: Some("4294967296"),
    },
    Case {
        file: "probe_helper_cap.scxml",
        document: HELPER_CAP,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 17,
        actual: Some("8"),
    },
    Case {
        file: "probe_extra_buffer.scxml",
        document: EXTRA_BUFFER,
        code: "validation/incompatible-attributes",
        line: 10,
        col: 20,
        actual: None,
    },
    Case {
        file: "probe_buffer_not_returned.scxml",
        document: BUFFER_NOT_RETURNED,
        code: "validation/incompatible-attributes",
        line: 9,
        col: 20,
        actual: None,
    },
    Case {
        file: "probe_record_list_index.scxml",
        document: RECORD_LIST_INDEX,
        code: "expression/unsupported-construct",
        line: 10,
        col: 15,
        actual: Some("xs[0]"),
    },
    Case {
        file: "probe_foreach_item_assign.scxml",
        document: FOREACH_ITEM_ASSIGN,
        code: "algorithm/lvalue-unsupported",
        line: 10,
        col: 27,
        actual: Some("x"),
    },
    Case {
        file: "probe_record_list_append_value.scxml",
        document: RECORD_LIST_APPEND_VALUE,
        code: "expression/unsupported-construct",
        line: 12,
        col: 25,
        actual: Some("x.wallTime"),
    },
    Case {
        file: "probe_list_of_string.scxml",
        document: LIST_OF_STRING,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 17,
        actual: Some("list&lt;string&gt;"),
    },
    Case {
        file: "probe_list_elem_mismatch.scxml",
        document: LIST_ELEM_MISMATCH,
        code: "validation/incompatible-attributes",
        line: 9,
        col: 20,
        actual: None,
    },
    Case {
        file: "probe_list_test_vector.scxml",
        document: LIST_TEST_VECTOR,
        code: "validation/attribute-rule-violated",
        line: 12,
        col: 20,
        actual: Some("1"),
    },
    Case {
        file: "probe_list_call_by_alias.scxml",
        document: LIST_CALL_BY_ALIAS,
        code: "expression/unsupported-construct",
        line: 10,
        col: 23,
        actual: Some("days"),
    },
    Case {
        file: "probe_list_call_by_name.scxml",
        document: LIST_CALL_BY_NAME,
        code: "expression/unsupported-construct",
        line: 10,
        col: 23,
        actual: Some("days.probe_list_callee"),
    },
    Case {
        file: "probe_list_call_in_expression.scxml",
        document: LIST_CALL_IN_EXPRESSION,
        code: "expression/unsupported-construct",
        line: 10,
        col: 24,
        actual: Some("days"),
    },
    Case {
        file: "probe_list_call_from_validator.scxml",
        document: LIST_CALL_FROM_VALIDATOR,
        code: "expression/unsupported-construct",
        line: 7,
        col: 29,
        actual: Some("days"),
    },
    Case {
        file: "probe_record_string_field.scxml",
        document: RECORD_STRING_FIELD,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 22,
        actual: Some("record:Named"),
    },
    Case {
        file: "probe_record_missing_field.scxml",
        document: RECORD_MISSING_FIELD,
        code: "validation/attribute-rule-violated",
        line: 10,
        col: 20,
        actual: Some("r"),
    },
    Case {
        file: "probe_record_unknown_field.scxml",
        document: RECORD_UNKNOWN_FIELD,
        code: "validation/attribute-rule-violated",
        line: 12,
        col: 24,
        actual: Some("counterr"),
    },
    Case {
        file: "probe_record_whole_assign.scxml",
        document: RECORD_WHOLE_ASSIGN,
        code: "expression/unsupported-construct",
        line: 14,
        col: 25,
        actual: Some("r"),
    },
    Case {
        file: "probe_record_call_other_schema.scxml",
        document: RECORD_CALL_OTHER_SCHEMA,
        code: "expression/type-mismatch",
        line: 11,
        col: 25,
        actual: Some("p"),
    },
    Case {
        file: "probe_record_call_from_validator.scxml",
        document: RECORD_CALL_FROM_VALIDATOR,
        code: "expression/unsupported-construct",
        line: 7,
        col: 29,
        actual: Some("wall"),
    },
    Case {
        file: "probe_buffer_as_value.scxml",
        document: BUFFER_AS_VALUE,
        code: "expression/unsupported-construct",
        line: 10,
        col: 20,
        actual: Some("out"),
    },
    Case {
        file: "probe_buffer_foreach.scxml",
        document: BUFFER_FOREACH,
        code: "algorithm/foreach-source-not-iterable",
        line: 11,
        col: 22,
        actual: Some("out"),
    },
    Case {
        file: "probe_record_as_operand.scxml",
        document: RECORD_AS_OPERAND,
        code: "expression/unsupported-construct",
        line: 11,
        col: 20,
        actual: Some("p"),
    },
];

/// Every case, and every document a case imports, written into one
/// directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for case in CASES {
        std::fs::write(dir.path().join(case.file), case.document).expect("write case");
    }
    for (file, document) in SUPPORT {
        std::fs::write(dir.path().join(file), document).expect("write support document");
    }
    dir
}

/// The document every call case imports is accepted on its own — the
/// refusal is of the call, not of a list-returning algorithm.
#[test]
fn a_list_returning_algorithm_is_accepted_for_a_host() {
    let dir = fixture();
    let output = Command::new(codegen_bin())
        .current_dir(dir.path())
        .args([
            "--error-format=json",
            "check",
            "probe_list_callee.scxml",
            "-l",
            "rust",
        ])
        .output()
        .expect("run sce-codegen");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An algorithm's call of a record algorithm with a record of the callee's
/// own schema is accepted on every backend — the call the record-slot
/// refusals above are the other side of (SCE_FORGE.md §4.12). The schema is
/// the same DOCUMENT under another alias, and then a standard schema reached
/// by `sce:std/…` on one side and a relative path on the other.
#[test]
fn a_record_algorithm_is_callable_from_an_algorithm_of_the_same_schema() {
    let dir = fixture();
    for file in [
        "probe_record_call_same_schema.scxml",
        "probe_record_call_std_schema.scxml",
    ] {
        for language in ["rust", "cpp", "c11", "go", "python", "kotlin"] {
            let output = Command::new(codegen_bin())
                .current_dir(dir.path())
                .args([
                    "--error-format=json",
                    "check",
                    file,
                    "-l",
                    language,
                    // Go names every import by module path (ignored elsewhere).
                    "--go-module-prefix",
                    "example.com/probe",
                ])
                .output()
                .expect("run sce-codegen");
            assert!(
                output.status.success(),
                "{file} {language}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// The one record `sce-codegen check` prints for `file` in JSON mode, or why
/// there is not exactly one. The rules are judged before any backend is
/// chosen, so one backend asks for all of them.
fn refusal(dir: &Path, file: &str) -> Result<serde_json::Value, String> {
    let output = Command::new(codegen_bin())
        .current_dir(dir)
        .args(["--error-format=json", "check", file, "-l", "rust"])
        .output()
        .expect("run sce-codegen");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return Err(format!("accepted the document; stderr: {stderr}"));
    }
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    match lines.as_slice() {
        [one] => serde_json::from_str(one).map_err(|e| format!("{e}: {stderr}")),
        _ => Err(format!("expected one record:\n{stderr}")),
    }
}

/// What is wrong with `record` as the refusal of `case`, if anything.
fn mismatches(case: &Case, record: &serde_json::Value) -> Vec<String> {
    let mut wrong = Vec::new();
    let expect = [
        ("code", record["code"].clone(), serde_json::json!(case.code)),
        (
            "location.line",
            record["location"]["line"].clone(),
            serde_json::json!(case.line),
        ),
        (
            "location.col",
            record["location"]["col"].clone(),
            serde_json::json!(case.col),
        ),
        (
            "actual",
            record["actual"].clone(),
            serde_json::json!(case.actual),
        ),
    ];
    for (field, got, want) in expect {
        if got != want {
            wrong.push(format!("{field} = {got}, want {want}"));
        }
    }
    // §3.1.1, read off the document rather than off the fixture's claim.
    if let Some(why) = common::wire_site::record_misplaced(record, case.document) {
        wrong.push(format!(
            "actual {} cannot be found: {why}",
            record["actual"]
        ));
    }
    wrong
}

#[test]
fn each_rule_names_the_row_and_column_of_its_attribute() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        let wrong = match refusal(dir.path(), case.file) {
            Ok(record) => mismatches(case, &record),
            Err(why) => vec![why],
        };
        if !wrong.is_empty() {
            failures.push(format!("{}: {}", case.file, wrong.join("; ")));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} refusals are not placed at their attribute:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}

/// The library facade places each refusal as the CLI does.
#[test]
fn the_library_route_places_each_refusal_as_the_cli_does() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        match sce_build::compile_forge_file(
            &dir.path().join(case.file),
            Language::Rust,
            &[],
            &ForgeCompileOptions::default(),
        ) {
            Ok(_) => failures.push(format!("{}: accepted", case.file)),
            Err(err) => {
                let at = (err.location.line, err.location.col);
                let want = (Some(case.line as u32), Some(case.col as u32));
                if at != want {
                    failures.push(format!("{}: at {at:?}, want {want:?}: {err}", case.file));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
