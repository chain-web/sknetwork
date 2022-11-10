use super::address::Address;
use libipld::Cid;
use num_bigint::BigUint;
use std::collections::HashMap;

pub struct Account {
    // current account
    account: Address,
    // number of current account transaction times
    nonce: BigUint,
    // contribute
    contribute: BigUint,
    // balance {age: amount}, age is transaction time
    balance: HashMap<u128, BigUint>,
    // contract database
    storageRoot: Cid,
    // contract code data
    codeCid: Cid,
    // contract owner
    owner: Address,
}

impl Account {
    pub fn new() -> Self {
        Self {
            account: Address::default(),
            nonce: BigUint::default(),
            contribute: BigUint::default(),
            balance: HashMap::default(),
            storageRoot: Cid::default(),
            codeCid: Cid::default(),
            owner: Address::default(),
        }
    }

    pub fn set_account(&mut self, did: String) {
      self.account.set_did(did);
    }

    pub fn plus_blance(&mut self, amount: BigUint, age: u128) {
      self.balance.insert(age, amount);
    }

    // pub fn from_bytes(bytes: Vec<u8>) -> Self {

    // }
}
