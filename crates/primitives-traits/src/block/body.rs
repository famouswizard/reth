//! Block body abstraction.

use alloc::fmt;

use alloy_consensus::Transaction;

use crate::{InMemorySize, MaybeSerde};

/// Abstraction for block's body.
#[auto_impl::auto_impl(&, Arc)]
pub trait BlockBody:
    Send
    + Sync
    + Unpin
    + Clone
    + Default
    + fmt::Debug
    + PartialEq
    + Eq
    + alloy_rlp::Encodable
    + alloy_rlp::Decodable
    + InMemorySize
    + MaybeSerde
{
    /// Ordered list of signed transactions as committed in block.
    // todo: requires trait for signed transaction
    type Transaction: Transaction;

    /// Returns reference to transactions in block.
    fn transactions(&self) -> &[Self::Transaction];
}
