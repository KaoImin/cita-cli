pub mod blockchain;

pub use self::blockchain::{Crypto, SignedTransaction, Transaction, UnverifiedTransaction};
use client::remove_0x;
use crypto::PubKey;
use crypto::{pubkey_to_address, sign, Encryption, Hashable, KeyPair, PrivateKey, Signature};
use hex;
use protobuf::Message as MessageTrait;
use protobuf::{parse_from_bytes, ProtobufEnum};
use serde_json::Value;
use types::H256;

use error::ToolError;
use std::str::FromStr;

impl UnverifiedTransaction {
    /// UnverifiedTransaction as JSON Value
    pub fn to_json(&self, encryption: Encryption) -> Result<Value, String> {
        let tx = self.transaction.get_ref();
        let pub_key = self.public_key(encryption)?;
        Ok(json!({
            "transaction": {
                "to": tx.to,
                "nonce": tx.nonce,
                "quota": tx.quota,
                "valid_until_block": tx.valid_until_block,
                "data": format!("0x{}", hex::encode(&tx.data)),
                "value": tx.value,
                "chain_id": tx.chain_id,
                "version": tx.version,
                "pub_key": pub_key.to_string(),
                "sender": pubkey_to_address(&pub_key),
            },
            "signature": format!("0x{}", hex::encode(&self.signature)),
            "crypto": self.crypto.value(),
        }))
    }

    /// Get the transaction public key
    pub fn public_key(&self, encryption: Encryption) -> Result<PubKey, String> {
        let bytes: Vec<u8> = self.get_transaction().write_to_bytes().unwrap();
        let hash = bytes.crypt_hash(encryption);
        let signature = self.get_signature();
        let sig = Signature::from(signature);

        match self.get_crypto() {
            Crypto::SECP => sig.recover(&hash),
            _ => Err("Mismatched encryption algorithm".to_string()),
        }
    }
}

impl FromStr for UnverifiedTransaction {
    type Err = ToolError;

    /// Parse UnverifiedTransaction from hex string
    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(remove_0x(content)).map_err(ToolError::Decode)?;
        parse_from_bytes(&bytes).map_err(ToolError::Proto)
    }
}

impl Transaction {
    /// Sign data
    pub fn sign(&self, sk: PrivateKey) -> SignedTransaction {
        let key_pair = KeyPair::from_privkey(sk);
        let pubkey = key_pair.pubkey();

        let unverified_tx = self.build_unverified(sk);

        // Build SignedTransaction
        let mut signed_tx = SignedTransaction::new();
        signed_tx.set_signer(pubkey.to_vec());
        let bytes: Vec<u8> = (&unverified_tx).write_to_bytes().unwrap();

        let hash = match sk {
            PrivateKey::Secp256k1(_) => bytes.crypt_hash(Encryption::Secp256k1),
            #[cfg(feature = "ed25519")]
            PrivateKey::Ed25519(_) => bytes.crypt_hash(Encryption::Ed25519),
            PrivateKey::Sm2(_) => bytes.crypt_hash(Encryption::Sm2),
            PrivateKey::Null => H256::default(),
        };

        signed_tx.set_tx_hash(hash.to_vec());
        signed_tx.set_transaction_with_sig(unverified_tx);
        signed_tx
    }

    /// Build unverified transaction
    pub fn build_unverified(&self, sk: PrivateKey) -> UnverifiedTransaction {
        let mut unverified_tx = UnverifiedTransaction::new();
        let bytes: Vec<u8> = self.write_to_bytes().unwrap();

        let hash = match sk {
            PrivateKey::Secp256k1(_) => bytes.crypt_hash(Encryption::Secp256k1),
            #[cfg(feature = "ed25519")]
            PrivateKey::Ed25519(_) => bytes.crypt_hash(Encryption::Ed25519),
            PrivateKey::Sm2(_) => bytes.crypt_hash(Encryption::Sm2),
            PrivateKey::Null => H256::default(),
        };

        unverified_tx.set_transaction(self.clone());
        let signature = sign(&sk, &hash);
        unverified_tx.set_signature(signature.to_vec());
        unverified_tx.set_crypto(Crypto::SECP);
        unverified_tx
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Just to show how to parse Transaction from bytes
    #[test]
    fn test_parse_from_bytes() {
        let content = hex::decode("0a580a28666666666666666666666666666666666666666666666666666666666666666666666666666666661220383865613735396361306465343537353930333965323664623866616633346618c0843d2098f7242a02abce12410eb039fe08783d62f30e1bb5542312e519e7f6bb84ba1c3c08101af902463fda5f1c0e4d54d93bab2541d0a4aa5b85e71dfbaf5206131db6d491b4ffd256e78c00").unwrap();
        let tx: UnverifiedTransaction = parse_from_bytes(&content).unwrap();
        assert_eq!("abce", hex::encode(&tx.transaction.get_ref().data));
    }
}
