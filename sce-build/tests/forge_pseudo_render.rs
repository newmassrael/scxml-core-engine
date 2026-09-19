// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The pseudocode rendering is total for the kinds it claims, and names
//! the kinds it does not.
//!
//! What these tests are FOR: the rendering exists so that a human who
//! approves it has approved the document. A renderer that drops a field
//! produces a text that reads perfectly well and is not the document,
//! and no compiler catches that. So the two algorithm/procedure tests
//! below are written as whole-output comparisons rather than as
//! `contains` assertions — a `contains` test passes on a rendering that
//! has silently lost the line after the one it looked for.

use sce_build::forge::model::{
    AlgorithmConst, AlgorithmConstType, AlgorithmModel, AlgorithmParam, AlgorithmSignature,
    AlgorithmStmt, BackpressurePolicy, BitSize, BoundedCollectionModel, BufferPoolModel,
    BufferPoolVariant, CachePolicy, CapacitySource, CodecField, CodecModel, CodecTestVector,
    CodecVariant, CollectionOrdering, ConcurrencyMode, ConditionModel, DecodedField,
    DecodedFieldValue, DecodedValue, Direction, Endian, EnumModel, EnumVariant, EventSchemaModel,
    FilterModel, FilterType, FlagDef, FlagInput, FoldBody, ForgeDocument, ForgeField, InboxConfig,
    InboxOrdering, InterpolationAxis, InterpolationMethod, InterpolationModel, LinkClass,
    LinkInboundEvent, LinkModel, LinkOutboundEvent, LookupEntry, LookupModel, MissPolicy,
    ObserverModel, OutOfBounds, OverflowPolicy, PeekByteSpec, PresentIfPredicate, PresentIfScope,
    ProcedureAssign, ProcedureDoneParam, ProcedureHelper, ProcedureModel, ProcedureSendAction,
    ProcedureState, ProcedureTransition, RangeRule, RateOfChangeRule, ReassemblyConfig, Retention,
    SceType, TestVector, TestVectorValue, ThresholdMonitor, TimerModel, TlvOverflowPolicy,
    TlvTerminateStrategy, TransformModel, ValidatorModel, ValidatorRules, VariantArm, WorkerModel,
};
use sce_build::forge::pseudo::{render, Unsupported};
use sce_build::provenance::RequirementId;

fn field(id: &str, t: SceType, dir: Direction) -> ForgeField {
    ForgeField {
        id: id.to_string(),
        sce_type: t,
        direction: dir,
        expr: None,
        quantity: None,
        max_size: None,
        default_covers: Vec::new(),
        retain: None,
    }
}

