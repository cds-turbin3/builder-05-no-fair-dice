//! On-chain constants.

/// House edge in basis points, withheld from a winning payout.
pub const HOUSE_EDGE_BASIS_POINTS: u16 = 150;

/// Minimum stake, in lamports.
pub const MIN_BET_LAMPORTS: u64 = 10_000_000;

/// Inclusive bounds on the player's guess (a percentile roll of 1..=99).
pub const MIN_ROLL: u8 = 1;
pub const MAX_ROLL: u8 = 99;

/// A bet older than this many slots can be refunded (the house never revealed).
pub const REFUND_TIMEOUT_SLOTS: u64 = 1000;
