mod p2p;
mod blockchain;

use futures::{
    prelude::*,
    select
};
use libp2p::mdns::{ MdnsConfig};
use libp2p::swarm::{Swarm, SwarmEvent};
use libp2p::{mdns::Mdns, floodsub::Floodsub};
use std::error::Error;
use async_std::{io, task};
use p2p::{BlockChainBehavior, ChainRequest};
use serde_json;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Peer id: {}", p2p::PEER_ID.clone());

    let transport = libp2p::development_transport(p2p::KEYPAIR.clone()).await?;

    let mut swarm = {
        let mdns = task::block_on(Mdns::new(MdnsConfig::default()))?;
        let mut behavior = BlockChainBehavior {
            floodsub: Floodsub::new(p2p::PEER_ID.clone()),
            mdns: mdns,
            blockchain: blockchain::BlockChain::new()
        };

        behavior.floodsub.subscribe(p2p::TOPIC.clone());
        Swarm::new(transport, behavior, p2p::PEER_ID.clone())
    };

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    swarm.listen_on("/ip4/192.168.56.1/tcp/0".parse()?)?;

    loop {
        select! {
            line = stdin.select_next_some() => {
                match line?.as_str() {
                    cmd if cmd.starts_with("send") => swarm.behaviour_mut().floodsub.publish(p2p::TOPIC.clone(), cmd.strip_prefix("send ").expect("can't parse message")),
                    "print blocks" => { println!("{}", serde_json::to_string_pretty(&swarm.behaviour().blockchain).unwrap()) },
                    cmd if cmd.starts_with("mine block ") => { 
                        let last_block = &swarm.behaviour().blockchain.blocks.last().unwrap();
                        let block = blockchain::Block::mine(
                            last_block.id + 1,
                            last_block.hash.clone(),
                            p2p::PEER_ID.clone().to_string(),
                            cmd.strip_prefix("mine block ").unwrap().to_string()
                        );
                        println!("Block #{} mined!", block.id);
                        println!("{}", serde_json::to_string_pretty(&block).unwrap());
                        
                        swarm.behaviour_mut().floodsub.publish(p2p::TOPIC.clone(), serde_json::to_string(&block).unwrap());
                        swarm.behaviour_mut().blockchain.add_block(block);
                    },
                    "chain request" => {
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::TOPIC.clone(), serde_json::to_string(&ChainRequest { from: p2p::PEER_ID.to_string() }).unwrap());
                    }
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