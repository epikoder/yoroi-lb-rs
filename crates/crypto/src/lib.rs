pub mod hash {
    use bcrypt::{hash, verify, BcryptError};
    static COST: u32 = 10;
    /// Generate bcrypt hash from string
    pub fn make(s: String) -> Result<String, BcryptError> {
        hash(s, COST)
    }

    /// Verify bcrypt hash from string
    pub fn check(s: String, h: &str) -> bool {
        verify(s, h).ok().unwrap()
    }
}

pub mod crypto_aes {
    use std::env;

    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
    use once_cell::sync::Lazy;

    type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    static CIPHER: Lazy<Aes> = Lazy::new(|| {
        let mut secret: [u8; 32] = [0; 32];
        let mut iv: [u8; 16] = [0; 16];

        secret.copy_from_slice(
            &env::var("AES_KEY")
                .or_else(|_| -> Result<std::string::String, env::VarError> {
                    panic!("Failed to retrieve AES_KEY environment variable",);
                })
                .unwrap()
                .as_bytes(),
        );
        iv.copy_from_slice(
            &env::var("AES_IV")
                .or_else(|_| -> Result<std::string::String, env::VarError> {
                    panic!("Failed to retrieve AES_IV environment variable");
                })
                .unwrap()
                .as_bytes(),
        );
        Aes {
            encoder: Aes256CbcEnc::new(&secret.into(), &iv.into()),
            decoder: Aes256CbcDec::new(&secret.into(), &iv.into()),
        }
    });

    struct Aes {
        encoder: Aes256CbcEnc,
        decoder: Aes256CbcDec,
    }

    impl Aes {
        #[cfg(test)]
        fn new_cipher() -> Self {
            let secret: [u8; 32] = [0x42; 32];
            let iv: [u8; 16] = [0x42; 16];
            Aes {
                encoder: Aes256CbcEnc::new(&secret.into(), &iv.into()),
                decoder: Aes256CbcDec::new(&secret.into(), &iv.into()),
            }
        }

        fn encode(&self, text: &[u8]) -> Vec<u8> {
            self.encoder.clone().encrypt_padded_vec_mut::<Pkcs7>(text)
        }

        fn decode(&self, text: &[u8]) -> Vec<u8> {
            self.decoder
                .clone()
                .decrypt_padded_vec_mut::<Pkcs7>(text)
                .unwrap()
        }
    }

    pub fn encode(text: &[u8]) -> Vec<u8> {
        CIPHER.encode(text)
    }
    pub fn decode(buf: &[u8]) -> Vec<u8> {
        CIPHER.decode(buf)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_encode() {
            let aes = Aes::new_cipher();

            let plaintext = b"8d487643-8a4c-4045-aa97-3498b7fa5e91";
            let result = aes.encode(plaintext);
            let output = aes.decode(&result);
            assert_eq!(plaintext[..], output[..]);
        }
    }
}

pub mod base64 {
    use base64::{prelude::*, DecodeError};
    pub fn encode<T: AsRef<[u8]>>(i: T) -> String {
        BASE64_STANDARD.encode(i)
    }

    pub fn decode<T: AsRef<[u8]>>(i: T) -> Result<Vec<u8>, DecodeError> {
        BASE64_STANDARD.decode(i)
    }
}
