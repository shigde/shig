use std::error::Error;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::LineEnding;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};

/// A private/public key pair used for HTTP signatures
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Private key in PEM format
    pub private_key: String,
    /// Public key in PEM format
    pub public_key: String,
}

impl KeyPair {
    /// Helper method to turn this into an openssl private key
    #[cfg(test)]
    pub(crate) fn private_key(&self) -> Result<RsaPrivateKey, Box<dyn Error>> {
        use rsa::pkcs8::DecodePrivateKey;
        Ok(RsaPrivateKey::from_pkcs8_pem(&self.private_key)?)
    }

    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut rng = rand::thread_rng();
        let rsa = RsaPrivateKey::new(&mut rng, 2048)?;
        let pkey = RsaPublicKey::from(&rsa);
        let public_key = pkey.to_public_key_pem(LineEnding::default())?;
        let private_key = rsa.to_pkcs8_pem(LineEnding::default())?.to_string();
        Ok(Self{
            private_key,
            public_key,
        })
    }

}


// pub fn encrypt(data: String, pub_key: RsaPublicKey) {
//     let mut rng = rand::thread_rng();
//     let enc_data = pub_key
//         .encrypt(&mut rng, Pkcs1v15Encrypt, &data.as_bytes())
//         .expect("failed to encrypt");
//     //assert_ne!(&data[..], &enc_data[..]);
// }
//
// pub fn decrypt(enc_data: &[u8], priv_key: RsaPrivateKey) {
//     let dec_data = priv_key
//         .decrypt(Pkcs1v15Encrypt, enc_data)
//         .expect("failed to decrypt");
//     //assert_eq!(&data[..], &dec_data[..]);
// }
