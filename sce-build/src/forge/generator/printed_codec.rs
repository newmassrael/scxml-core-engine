// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a reader in another document knows of a codec a backend prints from
//! its plan (docs/adr/0081 step 4), one shape for every backend whose value is
//! read through accessors and made by one constructor: what the constructor
//! takes, in its order, how each element of it is made of what an expression
//! gives, and each number the encode derives into a field, which the value
//! holds no field for (§3) and which the backend answers by a function or a
//! method of its own.
//!
//! A printer builds it from the plan, spelling each accessor and each answer
//! as its value presents them (`cpp_source::printed_codec`,
//! `go_source::printed_codec`); a procedure, a validator and an algorithm that
//! hold a value of the codec read it and make it again through it, each
//! backend spelling the read and the call by its own rule
//! (`PrintedCodec::cpp_read`, `PrintedCodec::go_read` and their rebuilds). What
//! is the same on every backend, which field a reader may name, which
//! assignment is a rebuild and which is refused, and the order of the
//! constructor's arguments, is decided here once.

use crate::forge::encode_plan::Value;

/// What a reader in another document knows of a codec a backend prints from
/// its plan: the parts its constructor takes and the numbers its encode
/// derives. Held by an import of the codec ([`super::ImportContext::printed`])
/// and by a compile's options for the element types an algorithm iterates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrintedCodec {
    /// What the constructor takes, in its order.
    pub(super) parts: Vec<PrintedPart>,
    /// Each number the encode derives into a field, each field once.
    pub(super) numbers: Vec<DerivedNumber>,
}

/// One thing a printed value's constructor takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PrintedPart {
    /// An element the value presents by an accessor of its own.
    Element {
        /// The field id.
        id: String,
        /// The accessor's name, as the backend spells it.
        accessor: String,
        /// How a value of it is made of what an expression gives.
        made: PrintedMade,
    },
    /// A group of elements one gate decides, or the arm of the variant: the
    /// value presents it whole by the accessor `accessor`, and no member of it
    /// is an element of its own.
    Whole {
        /// The accessor's name, as the backend spells it.
        accessor: String,
    },
}

/// How a printed value's element is made of what an expression gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrintedMade {
    /// Handed as the expression gives it: an integer, a flag set or an enum,
    /// or the optional type of one.
    AsGiven,
    /// A byte string, or a text where `text`, made of what the expression
    /// gives by the constructor of the bounded type the accessor presents it
    /// as, which refuses one past its bound or a text that is not UTF-8; and
    /// where `optional`, a gate decides it and the accessor presents the
    /// optional type of that bounded type.
    Bounded { text: bool, optional: bool },
    /// A body or a list, which no expression gives: `why` says what it is.
    Unmade { why: &'static str },
}

/// A number the encode of a printed codec derives into a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DerivedNumber {
    /// The field id.
    pub(super) id: String,
    /// The function (C++) or the method (Go) that answers it as the frame
    /// carries it, read off a value of the codec.
    pub(super) by: String,
    /// Whether a reader that holds a value, and nothing the encode is handed
    /// beside it, is answered it: a number whose measures a source does not
    /// say all of, as the length of a body handed an input of the encode's,
    /// is answered to no reader.
    pub(super) answered: bool,
}

/// How a reader reads a field of a printed value
/// ([`PrintedCodec::reading`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading<'p> {
    /// A number the encode derives, by what answers it.
    Number(&'p str),
    /// An element, by its accessor, made as `made` says.
    Element {
        accessor: &'p str,
        made: PrintedMade,
    },
}

/// A rebuild of a printed codec's value with one element assigned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrintedRebuild {
    /// The call of the constructor.
    pub(crate) value: String,
    /// For a byte string or a text, the call that makes the bounded value of
    /// what is assigned, which can fail and which `value` reads through the
    /// backend's local of it, and the value assigned, which the backend holds
    /// in a local of its own first and which the call reads there; `None`
    /// where the element is handed as the expression gives it.
    pub(crate) made: Option<(String, String)>,
}

impl PrintedMade {
    /// How an element that holds `value` is made, `optional` where a gate
    /// decides it.
    pub(super) fn of(value: &Value<'_>, optional: bool) -> Self {
        match value {
            Value::Bytes(_) => PrintedMade::Bounded {
                text: false,
                optional,
            },
            Value::Text(_) => PrintedMade::Bounded {
                text: true,
                optional,
            },
            Value::Body { .. } => PrintedMade::Unmade {
                why: "the body of a codec it holds",
            },
            Value::List { .. } => PrintedMade::Unmade {
                why: "a list of the entries of a codec it holds",
            },
            Value::Scalar(_) => PrintedMade::AsGiven,
        }
    }
}