/// Every statement form, every const form, and a test vector — one
/// document, compared whole.
#[test]
fn an_algorithm_renders_every_form_it_can_carry() {
    let m = AlgorithmModel {
        name: "crc".to_string(),
        signature: AlgorithmSignature {
            params: vec![
                AlgorithmParam {
                    name: "data".to_string(),
                    sce_type: SceType::Bytes,
                },
                AlgorithmParam {
                    name: "seed".to_string(),
                    sce_type: SceType::Uint16,
                },
            ],
            return_type: Some(SceType::Bytes),
            returns_max_size: Some(64),
        },
        consts: vec![
            AlgorithmConst {
                name: "POLY".to_string(),
                sce_type: AlgorithmConstType::Scalar(SceType::Uint16),
                init: Some("0x1021".to_string()),
                fold: None,
                compute_at_build: false,
            },
            AlgorithmConst {
                name: "TABLE".to_string(),
                sce_type: AlgorithmConstType::Array {
                    elem: SceType::Uint16,
                    len: 4,
                },
                init: None,
                fold: Some(FoldBody {
                    range_start: 0,
                    range_end: 4,
                    iter_var: "i".to_string(),
                    elem_type: SceType::Uint16,
                    body: vec![AlgorithmStmt::Var {
                        name: "acc".to_string(),
                        sce_type: SceType::Uint16,
                        init: "i".to_string(),
                        capacity: None,
                    }],
                    yield_expr: "acc".to_string(),
                }),
                compute_at_build: true,
            },
        ],
        body: vec![
            AlgorithmStmt::Var {
                name: "out".to_string(),
                sce_type: SceType::Bytes,
                init: "\"\"".to_string(),
                capacity: Some(32),
            },
            AlgorithmStmt::Assign {
                target: "crc".to_string(),
                expr: "seed".to_string(),
            },
            AlgorithmStmt::Append {
                target: "out".to_string(),
                expr: "crc".to_string(),
            },
            AlgorithmStmt::If {
                cond: "crc > 0".to_string(),
                then_body: vec![AlgorithmStmt::Return {
                    expr: Some("out".to_string()),
                }],
                else_body: Some(vec![AlgorithmStmt::Return { expr: None }]),
            },
            AlgorithmStmt::While {
                cond: "crc != 0".to_string(),
                body: vec![AlgorithmStmt::Call {
                    target: "step".to_string(),
                    args: vec!["crc".to_string(), "1".to_string()],
                }],
                max_iter: Some(16),
            },
            AlgorithmStmt::Foreach {
                item: "b".to_string(),
                source: "data".to_string(),
                body: vec![AlgorithmStmt::Assign {
                    target: "crc".to_string(),
                    expr: "crc ^ b".to_string(),
                }],
            },
            AlgorithmStmt::Return {
                expr: Some("out".to_string()),
            },
        ],
        test_vectors: vec![TestVector {
            hex: vec![0x01, 0xff],
            value: TestVectorValue::Uint(10673),
            source_line: 12,
        }],
        source_location: None,
    };

    let expected = "\
algorithm crc(data: bytes, seed: uint16) -> bytes returns-max 64
  const POLY: uint16 = 0x1021
  const TABLE: array<uint16, 4> = fold i in 0..4 -> uint16:
    var acc: uint16 = i
    yield acc
  var out: bytes cap 32 = \"\"
  crc = seed
  append out <- crc
  if crc > 0:
    return out
  else:
    return
  while crc != 0 max 16:
    call step(crc, 1)
  foreach b in data:
    crc = crc ^ b
  return out
  test 0x01ff -> uint 10673 @line 12
";

    assert_eq!(render(&ForgeDocument::Algorithm(m)).unwrap(), expected);
}

/// Every field clause, both state keywords, and both transition shapes.
#[test]
fn a_procedure_renders_every_form_it_can_carry() {
    let mut payload = field("payload", SceType::Bytes, Direction::In);
    payload.max_size = Some(64);

    let mut counter = field("counter", SceType::Uint16, Direction::Internal);
    counter.expr = Some("0".to_string());
    counter.retain = Some(Retention {
        scope: "nvm".to_string(),
        initial: "7".to_string(),
    });
    counter.default_covers = vec!["A".to_string(), "B".to_string()];

    let m = ProcedureModel {
        name: "unlock".to_string(),
        inputs: vec![field("seed", SceType::Uint32, Direction::In), payload],
        internals: vec![counter],
        helpers: vec![ProcedureHelper {
            name: "computeKey".to_string(),
            args: vec![SceType::Uint32, SceType::Bytes],
            returns: SceType::Bytes,
            returns_max_size: Some(8),
        }],
        initial: "request".to_string(),
        states: vec![
            ProcedureState {
                id: "request".to_string(),
                is_final: false,
                transitions: vec![
                    ProcedureTransition {
                        target: "granted".to_string(),
                        cond: Some("counter < 3".to_string()),
                        event: Some("reply".to_string()),
                        assigns: vec![ProcedureAssign {
                            location: "counter".to_string(),
                            expr: "counter + 1".to_string(),
                        }],
                        line: Some(9),
                    },
                    ProcedureTransition {
                        target: "denied".to_string(),
                        cond: None,
                        event: None,
                        assigns: Vec::new(),
                        line: None,
                    },
                ],
                on_entry_sends: vec![ProcedureSendAction {
                    service: "diag".to_string(),
                    subfunc: Some("0x27".to_string()),
                    addr: Some("seed".to_string()),
                    payload: Some("computeKey(seed, payload)".to_string()),
                    response_max_size: Some(16),
                }],
                done_params: Vec::new(),
                line: None,
            },
            ProcedureState {
                id: "granted".to_string(),
                is_final: true,
                transitions: Vec::new(),
                on_entry_sends: Vec::new(),
                done_params: vec![ProcedureDoneParam {
                    name: "key".to_string(),
                    expr: "counter".to_string(),
                }],
                line: None,
            },
        ],
        source_location: None,
    };

    let expected = "\
procedure unlock initial request
  in seed: uint32
  in payload: bytes max-size 64
  internal counter: uint16 = 0 retain nvm initial 7 default-covers A B
  helper computeKey(uint32, bytes) -> bytes returns-max 8
  state request:
    send diag subfunc 0x27 addr seed payload computeKey(seed, payload) response-max 16
    on reply when counter < 3 -> granted
      counter = counter + 1
    -> denied
  final granted:
    done key = counter
";

    assert_eq!(render(&ForgeDocument::Procedure(m)).unwrap(), expected);
}

