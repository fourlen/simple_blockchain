mod p2p;
mod blockchain;
mod transactions;

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
use crate::transactions::Transaction;

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
                    "/blocks" => { println!("Blocks: {}", serde_json::to_string_pretty(&swarm.behaviour().blockchain.blocks).unwrap()) },
                    "/mempool" => { println!("Mempool: {}", serde_json::to_string_pretty(&swarm.behaviour().blockchain.mempool).unwrap()) },
                    "/balance" => { println!("Balance: {}", &swarm.behaviour().blockchain.get_balance(&p2p::PEER_ID.clone().to_string())); }
                    "/mine" => { 
                        let last_block = &swarm.behaviour().blockchain.blocks.last().unwrap();
                        let block = blockchain::Block::mine(
                            last_block.id + 1,
                            last_block.hash.clone(),
                            p2p::PEER_ID.clone().to_string(),
                            swarm.behaviour().blockchain.mempool.clone()
                            // cmd.strip_prefix("/mine ").unwrap().to_string()
                        );

                        println!("Block #{} mined!", block.id);
                        println!("{}", serde_json::to_string_pretty(&block).unwrap());
                        
                        swarm.behaviour_mut().floodsub.publish(p2p::TOPIC.clone(), serde_json::to_string(&block).unwrap());
                        swarm.behaviour_mut().blockchain.add_block(block);
                    },
                    "/chain_request" => {
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::TOPIC.clone(), serde_json::to_string(&ChainRequest { from: p2p::PEER_ID.to_string() }).unwrap());
                    },
                    cmd if cmd.starts_with("/tx") => {
                        let args: Vec<&str> = cmd.split_ascii_whitespace().collect();
                        let tx = Transaction {
                            from: p2p::PEER_ID.clone().to_string(),
                            to: args[1].to_string(),
                            value: args[2].parse().unwrap()
                        };
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::TOPIC.clone(), serde_json::to_string(&tx).unwrap());
                        swarm
                            .behaviour_mut()
                            .blockchain
                            .add_transaction(tx); //add transaction to local mempool
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