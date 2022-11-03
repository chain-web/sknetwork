use libipld::{store::StoreParams, DefaultParams, Result};
use libp2p::{identify, ping, NetworkBehaviour, kad::{store::MemoryStore, Kademlia}, swarm::{behaviour::toggle::Toggle, ConnectionError}, PeerId, core::ConnectedPoint};
use libp2p_bitswap::{Bitswap, BitswapEvent, BitswapStore};
use libp2p::swarm;
use libp2p_broadcast::Broadcast;

use super::NetworkConfig;

pub(crate) type MyHandlerError = <<<NetworkBackendBehaviour<DefaultParams> as swarm::NetworkBehaviour>
    ::ConnectionHandler as swarm::IntoConnectionHandler>::Handler as swarm::ConnectionHandler>::Error;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
pub struct NetworkBackendBehaviour<P: StoreParams> {
    identify: Toggle<identify::Behaviour>,
    ping: Toggle<ping::Behaviour>,
    bitswap: Toggle<Bitswap<P>>,
}

pub enum Event {
    Identify(identify::Event),
    Ping(ping::Event),
    Bitswap(BitswapEvent),
}

impl From<identify::Event> for Event {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<ping::Event> for Event {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<BitswapEvent> for Event {
    fn from(event: BitswapEvent) -> Self {
        Self::Bitswap(event)
    }
}

impl<P: StoreParams> NetworkBackendBehaviour<P> {
    /// Create a Kademlia behaviour with the IPFS bootstrap nodes.
    pub async fn new<S: BitswapStore<Params = P>>(
        config: &mut NetworkConfig,
        store: S,
    ) -> Result<Self> {
        let public = config.node_key.public();
        let node_key = libp2p::identity::Keypair::Ed25519(config.node_key.clone());
        let node_name = config.node_name.clone();
        let peer_id = node_key.public().to_peer_id();
        // let mdns = if let Some(config) = config.mdns.take() {
        //     Some(Mdns::new(config).await?)
        // } else {
        //     None
        // };
        // let kad = if let Some(config) = config.kad.take() {
        //     let kad_store = MemoryStore::with_config(peer_id, config);
        //     Some(Kademlia::new(peer_id, kad_store))
        // } else {
        //     None
        // };
        let ping = config.ping.take().map(ping::Behaviour::new);
        let identify = if let Some(mut config) = config.identify.take() {
            config.local_public_key = node_key.public();
            config.agent_version = node_name.clone();
            Some(identify::Behaviour::new(config))
        } else {
            None
        };
        // let gossipsub = if let Some(config) = config.gossipsub.take() {
        //     let gossipsub = Gossipsub::new(MessageAuthenticity::Signed(node_key), config)
        //         .map_err(|err| anyhow::anyhow!("{}", err))?;
        //     Some(gossipsub)
        // } else {
        //     None
        // };
        let broadcast = config.broadcast.take().map(Broadcast::new);
        let bitswap = config
            .bitswap
            .take()
            .map(|config| Bitswap::new(config, store));
        Ok(Self {
            // bootstrap_complete: false,
            // peers: AddressBook::new(
            //     peer_id,
            //     node_name,
            //     public,
            //     config.port_reuse,
            //     config.enable_loopback,
            // ),
            // mdns: mdns.into(),
            // kad: kad.into(),
            ping: ping.into(),
            identify: identify.into(),
            bitswap: bitswap.into(),
            // gossipsub: gossipsub.into(),
            // broadcast: broadcast.into(),
            // queries: Default::default(),
            // subscriptions: Default::default(),
        })
    }
    fn inject_event(&mut self, _event: void::Void) {}

    pub(crate) fn connection_closed(
        &mut self,
        peer: PeerId,
        cp: ConnectedPoint,
        num_established: u32,
        error: Option<ConnectionError<MyHandlerError>>,
    ) {
        // self.peers
        //     .connection_closed(peer, cp, num_established, error);
    }
}