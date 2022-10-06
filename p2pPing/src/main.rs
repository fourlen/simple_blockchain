use futures::prelude::*;
use libp2p::ping::{Ping, PingConfig};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use std::error::Error;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keys.public());

    println!("Peer id: {}", peer_id);

    let transport = libp2p::development_transport(keys).await?;

    let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));

    let mut swarm = Swarm::new(transport, behaviour, peer_id);
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Args: {:#?}", std::env::args());

    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed: {}", addr);
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { listener_id: _, address } => { println!("New listener: {}", address)}
            SwarmEvent::Behaviour(event) => { println!("{:?}", event) }
            _ => {}
        }
    }
}