/// A newline inside an author string must not become a line of its own.
///
/// This is the hazard `comment_text` was built for, measured there on
/// generated comments: an id or expression is opaque by contract and may
/// carry anything, including a newline the author wrote as `&#10;`.
/// Here the damage would be structural — the grammar reads indentation,
/// so an injected line is an injected statement.
#[test]
fn a_newline_in_an_expression_does_not_become_a_line() {
    let m = AlgorithmModel {
        name: "hostile".to_string(),
        signature: AlgorithmSignature {
            params: Vec::new(),
            return_type: None,
            returns_max_size: None,
        },
        consts: Vec::new(),
        body: vec![AlgorithmStmt::Assign {
            target: "a".to_string(),
            expr: "1\n  return 0".to_string(),
        }],
        test_vectors: Vec::new(),
        source_location: None,
    };

    let rendered = render(&ForgeDocument::Algorithm(m)).unwrap();
    assert_eq!(
        rendered.lines().count(),
        2,
        "one header plus one statement; got:\n{rendered}"
    );
    assert!(
        rendered.contains("a = 1\\x0A  return 0"),
        "the newline should survive as an escape, not as a line break; got:\n{rendered}"
    );
}

/// Each refused kind names itself.
///
/// The `match` in `render` is exhaustive, so a kind added to
/// `ForgeDocument` cannot be forgotten — but a copy-pasted arm returning
/// a neighbour's name compiles, and the refusal would then send a
/// reader to the wrong kind. Only the kinds a fixture can build cheaply
/// are listed; the assertion is on the name, not on the count.
#[test]
fn a_refused_kind_names_itself() {
    let doc = ForgeDocument::Statechart(Box::default());
    assert_eq!(
        render(&doc).unwrap_err(),
        Unsupported { kind: "statechart" }
    );
    assert!(render(&doc)
        .unwrap_err()
        .to_string()
        .contains("refused rather than abbreviated"));
}

/// The rendering is a pure function of the model.
///
/// ⚠ Weak on its own — two calls in one process would agree even if the
/// module read a hash-ordered container, because the seed is per
/// process. It is here for the other half: it fails if the renderer ever
/// reads a clock or a counter, which is the cheap mistake. The
/// container question is answered structurally instead — the module
/// holds no map at all.
#[test]
fn two_renderings_of_one_model_agree() {
    let m = AlgorithmModel {
        name: "stable".to_string(),
        signature: AlgorithmSignature {
            params: vec![AlgorithmParam {
                name: "x".to_string(),
                sce_type: SceType::Uint8,
            }],
            return_type: Some(SceType::Uint8),
            returns_max_size: None,
        },
        consts: Vec::new(),
        body: vec![AlgorithmStmt::Return {
            expr: Some("x".to_string()),
        }],
        test_vectors: Vec::new(),
        source_location: None,
    };
    let doc = ForgeDocument::Algorithm(m);
    assert_eq!(render(&doc).unwrap(), render(&doc).unwrap());
}

