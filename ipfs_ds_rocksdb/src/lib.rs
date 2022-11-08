//! # IPFS rocksdb block store
//!
//! A block store for a rust implementation of [ipfs](https://ipfs.io/).

// use error::Context;
// pub use error::{BlockStoreError, Result};
// use fnv::FnvHashSet;
use libipld::{prelude::References, store::StoreParams, Block, Cid, Ipld};
use parking_lot::Mutex;
use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, Options, SingleThreaded, DB};
use std::{
    // borrow::Cow,
    // collections::HashSet,
    fmt,
    // iter::FromIterator,
    // mem,
    // ops::DerefMut,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};
// use tracing::*;
const MAX_SIZE: usize = 39;

pub type SingleThreadDb = DBWithThreadMode<SingleThreaded>;

/// Size targets for a store. Gc of non-pinned blocks will start once one of the size targets is exceeded.
///
/// There are targets for both block count and block size. The reason for this is that a store that has
/// a very large number of tiny blocks will become sluggish despite not having a large total size.
///
/// Size targets only apply to non-pinned blocks. Pinned blocks will never be gced even if exceeding one of the
/// size targets.
#[derive(Debug, Clone, Copy, Default)]
pub struct SizeTargets {
    /// target number of blocks.
    ///
    /// Up to this number, the store will retain everything even if not pinned.
    /// Once this number is exceeded, the store will run garbage collection of all
    /// unpinned blocks until the block criterion is met again.
    ///
    /// To completely disable storing of non-pinned blocks, set this to 0.
    /// Even then, the store will never delete pinned blocks.
    pub count: u64,

    /// target store size.
    ///
    /// Up to this size, the store will retain everything even if not pinned.
    /// Once this size is exceeded, the store will run garbage collection of all
    /// unpinned blocks until the size criterion is met again.
    ///
    /// The store will never delete pinned blocks.
    pub size: u64,
}

impl SizeTargets {
    pub fn new(count: u64, size: u64) -> Self {
        Self { count, size }
    }

    pub fn exceeded(&self, stats: &StoreStats) -> bool {
        stats.count > self.count || stats.size > self.size
    }

    /// Size targets that can not be reached. This can be used to disable gc.
    pub fn max_value() -> Self {
        Self {
            count: u64::max_value(),
            size: u64::max_value(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Synchronous {
    // this is the most conservative mode. This only works if we have few, large transactions
    Full,
    Normal,
    Off,
}

impl fmt::Display for Synchronous {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Synchronous::Full => "FULL",
            Synchronous::Normal => "NORMAL",
            Synchronous::Off => "OFF",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    size_targets: SizeTargets,
    pragma_synchronous: Synchronous,
    pragma_cache_pages: u64,
    // open in readonly mode
    read_only: bool,
    // create if it does not yet exist
    create: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size_targets: Default::default(),
            pragma_synchronous: Synchronous::Full, // most conservative setting
            pragma_cache_pages: 8192, // 32 megabytes with the default page size of 4096
            read_only: false,
            create: true,
        }
    }
}

impl Config {
    pub fn with_read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    /// Set size targets for the store
    pub fn with_size_targets(mut self, count: u64, size: u64) -> Self {
        self.size_targets = SizeTargets { count, size };
        self
    }
    pub fn with_pragma_synchronous(mut self, value: Synchronous) -> Self {
        self.pragma_synchronous = value;
        self
    }
    pub fn with_pragma_cache_pages(mut self, value: u64) -> Self {
        self.pragma_cache_pages = value;
        self
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
enum CloumnNames {
    Cid,
    Block,
    // Pin,
}

impl CloumnNames {
    fn as_str(&self) -> &'static str {
        match self {
            CloumnNames::Cid => "Cid",
            CloumnNames::Block => "Block",
            // CloumnNames::Pin => "Pin",
        }
    }
}

struct CidBuf {
    data: [u8; MAX_SIZE],
    size: u8,
}

impl CidBuf {
    fn len(&self) -> usize {
        self.size as usize
    }
}

impl Default for CidBuf {
    fn default() -> Self {
        Self {
            size: 0,
            data: [0; MAX_SIZE],
        }
    }
}

impl std::io::Write for CidBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.len();
        let cap: usize = MAX_SIZE - len;
        let n = cap.min(buf.len());
        self.data[len..len + n].copy_from_slice(&buf[0..n]);
        self.size += n as u8;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct BlockStore<S> {
    db: DBWithThreadMode<SingleThreaded>,
    expired_temp_pins: Arc<Mutex<Vec<i64>>>,
    // config: Config,
    // db_path: PathBuf,
    // recompute_done: Arc<AtomicBool>,
    _s: std::marker::PhantomData<S>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreStats {
    count: u64,
    size: u64,
    page_size: u64,
    used_pages: u64,
    free_pages: u64,
}

impl StoreStats {
    /// Total number of blocks in the store
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Total size of blocks in the store
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Page size used by the SQLite DB
    pub fn page_size(&self) -> u64 {
        self.page_size
    }

    /// Number of used pages in the SQLite DB
    ///
    /// Multiply this with [`page_size`](#method.page_size) to obtain an upper bound
    /// on how much space is actually used. The value returned by [`size`](#method.size)
    /// will always be smaller than this, since it only counts net block data, without
    /// overhead. A large difference suggests the need for calling `vacuum`.
    pub fn used_pages(&self) -> u64 {
        self.used_pages
    }

    /// Number of unused pages in the SQLite DB
    ///
    /// The DB file can be shrunk by at least this page count by calling `vacuum`, which often is
    /// a long-running procedure.
    pub fn free_pages(&self) -> u64 {
        self.free_pages
    }
}

/// a handle that contains a temporary pin
///
/// Dropping this handle enqueues the pin for dropping before the next gc.
// do not implement Clone for this!
pub struct TempPin {
    id: i64,
    expired_temp_pins: Arc<Mutex<Vec<i64>>>,
}

impl TempPin {
    fn new(expired_temp_pins: Arc<Mutex<Vec<i64>>>) -> Self {
        Self {
            id: 0,
            expired_temp_pins,
        }
    }
}

/// dump the temp alias id so you can find it in the database
impl fmt::Debug for TempPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("TempAlias");
        if self.id > 0 {
            builder.field("id", &self.id);
        } else {
            builder.field("unused", &true);
        }
        builder.finish()
    }
}

impl Drop for TempPin {
    fn drop(&mut self) {
        if self.id > 0 {
            self.expired_temp_pins.lock().push(self.id);
        }
    }
}
impl<S> BlockStore<S>
where
    S: StoreParams,
    Ipld: References<S::Codecs>,
{
    pub fn open_path(db_path: PathBuf, config: Config) -> Result<Self, anyhow::Error> {
        let mut colum_opts = Options::default();
        colum_opts.set_max_write_buffer_number(16);
        let cid = ColumnFamilyDescriptor::new(CloumnNames::Cid.as_str(), colum_opts.clone());
        let block = ColumnFamilyDescriptor::new(CloumnNames::Block.as_str(), colum_opts.clone());

        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(config.create);
        let db = DB::open_cf_descriptors(&db_opts, db_path.clone(), vec![cid, block]).unwrap();

        let this = Self {
            db,
            expired_temp_pins: Arc::new(Mutex::new(Vec::new())),
            // config,
            // db_path,
            // recompute_done: Arc::new(AtomicBool::new(false)),
            _s: std::marker::PhantomData,
        };
        // std::thread::spawn(move || {
        //     if let Err(e) = recompute_store_stats(&mut conn.conn) {
        //         tracing::error!("cannot recompute store stats: {}", e);
        //     }
        //     // This is done to avoid GC doing a wal_checkpoint(RESTART) while the above
        //     // long-running query is ongoing, since that would block all writers during
        //     // that period.
        //     conn.recompute_done.store(true, Ordering::SeqCst);
        // });

        // if this.config.cache_tracker.has_persistent_state() {
        //     let ids = in_txn(
        //         &mut this.conn,
        //         Some(("get IDs", Duration::from_secs(1))),
        //         false,
        //         get_ids,
        //     )?;
        //     this.config.cache_tracker.retain_ids(&ids);
        // }
        Ok(this)
    }

    /// Create a persistent block store with the given config
    pub fn open(path: PathBuf, config: Config) -> Result<Self, anyhow::Error> {
        Self::open_path(path, config)
    }

    /// Open the file at the given path for testing.
    ///
    /// This will create a writeable in-memory database that is initialized with the content
    /// of the file at the given path.
    // pub fn open_test(path: impl AsRef<Path>, config: Config) -> Result<Self, rocksdb::Error> {
    //     // let mut conn = Self::create_connection(DbPath::Memory, &config)?;
    //     // debug!(
    //     //     "Restoring in memory database from {}",
    //     //     path.as_ref().display()
    //     // );
    //     // conn.restore(
    //     //     DatabaseName::Main,
    //     //     path,
    //     //     Some(|p: rusqlite::backup::Progress| {
    //     //         let percent = if p.pagecount == 0 {
    //     //             100
    //     //         } else {
    //     //             (p.pagecount - p.remaining) * 100 / p.pagecount
    //     //         };
    //     //         if percent % 10 == 0 {
    //     //             debug!("Restoring: {} %", percent);
    //     //         }
    //     //     }),
    //     // )
    //     // .ctx("restoring test DB from backup")?;
    //     // let ids = in_txn(
    //     //     &mut conn,
    //     //     Some(("get ids", Duration::from_secs(1))),
    //     //     false,
    //     //     get_ids,
    //     // )?;
    //     // config.cache_tracker.retain_ids(&ids);
    //     // Ok(Self {
    //     //     conn,
    //     //     expired_temp_pins: Arc::new(Mutex::new(Vec::new())),
    //     //     config,
    //     //     db_path: DbPath::Memory,
    //     //     recompute_done: Arc::new(AtomicBool::new(true)),
    //     //     _s: PhantomData,
    //     // })
    // }

    // pub fn backup(&mut self, path: PathBuf) -> Result<(), rocksdb::Error> {
    //     // in_txn(&mut self.conn, None, false, move |txn| {
    //     //     txn.backup(DatabaseName::Main, path.as_path(), None)
    //     //         .ctx("backing up DB")
    //     // })
    //     Ok(())
    // }

    // pub fn flush(&mut self) -> Result<(), rocksdb::Error> {
    //     // in_txn(&mut self.conn, None, false, |txn| {
    //     //     txn.pragma_update(None, "wal_checkpoint", &"TRUNCATE")
    //     //         .ctx("flushing WAL")
    //     // })
    //     Ok(())
    // }

    // pub fn integrity_check(&mut self) -> crate::Result<()> {
    //     let result = integrity_check(&mut self.conn)?;
    //     if result == vec!["ok".to_owned()] {
    //         Ok(())
    //     } else {
    //         let error_text = result.join(";");
    //         Err(crate::error::BlockStoreError::SqliteError(
    //             rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(11), Some(error_text)),
    //             "checking integrity",
    //         ))
    //     }
    //     // FIXME add actual integrity check on the stored blocks
    // }

    // pub fn transaction(&mut self) -> Transaction<'_, S> {
    //     Transaction::new(self)
    // }

    /// Get a temporary alias for safely adding blocks to the store
    pub fn temp_pin(&self) -> TempPin {
        TempPin::new(self.expired_temp_pins.clone())
    }

    pub fn put_block(
        &mut self,
        block: Block<S>,
        pin: Option<&mut TempPin>,
    ) -> Result<(), anyhow::Error> {
        let block_exists = self.has_block(block.cid()).unwrap();
        // println!(
        //     "block:\n data{:?},\n hash {:?}",
        //     block.data(),
        //     block.hash().to_bytes()
        // );
        // println!("block exists: {}", block_exists);
        if !block_exists {
            // try get links
            let mut links = Vec::new();
            block.references(&mut links).unwrap();
            // println!("link size: {}", links.len());

            if links.len() > 0 {
                // save links
                for link in links {
                    self.db.put(link.hash().to_bytes(), link.to_bytes())?;
                    let c = self.db.cf_handle(CloumnNames::Cid.as_str()).unwrap();
                    // let mut buf = CidBuf::default();
                    // link.write_bytes(&mut buf).unwrap();
                    // println!("add link:\n {:?},\n {:?}", buf.data, link.to_bytes());
                    self.db.put_cf(c, link.hash().to_bytes(), link.to_bytes())?;
                }
            }

            // save block
            let c = self.db.cf_handle(CloumnNames::Block.as_str()).unwrap();
            let _res = self
                .db
                .put_cf(c, block.cid().hash().to_bytes(), block.data())?;
            // if res.as_ref().err().is_some() {
            //     return Err(anyhow::Error::msg(res.err().unwrap().to_string()));
            // }
        }

        if pin.is_some() {
            // TODO pin
        }

        Ok(())
    }

    /// Check if we have a cid, if has cid, we may not have the data yet
    pub fn has_cid(&mut self, cid: &Cid) -> anyhow::Result<bool> {
        let c = self.db.cf_handle(CloumnNames::Cid.as_str()).unwrap();
        let cid_res = self.db.get_cf(c, cid.hash().to_bytes());
        // println!("has_cid{:?}", cid_res.clone().unwrap().is_some());
        Ok(cid_res.unwrap().is_some() || self.has_block(cid).unwrap())
    }

    /// Check if we have a block
    pub fn has_block(&mut self, cid: &Cid) -> anyhow::Result<bool> {
        let c = self.db.cf_handle(CloumnNames::Block.as_str()).unwrap();
        let block_res = self.db.get_cf(c, cid.hash().to_bytes());
        // println!("has_block{:?}", block_res.clone().unwrap().is_some());
        Ok(block_res.unwrap().is_some())
    }

    /// Get a cid
    pub fn get_cid(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let c = self.db.cf_handle(CloumnNames::Cid.as_str()).unwrap();
        let cid_res = self.db.get_cf(c, cid.hash().to_bytes()).unwrap();
        // println!("has_cid{:?}", cid_res.clone().unwrap().is_some());
        Ok(cid_res)
    }

    /// Get a block
    pub fn get_block(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let c = self.db.cf_handle(CloumnNames::Block.as_str()).unwrap();
        let block_res = self.db.get_cf(c, cid.hash().to_bytes()).unwrap();
        Ok(block_res)
    }

    /// get the set of descendants of an id for which we do not have the data yet.
    /// The value itself is included.
    /// It is safe to call this method for a cid we don't have yet.
    pub fn get_missing_blocks(&mut self, cid: &Cid) -> anyhow::Result<Vec<Cid>> {
        let res = Vec::new();
        // let mut links = Vec::new();
        // cid.references(&mut links).unwrap();
        Ok(res)
    }

    // pub fn cleanup_temp_pins(&mut self) -> Result<()> {
    //     // atomically grab the expired_temp_pins until now
    //     let expired_temp_pins = mem::take(self.expired_temp_pins.lock().deref_mut());
    //     in_txn(
    //         &mut self.conn,
    //         Some(("dropping expired temp_pins", Duration::from_millis(100))),
    //         true,
    //         move |txn| {
    //             // get rid of dropped temp aliases, this should be fast
    //             for id in expired_temp_pins.iter() {
    //                 delete_temp_pin(txn, *id)?;
    //             }
    //             Ok(())
    //         },
    //     )
    // }

    // pub fn gc(&mut self) -> Result<(), rocksdb::Error> {
    //     self.cleanup_temp_pins()?;
    //     self.flush()?;
    //     incremental_gc(
    //         &mut self.conn,
    //         usize::MAX,
    //         Duration::from_secs(u32::MAX.into()),
    //         self.config.size_targets,
    //     )?;
    //     self.vacuum()?;
    //     Ok(())
    // }

    // fn maybe_checkpoint(&mut self) -> Result<(), rocksdb::Error> {
    //     if self.recompute_done.load(Ordering::SeqCst) {
    //         self.conn
    //             .pragma_update(None, "journal_size_limit", 10_000_000i64)
    //             .ctx("setting journal_size_limit")?;
    //         self.conn
    //             .pragma_update(None, "wal_checkpoint", &"RESTART")
    //             .ctx("running wal_checkpoint(RESTART)")?;
    //     }
    //     Ok(())
    // }

    // pub fn incremental_gc(&mut self, min_blocks: usize, max_duration: Duration) -> Result<bool, rocksdb::Error> {
    //     // let stats = self.get_store_stats()?;
    //     // let _span = tracing::debug_span!("incGC", stats = ?&stats).entered();
    //     // self.cleanup_temp_pins()?;
    //     // self.maybe_checkpoint()?;
    //     // let ret = incremental_gc(
    //     //     &mut self.conn,
    //     //     min_blocks,
    //     //     max_duration,
    //     //     self.config.size_targets,
    //     //     &self.config.cache_tracker,
    //     // )?;
    //     // self.maybe_checkpoint()?;
    //     // in_txn(
    //     //     &mut self.conn,
    //     //     Some(("incremental_vacuum", Duration::from_millis(500))),
    //     //     false,
    //     //     |txn| {
    //     //         txn.execute_batch("PRAGMA incremental_vacuum")
    //     //             .ctx("incremental vacuum")
    //     //     },
    //     // )?;
    // }
}

// macro_rules! delegate {
//     ($($(#[$attr:meta])*$n:ident$(<$v:ident : $vt:path>)?($($arg:ident : $typ:ty),*) -> $ret:ty;)+) => {
//         $(
//             $(#[$attr])*
//             pub fn $n$(<$v: $vt>)?(&mut self, $($arg: $typ),*) -> $ret {
//               self.conn
//                 // let mut txn = self.transaction();
//                 // let ret = txn.$n($($arg),*)?;
//                 // txn.commit()?;
//                 // Ok(ret)
//             }
//         )+
//     };
// }
