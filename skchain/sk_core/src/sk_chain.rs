use crate::block_service::BlockService;

use super::genesis::GenesisConfig;
use libipld::DefaultParams;
use sk_common::events::SkEvents;
use sk_fs::Skfs;
use sk_net::NetworkService;

pub struct SkChain {
    // pub(crate) realm: Realm,
    version: String,
    pub lifecycle_events: SkEvents,
    pub genesis: GenesisConfig,
    pub network: NetworkService<DefaultParams>,
    pub fs: Skfs<DefaultParams>,
    pub block_service: BlockService,
}

impl SkChain {
    #[inline]
    pub fn subscribe_event(&mut self) {
        self.lifecycle_events.register_all(|key, msg| {
            let mut str_msg = key.to_string();
            str_msg.push_str(" ");
            str_msg.push_str(&msg);
            println!("{:}", str_msg);
        })
    }

    pub fn init(&mut self) {
        self.subscribe_event();
        self.check_genesis_block();
    }
}

pub struct SkChainBuilder {
    pub genesis: GenesisConfig,
    pub network: NetworkService<DefaultParams>,
    pub fs: Skfs<DefaultParams>,
}

impl SkChainBuilder {
    pub fn build(self) -> SkChain {
        let sk = SkChain {
            version: "0.0.1".to_string(),
            lifecycle_events: SkEvents::new(),
            genesis: self.genesis,
            network: self.network,
            fs: self.fs.clone(),
            block_service: BlockService::new(self.fs.clone()),
        };
        sk
    }
}
