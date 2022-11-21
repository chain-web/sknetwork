use num_bigint::BigUint;
use serde::ser::{SerializeStruct, Serializer, SerializeMap};
use serde::Serialize;
use std::collections::HashMap;

#[derive(PartialEq, Debug)]
pub struct Balance {
    inner: HashMap<u128, BigUint>,
}

impl Serialize for Balance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 1 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("Balance", 1)?;

        let mut map = serializer.serialize_map(Some(self.inner.len()))?;
        for (k, v) in self.inner {
            map.serialize_entry(&k.to_be_bytes(), &v.to_bytes_be())?;
        }
        let res= map.end();
        // map.
        state.serialize_field("inner", )?;
        state.end()
    }
}
