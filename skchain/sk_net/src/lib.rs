mod net;

use futures::StreamExt;
use ipfs_ds_rocksdb::BlockStore;
use ipfs_ds_rocksdb::Config;
use libipld::prelude::References;
use libipld::store::{DefaultParams, StoreParams};
use libipld::Block;
use libipld::Cid;
use libipld::Ipld;
use libp2p::core::upgrade;
use libp2p::core::Transport;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{GetClosestPeersError, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::swarm::SwarmEvent;
use libp2p::Swarm;
use libp2p::{dns, mplex, noise, tcp, websocket, yamux};
use libp2p::{identity, Multiaddr, PeerId};
use libp2p_bitswap::BitswapStore;
use net::{NetworkConfig, NetworkService};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::{env, error::Error, str::FromStr, time::Duration};
use tokio;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageConfig {
    /// The path to use for the block store. If it is `None` an in-memory block
    /// store will be used.
    pub path: Option<PathBuf>,
    /// The path to use for the database that persists block accesses times for
    /// the LRU cache. If this is set to 'None', access times will not be
    /// persisted but just tracked in memory.
    ///
    /// You can point this to the same file as the main block store, but this is
    /// not recommended.
    pub access_db_path: Option<PathBuf>,
    /// The target number of blocks.
    ///
    /// Up to this number, the store will retain everything even if
    /// not pinned. Once this number is exceeded, the store will run garbage
    /// collection of all unpinned blocks until the block criterion is met
    /// again.
    ///
    /// To completely disable storing of non-pinned blocks, set this to 0. Even
    /// then, the store will never delete pinned blocks.
    pub cache_size_blocks: u64,
    /// The target store size.
    ///
    /// Up to this size, the store will retain everything even if not pinned.
    /// Once this size is exceeded, the store will run garbage collection of
    /// all unpinned blocks until the size criterion is met again.
    ///
    /// The store will never delete pinned blocks.
    pub cache_size_bytes: u64,
    /// The interval at which the garbage collector is run.
    ///
    /// Note that this is implemented as delays between gcs, so it will not run
    /// exactly at this interval, but there will be some drift if gc takes
    /// long.
    pub gc_interval: Duration,
    /// The minimum number of blocks to collect in any case.
    ///
    /// Using this parameter, it is possible to guarantee a minimum rate with
    /// which the gc will be able to keep up. It is `gc_min_blocks` /
    /// `gc_interval`.
    pub gc_min_blocks: usize,
    /// The target maximum gc duration of a single garbage collector run.
    ///
    /// This can not be guaranteed, since we guarantee to collect at least
    /// `gc_min_blocks`. But as soon as this duration is exceeded, the
    /// incremental gc will stop doing additional work.
    pub gc_target_duration: Duration,
}

impl StorageConfig {
    /// Creates a new `StorageConfig`.
    pub fn new(
        path: Option<PathBuf>,
        access_db_path: Option<PathBuf>,
        cache_size: u64,
        gc_interval: Duration,
    ) -> Self {
        Self {
            path,
            access_db_path,
            cache_size_blocks: cache_size,
            cache_size_bytes: u64::MAX,
            gc_interval,
            gc_min_blocks: usize::MAX,
            gc_target_duration: Duration::new(u64::MAX, 1_000_000_000 - 1),
        }
    }
}

#[derive(Clone)]
pub struct StorageService<S: StoreParams> {
    inner: Arc<StorageServiceInner<S>>,
}

impl<S: StoreParams> StorageService<S>
where
    Ipld: References<S::Codecs>,
{
    pub fn open(config: StorageConfig) -> anyhow::Result<Self> {
        let inner = StorageServiceInner::open(config)?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}
struct StorageServiceInner<S: StoreParams> {
    store: Arc<Mutex<BlockStore<S>>>,
    gc_target_duration: Duration,
    gc_min_blocks: usize,
    // gc_task: Option<JoinHandle<()>>,
}

impl<S: StoreParams> Drop for StorageServiceInner<S> {
    fn drop(&mut self) {
        // if let Some(t) = self.gc_task.take() {
        //     t.abort()
        // }
    }
}

impl<S: StoreParams> StorageServiceInner<S>
where
    Ipld: References<S::Codecs>,
{
    pub fn open(config: StorageConfig) -> anyhow::Result<Self> {
        let store_config = Config::default();
        // create DB
        let store = BlockStore::open(config.path.unwrap(), store_config).unwrap();

        let store = Arc::new(Mutex::new(store));

        Ok(Self {
            gc_target_duration: config.gc_target_duration,
            gc_min_blocks: config.gc_min_blocks,
            store,
        })
    }
}

struct BitswapStorage<P: StoreParams>(StorageService<P>);

impl<P: StoreParams> BitswapStore for BitswapStorage<P>
where
    Ipld: References<P::Codecs>,
{
    type Params = P;

    fn contains(&mut self, cid: &Cid) -> anyhow::Result<bool> {
        self.0.contains(cid)
    }

    fn get(&mut self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>> {
        self.0.get(cid)
    }

    fn insert(&mut self, block: &Block<P>) -> anyhow::Result<()> {
        self.0.insert(block.clone())
    }

    fn missing_blocks(&mut self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        self.0.missing_blocks(cid)
    }
}

impl<S: StoreParams> StorageService<S>
where
    Ipld: References<S::Codecs>,
{
    pub fn contains(&self, cid: &Cid) -> anyhow::Result<bool> {
        let mut lock = self.inner.store.lock();
        lock.unwrap().has_cid(cid)
    }

    pub fn get(&self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>> {
        let mut lock = self.inner.store.lock();
        lock.unwrap().get_block(cid)
    }

    pub fn insert(&self, block: Block<S>) -> anyhow::Result<()> {
        let mut lock = self.inner.store.lock();
        lock.unwrap().put_block(block, None)
    }

    pub fn missing_blocks(&self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        let mut lock = self.inner.store.lock();
        lock.unwrap().get_missing_blocks(cid)
    }
}

const BOOTNODES: [&str; 4] = [
    "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
];

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let local_key = identity::ed25519::Keypair::generate();
    // let local_peer_id = PeerId::from(local_key.public());
    let config = NetworkConfig::new(local_key);
    let sweep_interval = std::time::Duration::from_millis(10000);
    let storage_config = StorageConfig::new(
        Some(Path::new("").join("../../blocks")),
        None,
        0,
        sweep_interval,
    );
    let storage = StorageService::<DefaultParams>::open(storage_config).unwrap();
    let bitswap = BitswapStorage(storage.clone());

    let network = NetworkService::new(config, bitswap).await?;
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
