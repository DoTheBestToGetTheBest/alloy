//! EIP-4844 sidecar type

#[cfg(feature = "kzg")]
use crate::eip4844::env_settings::EnvKzgSettings;
#[cfg(any(test, feature = "arbitrary"))]
use crate::eip4844::MAX_BLOBS_PER_BLOCK;
use crate::{
    eip4844::{
        kzg_to_versioned_hash, Blob, Bytes48, BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_PROOF,
    },
    BlockNumHash,
};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use alloy_primitives::{bytes::BufMut, FixedBytes, B256};
use alloy_rlp::{Decodable, Encodable};
#[cfg(feature = "kzg")]
use c_kzg::KzgProof;
#[cfg(feature = "kzg")]
use core::str::FromStr;
#[cfg(feature = "serde")]
use serde::Deserialize;
#[cfg(feature = "kzg")]
use sha2::{Digest, Sha256};
#[cfg(feature = "kzg")]
/// The versioned hash version for KZG.
pub(crate) const VERSIONED_HASH_VERSION_KZG: u8 = 0x01;
/// This represents a set of blobs, and its corresponding commitments and proofs.
///
/// This type encodes and decodes the fields without an rlp header.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "BlobTxSidecar")]
pub struct BlobTransactionSidecar {
    /// The blob data.
    #[cfg_attr(
        all(debug_assertions, feature = "serde"),
        serde(deserialize_with = "deserialize_blobs")
    )]
    pub blobs: Vec<Blob>,
    /// The blob commitments.
    pub commitments: Vec<Bytes48>,
    /// The blob proofs.
    pub proofs: Vec<Bytes48>,
}

impl BlobTransactionSidecar {
    /// Creates an iterator
    pub fn iter(&self) -> impl Iterator<Item = BlobTransactionSidecarItem> + '_ {
        self.blobs.iter().zip(&self.commitments).zip(&self.proofs).enumerate().map(
            |(index, ((blob, commitment), proof))| BlobTransactionSidecarItem {
                index,
                blob: *blob,
                kzg_commitment: *commitment,
                kzg_proof: *proof,
            },
        )
    }
}
#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for BlobTransactionSidecar {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let num_blobs = u.int_in_range(1..=MAX_BLOBS_PER_BLOCK)?;
        let mut blobs = Vec::with_capacity(num_blobs);
        for _ in 0..num_blobs {
            blobs.push(Blob::arbitrary(u)?);
        }

        let mut commitments = Vec::with_capacity(num_blobs);
        let mut proofs = Vec::with_capacity(num_blobs);
        for _ in 0..num_blobs {
            commitments.push(Bytes48::arbitrary(u)?);
            proofs.push(Bytes48::arbitrary(u)?);
        }

        Ok(Self { blobs, commitments, proofs })
    }
}

impl BlobTransactionSidecar {
    /// Constructs a new [BlobTransactionSidecar] from a set of blobs, commitments, and proofs.
    pub const fn new(blobs: Vec<Blob>, commitments: Vec<Bytes48>, proofs: Vec<Bytes48>) -> Self {
        Self { blobs, commitments, proofs }
    }

    /// Creates a new instance from the given KZG types.
    #[cfg(feature = "kzg")]
    pub fn from_kzg(
        blobs: Vec<c_kzg::Blob>,
        commitments: Vec<c_kzg::Bytes48>,
        proofs: Vec<c_kzg::Bytes48>,
    ) -> Self {
        // transmutes the vec of items, see also [core::mem::transmute](https://doc.rust-lang.org/std/mem/fn.transmute.html)
        unsafe fn transmute_vec<U, T>(input: Vec<T>) -> Vec<U> {
            let mut v = core::mem::ManuallyDrop::new(input);
            Vec::from_raw_parts(v.as_mut_ptr() as *mut U, v.len(), v.capacity())
        }

        // SAFETY: all types have the same size and alignment
        unsafe {
            let blobs = transmute_vec::<Blob, c_kzg::Blob>(blobs);
            let commitments = transmute_vec::<Bytes48, c_kzg::Bytes48>(commitments);
            let proofs = transmute_vec::<Bytes48, c_kzg::Bytes48>(proofs);
            Self { blobs, commitments, proofs }
        }
    }

