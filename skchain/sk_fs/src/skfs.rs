/// convenience re-export of configuration types from libp2p
pub mod config {
    pub use libp2p::{
        dns::{ResolverConfig, ResolverOpts},
        gossipsub::GossipsubConfig,
        kad::record::store::MemoryStoreConfig as KadConfig,
        mdns::MdnsConfig,
        ping,
    };
    pub use libp2p_bitswap::BitswapConfig;
    pub use libp2p_broadcast::BroadcastConfig;
}

use ipfs_ds_rocksdb::BlockStore;
pub use libipld::{store::DefaultParams, Block, Cid};
pub use libp2p::{
    core::{ ConnectedPoint, Multiaddr, PeerId},
    identity,
    kad::{kbucket::Key as BucketKey, record::Key, PeerRecord, Quorum, Record},
    multiaddr,
    swarm::{AddressRecord, AddressScore},
};

// use chrono::{DateTime, Utc};
use futures::stream::Stream;
use libipld::{
    codec::References,
    error::BlockNotFound,
    store::{Store, StoreParams},
    Ipld, Result,
};
use libp2p::identity::ed25519::{Keypair, PublicKey};
use libp2p_bitswap::BitswapStore;
use parking_lot::Mutex;
use sk_net::{NetworkConfig, NetworkService};
// use prometheus::Registry;
use std::{collections::HashSet, path::{Path, PathBuf}, sync::Arc, time::Duration};

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
        let store_config = ipfs_ds_rocksdb::Config::default();
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

 pub struct BitswapStorage<P: StoreParams>(pub StorageService<P>);

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
        lock.has_cid(cid)
    }

    pub fn get(&self, cid: &Cid) -> anyhow::Result<Option<Vec<u8>>> {
        let mut lock = self.inner.store.lock();
        lock.get_block(cid)
    }

    pub fn insert(&self, block: Block<S>) -> anyhow::Result<()> {
        let mut lock = self.inner.store.lock();
        lock.put_block(block, None)
    }

    pub fn missing_blocks(&self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        let mut lock = self.inner.store.lock();
        lock.get_missing_blocks(cid)
    }
}

/// Skfs configuration.
pub struct Config {
    /// Storage configuration.
    pub storage: StorageConfig,
    /// Network configuration.
    pub network: NetworkService<DefaultParams>,
}

/// Skfs client.
#[derive(Clone)]
pub struct Skfs<P: StoreParams> {
    pub storage: StorageService<P>,
    pub network: NetworkService<DefaultParams>,
}

impl<P: StoreParams> std::fmt::Debug for Skfs<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Skfs").finish()
    }
}