/// Each declarative kind, with every optional field populated.
///
/// Whole-output comparisons again, and for these kinds the reason is
/// sharper than elsewhere: their documents ARE their models — no
/// backend-derived field, nothing the parser computed — so a field
/// missing from the output is a field missing from the review, with
/// nothing downstream to notice.
#[test]
fn each_declarative_kind_renders_every_field_it_can_carry() {
    let cond = ConditionModel {
        name: "hot".to_string(),
        inputs: vec![field("t", SceType::Float64, Direction::In)],
        expr: "t > 90".to_string(),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Condition(cond)).unwrap(),
        "condition hot\n  in t: float64\n  when t > 90\n"
    );

    let mut out_field = field("c", SceType::Float64, Direction::Out);
    out_field.expr = Some("f * 2".to_string());
    let tr = TransformModel {
        name: "scale".to_string(),
        inputs: vec![field("f", SceType::Uint16, Direction::In)],
        outputs: vec![out_field],
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Transform(tr)).unwrap(),
        "transform scale\n  in f: uint16\n  out c: float64 = f * 2\n"
    );

    let val = ValidatorModel {
        name: "plaus".to_string(),
        inputs: vec![field("v", SceType::Int32, Direction::In)],
        rules: ValidatorRules {
            ranges: vec![RangeRule {
                id: "v".to_string(),
                min: Some("-10".to_string()),
                max: Some("10".to_string()),
            }],
            rate_of_changes: vec![RateOfChangeRule {
                id: "v".to_string(),
                max_delta: "5".to_string(),
                sample_interval_ms: 100,
            }],
            plausibility: Some("v != 0".to_string()),
        },
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Validator(val)).unwrap(),
        "validator plaus\n  in v: int32\n  range v min -10 max 10\n  \
         rate v max-delta 5 interval 100ms\n  plausibility v != 0\n"
    );

    let ev = EventSchemaModel {
        name: "tick".to_string(),
        event_name: "bus.tick".to_string(),
        fields: vec![field("seq", SceType::Uint8, Direction::In)],
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::EventSchema(ev)).unwrap(),
        "event-schema tick event bus.tick\n  in seq: uint8\n"
    );

    let en = EnumModel {
        name: "nrc".to_string(),
        underlying_type: SceType::Uint8,
        variants: vec![EnumVariant {
            name: "reject".to_string(),
            value: 16,
            source_line: Some(29),
        }],
        strict_variants: true,
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Enum(en)).unwrap(),
        "enum nrc: uint8 strict\n  variant reject = 16 @line 29\n"
    );

    let ti = TimerModel {
        name: "sched".to_string(),
        period_us: 2_000_000,
        reset_on_event: Some("beat".to_string()),
        cancel_on_state_exit: Some("idle".to_string()),
        fire_event: "tick".to_string(),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Timer(ti)).unwrap(),
        "timer sched period 2000000us fire tick reset-on beat cancel-on-exit idle\n"
    );

    let lk = LookupModel {
        name: "gear".to_string(),
        input: field("raw", SceType::Uint8, Direction::In),
        output: field("name", SceType::String, Direction::Out),
        entries: vec![LookupEntry {
            key: "0".to_string(),
            value: "PARK".to_string(),
            requirements: vec![RequirementId("REQ-1".to_string())],
        }],
        miss_policy: MissPolicy::Default("NEUTRAL".to_string()),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Lookup(lk)).unwrap(),
        "lookup gear\n  in raw: uint8\n  out name: string\n  \
         0 -> PARK req REQ-1\n  miss default NEUTRAL\n"
    );
}

