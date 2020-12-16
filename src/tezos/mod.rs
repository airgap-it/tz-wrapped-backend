pub mod coding;
pub mod contract;
pub mod micheline;
pub mod utils;

use base58check::{FromBase58Check, ToBase58Check};
use micheline::TzError;
use sodiumoxide::crypto::sign;

use crate::{api::models::error::APIError, crypto};

pub fn edsig_to_bytes(signature: &str) -> Result<[u8; sign::SIGNATUREBYTES], APIError> {
    if !signature.starts_with("edsig") {
        return Err(APIError::InvalidSignature);
    }

    let (_version, decoded) = signature
        .from_base58check()
        .map_err(|_error| APIError::InvalidSignature)?;

    let decode_without_prefix = &decoded[4..];

    if decode_without_prefix.len() != sign::SIGNATUREBYTES {
        return Err(APIError::InvalidSignature);
    }

    let mut result: [u8; sign::SIGNATUREBYTES] = [0; sign::SIGNATUREBYTES];
    result.copy_from_slice(decode_without_prefix);

    Ok(result)
}

pub fn edpk_to_bytes(pk: &str) -> Result<[u8; sign::PUBLICKEYBYTES], APIError> {
    if !pk.starts_with("edpk") {
        return Err(APIError::InvalidPublicKey);
    }

    let (_version, decoded) = pk
        .from_base58check()
        .map_err(|_error| APIError::InvalidPublicKey)?;

    let decode_without_prefix = &decoded[3..];

    if decode_without_prefix.len() != sign::PUBLICKEYBYTES {
        return Err(APIError::InvalidPublicKey);
    }

    let mut result: [u8; sign::PUBLICKEYBYTES] = [0; sign::PUBLICKEYBYTES];
    result.copy_from_slice(decode_without_prefix);

    Ok(result)
}

pub fn edpk_to_tz1(pk: &str) -> Result<String, APIError> {
    let pk_bytes = edpk_to_bytes(pk)?;

    let hash = crypto::generic_hash(&pk_bytes, 20).map_err(|_error| TzError::InvalidArgument)?;
    let mut result = Vec::<u8>::new();

    result.extend_from_slice(&vec![161, 159]);
    result.extend_from_slice(hash.as_ref());

    Ok(result.to_base58check(6))
}
