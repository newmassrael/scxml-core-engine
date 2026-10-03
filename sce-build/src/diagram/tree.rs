// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A serialised document with its fields in the order the model declares
//! them.
//!
//! `serde_json::Value` keeps an object's keys sorted, which suits a wire
//! format and ruins a page: a lookup's `name`, `key_type` and `entries`
//! would read in the alphabet's order and not in the order the model
//! states them. This tree is what the document's own `Serialize` writes —
//! the same names, the same tags, the same skipped fields as
//! `sce-codegen --emit-ast` — with a record's fields left where the model
//! put them. A map is the one exception: its entries are sorted by key, so
//! a map held in a hash table cannot make one document two sheets.

use serde::ser::{self, Serialize};
use std::fmt::Display;

/// One value of a serialised document.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Bool(bool),
    /// A number as its decimal text, so no digit is rounded on the way.
    Number(String),
    Text(String),
    List(Vec<Node>),
    /// Named values in declaration order.
    Record(Vec<(String, Node)>),
}

impl Node {
    /// A value one table cell holds: a scalar, or a list or record with
    /// nothing in it.
    pub fn is_inline(&self) -> bool {
        match self {
            Node::List(items) => items.is_empty(),
            Node::Record(fields) => fields.is_empty(),
            _ => true,
        }
    }

    /// This tree without the fields named in `keys`, at any depth.
    pub fn without(self, keys: &[&str]) -> Node {
        match self {
            Node::List(items) => Node::List(items.into_iter().map(|n| n.without(keys)).collect()),
            Node::Record(fields) => Node::Record(
                fields
                    .into_iter()
                    .filter(|(k, _)| !keys.contains(&k.as_str()))
                    .map(|(k, v)| (k, v.without(keys)))
                    .collect(),
            ),
            other => other,
        }
    }
}

/// Why a value could not be read as a tree.
#[derive(Debug)]
pub struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: Display>(message: T) -> Self {
        Error(message.to_string())
    }
}

/// `value` as a tree.
pub fn of<T: Serialize + ?Sized>(value: &T) -> Result<Node, Error> {
    value.serialize(Builder)
}

struct Builder;

struct Seq(Vec<Node>);

struct Variant<T> {
    name: &'static str,
    inner: T,
}

struct Fields(Vec<(String, Node)>);

struct Entries {
    entries: Vec<(String, Node)>,
    pending: Option<String>,
}

fn number(text: String) -> Node {
    Node::Number(text)
}

impl ser::Serializer for Builder {
    type Ok = Node;
    type Error = Error;
    type SerializeSeq = Seq;
    type SerializeTuple = Seq;
    type SerializeTupleStruct = Seq;
    type SerializeTupleVariant = Variant<Seq>;
    type SerializeMap = Entries;
    type SerializeStruct = Fields;
    type SerializeStructVariant = Variant<Fields>;

