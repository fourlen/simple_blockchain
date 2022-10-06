use futures::{
    prelude::*,
    select
};
use libp2p::NetworkBehaviour;
use libp2p::mdns::{MdnsEvent, MdnsConfig};
use libp2p::swarm::{Swarm, SwarmEvent, NetworkBehaviourEventProcess};
use libp2p::{identity, PeerId, mdns::Mdns, floodsub::{self, Floodsub, FloodsubEvent}};
use std::error::Error;
use async_std::{io, task};


#[derive(NetworkBehaviour)]
struct TestBehavior {
    mdns: Mdns,
    floodsub: Floodsub
}


impl NetworkBehaviourEventProcess<FloodsubEvent> for TestBehavior {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                println!(
                    "Received: {:?} from {:?}",
                    String::from_utf8_lossy(&message.data),
                    message.source
                );
            },
            _ => {}
        }
    }
}


impl NetworkBehaviourEventProcess<MdnsEvent> for TestBehavior {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self
                        .floodsub
                        .add_node_to_partial_view(peer);
                    println!("Discovered new peer!: {}", peer);
                }
            },
            MdnsEvent::Expired(list) => {

                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                    println!("Peer disconnected: {}", peer);
                }
            }
        }
    }
}


#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();


    let keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keys.public());

    println!("Peer id: {}", peer_id);

    let transport = libp2p::development_transport(keys).await?;

    let floodsub_topic = floodsub::Topic::new("test");

    let mut swarm = {
        let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
        let mut behavior = TestBehavior {
            floodsub: Floodsub::new(peer_id),
            mdns: mdns
        };

        behavior.floodsub.subscribe(floodsub_topic.clone());
        Swarm::new(transport, behavior, peer_id)
    };

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        select! {
            line = stdin.select_next_some() => {
                match line?.as_str() {
                    cmd if cmd.starts_with("send") => swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), cmd.strip_prefix("send ").expect("can't parse message")),
                    _ => {}
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?}", address);
                },
                _ => {}
            }
        }   
    }
}