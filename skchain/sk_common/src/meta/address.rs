use std::default;

use ed25519_dalek::PublicKey;

pub struct Address {
    did: PublicKey,
}

impl Address {
    pub fn new(did: PublicKey) -> Self {
        Self { did }
    }

    pub fn default() -> Self {
        Self {
            did: PublicKey::default(),
        }
    }

    pub fn set_did(&mut self, did: String) {
        self.did = PublicKey::from_bytes(did.as_bytes()).unwrap();
    }
}
