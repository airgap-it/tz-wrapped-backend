pub mod coding;
pub mod contract;
pub mod micheline;
pub mod utils;

use base58check::{FromBase58Check, ToBase58Check};
use derive_more::{Display, Error};
use sodiumoxide::crypto::sign;

use crate::crypto;

#[derive(Error, Display, Debug)]
pub enum TzError {
    InvalidIndex,
    InvalidType,
    InvalidArgument,
    NetworkFailure,
    ParsingFailure,
    InvalidPublicKey,
    InvalidSignature,
    HashFailure,
    HexDecodingFailure,
}

impl From<serde_json::Error> for TzError {
    fn from(_: serde_json::Error) -> Self {
        TzError::ParsingFailure
    }
}

impl From<num_bigint::ParseBigIntError> for TzError {
    fn from(_: num_bigint::ParseBigIntError) -> Self {
        TzError::ParsingFailure
    }
}

pub fn edsig_to_bytes(signature: &str) -> Result<[u8; sign::SIGNATUREBYTES], TzError> {
    if !signature.starts_with("edsig") {
        return Err(TzError::InvalidSignature);
    }

    let (_version, decoded) = signature
        .from_base58check()
        .map_err(|_error| TzError::InvalidSignature)?;

    let decode_without_prefix = &decoded[4..];

    if decode_without_prefix.len() != sign::SIGNATUREBYTES {
        return Err(TzError::InvalidSignature);
    }

    let mut result: [u8; sign::SIGNATUREBYTES] = [0; sign::SIGNATUREBYTES];
    result.copy_from_slice(decode_without_prefix);

    Ok(result)
}

pub fn edpk_to_bytes(pk: &str) -> Result<[u8; sign::PUBLICKEYBYTES], TzError> {
    if !pk.starts_with("edpk") {
        return Err(TzError::InvalidPublicKey);
    }

    let (_version, decoded) = pk
        .from_base58check()
        .map_err(|_error| TzError::InvalidPublicKey)?;

    let decode_without_prefix = &decoded[3..];

    if decode_without_prefix.len() != sign::PUBLICKEYBYTES {
        return Err(TzError::InvalidPublicKey);
    }

    let mut result: [u8; sign::PUBLICKEYBYTES] = [0; sign::PUBLICKEYBYTES];
    result.copy_from_slice(decode_without_prefix);

    Ok(result)
}

pub fn edpk_to_tz1(pk: &str) -> Result<String, TzError> {
    let pk_bytes = edpk_to_bytes(pk)?;

    let hash = crypto::generic_hash(&pk_bytes, 20).map_err(|_error| TzError::InvalidArgument)?;
    let mut result = Vec::<u8>::new();

    result.extend_from_slice(&vec![161, 159]);
    result.extend_from_slice(hash.as_ref());

    Ok(result.to_base58check(6))
}
