use super::genesis::GenesisConfig;
use sk_common::events::SkEvents;

pub struct SkChain {
    // pub(crate) realm: Realm,
    version: String,
    pub lifecycle_events: SkEvents,
    pub(crate) genesis: GenesisConfig,
}

impl SkChain {
    #[inline]
    pub fn subscribe_event(&mut self) {
        self.lifecycle_events.registerAll(|key, msg| {
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
}

impl SkChainBuilder {
    pub fn build(self) -> SkChain {
        let sk = SkChain {
            version: "0.0.1".to_string(),
            lifecycle_events: SkEvents::new(),
            genesis: self.genesis,
        };
        sk
    }
}
