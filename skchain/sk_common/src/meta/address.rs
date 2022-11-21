use serde::ser::{Serializer, SerializeStruct};
use serde::Serialize;

use ed25519_dalek::PublicKey;

#[derive(PartialEq, Debug)]
pub struct Address {
    did: PublicKey,
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 1 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("Address", 1)?;
        let did_bytes = self.did.to_bytes();
        state.serialize_field("did", &did_bytes)?;
        state.end()
    }
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

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
}
