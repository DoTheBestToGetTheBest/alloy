//! This module extends the Ethereum JSON-RPC provider with the Debug namespace's RPC methods.
use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{hex, Bytes, TxHash, B256};
use alloy_rpc_types_eth::{
    state::StateOverride, Block, BlockNumberOrTag, EthCallResponse, StateContext,
    TransactionRequest,
};
use alloy_rpc_types_trace::geth::{
    BlockTraceResult, GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TraceResult,
};
use alloy_transport::{Transport, TransportResult};

/// Debug namespace rpc interface that gives access to several non-standard RPC methods.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait DebugApi<N, T>: Send + Sync {
    /// Returns an RLP-encoded header.
    async fn debug_get_raw_header(&self, block: BlockNumberOrTag) -> TransportResult<Bytes>;

    /// Retrieves and returns the RLP encoded block by number, hash or tag.
    async fn debug_get_raw_block(&self, block: BlockNumberOrTag) -> TransportResult<Bytes>;

    /// Returns an EIP-2718 binary-encoded transaction.
    async fn debug_get_raw_transaction(&self, hash: TxHash) -> TransportResult<Bytes>;

    /// Returns an array of EIP-2718 binary-encoded receipts.
    async fn debug_get_raw_receipts(&self, block: BlockNumberOrTag) -> TransportResult<Vec<Bytes>>;

    /// Returns an array of recent bad blocks that the client has seen on the network.
    async fn debug_get_bad_blocks(&self) -> TransportResult<Vec<Block>>;

    /// Returns the structured logs created during the execution of EVM between two blocks
    /// (excluding start) as a JSON object.
    async fn debug_trace_chain(
        &self,
        start_exclusive: BlockNumberOrTag,
        end_inclusive: BlockNumberOrTag,
    ) -> TransportResult<Vec<BlockTraceResult>>;

    /// The debug_traceBlock method will return a full stack trace of all invoked opcodes of all
    /// transaction that were included in this block.
    ///
    /// This expects an RLP-encoded block.
    ///
    /// # Note
    ///
    /// The parent of this block must be present, or it will fail.
    async fn debug_trace_block(
        &self,
        rlp_block: &[u8],
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>>;

    /// Reruns the transaction specified by the hash and returns the trace.
    ///
    /// It will replay any prior transactions to achieve the same state the transaction was executed
    /// in.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace>;

    /// Return a full stack trace of all invoked opcodes of all transaction that were included in
    /// this block.
    ///
    /// The parent of the block must be present or it will fail.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_block_by_hash(
        &self,
        block: B256,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>>;

    /// Same as `debug_trace_block_by_hash` but block is specified by number.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_block_by_number(
        &self,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>>;

    /// Executes the given transaction without publishing it like `eth_call` and returns the trace
    /// of the execution.
    ///
    /// The transaction will be executed in the context of the given block number or tag.
    /// The state its run on is the state of the previous block.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    ///
    /// Not all nodes support this call.
    async fn debug_trace_call(
        &self,
        tx: TransactionRequest,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<GethTrace>;

    /// Same as `debug_trace_call` but it used to run and trace multiple transactions at once.
    ///
    /// [GethDebugTracingOptions] can be used to specify the trace options.
    ///
    /// # Note
    ///
    /// Not all nodes support this call.
    async fn debug_trace_call_many(
        &self,
        txs: Vec<TransactionRequest>,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<Vec<GethTrace>>;

    /// Simulate arbitrary number of transactions at an arbitrary blockchain index, with the
    /// optionality of state overrides
    async fn call_many(
        &self,
        tx: TransactionRequest,
        state_context: Option<StateContext>,
        state_override: Option<StateOverride>,
    ) -> TransportResult<Vec<EthCallResponse>>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> DebugApi<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn call_many(
        &self,
        tx: TransactionRequest,
        state_context: Option<StateContext>,
        state_override: Option<StateOverride>,
    ) -> TransportResult<Vec<EthCallResponse>> {
        self.client().request("callMany", (tx, state_context, state_override)).await
    }
    async fn debug_get_raw_header(&self, block: BlockNumberOrTag) -> TransportResult<Bytes> {
        self.client().request("debug_getRawHeader", (block,)).await
    }

    async fn debug_get_raw_block(&self, block: BlockNumberOrTag) -> TransportResult<Bytes> {
        self.client().request("debug_getRawBlock", (block,)).await
    }

    async fn debug_get_raw_transaction(&self, hash: TxHash) -> TransportResult<Bytes> {
        self.client().request("debug_getRawTransaction", (hash,)).await
    }

    async fn debug_get_raw_receipts(&self, block: BlockNumberOrTag) -> TransportResult<Vec<Bytes>> {
        self.client().request("debug_getRawReceipts", (block,)).await
    }

    async fn debug_get_bad_blocks(&self) -> TransportResult<Vec<Block>> {
        self.client().request("debug_getBadBlocks", ()).await
    }

    async fn debug_trace_chain(
        &self,
        start_exclusive: BlockNumberOrTag,
        end_inclusive: BlockNumberOrTag,
    ) -> TransportResult<Vec<BlockTraceResult>> {
        self.client().request("debug_traceChain", (start_exclusive, end_inclusive)).await
    }

    async fn debug_trace_block(
        &self,
        rlp_block: &[u8],
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>> {
        let rlp_block = hex::encode_prefixed(rlp_block);
        self.client().request("debug_traceBlock", (rlp_block, trace_options)).await
    }

    async fn debug_trace_transaction(
        &self,
        hash: TxHash,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<GethTrace> {
        self.client().request("debug_traceTransaction", (hash, trace_options)).await
    }

    async fn debug_trace_block_by_hash(
        &self,
        block: B256,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>> {
        self.client().request("debug_traceBlockByHash", (block, trace_options)).await
    }

    async fn debug_trace_block_by_number(
        &self,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingOptions,
    ) -> TransportResult<Vec<TraceResult>> {
        self.client().request("debug_traceBlockByNumber", (block, trace_options)).await
    }

    async fn debug_trace_call(
        &self,
        tx: TransactionRequest,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<GethTrace> {
        self.client().request("debug_traceCall", (tx, block, trace_options)).await
    }

    async fn debug_trace_call_many(
        &self,
        txs: Vec<TransactionRequest>,
        block: BlockNumberOrTag,
        trace_options: GethDebugTracingCallOptions,
    ) -> TransportResult<Vec<GethTrace>> {
        self.client().request("debug_traceCallMany", (txs, block, trace_options)).await
    }
}

#[cfg(test)]
mod test {
    use crate::{ProviderBuilder, WalletProvider};

    use super::*;
    use alloy_network::TransactionBuilder;
    use alloy_node_bindings::Geth;
    use alloy_primitives::{address, U256};

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[tokio::test]
    async fn test_debug_trace_transaction() {
        init_tracing();
        let provider = ProviderBuilder::new().with_recommended_fillers().on_anvil_with_wallet();
        let from = provider.default_signer_address();

        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest::default()
            .from(from)
            .to(address!("deadbeef00000000deadbeef00000000deadbeef"))
            .value(U256::from(100))
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);
        let pending = provider.send_transaction(tx).await.unwrap();
        let receipt = pending.get_receipt().await.unwrap();

        let hash = receipt.transaction_hash;
        let trace_options = GethDebugTracingOptions::default();

        let trace = provider.debug_trace_transaction(hash, trace_options).await.unwrap();

        if let GethTrace::Default(trace) = trace {
            assert_eq!(trace.gas, 21000)
        }
    }

    #[tokio::test]
    async fn test_debug_trace_call() {
        init_tracing();
        let provider = ProviderBuilder::new().on_anvil_with_wallet();
        let from = provider.default_signer_address();
        let gas_price = provider.get_gas_price().await.unwrap();
        let tx = TransactionRequest::default()
            .from(from)
            .with_input("0xdeadbeef")
            .max_fee_per_gas(gas_price + 1)
            .max_priority_fee_per_gas(gas_price + 1);

        let trace = provider
            .debug_trace_call(tx, BlockNumberOrTag::Latest, GethDebugTracingCallOptions::default())
            .await
            .unwrap();

        if let GethTrace::Default(trace) = trace {
            assert!(!trace.struct_logs.is_empty());
        }
    }

    #[tokio::test]
    async fn call_debug_get_raw_header() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let rlp_header = provider
            .debug_get_raw_header(BlockNumberOrTag::default())
            .await
            .expect("debug_getRawHeader call should succeed");

        assert!(!rlp_header.is_empty());
    }

    #[tokio::test]
    async fn call_debug_get_raw_block() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let rlp_block = provider
            .debug_get_raw_block(BlockNumberOrTag::default())
            .await
            .expect("debug_getRawBlock call should succeed");

        assert!(!rlp_block.is_empty());
    }

    #[tokio::test]
    async fn call_debug_get_raw_receipts() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let result = provider.debug_get_raw_receipts(BlockNumberOrTag::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn call_debug_get_bad_blocks() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let result = provider.debug_get_bad_blocks().await;
        assert!(result.is_ok());
    }
}
