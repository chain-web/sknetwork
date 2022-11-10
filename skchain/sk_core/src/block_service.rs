use bincode::{deserialize, serialize};
use std::collections::HashMap;

use libipld::{
    cbor::DagCborCodec,
    multihash::{Code, MultihashDigest},
    prelude::Codec,
    Cid, DagCbor, DefaultParams,
};
use sk_fs::Skfs;

type Block = libipld::Block<libipld::DefaultParams>;

#[derive(Debug, DagCbor)]
pub struct BlockNode {
    links: Vec<Cid>,
    text: String,
}

impl BlockNode {
    pub fn leaf(text: &str) -> Self {
        Self {
            links: Vec::new(),
            text: text.into(),
        }
    }

    pub fn branch(text: &str, links: impl IntoIterator<Item = Cid>) -> Self {
        Self {
            links: links.into_iter().collect(),
            text: text.into(),
        }
    }

    pub fn default() -> Self {
        Self {
            links: Vec::default(),
            text: String::default(),
        }
    }

    pub fn to_block(&self) -> Block {
        let bytes = DagCborCodec.encode(self).unwrap();
        let hash = Code::Sha2_256.digest(&bytes);
        Block::new_unchecked(Cid::new_v1(0x71, hash), bytes)
    }

    // pub fn from_block() -> Self {}
}

fn gen_link(name: &str, children: Vec<&Block>) -> Block {
    let ipld = BlockNode::branch(name, children.iter().map(|b| *b.cid()).collect::<Vec<_>>());
    let bytes = DagCborCodec.encode(&ipld).unwrap();
    let hash = Code::Sha2_256.digest(&bytes);
    Block::new_unchecked(Cid::new_v1(0x71, hash), bytes)
}

fn gen_block(name: &str) -> Block {
    let ipld = BlockNode::leaf(name);
    let bytes = DagCborCodec.encode(&ipld).unwrap();
    let hash = Code::Sha2_256.digest(&bytes);
    Block::new_unchecked(Cid::new_v1(0x71, hash), bytes)
}

fn gen_block_by_bytes(bytes: &Vec<u8>) -> Block {
    let hash = Code::Sha2_256.digest(&bytes);
    Block::new_unchecked(Cid::new_v1(0x71, hash), bytes.clone())
}

struct BlockStore {
    root_cid: Cid,
    root_blocks: BlockNode, // index every 10k block array's BlockNode cid
    sub_blocks: HashMap<u32, BlockNode>, // all blocks ,key is floor(block number)/10000
}

impl BlockStore {
    pub fn new() -> Self {
        BlockStore {
            root_cid: Cid::default(),
            root_blocks: BlockNode::default(),
            sub_blocks: HashMap::default(),
        }
    }

    pub fn check_blocks(self) {}
    pub fn load_current_10k_blocks(self) {}

    pub fn set_root_cid(mut self, cid: Cid) {
        self.root_cid = cid;
    }
}

pub struct BlockService {
    fs: Skfs<DefaultParams>,
    store: BlockStore,
}

impl BlockService {
    pub fn new(fs: Skfs<DefaultParams>) -> Self {
        BlockService {
            fs,
            store: BlockStore::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.store.root_cid.codec() == 0
    }

    pub fn init(self, cid: Cid) {
        self.store.set_root_cid(cid);
    }

    pub fn save(self) {
        let block = self.store.root_blocks.to_block();
        self.fs.storage.insert(block.clone()).unwrap();
        self.store.set_root_cid(*block.cid());
    }
}
