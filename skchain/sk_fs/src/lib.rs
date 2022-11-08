mod skfs;

use futures::StreamExt;
use libipld::DefaultParams;
use libp2p::core::upgrade;
use libp2p::core::Transport;
use libp2p::identity::ed25519::Keypair;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{GetClosestPeersError, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::swarm::SwarmEvent;
use libp2p::Swarm;
use libp2p::{dns, mplex, noise, tcp, websocket, yamux};
use libp2p::{identity, Multiaddr, PeerId};
use sk_net::NetworkConfig;
use sk_net::NetworkService;
pub use skfs::{Config, Skfs, StorageConfig, StorageService, BitswapStorage};
use std::path::Path;
use std::path::PathBuf;
use std::{env, error::Error, str::FromStr, time::Duration};
use tempdir::TempDir;
// use tokio::task;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let tmp = TempDir::new("skfs")?;
    let sweep_interval = Duration::from_millis(10000);
    let mut pb: PathBuf = PathBuf::new();
    pb.push(tmp);
    // let path = Path::new("").join("../../blocks");
    let storage_config = StorageConfig::new(Some(pb), None, 10, sweep_interval);

    let mut config = NetworkConfig::new(Keypair::generate());
    let storage = StorageService::open(storage_config)?;

    let bitswap = BitswapStorage::<DefaultParams>(storage.clone());

    let network = NetworkService::new(config, bitswap).await?;

    let skfs = Skfs::<DefaultParams> { storage, network };
    anyhow::Ok(())
}

mod test {
    #[test]
    fn start_node() {
        println!("before start");
        let _ = super::main();
    }
}
