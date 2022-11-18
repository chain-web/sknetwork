use super::sk_chain::SkChain;
use num::BigUint;
use num_bigint::ToBigUint;
use num_traits::Zero;
use sk_common::events::LifeCycleEvents;
use sk_common::timer::now;
use sk_common::{Account, Blake2Hasher};
use std::collections::HashMap;
use sk_trie::{TrieDBMutBuilder, MemoryDB, LayoutV0, TrieHash};

#[derive(Debug)]
pub struct Genesis {
    // pub(crate) realm: Realm,
    // version:
    genesis: GenesisConfig, // intrinsics: Intrinsics,
}

#[derive(Debug)]
pub struct GenesisConfig {
    hash: String,
    parent: String,
    logs_bloom: String,
    difficulty: BigUint,
    number: BigUint,
    cu_limit: BigUint,
    timestamp: u128,
    alloc: HashMap<String, AllocConfig>,
}

#[derive(Debug)]
pub struct AllocConfig {
    balance: BigUint,
}

pub struct GenesisConfigBuilder {}

impl GenesisConfigBuilder {
    pub fn build_local() -> GenesisConfig {
        let default_cu_limit: u32 = 10000;
        GenesisConfig {
            hash: "".to_string(),
            parent: "".to_string(),
            logs_bloom: "".to_string(),
            difficulty: BigUint::zero(),
            number: BigUint::zero(),
            cu_limit: default_cu_limit.to_biguint().unwrap(),
            timestamp: now(),
            alloc: HashMap::new(),
        }
    }
}

impl SkChain {
    pub fn check_genesis_block(&mut self) {
        self.lifecycle_events
            .emit(LifeCycleEvents::InitGenesis, "env: local".to_string());
        if self.block_service.is_empty() {
            // cold start
            let stateRoot = self.initAlloc();
        }
        // if (!self.blockService.needGenseis()) {
        //   // 不是完全冷启动
        //   // this.checkGenesis();
        //   lifecycleEvents.emit(LifecycleStap.checkedGenesisBlock);
        // } else {
        //   // 完全冷启动

        //   // 初始化预设账号
        //   const stateRoot = await this.initAlloc(this.genesis.alloc);

        //   // 创建创世区块
        //   const logsBloom = new BloomFilter();
        //   logsBloom.loadData(this.genesis.logsBloom);
        //   const genesisBlockHeader: BlockHeaderData = {
        //     parent: this.genesis.parent,
        //     stateRoot,
        //     transactionsRoot: (
        //       await this.chain.db.dag.put(createEmptyNode('transactions-root'))
        //     ).toString(),
        //     receiptsRoot: (
        //       await this.chain.db.dag.put(createEmptyNode('receipts-root'))
        //     ).toString(),
        //     logsBloom,
        //     difficulty: this.genesis.difficulty,
        //     number: this.genesis.number,
        //     cuLimit: this.genesis.cuLimit,
        //     cuUsed: new BigNumber(0),
        //     ts: this.genesis.timestamp,
        //     slice: [1, 0],
        //     body: (await this.chain.db.dag.put([])).toString(),
        //   };
        //   const genesisBlock = new Block(genesisBlockHeader);
        //   genesisBlock.body = { transactions: [] };
        //   await genesisBlock.genHash(this.chain.db);
        //   const cid = await genesisBlock.commit(this.chain.db);
        //   // 将创世块cid存储到块索引
        //   await this.chain.blockService.addBlockCidByNumber(
        //     cid.toString(),
        //     genesisBlock.header.number,
        //   );

        //   lifecycleEvents.emit(LifecycleStap.checkedGenesisBlock);
        // }
    }

    // set init account to block0
    fn initAlloc(&self) {
        // generate init account
        let mut accounts = Vec::new();
        if !self.genesis.alloc.is_empty() {
            for item in self.genesis.alloc.iter() {
                let mut account = Account::new();
                account.set_account(item.0.to_string());
                // set balance to account
                account.plus_blance(item.1.balance.clone(), 0);
                accounts.push(account);
            }
        }

        // generate init StateRoot
        type LayoutV0Bk2 = LayoutV0<Blake2Hasher>;
		let mut db = MemoryDB::default();
		let mut root = TrieHash::<LayoutV0Bk2>::default();
        let initStateRoot = TrieDBMutBuilder::<LayoutV0Bk2>::new(&mut db, &mut root);
        for account in accounts {
            // 
        }
        //   const initStateRoot = new Mpt(
        //     this.chain.db,
        //     (await this.chain.db.dag.put(createEmptyNode('state-root'))).toString(),
        //   );
        //   await initStateRoot.initRootTree();
        //   for (const account of accounts) {
        //     await initStateRoot.updateKey(
        //       account.account.did,
        //       await account.commit(this.chain.db),
        //     );
        //   }
        //   return (await initStateRoot.save()).toString();
    }

    // 检查链合法性
    // checkGenesis(genesisBlock: Block) {
    //   // 暂时未确定，要搞什么
    // }
}
