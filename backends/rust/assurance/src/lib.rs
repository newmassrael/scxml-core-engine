// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A seeded mutator for what a parser is given.
//!
//! [`mutate`] changes bytes the way a bad disk, a bad peer and a bad day do: a bit, a byte, a cut, a
//! hole, a repeat, a splice from another input, a number that does not fit. [`mutate_json`] changes a
//! JSON value by its structure, so that what it makes still parses and reaches the checks behind the
//! parser: a string made empty or very long or not ASCII, a number made huge or negative, a key
//! dropped, a value given another type. Both are driven by [`Rng`], which is the same on every machine,
//! so that a failing input is the run's seed and number and is made again.

use serde_json::{Map, Number, Value};

/// splitmix64: a seeded source of numbers that is the same everywhere.
pub struct Rng(pub u64);

impl Rng {
    pub fn bits(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n` (`n` must not be 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.bits() % n as u64) as usize
    }
}

/// Bytes that a parser meets and a person does not write: edges of JSON, of strings, of numbers.
const INTERESTING: [u8; 14] = [
    0x00, 0xff, 0x7f, b'"', b'\\', b'{', b'}', b'[', b']', b':', b',', b'0', b'-', 0x80,
];

/// Numbers that do not fit what they are read into.
const NUMBERS: [&str; 8] = [
    "0",
    "-1",
    "4294967296",
    "18446744073709551615",
    "18446744073709551616",
    "9223372036854775808",
    "-9223372036854775809",
    "1e308",
];

/// The most a mutation makes an input longer by.
const GROWTH: usize = 4096;

/// `input`, changed in a few places. `others` are inputs of the same kind to splice from.
pub fn mutate(input: &[u8], rng: &mut Rng, others: &[Vec<u8>]) -> Vec<u8> {
    let mut out = input.to_vec();
    for _ in 0..1 + rng.below(4) {
        let length = out.len();
        match rng.below(9) {
            0 if length > 0 => {
                let at = rng.below(length);
                out[at] ^= 1 << rng.below(8);
            }
            1 if length > 0 => {
                let at = rng.below(length);
                out[at] = if rng.below(2) == 0 {
                    INTERESTING[rng.below(INTERESTING.len())]
                } else {
                    rng.bits() as u8
                };
            }
            2 if length > 0 => out.truncate(rng.below(length)),
            3 if length > 1 => {
                let from = rng.below(length);
                let to = from + rng.below(length - from + 1);
                out.drain(from..to);
            }
            4 if length > 1 => {
                let from = rng.below(length);
                let to = (from + 1 + rng.below(64)).min(length);
                let piece = out[from..to].to_vec();
                for _ in 0..1 + rng.below(8) {
                    if out.len() + piece.len() <= length + GROWTH {
                        let at = rng.below(out.len() + 1);
                        out.splice(at..at, piece.iter().copied());
                    }
                }
            }
            5 => {
                let at = rng.below(out.len() + 1);
                let fresh: Vec<u8> = (0..1 + rng.below(16)).map(|_| rng.bits() as u8).collect();
                out.splice(at..at, fresh);
            }
            6 if !others.is_empty() => {
                let other = &others[rng.below(others.len())];
                if !other.is_empty() && !out.is_empty() {
                    let from = rng.below(other.len());
                    let to = (from + 1 + rng.below(other.len() - from)).min(other.len());
                    let at = rng.below(out.len() + 1);
                    out.splice(at..at, other[from..to].iter().copied());
                }
            }
            7 => {
                // A run of digits becomes a number that does not fit.
                if let Some(start) = out.iter().position(u8::is_ascii_digit) {
                    let mut end = start;
                    while end < out.len() && out[end].is_ascii_digit() {
                        end += 1;
                    }
                    let number = NUMBERS[rng.below(NUMBERS.len())].as_bytes().to_vec();
                    out.splice(start..end, number);
                }
            }
            8 if length > 0 => {
                let at = rng.below(length);
                out.insert(at, INTERESTING[rng.below(INTERESTING.len())]);
            }
            _ => {}
        }
        if out.len() > length + GROWTH {
            out.truncate(length + GROWTH);
        }
    }
    out
}

/// Text that is not UTF-8 is a parser's input too, and reaches `&str` parsers as what a lossy read
/// makes of it.
pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A value of `value`'s JSON, changed in one place by its structure.
pub fn mutate_json(value: &Value, rng: &mut Rng) -> Value {
    let mut out = value.clone();
    let mut paths = Vec::new();
    collect(&out, &mut Vec::new(), &mut paths);
    if paths.is_empty() {
        return out;
    }
    let path = paths[rng.below(paths.len())].clone();
    change(&mut out, &path, rng);
    out
}

type Path = Vec<Step>;

#[derive(Clone)]
enum Step {
    Key(String),
    Index(usize),
}

fn collect(value: &Value, here: &mut Path, found: &mut Vec<Path>) {
    found.push(here.clone());
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                here.push(Step::Key(key.clone()));
                collect(child, here, found);
                here.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                here.push(Step::Index(index));
                collect(child, here, found);
                here.pop();
            }
        }
        _ => {}
    }
}

fn change(root: &mut Value, path: &[Step], rng: &mut Rng) {
    let Some((last, parents)) = path.split_last() else {
        *root = replacement(root, rng);
        return;
    };
    let mut at = root;
    for step in parents {
        at = match (step, at) {
            (Step::Key(key), Value::Object(map)) => match map.get_mut(key) {
                Some(child) => child,
                None => return,
            },
            (Step::Index(index), Value::Array(items)) => match items.get_mut(*index) {
                Some(child) => child,
                None => return,
            },
            _ => return,
        };
    }
    match (last, at) {
        (Step::Key(key), Value::Object(map)) => {
            if rng.below(5) == 0 {
                map.remove(key);
            } else if let Some(child) = map.get_mut(key) {
                *child = replacement(child, rng);
            }
        }
        (Step::Index(index), Value::Array(items)) => {
            if rng.below(5) == 0 && *index < items.len() {
                items.remove(*index);
            } else if let Some(child) = items.get_mut(*index) {
                *child = replacement(child, rng);
            }
        }
        _ => {}
    }
}

fn replacement(current: &Value, rng: &mut Rng) -> Value {
    match rng.below(10) {
        0 => Value::Null,
        1 => Value::Bool(rng.below(2) == 0),
        2 => Value::String(String::new()),
        3 => Value::String("x".repeat(1 + rng.below(2000))),
        4 => Value::String("\u{0}\u{7f}é\u{1F600}\u{202E}".to_string()),
        5 => Value::String(NUMBERS[rng.below(NUMBERS.len())].to_string()),
        6 => Value::Number(Number::from(rng.bits())),
        7 => Value::Number(Number::from(-(rng.below(1000) as i64))),
        8 => Value::Array(vec![current.clone(), current.clone()]),
        _ => Value::Object(Map::new()),
    }
}
