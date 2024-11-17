/// Ethereum primitive types.
#[derive(Debug, Default, Clone)]
pub struct EthPrimitives;

impl reth_primitives_traits::FullNodePrimitives for EthPrimitives {
    type Block = crate::Block;
    type SignedTx = crate::TransactionSigned;
    type TxType = crate::TxType;
    type Receipt = crate::Receipt;
}
