use alloy_primitives::{Keccak256, B256, U256};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use std::sync::OnceLock;
/// Represents an Account in the account trie.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, RlpDecodable, RlpEncodable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Account {
    /// The account's nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub nonce: u64,
    /// The account's balance.
    pub balance: U256,
    /// The hash of the storage account data.
    pub storage_root: B256,
    /// The hash of the code of the account.
    pub code_hash: B256,
}

impl Account {
    /// compute  hash as committed to in the MPT trie with memo
    pub fn trie_hash(&self) -> B256 {
        static TRIE_HASH: OnceLock<B256> = OnceLock::new();
        *TRIE_HASH.get_or_init(|| {
            let encoded = self.encode_to_vec();
            let mut hasher = Keccak256::new();
            hasher.update(&encoded);
            hasher.finalize()
        })
    }

    /// compute  hash as committed to in the MPT trie without memo
    pub fn trie_hash_slow(&self) -> B256 {
        let mut buf = vec![];
        self.encode(&mut buf);
        let mut hasher = Keccak256::new();
        hasher.update(&buf);
        hasher.finalize()
    }

    /// encode  to a `Vec<u8>`
    fn encode_to_vec(&self) -> Vec<u8> {
        let mut out = vec![];
        self.encode(&mut out);
        out
    }
}