impl PrintedCodec {
    /// Whether a reader may name the field `id` of a value of the codec: a
    /// number the encode derives that a reader holding the value is answered,
    /// or an element the value presents by an accessor of its own. A member of
    /// a group, which the value presents whole, is not, nor is a number no
    /// reader is answered.
    pub(crate) fn reads(&self, id: &str) -> bool {
        self.reading(id).is_some()
    }

    /// How a reader reads the field `id`, or `None` where it may not name it
    /// ([`PrintedCodec::reads`]). A number the encode derives is read by what
    /// answers it, since the value holds no field for it (docs/adr/0081 §3),
    /// and a length the value states while its content is absent as the
    /// number the frame carries, which that answers too.
    pub(super) fn reading(&self, id: &str) -> Option<Reading<'_>> {
        if let Some(number) = self.numbers.iter().find(|number| number.id == id) {
            return number
                .answered
                .then_some(Reading::Number(number.by.as_str()));
        }
        self.element(id)
            .map(|(accessor, made)| Reading::Element { accessor, made })
    }

    /// The accessor of the element the value presents as `id`, and how it is
    /// made; `None` where it presents no such element of its own.
    fn element(&self, id: &str) -> Option<(&str, PrintedMade)> {
        self.parts.iter().find_map(|part| match part {
            PrintedPart::Element {
                id: element,
                accessor,
                made,
            } if element == id => Some((accessor.as_str(), *made)),
            _ => None,
        })
    }

    /// The rebuild of the value `held`, made by `constructor`, with the
    /// element `element` assigned `value` and everything else the constructor
    /// takes read by its accessor, in the constructor's order: a group and
    /// the arm of the variant whole. An element handed as the expression gives
    /// it is the argument `value`; a byte string or a text is made into its
    /// bounded value first, by the call `bounded` spells for its accessor,
    /// whether it is a text and whether a gate decides it, answered with the
    /// argument the constructor reads the made value by.
    ///
    /// Refused, with why, where the value presents no `element` of its own: a
    /// number the encode derives, which no value holds, a member of a group,
    /// and a body or a list, which no expression gives.
    pub(super) fn rebuilt_by(
        &self,
        held: &str,
        constructor: &str,
        element: &str,
        value: &str,
        bounded: impl Fn(&str, bool, bool) -> (String, String),
    ) -> Result<PrintedRebuild, String> {
        let Some((accessor, made)) = self.element(element) else {
            return Err(if self.numbers.iter().any(|number| number.id == element) {
                format!(
                    "the encode derives `{element}` from what it measures (docs/adr/0081 §3), \
                         so its value holds no `{element}` to assign"
                )
            } else {
                format!(
                    "`{element}` is a member of a group the value presents whole, so it is \
                         not assigned alone"
                )
            });
        };
        let (made, assigned) = match made {
            PrintedMade::AsGiven => (None, value.to_string()),
            PrintedMade::Bounded { text, optional } => {
                let (call, assigned) = bounded(accessor, text, optional);
                (Some((call, value.to_string())), assigned)
            }
            PrintedMade::Unmade { why } => {
                return Err(format!(
                    "`{element}` is {why}, which the value is made of whole and no expression \
                     gives"
                ))
            }
        };
        let arguments: Vec<String> = self
            .parts
            .iter()
            .map(|part| match part {
                PrintedPart::Element { id, .. } if id == element => assigned.clone(),
                PrintedPart::Element { accessor, .. } | PrintedPart::Whole { accessor } => {
                    format!("{held}.{accessor}()")
                }
            })
            .collect();
        Ok(PrintedRebuild {
            value: format!("{constructor}({})", arguments.join(", ")),
            made,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A codec whose value presents an integer `kind`, a byte string `blob`
    /// a gate decides, a text `name`, a body `inner` and a group `opts`, in
    /// that order, and whose encode derives `len`, answered to a reader, and
    /// `size`, answered to none; `stated` is a length the value presents and
    /// the encode answers too. The accessors are spelled as Go spells them.
    fn codec() -> PrintedCodec {
        let element = |id: &str, made: PrintedMade| PrintedPart::Element {
            id: id.to_string(),
            accessor: crate::filters::to_pascal_case(id.to_string()),
            made,
        };
        PrintedCodec {
            parts: vec![
                element("kind", PrintedMade::AsGiven),
                element(
                    "blob",
                    PrintedMade::Bounded {
                        text: false,
                        optional: true,
                    },
                ),
                element(
                    "name",
                    PrintedMade::Bounded {
                        text: true,
                        optional: false,
                    },
                ),
                element(
                    "inner",
                    PrintedMade::Unmade {
                        why: "the body of a codec it holds",
                    },
                ),
                element("stated", PrintedMade::AsGiven),
                PrintedPart::Whole {
                    accessor: "FlagsOpts".to_string(),
                },
            ],
            numbers: vec![
                DerivedNumber {
                    id: "len".to_string(),
                    by: "WireLen".to_string(),
                    answered: true,
                },
                DerivedNumber {
                    id: "size".to_string(),
                    by: "WireSize".to_string(),
                    answered: false,
                },
                DerivedNumber {
                    id: "stated".to_string(),
                    by: "WireStated".to_string(),
                    answered: true,
                },
            ],
        }
    }

    /// A reader names an element and an answered number, and neither a
    /// number no reader is answered nor a member of a group; a length the
    /// value states is read as the frame carries it.
    ///
    /// Turns red where `PrintedCodec::reading` reads an element before a
    /// number of its id, or offers a number whose `answered` is false.
    #[test]
    fn a_reader_names_what_the_value_presents_and_what_it_is_answered() {
        let printed = codec();
        assert!(printed.reads("kind") && printed.reads("len"));
        assert!(!printed.reads("size"), "no reader is answered `size`");
        assert!(!printed.reads("opt_a"), "a member of a group is read whole");
        assert_eq!(
            printed.reading("stated"),
            Some(Reading::Number("WireStated"))
        );
        assert_eq!(
            printed.go_read("p.f", "stated").as_deref(),
            Some("p.f.WireStated()")
        );
        assert_eq!(printed.go_read("p.f", "size"), None);
    }

    /// Go reads a byte string or a text as what its bounded value holds, the
    /// optional type of it where a gate decides it; C++ reads a number by the
    /// function of the codec's namespace and an element by its accessor.
    ///
    /// Turns red where `PrintedCodec::go_read` reads a bounded element by its
    /// method alone, or where `PrintedCodec::cpp_read` reads a number off the
    /// value.
    #[test]
    fn each_backend_spells_the_read_by_its_own_rule() {
        let printed = codec();
        assert_eq!(
            printed.go_read("p.f", "blob").as_deref(),
            Some("codec.OptionalBytes(p.f.Blob())")
        );
        assert_eq!(
            printed.go_read("p.f", "name").as_deref(),
            Some("p.f.Name().String()")
        );
        assert_eq!(
            printed.go_read("p.f", "kind").as_deref(),
            Some("p.f.Kind()")
        );
        assert_eq!(
            printed.cpp_read("f_", "::Ns", "len").as_deref(),
            Some("::Ns::WireLen(f_)")
        );
        assert_eq!(
            printed.cpp_read("f_", "::Ns", "kind").as_deref(),
            Some("f_.Kind()")
        );
    }

    /// A rebuild hands the constructor every part in its order, the assigned
    /// element where it stands, a bounded one as the local the backend makes
    /// it in, and a group whole.
    ///
    /// Turns red where `PrintedCodec::rebuilt_by` drops a whole part, reorders
    /// the parts, or hands a bounded element as the expression gives it.
    #[test]
    fn a_rebuild_hands_the_constructor_each_part_in_its_order() {
        let printed = codec();
        let rebuilt = printed
            .rebuilt_by("h", "Make", "kind", "v", |_, _, _| unreachable!())
            .expect("an integer is assigned");
        assert_eq!(
            rebuilt.value,
            "Make(v, h.Blob(), h.Name(), h.Inner(), h.Stated(), h.FlagsOpts())"
        );
        assert_eq!(rebuilt.made, None);
        let rebuilt = printed
            .go_rebuilt("h", "pkg.MakeT", Some("pkg"), "T", "blob", "v")
            .expect("a byte string is assigned");
        assert_eq!(
            rebuilt.value,
            "pkg.MakeT(h.Kind(), sceMade, h.Name(), h.Inner(), h.Stated(), h.FlagsOpts())"
        );
        assert_eq!(
            rebuilt.made,
            Some((
                "codec.MakeOptionalBytes[pkg.TBlobBound](sceValue)".to_string(),
                "v".to_string()
            ))
        );
    }

    /// A rebuild is refused, naming why, for a number the encode derives, a
    /// member of a group and a body, which no expression gives.
    ///
    /// Turns red where `PrintedCodec::rebuilt_by` hands a body as the
    /// expression gives it, or names a derived number as a member of a group.
    #[test]
    fn a_rebuild_of_what_the_value_does_not_present_is_refused_by_why() {
        let printed = codec();
        let refused = |element: &str| {
            printed
                .rebuilt_by("h", "Make", element, "v", |_, _, _| unreachable!())
                .expect_err("the assign is refused")
        };
        assert!(refused("len").contains("the encode derives `len`"));
        assert!(refused("size").contains("the encode derives `size`"));
        assert!(refused("opt_a").contains("member of a group"));
        assert!(refused("inner").contains("the body of a codec it holds"));
    }
}
