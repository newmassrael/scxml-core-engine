// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Physical-quantity annotation primitives for numeric fields.
//
// Codegen-effective `sce:quantity` / `sce:scale`
// / `sce:offset` attributes on `<sce:field>` and `<data>` elements declare a
// linear `physical = raw * scale + offset` conversion in a named unit. The
// resulting types thread through expression inference so that two operands
// carrying different units cannot be combined arithmetically. ARXML
// COMPU-METHOD blocks (the largest single source of automotive structured
// type data) map onto this layer directly.
//
// Design constraints:
//
// * `InferredType` is `Copy`. To keep that invariant after extending the
//   lattice with `Quantity { base, scale, offset, unit }` every component
//   must itself be `Copy`. `Rational` uses fixed-width `i64` components and
//   `UnitTag(u16)` indexes into a process-wide interned registry — neither
//   carries heap allocation per value.
//
// * Unit strings are opaque to the type system. The registry recommends SI
//   base units (`s`, `m`, `kg`, `A`, `K`, `mol`, `cd`) but does not enforce.
//   Two unit tags compare equal iff they were interned from the same
//   `&str`, which is exactly the semantics needed for "incompatible units
//   in arithmetic" detection.
//
// * `Rational::reduce` is canonical: `denom` is always positive, sign lives
//   on `num`, and `gcd(|num|, |denom|) == 1`. This ensures `PartialEq /
//   Eq / Hash` agree with the underlying rational value.

use std::sync::{Mutex, OnceLock};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Rational — canonical-form `num / denom` with `denom > 0`
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Exact rational scale or offset constant. Fixed-width `i64` keeps the
/// value `Copy` so it can sit inside `InferredType::Quantity` without
/// breaking the lattice's `Copy` invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i64,
    denom: i64,
}

impl Rational {
    /// Construct a canonical-form rational. Returns `None` for a zero
    /// denominator or when reduction would overflow `i64`.
    pub fn new(num: i64, denom: i64) -> Option<Self> {
        if denom == 0 {
            return None;
        }
        let (n, d) = if denom < 0 {
            // Move the sign onto the numerator so `denom > 0` is invariant.
            (num.checked_neg()?, denom.checked_neg()?)
        } else {
            (num, denom)
        };
        let g = gcd_i64(n.unsigned_abs(), d as u64);
        // gcd is in `1..=min(|n|,d)`; both divisions are exact.
        Some(Self {
            num: n / g as i64,
            denom: d / g as i64,
        })
    }

    /// Integer rational `n / 1`.
    pub fn from_int(n: i64) -> Self {
        Self { num: n, denom: 1 }
    }

    /// `0 / 1` — the additive identity.
    pub fn zero() -> Self {
        Self { num: 0, denom: 1 }
    }

    /// `1 / 1` — the multiplicative identity.
    pub fn one() -> Self {
        Self { num: 1, denom: 1 }
    }

    pub fn numerator(self) -> i64 {
        self.num
    }

    pub fn denominator(self) -> i64 {
        self.denom
    }

    /// Convert to `f64`. Used by codegen to emit literal coefficients
    /// when the target language has no rational type (every supported
    /// language target: f64 is the canonical representation).
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.denom as f64
    }

    /// `true` when the rational represents `0`.
    pub fn is_zero(self) -> bool {
        self.num == 0
    }

    /// `true` when the rational represents `1`.
    pub fn is_one(self) -> bool {
        self.num == self.denom
    }

    /// Parse a textual rational. Accepts:
    /// * decimal integers   — `42`, `-17`, `0`
    /// * decimal fractions  — `0.5`, `-40.25`, `1.0`
    /// * explicit ratios    — `1/2`, `-1/100`
    ///
    /// Rejects scientific notation, hexadecimal, NaN/Inf, leading `+`, and
    /// any form that would not reduce to a canonical `Rational`.
    pub fn parse(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }
        // Explicit ratio form `num/denom`.
        if let Some(slash) = s.find('/') {
            let n: i64 = s[..slash].trim().parse().ok()?;
            let d: i64 = s[slash + 1..].trim().parse().ok()?;
            return Self::new(n, d);
        }
        // Decimal fraction `int.frac`.
        if let Some(dot) = s.find('.') {
            let whole_part = &s[..dot];
            let frac_part = &s[dot + 1..];
            if frac_part.is_empty() || !frac_part.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let sign = whole_part.starts_with('-');
            let whole_digits = whole_part.strip_prefix('-').unwrap_or(whole_part);
            if whole_digits.is_empty() || !whole_digits.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let denom: i64 = 10i64.checked_pow(frac_part.len() as u32)?;
            let whole_n: i64 = whole_digits.parse().ok()?;
            let frac_n: i64 = frac_part.parse().ok()?;
            let num = whole_n.checked_mul(denom)?.checked_add(frac_n)?;
            let signed_num = if sign { num.checked_neg()? } else { num };
            return Self::new(signed_num, denom);
        }
        // Bare integer.
        let n: i64 = s.parse().ok()?;
        Some(Self { num: n, denom: 1 })
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.denom == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.denom)
        }
    }
}

