//! Log Sequence Number (LSN) a 64-bit WAL byte position.
//!
//! rendering an LSN as `HIGH/LOW`, where both halves are
//! uppercase-or-lowercase hexadecimal and together encode a single u64:
//!
//!   value = (high as u64) << 32 | (low as u64)
//!
//! Because the textual form is fixed-width hex, lexicographic ordering of
//! the 16-char hex string is identical to numeric ordering of the u64.
//! That equivalence is the whole reason we can compare LSNs as bytes or
//! as numbers interchangeably.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

// Number of hex digits in the fixed width rendering of a u64.
const HEX_WIDTH: usize = 16;

// A Log Sequence Number.
// Internally a `u64`. Ordering is derived from the `u64`, which is
// equivalent to lexicographic ordering of the fixed-width hex form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Lsn(u64);

impl Lsn {
    /// the lowest valid LSN, `0/0`.
    pub const ZERO: Lsn = Lsn(0);

    /// the highest valid LSN, `FFFFFFFF/FFFFFFFF`.
    pub const MAX: Lsn = Lsn(u64::MAX);

    /// construct from a raw u64.
    #[inline]
    pub const fn from_u64(v: u64) -> Self {
        Lsn(v)
    }

    /// extract the raw u64.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// high 32 bits, the part before the `/` in the output
    #[inline]
    pub const fn high(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Low 32 bits the part after the `/`.
    #[inline]
    pub const fn low(self) -> u32 {
        (self.0 & 0xFFFF_FFFF) as u32
    }

    /// parse the `HIGH/LOW` postgreSQL textual form
    /// accepts upper or lower case hex, and leading zeros.
    /// Does not accept more than 8 hex digits per side(that would overflow the 32-bit half).
    pub fn from_pgoutput(s: &str) -> Result<Self, LsnError>{
        s.parse()
    }

    /// Render as PostgreSQL `HIGH/LOW` uppercase, no leading zeros
    /// beyond what's needed, matching `pg_lsn` output.
    pub fn to_pgoutput(self) -> String {
        format!("{:X}/{:X}", self.high(), self.low())
    }

    /// Fixed-width 16 uppercase hex digits, no separator.
    /// This is the canonical *sortable* form. `Lsn::to_hex16(a) <= Lsn::to_hex16(b)`
    /// iff `a <= b`.
    pub fn to_hex16(self) -> String {
        format!("{:016X}", self.0)
    }
}

// ordering: delegate entirely to the u64
impl Ord for Lsn {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Lsn {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Display/debug
impl fmt::Display for Lsn {
    /// PostgreSQL-style `HIGH/LOW`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}/{:X}", self.high(), self.low())
    }
}

// parsing
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LsnError {
    #[error("LSN is missing the '/' separator: {0:?}")]
    MissingSeparator(String),

    #[error("LSN has more than one '/': {0:?}")]
    TooManySeparators(String),

    #[error("high half is empty or not valid hex: {0:?}")]
    InvalidHigh(String),

    #[error("low half is empty or not valid hex: {0:?}")]
    InvalidLow(String),

    #[error("high half overflows 32 bits (max 8 hex digits): {0:?}")]
    HighOverflow(String),

    #[error("low half overflows 32 bits (max 8 hex digits): {0:?}")]
    LowOverflow(String),
}

impl FromStr for Lsn {
    type Err = LsnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Exactly one '/'.
        let (hi_s, lo_s) = match s.split_once('/') {
            None => return Err(LsnError::MissingSeparator(s.to_string())),
            Some((h, l)) => {
                if l.contains('/') {
                    return Err(LsnError::TooManySeparators(s.to_string()));
                }
                (h, l)
            }
        };

        if hi_s.is_empty() {
            return Err(LsnError::InvalidHigh(s.to_string()));
        }
        if lo_s.is_empty() {
            return Err(LsnError::InvalidLow(s.to_string()));
        }
        if hi_s.len() > 8 {
            return Err(LsnError::HighOverflow(s.to_string()));
        }
        if lo_s.len() > 8 {
            return Err(LsnError::LowOverflow(s.to_string()));
        }

        let high = u32::from_str_radix(hi_s, 16)
            .map_err(|_| LsnError::InvalidHigh(s.to_string()))?;
        let low = u32::from_str_radix(lo_s, 16)
            .map_err(|_| LsnError::InvalidLow(s.to_string()))?;

        Ok(Lsn(((high as u64) << 32) | (low as u64)))
    }
}


