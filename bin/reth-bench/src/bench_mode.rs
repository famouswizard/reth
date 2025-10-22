//! The benchmark mode defines whether the benchmark should run for a closed or open range of
//! blocks.
use std::ops::RangeInclusive;

/// Whether or not the benchmark should run as a continuous stream of payloads.
#[derive(Debug, PartialEq, Eq)]
pub enum BenchMode {
    // TODO: just include the start block in `Continuous`
    /// Run the benchmark as a continuous stream of payloads, until the benchmark is interrupted.
    Continuous,
    /// Run the benchmark for a specific range of blocks.
    Range(RangeInclusive<u64>),
}

impl BenchMode {
    /// Check if the block number is in the range
    pub fn contains(&self, block_number: u64) -> bool {
        match self {
            Self::Continuous => true,
            Self::Range(range) => range.contains(&block_number),
        }
    }

    /// Returns the number of blocks in the range, or None for continuous mode.
    pub const fn block_count(&self) -> Option<u64> {
        match self {
            Self::Continuous => None,
            Self::Range(range) => {
                let start = *range.start();
                let end = *range.end();
                Some(end.saturating_sub(start).saturating_add(1))
            }
        }
    }

    /// Create a [`BenchMode`] from optional `from` and `to` fields.
    pub fn new(from: Option<u64>, to: Option<u64>) -> Result<Self, eyre::Error> {
        // If neither `--from` nor `--to` are provided, we will run the benchmark continuously,
        // starting at the latest block.
        match (from, to) {
            (Some(from), Some(to)) => Ok(Self::Range(from..=to)),
            (None, None) => Ok(Self::Continuous),
            _ => {
                // both or neither are allowed, everything else is ambiguous
                Err(eyre::eyre!("`from` and `to` must be provided together, or not at all."))
            }
        }
    }
}