impl<P: StoreParams> Skfs<P>
where
    Ipld: References<P::Codecs>,
{
    /// Creates a new `Skfs` from a `Config`.
    ///
    /// This starts three background tasks. The swarm, garbage collector and the
    /// dht cleanup tasks run in the background.
    pub async fn new(config: Config) -> Result<Self> {
        Self::new0(config).await
    }
    async fn new0(config: Config) -> Result<Self> {
        let storage = StorageService::open(config.storage)?;
        let bitswap = BitswapStorage(storage.clone());
        Ok(Self { storage, network: config.network })
    }

    // /// Returns the local `PublicKey`.
    // pub fn local_public_key(&self) -> PublicKey {
    //     self.network.local_public_key()
    // }

    // /// Returns the local `PeerId`.
    // pub fn local_peer_id(&self) -> PeerId {
    //     self.network.local_peer_id()
    // }

    // /// Returns the local node name.
    // pub fn local_node_name(&self) -> String {
    //     self.network.local_node_name()
    // }

    // /// Listens on a new `Multiaddr`.
    // pub fn listen_on(&self, addr: Multiaddr) -> Result<impl Stream<Item = ListenerEvent>> {
    //     self.network.listen_on(addr)
    // }

    // /// Returns the currently active listener addresses.
    // pub fn listeners(&self) -> Vec<Multiaddr> {
    //     self.network.listeners()
    // }

    // /// Adds an external address.
    // pub fn add_external_address(&self, addr: Multiaddr) {
    //     self.network.add_external_address(addr)
    // }

    // /// Returns the currently used external addresses.
    // pub fn external_addresses(&self) -> Vec<AddressRecord> {
    //     self.network.external_addresses()
    // }

    // /// Adds a known `Multiaddr` for a `PeerId`.
    // pub fn add_address(&self, peer: &PeerId, addr: Multiaddr) {
    //     self.network.add_address(peer, addr)
    // }

    // /// Removes a `Multiaddr` for a `PeerId`.
    // pub fn remove_address(&self, peer: &PeerId, addr: &Multiaddr) {
    //     self.network.remove_address(peer, addr)
    // }

    // /// Removes all unconnected peers without addresses which have been
    // /// in this state for at least the given duration
    // pub fn prune_peers(&self, min_age: Duration) {
    //     self.network.prune_peers(min_age);
    // }

    // /// Dials a `PeerId` using a known address.
    // pub fn dial(&self, peer: &PeerId) {
    //     self.network.dial(peer);
    // }

    // /// Dials a `PeerId` using `Multiaddr`.
    // pub fn dial_address(&self, peer: &PeerId, addr: Multiaddr) {
    //     self.network.dial_address(peer, addr);
    // }

    // /// Bans a `PeerId` from the swarm, dropping all existing connections and
    // /// preventing new connections from the peer.
    // pub fn ban(&self, peer: PeerId) {
    //     self.network.ban(peer)
    // }

    // /// Unbans a previously banned `PeerId`.
    // pub fn unban(&self, peer: PeerId) {
    //     self.network.unban(peer)
    // }

    // /// Returns the known peers.
    // pub fn peers(&self) -> Vec<PeerId> {
    //     self.network.peers()
    // }

    // /// Returns a list of connected peers.
    // pub fn connections(&self) -> Vec<(PeerId, Multiaddr, DateTime<Utc>, Direction)> {
    //     self.network.connections()
    // }

    // /// Returns `true` if there is a connection to peer.
    // pub fn is_connected(&self, peer: &PeerId) -> bool {
    //     self.network.is_connected(peer)
    // }

    // /// Returns the `PeerInfo` of a peer.
    // pub fn peer_info(&self, peer: &PeerId) -> Option<PeerInfo> {
    //     self.network.peer_info(peer)
    // }

    // /// Bootstraps the dht using a set of bootstrap nodes. After bootstrap
    // /// completes it provides all blocks in the block store.
    // pub async fn bootstrap(&self, nodes: &[(PeerId, Multiaddr)]) -> Result<()> {
    //     self.network.bootstrap(nodes).await?;
    //     Ok(())
    // }

    // /// Returns true if the dht was bootstrapped.
    // pub fn is_bootstrapped(&self) -> bool {
    //     self.network.is_bootstrapped()
    // }

    // /// Gets the closest peer to a key. Useful for finding the `Multiaddr` of a
    // /// `PeerId`.
    // pub async fn get_closest_peers<K>(&self, key: K) -> Result<()>
    // where
    //     K: Into<BucketKey<K>> + Into<Vec<u8>> + Clone,
    // {
    //     self.network.get_closest_peers(key).await?;
    //     Ok(())
    // }

    // /// Gets providers of a key from the dht.
    // pub async fn providers(&self, key: Key) -> Result<HashSet<PeerId>> {
    //     self.network.providers(key).await
    // }

    // /// Provides a key in the dht.
    // pub async fn provide(&self, key: Key) -> Result<()> {
    //     self.network.provide(key).await
    // }

    // /// Stops providing a key in the dht.
    // pub fn unprovide(&self, key: &Key) {
    //     self.network.unprovide(key)
    // }

    // /// Gets a record from the dht.
    // pub async fn get_record(&self, key: Key, quorum: Quorum) -> Result<Vec<PeerRecord>> {
    //     self.network.get_record(key, quorum).await
    // }

    // /// Puts a new record in the dht.
    // pub async fn put_record(&self, record: Record, quorum: Quorum) -> Result<()> {
    //     self.network.put_record(record, quorum).await
    // }

    // /// Removes a record from the dht.
    // pub fn remove_record(&self, key: &Key) {
    //     self.network.remove_record(key)
    // }

    // /// Subscribes to a `topic` returning a `Stream` of messages. If all
    // /// `Stream`s for a topic are dropped it unsubscribes from the `topic`.
    // pub fn subscribe(&self, topic: &str) -> Result<impl Stream<Item = GossipEvent>> {
    //     self.network.subscribe(topic)
    // }

    // /// Publishes a new message in a `topic`, sending the message to all
    // /// subscribed peers.
    // pub fn publish(&self, topic: &str, msg: Vec<u8>) -> Result<()> {
    //     self.network.publish(topic, msg)
    // }

    // /// Publishes a new message in a `topic`, sending the message to all
    // /// subscribed connected peers.
    // pub fn broadcast(&self, topic: &str, msg: Vec<u8>) -> Result<()> {
    //     self.network.broadcast(topic, msg)
    // }

    // /// Creates a temporary pin in the block store. A temporary pin is not
    // /// persisted to disk and is released once it is dropped.
    // pub fn create_temp_pin(&self) -> Result<TempPin> {
    //     self.storage.create_temp_pin()
    // }

    // /// Adds a new root to a temporary pin.
    // pub fn temp_pin(&self, tmp: &mut TempPin, cid: &Cid) -> Result<()> {
    //     self.storage.temp_pin(tmp, std::iter::once(*cid))
    // }

    // /// Returns an `Iterator` of `Cid`s stored in the block store.
    // pub fn iter(&self) -> Result<impl Iterator<Item = Cid>> {
    //     self.storage.iter()
    // }

    // /// Checks if the block is in the block store.
    // pub fn contains(&self, cid: &Cid) -> Result<bool> {
    //     self.storage.contains(cid)
    // }

    // /// Returns a block from the block store.
    // pub fn get(&self, cid: &Cid) -> Result<Block<P>> {
    //     if let Some(data) = self.storage.get(cid)? {
    //         let block = Block::new_unchecked(*cid, data);
    //         Ok(block)
    //     } else {
    //         Err(BlockNotFound(*cid).into())
    //     }
    // }

    // /// Either returns a block if it's in the block store or tries to retrieve
    // /// it from a peer.
    // pub async fn fetch(&self, cid: &Cid, providers: Vec<PeerId>) -> Result<Block<P>> {
    //     if let Some(data) = self.storage.get(cid)? {
    //         let block = Block::new_unchecked(*cid, data);
    //         return Ok(block);
    //     }
    //     if !providers.is_empty() {
    //         self.network.get(*cid, providers.into_iter()).await?;
    //         if let Some(data) = self.storage.get(cid)? {
    //             let block = Block::new_unchecked(*cid, data);
    //             return Ok(block);
    //         }
    //         tracing::error!("block evicted too soon. use a temp pin to keep the block around.");
    //     }
    //     Err(BlockNotFound(*cid).into())
    // }

    // /// Inserts a block in to the block store.
    // pub fn insert(&self, block: Block<P>) -> Result<()> {
    //     self.storage.insert(block)?;
    //     Ok(())
    // }

    // /// Manually runs garbage collection to completion. This is mainly useful
    // /// for testing and administrative interfaces. During normal operation,
    // /// the garbage collector automatically runs in the background.
    // pub async fn evict(&self) -> Result<()> {
    //     self.storage.evict().await
    // }

    // pub fn sync(&self, cid: &Cid, providers: Vec<PeerId>) -> SyncQuery<P> {
    //     let missing = self.storage.missing_blocks(cid).ok().unwrap_or_default();
    //     tracing::trace!(cid = %cid, missing = %missing.len(), "sync");
    //     self.network.sync(*cid, providers, missing)
    // }

    // /// Creates, updates or removes an alias with a new root `Cid`.
    // pub fn alias<T: AsRef<[u8]> + Send + Sync>(&self, alias: T, cid: Option<&Cid>) -> Result<()> {
    //     self.storage.alias(alias.as_ref(), cid)
    // }

    // /// List all known aliases.
    // pub fn aliases(&self) -> Result<Vec<(Vec<u8>, Cid)>> {
    //     self.storage.aliases()
    // }

    // /// Returns the root of an alias.
    // pub fn resolve<T: AsRef<[u8]> + Send + Sync>(&self, alias: T) -> Result<Option<Cid>> {
    //     self.storage.resolve(alias.as_ref())
    // }

    // /// Returns a list of aliases preventing a `Cid` from being garbage
    // /// collected.
    // pub fn reverse_alias(&self, cid: &Cid) -> Result<Option<HashSet<Vec<u8>>>> {
    //     self.storage.reverse_alias(cid)
    // }

    // /// Flushes the block store. After `flush` completes successfully it is
    // /// guaranteed that all writes have been persisted to disk.
    // pub async fn flush(&self) -> Result<()> {
    //     self.storage.flush().await
    // }

    // /// Perform a set of storage operations in a batch
    // ///
    // /// The batching concerns only the CacheTracker, it implies no atomicity
    // /// guarantees!
    // pub fn batch_ops<R>(&self, f: impl FnOnce(&mut Batch<'_, P>) -> Result<R>) -> Result<R> {
    //     self.storage.rw("batch_ops", f)
    // }

    // /// Registers prometheus metrics in a registry.
    // pub fn register_metrics(&self, registry: &Registry) -> Result<()> {
    //     self.storage.register_metrics(registry)?;
    //     self.network.register_metrics(registry)?;
    //     Ok(())
    // }

    // /// Subscribes to the swarm event stream.
    // pub fn swarm_events(&self) -> SwarmEvents {
    //     self.network.swarm_events()
    // }
}