impl serde::Serialize for Rational {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        // Stringify so JSON consumers see a stable textual form regardless
        // of whether the value was authored as `0.5` or `1/2`.
        ser.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
impl schemars::JsonSchema for Rational {
    fn schema_name() -> String {
        "Rational".to_owned()
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Rationals serialize as `"<num>"` or `"<num>/<denom>"`; the
        // pattern allows both forms (no scientific notation, no float
        // text — decimal-text is normalised at parse time).
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            string: Some(Box::new(schemars::schema::StringValidation {
                pattern: Some(r"^-?\d+(/\d+)?$".to_owned()),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

fn gcd_i64(a: u64, b: u64) -> u64 {
    if b == 0 {
        a.max(1)
    } else {
        gcd_i64(b, a % b)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// UnitTag — interned identifier for an opaque unit string
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Opaque tag for a unit string interned in the process-wide registry.
///
/// Two `UnitTag` values compare equal iff they were interned from the
/// same `&str`. The actual unit string is recoverable via [`Self::as_str`]
/// for diagnostic rendering; equality and hashing are decided entirely
/// by the underlying `u16` index so they stay `Copy`-cheap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitTag(u16);

impl UnitTag {
    /// Intern `unit` and return its tag. Repeated interning of the same
    /// string returns the same tag.
    pub fn intern(unit: &str) -> Self {
        UnitRegistry::global().intern(unit)
    }

    /// Recover the original unit string. Lifetime is `'static` because
    /// interned strings are leaked into a process-wide registry.
    pub fn as_str(self) -> &'static str {
        UnitRegistry::global().lookup(self)
    }
}

impl std::fmt::Display for UnitTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for UnitTag {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

#[cfg(test)]
impl schemars::JsonSchema for UnitTag {
    fn schema_name() -> String {
        "UnitTag".to_owned()
    }
    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some(
                    "Opaque unit string. SI base units (`s`, `m`, `kg`, `A`, `K`, `mol`, \
                     `cd`) are recommended but not enforced."
                        .to_owned(),
                ),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

/// Process-wide intern table for unit strings.
///
/// Real builds carry a single-digit number of unique unit strings, so a
/// linear-scan `Vec<&'static str>` is faster than a `HashMap` of the same
/// size and avoids the hash-DoS surface. Interned strings are
/// `Box::leak`'d so their `&'static` lifetime stays valid for the whole
/// process.
struct UnitRegistry {
    units: Mutex<Vec<&'static str>>,
}

impl UnitRegistry {
    fn global() -> &'static UnitRegistry {
        static REG: OnceLock<UnitRegistry> = OnceLock::new();
        REG.get_or_init(|| UnitRegistry {
            units: Mutex::new(Vec::new()),
        })
    }

    fn intern(&self, unit: &str) -> UnitTag {
        let mut guard = self.units.lock().expect("UnitRegistry mutex poisoned");
        if let Some(idx) = guard.iter().position(|existing| *existing == unit) {
            return UnitTag(idx as u16);
        }
        let leaked: &'static str = Box::leak(unit.to_owned().into_boxed_str());
        let idx = guard.len();
        assert!(
            idx < u16::MAX as usize,
            "UnitRegistry exhausted u16 index space — declared {idx} unit strings",
        );
        guard.push(leaked);
        UnitTag(idx as u16)
    }

    fn lookup(&self, tag: UnitTag) -> &'static str {
        let guard = self.units.lock().expect("UnitRegistry mutex poisoned");
        guard
            .get(tag.0 as usize)
            .copied()
            .expect("UnitTag references unregistered unit — must be constructed via intern()")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// NumericBaseType — the int/float carrier of a Quantity
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The underlying numeric representation that a Quantity layers
/// physical-unit semantics over. Mirrors `InferredType::Int` and
/// `InferredType::Float` so quantity-stripping operations (e.g., asking
/// the raw type that flows on the wire) map back without information
/// loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericBaseType {
    Int { signed: bool, bits: u8 },
    Float { bits: u8 },
}

impl NumericBaseType {
    pub fn is_float(self) -> bool {
        matches!(self, Self::Float { .. })
    }
}

impl std::fmt::Display for NumericBaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumericBaseType::Int { signed: true, bits } => write!(f, "i{bits}"),
            NumericBaseType::Int {
                signed: false,
                bits,
            } => write!(f, "u{bits}"),
            NumericBaseType::Float { bits } => write!(f, "f{bits}"),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Quantity — the linear-physical-conversion descriptor
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Triple `(scale, offset, unit)` declared on a numeric field. The
/// physical interpretation of a raw value `x` is `x * scale + offset`,
/// carrying unit `unit`. The base numeric type stays in the field's
/// `SceType`; this struct only carries the conversion metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Quantity {
    pub scale: Rational,
    pub offset: Rational,
    pub unit: UnitTag,
}

impl Quantity {
    /// `true` when this quantity is a pure unit annotation with no
    /// numerical conversion (`scale = 1, offset = 0`). Codegen may
    /// elide the conversion arithmetic in that case and still emit the
    /// physical accessor (typed as the host f64 / equivalent).
    pub fn is_identity(self) -> bool {
        self.scale.is_one() && self.offset.is_zero()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_reduces_to_canonical_form() {
        let r = Rational::new(4, 8).unwrap();
        assert_eq!(r.numerator(), 1);
        assert_eq!(r.denominator(), 2);
    }

    #[test]
    fn rational_moves_sign_to_numerator() {
        let r = Rational::new(3, -6).unwrap();
        assert_eq!(r.numerator(), -1);
        assert_eq!(r.denominator(), 2);
    }

    #[test]
    fn rational_rejects_zero_denominator() {
        assert!(Rational::new(1, 0).is_none());
    }

    #[test]
    fn rational_parse_integer() {
        assert_eq!(Rational::parse("42"), Some(Rational::from_int(42)));
        assert_eq!(Rational::parse("-17"), Some(Rational::from_int(-17)));
        assert_eq!(Rational::parse(" 0 "), Some(Rational::zero()));
    }

    #[test]
    fn rational_parse_decimal() {
        let r = Rational::parse("0.5").unwrap();
        assert_eq!(r.numerator(), 1);
        assert_eq!(r.denominator(), 2);
        let r = Rational::parse("-40.25").unwrap();
        assert_eq!(r.numerator(), -161);
        assert_eq!(r.denominator(), 4);
    }

    #[test]
    fn rational_parse_explicit_ratio() {
        let r = Rational::parse("1/100").unwrap();
        assert_eq!(r.numerator(), 1);
        assert_eq!(r.denominator(), 100);
        let r = Rational::parse("-3/12").unwrap();
        assert_eq!(r.numerator(), -1);
        assert_eq!(r.denominator(), 4);
    }

    #[test]
    fn rational_parse_rejects_malformed() {
        assert!(Rational::parse("").is_none());
        assert!(Rational::parse("abc").is_none());
        assert!(Rational::parse("1e2").is_none());
        assert!(Rational::parse("0x10").is_none());
        assert!(Rational::parse("1/").is_none());
        assert!(Rational::parse("1.").is_none());
        assert!(Rational::parse("NaN").is_none());
    }

    #[test]
    fn rational_to_f64_matches_decimal_form() {
        assert_eq!(Rational::parse("0.5").unwrap().to_f64(), 0.5);
        assert_eq!(Rational::parse("-40").unwrap().to_f64(), -40.0);
        assert_eq!(Rational::parse("1/4").unwrap().to_f64(), 0.25);
    }

    #[test]
    fn rational_display_omits_denom_one() {
        assert_eq!(format!("{}", Rational::from_int(7)), "7");
        assert_eq!(format!("{}", Rational::parse("0.5").unwrap()), "1/2");
    }

    #[test]
    fn rational_zero_and_one_predicates() {
        assert!(Rational::zero().is_zero());
        assert!(!Rational::zero().is_one());
        assert!(Rational::one().is_one());
        assert!(!Rational::one().is_zero());
        // Reduction means `2/2` is `1/1`, satisfying `is_one`.
        assert!(Rational::new(2, 2).unwrap().is_one());
    }

    #[test]
    fn unit_tag_intern_roundtrip() {
        let a = UnitTag::intern("celsius");
        let b = UnitTag::intern("celsius");
        let c = UnitTag::intern("kelvin");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.as_str(), "celsius");
        assert_eq!(c.as_str(), "kelvin");
    }

    #[test]
    fn unit_tag_display() {
        let t = UnitTag::intern("m/s^2");
        assert_eq!(format!("{t}"), "m/s^2");
    }

    #[test]
    fn quantity_is_identity_detects_trivial_conversion() {
        let id = Quantity {
            scale: Rational::one(),
            offset: Rational::zero(),
            unit: UnitTag::intern("dimensionless-ident-test"),
        };
        assert!(id.is_identity());

        let non_id = Quantity {
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::from_int(-40),
            unit: UnitTag::intern("celsius-id-test"),
        };
        assert!(!non_id.is_identity());
    }

    #[test]
    fn numeric_base_type_display() {
        assert_eq!(
            format!(
                "{}",
                NumericBaseType::Int {
                    signed: false,
                    bits: 8
                }
            ),
            "u8",
        );
        assert_eq!(
            format!(
                "{}",
                NumericBaseType::Int {
                    signed: true,
                    bits: 32
                }
            ),
            "i32",
        );
        assert_eq!(format!("{}", NumericBaseType::Float { bits: 64 }), "f64",);
    }
}