    /// Verifies that the versioned hashes are valid for this sidecar's blob data, commitments, and
    /// proofs.
    ///
    /// Takes as input the [KzgSettings](c_kzg::KzgSettings), which should contain the parameters
    /// derived from the KZG trusted setup.
    ///
    /// This ensures that the blob transaction payload has the same number of blob data elements,
    /// commitments, and proofs. Each blob data element is verified against its commitment and
    /// proof.
    ///
    /// Returns [BlobTransactionValidationError::InvalidProof] if any blob KZG proof in the response
    /// fails to verify, or if the versioned hashes in the transaction do not match the actual
    /// commitment versioned hashes.
    #[cfg(feature = "kzg")]
    pub fn validate(
        &self,
        blob_versioned_hashes: &[B256],
        proof_settings: &c_kzg::KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        // Ensure the versioned hashes and commitments have the same length.
        if blob_versioned_hashes.len() != self.commitments.len() {
            return Err(c_kzg::Error::MismatchLength(format!(
                "There are {} versioned commitment hashes and {} commitments",
                blob_versioned_hashes.len(),
                self.commitments.len()
            ))
            .into());
        }

        // calculate versioned hashes by zipping & iterating
        for (versioned_hash, commitment) in
            blob_versioned_hashes.iter().zip(self.commitments.iter())
        {
            let commitment = c_kzg::KzgCommitment::from(commitment.0);

            // calculate & verify versioned hash
            let calculated_versioned_hash = kzg_to_versioned_hash(commitment.as_slice());
            if *versioned_hash != calculated_versioned_hash {
                return Err(BlobTransactionValidationError::WrongVersionedHash {
                    have: *versioned_hash,
                    expected: calculated_versioned_hash,
                });
            }
        }

        // SAFETY: ALL types have the same size
        let res = unsafe {
            c_kzg::KzgProof::verify_blob_kzg_proof_batch(
                // blobs
                core::mem::transmute::<&[Blob], &[c_kzg::Blob]>(self.blobs.as_slice()),
                // commitments
                core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.commitments.as_slice()),
                // proofs
                core::mem::transmute::<&[Bytes48], &[c_kzg::Bytes48]>(self.proofs.as_slice()),
                proof_settings,
            )
        }
        .map_err(BlobTransactionValidationError::KZGError)?;

        if res {
            Ok(())
        } else {
            Err(BlobTransactionValidationError::InvalidProof)
        }
    }

    /// Returns an iterator over the versioned hashes of the commitments.
    pub fn versioned_hashes(&self) -> impl Iterator<Item = B256> + '_ {
        self.commitments.iter().map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Returns the versioned hash for the blob at the given index, if it
    /// exists.
    pub fn versioned_hash_for_blob(&self, blob_index: usize) -> Option<B256> {
        self.commitments.get(blob_index).map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, __without__ a RLP header.
    ///
    /// This encodes the fields in the following order:
    /// - `blobs`
    /// - `commitments`
    /// - `proofs`
    #[inline]
    pub(crate) fn encode_inner(&self, out: &mut dyn BufMut) {
        // Encode the blobs, commitments, and proofs
        self.blobs.encode(out);
        self.commitments.encode(out);
        self.proofs.encode(out);
    }

    /// Outputs the RLP length of the [BlobTransactionSidecar] fields, without a RLP header.
    pub fn fields_len(&self) -> usize {
        self.blobs.length() + self.commitments.length() + self.proofs.length()
    }

    /// Calculates a size heuristic for the in-memory size of the [BlobTransactionSidecar].
    #[inline]
    pub fn size(&self) -> usize {
        self.blobs.len() * BYTES_PER_BLOB + // blobs
            self.commitments.len() * BYTES_PER_COMMITMENT + // commitments
            self.proofs.len() * BYTES_PER_PROOF // proofs
    }
}

impl Encodable for BlobTransactionSidecar {
    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, without a RLP header.
    fn encode(&self, s: &mut dyn BufMut) {
        self.encode_inner(s);
    }

    fn length(&self) -> usize {
        self.fields_len()
    }
}

impl Decodable for BlobTransactionSidecar {
    /// Decodes the inner [BlobTransactionSidecar] fields from RLP bytes, without a RLP header.
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            blobs: Decodable::decode(buf)?,
            commitments: Decodable::decode(buf)?,
            proofs: Decodable::decode(buf)?,
        })
    }
}

// Helper function to deserialize boxed blobs
#[cfg(all(debug_assertions, feature = "serde"))]
fn deserialize_blobs<'de, D>(deserializer: D) -> Result<Vec<Blob>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use serde::Deserialize;

    let raw_blobs = Vec::<alloy_primitives::Bytes>::deserialize(deserializer)?;
    let mut blobs = Vec::with_capacity(raw_blobs.len());
    for blob in raw_blobs {
        blobs.push(Blob::try_from(blob.as_ref()).map_err(serde::de::Error::custom)?);
    }
    Ok(blobs)
}

/// An error that can occur when validating a [BlobTransactionSidecar::validate].
#[derive(Debug)]
#[cfg(feature = "kzg")]
pub enum BlobTransactionValidationError {
    /// Proof validation failed.
    InvalidProof,
    /// An error returned by [`c_kzg`].
    KZGError(c_kzg::Error),
    /// The inner transaction is not a blob transaction.
    NotBlobTransaction(u8),
    /// Error variant for thrown by EIP-4844 tx variants without a sidecar.
    MissingSidecar,
    /// The versioned hash is incorrect.
    WrongVersionedHash {
        /// The versioned hash we got
        have: B256,
        /// The versioned hash we expected
        expected: B256,
    },
}

#[cfg(all(feature = "kzg", feature = "std"))]
impl std::error::Error for BlobTransactionValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidProof { .. } => None,
            Self::KZGError(source) => Some(source),
            Self::NotBlobTransaction { .. } => None,
            Self::MissingSidecar { .. } => None,
            Self::WrongVersionedHash { .. } => None,
        }
    }
}

