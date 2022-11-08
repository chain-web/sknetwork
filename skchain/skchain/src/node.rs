use std::{time::Duration, path::PathBuf};

use libipld::DefaultParams;
use libp2p::identity::ed25519::Keypair;
use sk_core::{genesis::GenesisConfigBuilder, sk_chain::{SkChainBuilder, SkChain}};
use sk_fs::{StorageConfig, Skfs, Config, BitswapStorage, StorageService};
use sk_net::{NetworkConfig, NetworkService};
use tempdir::TempDir;
use tokio;

#[tokio::main]
pub async fn start_node() -> anyhow::Result<SkChain> {
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

    let skfs = Skfs::<DefaultParams> { storage, network: network.clone() };

    let node = SkChainBuilder {
        genesis: GenesisConfigBuilder::build_local(),
        fs:skfs,
        network
    }
    .build();
    anyhow::Ok(node)
}
