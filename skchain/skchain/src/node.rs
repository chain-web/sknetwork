use sk_core::{genesis::GenesisConfigBuilder, sk_chain::{SkChainBuilder, SkChain}};

pub fn start_node() -> SkChain {
    let node = SkChainBuilder {
        genesis: GenesisConfigBuilder::build_local(),
    }
    .build();
    node
}
