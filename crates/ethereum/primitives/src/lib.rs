//! Ethereum L1 data primitives

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use reth_node_types::NodePrimitives;
use reth_primitives::{Block, Receipt};

/// Ethereum primitive types.
#[derive(Debug, Clone, Default)]
pub struct EthPrimitives;

impl NodePrimitives for EthPrimitives {
    type Block = Block;
    type Receipt = Receipt;
}