// #[async_trait]
// impl<P: StoreParams> Store for Skfs<P>
// where
//     Ipld: References<P::Codecs>,
// {
//     type Params = P;
//     type TempPin = Arc<Mutex<TempPin>>;

//     fn create_temp_pin(&self) -> Result<Self::TempPin> {
//         Ok(Arc::new(Mutex::new(Skfs::create_temp_pin(self)?)))
//     }

//     fn temp_pin(&self, tmp: &Self::TempPin, cid: &Cid) -> Result<()> {
//         Skfs::temp_pin(self, &mut *tmp.lock(), cid)
//     }

//     fn contains(&self, cid: &Cid) -> Result<bool> {
//         Skfs::contains(self, cid)
//     }

//     fn get(&self, cid: &Cid) -> Result<Block<P>> {
//         Skfs::get(self, cid)
//     }

//     fn insert(&self, block: &Block<P>) -> Result<()> {
//         let _ = Skfs::insert(self, block.clone())?;
//         Ok(())
//     }

//     fn alias<T: AsRef<[u8]> + Send + Sync>(&self, alias: T, cid: Option<&Cid>) -> Result<()> {
//         Skfs::alias(self, alias, cid)
//     }

//     fn resolve<T: AsRef<[u8]> + Send + Sync>(&self, alias: T) -> Result<Option<Cid>> {
//         Skfs::resolve(self, alias)
//     }