#[cfg(feature = "kzg")]
impl core::fmt::Display for BlobTransactionValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidProof => f.write_str("invalid KZG proof"),
            Self::KZGError(err) => {
                write!(f, "KZG error: {:?}", err)
            }
            Self::NotBlobTransaction(err) => {
                write!(f, "unable to verify proof for non blob transaction: {}", err)
            }
            Self::MissingSidecar => {
                f.write_str("eip4844 tx variant without sidecar being used for verification.")
            }
            Self::WrongVersionedHash { have, expected } => {
                write!(f, "wrong versioned hash: have {}, expected {}", have, expected)
            }
        }
    }
}

#[cfg(feature = "kzg")]
impl From<c_kzg::Error> for BlobTransactionValidationError {
    fn from(source: c_kzg::Error) -> Self {
        Self::KZGError(source)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A blob sidecar.
pub struct BlobTransactionSidecarItem {
    /// The index.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub index: usize,
    /// The index.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub blob: Blob,
    /// The KZG commitment.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_commitment"))]
    pub kzg_commitment: Bytes48,
    /// The KZG proof.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_proof"))]
    pub kzg_proof: Bytes48,
}
#[cfg(feature = "kzg")]
impl BlobTransactionSidecarItem {
    /// `VERSIONED_HASH_VERSION_KZG ++ sha256(commitment)[1..]`
    pub fn to_kzg_versioned_hash(&self) -> [u8; 32] {
        let commitment = self.kzg_commitment.as_slice();
        let mut hash: [u8; 32] = Sha256::digest(commitment).into();
        hash[0] = VERSIONED_HASH_VERSION_KZG;
        hash
    }

    /// Verifies the KZG proof of a blob to ensure its integrity and correctness.
    pub fn verify_blob_kzg_proof(&self) -> Result<bool, BlobTransactionValidationError> {
        let binding = EnvKzgSettings::Default;
        let settings = binding.get();

        let blob = c_kzg::Blob::from_bytes(self.blob.as_slice())
            .map_err(BlobTransactionValidationError::KZGError)?;

        let commitment = c_kzg::Bytes48::from_bytes(self.kzg_commitment.as_slice())
            .map_err(BlobTransactionValidationError::KZGError)?;

        let proof = c_kzg::Bytes48::from_bytes(self.kzg_proof.as_slice())
            .map_err(BlobTransactionValidationError::KZGError)?;

        let result = KzgProof::verify_blob_kzg_proof(&blob, &commitment, &proof, settings)
            .map_err(BlobTransactionValidationError::KZGError)?;

        if result {
            Ok(true)
        } else {
            Err(BlobTransactionValidationError::InvalidProof)
        }
    }
    /// Verify the blob sidecar against it's [BlockNumHash].

    pub fn verify_blob(&self, hash: &BlockNumHash) -> Result<(), BlobTransactionValidationError> {
        if self.index != hash.number as usize {
            let blob_hash_part = FixedBytes::<32>::from_slice(&self.blob[0..32]);
            return Err(BlobTransactionValidationError::WrongVersionedHash {
                have: blob_hash_part,
                expected: hash.hash,
            });
        }

        let computed_hash = self.to_kzg_versioned_hash();
        if computed_hash != hash.hash {
            return Err(BlobTransactionValidationError::WrongVersionedHash {
                have: computed_hash.into(),
                expected: alloy_primitives::FixedBytes(*hash.hash),
            });
        }

        match self.verify_blob_kzg_proof() {
            Ok(result) if result => Ok(()),
            Ok(_) => Err(BlobTransactionValidationError::InvalidProof),
            Err(_) => Err(BlobTransactionValidationError::InvalidProof),
        }
    }
}
#[cfg(feature = "serde")]
fn parse_u64_string<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: core::fmt::Display,
{
    String::deserialize(de)?.parse().map_err(serde::de::Error::custom)
}
#[cfg(test)]
mod tests {
    use super::*;

    use arbitrary::Arbitrary;

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_blob() {
        let blob = BlobTransactionSidecar {
            blobs: vec![Blob::default(), Blob::default(), Blob::default(), Blob::default()],
            commitments: vec![
                Bytes48::default(),
                Bytes48::default(),
                Bytes48::default(),
                Bytes48::default(),
            ],
            proofs: vec![
                Bytes48::default(),
                Bytes48::default(),
                Bytes48::default(),
                Bytes48::default(),
            ],
        };

        let s = serde_json::to_string(&blob).unwrap();
        let deserialized: BlobTransactionSidecar = serde_json::from_str(&s).unwrap();
        assert_eq!(blob, deserialized);
    }

    #[test]
    fn test_arbitrary_blob() {
        let mut unstructured = arbitrary::Unstructured::new(b"unstructured blob");
        let _blob = BlobTransactionSidecar::arbitrary(&mut unstructured).unwrap();
    }
}
