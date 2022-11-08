mod net;

use futures::StreamExt;
use ipfs_ds_rocksdb::BlockStore;
use ipfs_ds_rocksdb::Config;
use libipld::prelude::References;
use libipld::store::{DefaultParams, StoreParams};
use libipld::Block;
use libipld::Cid;
use libipld::Ipld;
use libp2p::kad;
use libp2p::{identity, Multiaddr, PeerId};
use libp2p_bitswap::BitswapStore;
pub use net::{NetworkConfig, NetworkService};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::{env, error::Error, str::FromStr, time::Duration};
use tokio;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    Ok(())
}

mod test {
    use std::{path::Path, fs};

    #[test]
    fn start_node() {
        println!("before start");
        let _ = super::main();
        let path = Path::new("").join("../../blocks");
        let _ = fs::remove_dir_all(path);
    }
}