//     fn reverse_alias(&self, cid: &Cid) -> Result<Option<Vec<Vec<u8>>>> {
//         Skfs::reverse_alias(self, cid).map(|x| x.map(|x| x.into_iter().collect()))
//     }

//     async fn flush(&self) -> Result<()> {
//         Skfs::flush(self).await
//     }

//     async fn fetch(&self, cid: &Cid) -> Result<Block<Self::Params>> {
//         Skfs::fetch(self, cid, self.peers()).await
//     }

//     async fn sync(&self, cid: &Cid) -> Result<()> {
//         Skfs::sync(self, cid, self.peers()).await
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{join, stream::StreamExt};
    use libipld::{
        alias, cbor::DagCborCodec, ipld, multihash::Code, raw::RawCodec, store::DefaultParams,
    };
    use std::time::Duration;
    use tempdir::TempDir;

    // async fn create_store(enable_mdns: bool) -> Result<(Skfs<DefaultParams>, TempDir)> {
    //     let tmp = TempDir::new("skfs")?;
    //     let sweep_interval = Duration::from_millis(10000);
    //     let path = Path::new("").join("../../blocks");
    //     let storage = StorageConfig::new(Some(path), None, 10, sweep_interval);

    //     let mut network = NetworkConfig::new(Keypair::generate());
    //     if !enable_mdns {
    //         network.mdns = None;
    //     }

    //     let skfs = Skfs::new(Config { storage, network }).await?;
    //     // skfs.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())?
    //     //     .next()
    //     //     .await
    //     //     .unwrap();
    //     Ok((skfs, tmp))
    // }

    // fn create_block(bytes: &[u8]) -> Result<Block<DefaultParams>> {
    //     Block::encode(RawCodec, Code::Blake3_256, bytes)
    // }
}