    fn serialize_bool(self, v: bool) -> Result<Node, Error> {
        Ok(Node::Bool(v))
    }
    fn serialize_i8(self, v: i8) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_i16(self, v: i16) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_i32(self, v: i32) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_i64(self, v: i64) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_i128(self, v: i128) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_u8(self, v: u8) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_u16(self, v: u16) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_u32(self, v: u32) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_u64(self, v: u64) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_u128(self, v: u128) -> Result<Node, Error> {
        Ok(number(v.to_string()))
    }
    fn serialize_f32(self, v: f32) -> Result<Node, Error> {
        self.serialize_f64(v.to_string().parse().unwrap_or(f64::from(v)))
    }
    /// What `serde_json` does: a number that is not finite has no JSON
    /// spelling and is null.
    fn serialize_f64(self, v: f64) -> Result<Node, Error> {
        Ok(match serde_json::Number::from_f64(v) {
            Some(n) => number(n.to_string()),
            None => Node::Null,
        })
    }
    fn serialize_char(self, v: char) -> Result<Node, Error> {
        Ok(Node::Text(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Node, Error> {
        Ok(Node::Text(v.to_string()))
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Node, Error> {
        Ok(Node::List(
            v.iter().map(|b| number(b.to_string())).collect(),
        ))
    }
    fn serialize_none(self) -> Result<Node, Error> {
        Ok(Node::Null)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Node, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Node, Error> {
        Ok(Node::Null)
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Node, Error> {
        Ok(Node::Null)
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Node, Error> {
        Ok(Node::Text(variant.to_string()))
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<Node, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Node, Error> {
        Ok(Node::Record(vec![(
            variant.to_string(),
            value.serialize(Builder)?,
        )]))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Seq, Error> {
        Ok(Seq(Vec::with_capacity(len.unwrap_or(0))))
    }
    fn serialize_tuple(self, len: usize) -> Result<Seq, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Seq, Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Variant<Seq>, Error> {
        Ok(Variant {
            name: variant,
            inner: Seq(Vec::with_capacity(len)),
        })
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Entries, Error> {
        Ok(Entries {
            entries: Vec::with_capacity(len.unwrap_or(0)),
            pending: None,
        })
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Fields, Error> {
        Ok(Fields(Vec::with_capacity(len)))
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Variant<Fields>, Error> {
        Ok(Variant {
            name: variant,
            inner: Fields(Vec::with_capacity(len)),
        })
    }
}

impl ser::SerializeSeq for Seq {
    type Ok = Node;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.0.push(value.serialize(Builder)?);
        Ok(())
    }
    fn end(self) -> Result<Node, Error> {
        Ok(Node::List(self.0))
    }
}

impl ser::SerializeTuple for Seq {
    type Ok = Node;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Node, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for Seq {
    type Ok = Node;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Node, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for Variant<Seq> {
    type Ok = Node;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(&mut self.inner, value)
    }
    fn end(self) -> Result<Node, Error> {
        Ok(Node::Record(vec![(
            self.name.to_string(),
            Node::List(self.inner.0),
        )]))
    }
}

impl ser::SerializeStruct for Fields {
    type Ok = Node;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.0.push((key.to_string(), value.serialize(Builder)?));
        Ok(())
    }
    fn end(self) -> Result<Node, Error> {
        Ok(Node::Record(self.0))
    }
}

impl ser::SerializeStructVariant for Variant<Fields> {
    type Ok = Node;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
    }
    fn end(self) -> Result<Node, Error> {
        Ok(Node::Record(vec![(
            self.name.to_string(),
            Node::Record(self.inner.0),
        )]))
    }
}

impl ser::SerializeMap for Entries {
    type Ok = Node;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        match key.serialize(Builder)? {
            Node::Text(s) | Node::Number(s) => {
                self.pending = Some(s);
                Ok(())
            }
            Node::Bool(b) => {
                self.pending = Some(b.to_string());
                Ok(())
            }
            other => Err(Error(format!("a map key must be text, got {other:?}"))),
        }
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .pending
            .take()
            .ok_or_else(|| Error("a map value came before its key".to_string()))?;
        self.entries.push((key, value.serialize(Builder)?));
        Ok(())
    }
    fn end(mut self) -> Result<Node, Error> {
        self.entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Node::Record(self.entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::collections::{BTreeMap, HashMap};

    #[derive(Serialize)]
    struct Inner {
        zebra: u8,
        apple: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum Tagged {
        First(Inner),
        Second { value: f64 },
    }

    #[derive(Serialize)]
    struct Outer {
        zeta: Inner,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        skipped: Vec<u8>,
        alpha: Vec<Tagged>,
        #[serde(rename = "renamed-field")]
        beta: bool,
    }

    fn sample() -> Outer {
        Outer {
            zeta: Inner {
                zebra: 7,
                apple: None,
            },
            skipped: Vec::new(),
            alpha: vec![
                Tagged::First(Inner {
                    zebra: 1,
                    apple: Some("x".into()),
                }),
                Tagged::Second { value: 0.5 },
            ],
            beta: true,
        }
    }

    fn keys(node: &Node) -> Vec<&str> {
        match node {
            Node::Record(f) => f.iter().map(|(k, _)| k.as_str()).collect(),
            other => panic!("{other:?}"),
        }
    }

    /// Fields come in the order the model declares them, the names and
    /// skips are the document's own, and a tag leads its enum's fields —
    /// where `serde_json::Value` would have sorted them all.
    #[test]
    fn fields_keep_declaration_order_and_the_documents_own_names() {
        let tree = of(&sample()).expect("reads");
        assert_eq!(keys(&tree), ["zeta", "alpha", "renamed-field"]);
        let Node::Record(fields) = &tree else {
            unreachable!()
        };
        assert_eq!(keys(&fields[0].1), ["zebra", "apple"]);
        let Node::List(items) = &fields[1].1 else {
            panic!()
        };
        assert_eq!(keys(&items[0]), ["kind", "zebra", "apple"]);
        assert_eq!(keys(&items[1]), ["kind", "value"]);
        assert_eq!(fields[0].1, of(&sample().zeta).unwrap());
    }

    /// The tree says what `serde_json` says, field for field: reading the
    /// JSON back and sorting both sides agrees.
    #[test]
    fn the_tree_agrees_with_the_documents_json() {
        fn json(node: &Node) -> serde_json::Value {
            match node {
                Node::Null => serde_json::Value::Null,
                Node::Bool(b) => (*b).into(),
                Node::Number(n) => serde_json::from_str(n).expect("a number"),
                Node::Text(s) => s.clone().into(),
                Node::List(l) => l.iter().map(json).collect(),
                Node::Record(f) => f.iter().map(|(k, v)| (k.clone(), json(v))).collect(),
            }
        }
        let value = sample();
        assert_eq!(
            json(&of(&value).expect("reads")),
            serde_json::to_value(&value).expect("json")
        );
    }

    /// A map is sorted by key, so a hash table cannot reorder a sheet.
    #[test]
    fn a_map_is_sorted_by_key_whatever_its_own_order() {
        let hashed: HashMap<String, u8> = (0..20).map(|i| (format!("k{:02}", 19 - i), i)).collect();
        let tree = of(&hashed).expect("reads");
        let Node::Record(entries) = &tree else {
            panic!()
        };
        let got: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
        let mut sorted = got.clone();
        sorted.sort_unstable();
        assert_eq!(got, sorted);
        let ordered: BTreeMap<String, u8> = hashed.into_iter().collect();
        assert_eq!(of(&ordered).expect("reads"), tree);
    }

    /// A number that is not finite has no JSON spelling and is null; a
    /// float keeps its shortest round-trip digits.
    #[test]
    fn numbers_keep_their_digits() {
        assert_eq!(of(&f64::NAN).unwrap(), Node::Null);
        assert_eq!(of(&0.1f64).unwrap(), Node::Number("0.1".into()));
        assert_eq!(of(&0.1f32).unwrap(), Node::Number("0.1".into()));
        assert_eq!(of(&u64::MAX).unwrap(), Node::Number(u64::MAX.to_string()));
        assert_eq!(of(&i128::MIN).unwrap(), Node::Number(i128::MIN.to_string()));
    }

    /// Dropping named fields reaches every depth and nothing else.
    #[test]
    fn without_drops_the_named_fields_at_any_depth() {
        let tree = of(&sample()).expect("reads").without(&["apple", "kind"]);
        let Node::Record(fields) = &tree else {
            panic!()
        };
        assert_eq!(keys(&fields[0].1), ["zebra"]);
        let Node::List(items) = &fields[1].1 else {
            panic!()
        };
        assert_eq!(keys(&items[0]), ["zebra"]);
        assert_eq!(keys(&items[1]), ["value"]);
    }

    /// Inline is what one cell can hold.
    #[test]
    fn only_a_non_empty_list_or_record_is_not_inline() {
        assert!(Node::Null.is_inline() && Node::List(vec![]).is_inline());
        assert!(Node::Record(vec![]).is_inline());
        assert!(!Node::List(vec![Node::Null]).is_inline());
        assert!(!Node::Record(vec![("a".into(), Node::Null)]).is_inline());
    }
}