/// The four signal-shaped kinds, with every optional field populated.
#[test]
fn each_signal_kind_renders_every_field_it_can_carry() {
    let fi = FilterModel {
        name: "smooth".to_string(),
        input: field("raw", SceType::Float64, Direction::In),
        output: field("out", SceType::Float64, Direction::Out),
        filter_type: FilterType::LowPass,
        window: Some(4),
        alpha: Some(0.25),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Filter(fi)).unwrap(),
        "filter smooth low-pass window 4 alpha 0.25\n  \
         in raw: float64\n  out out: float64\n"
    );

    let ob = ObserverModel {
        name: "heat".to_string(),
        inputs: vec![field("t", SceType::Float64, Direction::In)],
        monitors: vec![ThresholdMonitor {
            id: "alarm".to_string(),
            enter_expr: "t > 110".to_string(),
            leave_expr: Some("t < 100".to_string()),
            on_enter: "raise".to_string(),
            on_leave: Some("clear".to_string()),
        }],
        event_domain: Some("diag".to_string()),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Observer(ob)).unwrap(),
        "observer heat domain diag\n  in t: float64\n  \
         monitor alarm enter t > 110 on-enter raise leave t < 100 on-leave clear\n"
    );

    let ip = InterpolationModel {
        name: "map".to_string(),
        inputs: vec![field("rpm", SceType::Uint16, Direction::In)],
        output: field("ms", SceType::Float64, Direction::Out),
        method: InterpolationMethod::Linear,
        out_of_bounds: OutOfBounds::Extrapolate,
        axes: vec![InterpolationAxis {
            input_id: "rpm".to_string(),
            breakpoints: vec![800.0, 1200.5],
        }],
        values: vec![2.1, 4.5],
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Interpolation(ip)).unwrap(),
        "interpolation map method linear out-of-bounds extrapolate\n  \
         in rpm: uint16\n  out ms: float64\n  \
         axis rpm breakpoints 800 1200.5\n  values 2.1 4.5\n"
    );

    let bc = BoundedCollectionModel {
        name: "table".to_string(),
        element_type: "entry".to_string(),
        capacity: CapacitySource::DeployKey {
            key: "tables.max".to_string(),
        },
        index_by: Some("id".to_string()),
        on_overflow: OverflowPolicy::OldestWins,
        ordering: CollectionOrdering::SortedByIndex,
        concurrency: ConcurrencyMode::MultiWriter,
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::BoundedCollection(bc)).unwrap(),
        "bounded-collection table of entry capacity deploy-key tables.max \
         overflow oldest-wins ordering sorted-by-index concurrency \
         multi-writer index-by id\n"
    );
}

/// The codec kind, with every optional field of its widest type set.
///
/// `CodecField` carries eighteen fields, which is why the rendering is
/// a block rather than a line and why this assertion is written whole:
/// with eighteen optionals, a `contains` check would pass while most of
/// them were missing.
#[test]
fn a_codec_renders_every_field_it_can_carry() {
    let f = CodecField {
        id: "payload".to_string(),
        sce_type: SceType::Bytes,
        byte_offset: 4,
        bit_offset: Some(2),
        bit_size: BitSize::TlvChain {
            max_depth: 3,
            on_overflow: TlvOverflowPolicy::Truncate,
            terminate_on: TlvTerminateStrategy::EntryFlag {
                flag_name: "more".to_string(),
            },
        },
        endian: Some(Endian::Little),
        max_size: Some(64),
        length_field: Some("len".to_string()),
        flags: vec![FlagDef {
            name: "more".to_string(),
            bit: 7,
            width: 1,
            value: Some(1),
        }],
        present_if: Some(PresentIfPredicate {
            scope: PresentIfScope::Input,
            field_id: "hdr".to_string(),
            flag_name: "ext".to_string(),
            negate: true,
            or_with: Some(Box::new(PresentIfPredicate {
                scope: PresentIfScope::Local,
                field_id: "hdr".to_string(),
                flag_name: "alt".to_string(),
                negate: false,
                or_with: None,
            })),
        }),
        repeat_body_alias: Some("rb".to_string()),
        max_count: Some(9),
        tlv_chain_body_alias: Some("tb".to_string()),
        dma_burst_align: Some(16),
        embed_body_alias: Some("eb".to_string()),
        embed_length_from: Some("len".to_string()),
        length_arith: Some(-2),
        quantity: None,
    };

    let m = CodecModel {
        name: "env".to_string(),
        default_endian: Endian::Big,
        input_length: Some(32),
        fields: vec![f],
        variant: Some(CodecVariant {
            tag_field: Some("hdr".to_string()),
            tag_flag: Some("mid".to_string()),
            arms: vec![VariantArm {
                value: 1,
                body_alias: "one".to_string(),
                is_default: false,
            }],
            default_arm: Some(VariantArm {
                value: 0,
                body_alias: "zero".to_string(),
                is_default: true,
            }),
            peek_byte: Some(PeekByteSpec {
                id: "pk".to_string(),
                flags: vec![FlagDef {
                    name: "k".to_string(),
                    bit: 0,
                    width: 2,
                    value: None,
                }],
            }),
        }),
        flag_inputs: vec![FlagInput {
            name: "hdr".to_string(),
            width: 8,
        }],
        test_vectors: vec![CodecTestVector {
            hex: vec![0xab],
            decoded: DecodedValue::Plain {
                fields: vec![DecodedField {
                    name: "payload".to_string(),
                    value: DecodedFieldValue::Bytes(vec![0x01]),
                }],
            },
            source_line: 7,
        }],
        source_location: None,
    };

    let expected = "\
codec env endian big input-length 32
  flag-input hdr width 8
  field payload: bytes at 4.2 size tlv-chain max-depth 3 on-overflow truncate \
terminate entry-flag more
    endian little
    max-size 64
    length-field len
    length-arith -2
    max-count 9
    repeat-body rb
    tlv-body tb
    embed-body eb
    embed-length-from len
    dma-align 16
    present-if not input:hdr.ext or local:hdr.alt
    flag more bit 7 width 1 value 1
  variant tag-field hdr tag-flag mid peek-byte pk
    peek-flag k bit 0 width 2
    arm 1 -> one
    default-arm 0 -> zero default
  test 0xab @line 7
    payload = bytes 0x01
";

    assert_eq!(render(&ForgeDocument::Codec(m)).unwrap(), expected);
}

