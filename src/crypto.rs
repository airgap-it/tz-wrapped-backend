use sodiumoxide::{ crypto::sign, randombytes, crypto::generichash };

pub fn generate_random_bytes(size: usize) -> Vec<u8> {
    randombytes::randombytes(size)
}

pub fn sign_detached(message: &[u8], private_key: [u8; sign::SECRETKEYBYTES]) -> [u8; sign::SIGNATUREBYTES] {
    let secret = sign::SecretKey(private_key);
    let signature = sign::sign_detached(message, &secret);
    return signature.0;
}

pub fn verify_detached(message: &[u8], signature: [u8; sign::SIGNATUREBYTES], public_key: [u8; sign::PUBLICKEYBYTES]) -> bool {
    let key = sign::PublicKey(public_key);
    let sig = sign::Signature(signature);
    sign::verify_detached(&sig, &message, &key)
}

pub fn generic_hash(payload: &[u8], size: usize) -> Result<Vec<u8>, ()> {
    let mut hasher = generichash::State::new(size, None)?;
    hasher.update(payload)?;
    let hash = hasher.finalize()?;
    
    Ok(hash.as_ref().to_owned())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sign_verify() -> () {
        let message: &[u8] = "test message".as_bytes();
        let key_pair = sign::gen_keypair();
        let signature = sign_detached(message, key_pair.1.0);
        
        let verified = verify_detached(message, signature, key_pair.0.0);

        assert!(verified)
    }

    #[test]
    fn test_random_bytes() -> () {
        let random = generate_random_bytes(32);

        assert_eq!(random.len(), 32);
    }
}