/// The three MCU kinds, with every optional field populated.
///
/// The enum spellings asserted here are the grammar's
/// (`xs:enumeration`), not serde's: `raw_eth` and `acq_rel` are snake
/// while `signal-event` and `non-cacheable` are kebab, and rendering
/// either family by the Rust enum's `rename_all` would print a word no
/// author ever wrote.
#[test]
fn each_mcu_kind_renders_every_field_it_can_carry() {
    let wk = WorkerModel {
        name: "rx".to_string(),
        link_rx: "udp0".to_string(),
        inbox: InboxConfig {
            depth: 16,
            ordering: InboxOrdering::AcqRel,
        },
        outbox: Some("tx0".to_string()),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Worker(wk)).unwrap(),
        "worker rx link-rx udp0 inbox depth 16 ordering acq_rel outbox tx0\n"
    );

    let bp = BufferPoolModel {
        name: "pool".to_string(),
        slot_count: 8,
        slot_size: 256,
        section: "sram1".to_string(),
        alignment: 32,
        dma_channel: Some("ch3".to_string()),
        cache_policy: CachePolicy::NonCacheable,
        variant: BufferPoolVariant::Reassembly(ReassemblyConfig {
            max_fragments_per_message: 4,
            reassembly_timeout_ms: 500,
            per_peer_quota: 2,
        }),
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::BufferPool(bp)).unwrap(),
        "buffer-pool pool slots 8 size 256 section sram1 align 32 \
         cache non-cacheable dma ch3\n  \
         reassembly max-fragments 4 timeout 500ms per-peer-quota 2\n"
    );

    let lk = LinkModel {
        name: "eth".to_string(),
        class: LinkClass::RawEth,
        framer: "frame".to_string(),
        backpressure: BackpressurePolicy::SignalEvent,
        inbound: vec![LinkInboundEvent {
            event: "rx".to_string(),
            when: Some("len > 0".to_string()),
        }],
        outbound: vec![LinkOutboundEvent {
            event: "tx".to_string(),
            encode: "enc".to_string(),
        }],
        rx_pool: Some("rp".to_string()),
        tx_pool: Some("tp".to_string()),
        stage_pool: Some("sp".to_string()),
        accept_stage_copy_rate: true,
        source_location: None,
    };
    assert_eq!(
        render(&ForgeDocument::Link(lk)).unwrap(),
        "link eth class raw_eth framer frame backpressure signal-event \
         accept-stage-copy-rate\n  rx-pool rp\n  tx-pool tp\n  \
         stage-pool sp\n  inbound rx when len > 0\n  \
         outbound tx encode enc\n"
    );
}
