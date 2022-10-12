
use libp2p::NetworkBehaviour;
use libp2p::mdns::{MdnsEvent};
use libp2p::swarm::{NetworkBehaviourEventProcess};
use libp2p::{identity, PeerId, mdns::Mdns, floodsub::{Floodsub, FloodsubEvent, Topic}};
use serde::{Serialize, Deserialize};
use crate::blockchain::{BlockChain, Block};
use once_cell::sync::Lazy;
use crate::transactions::Transaction;


pub static KEYPAIR: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_secp256k1);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(KEYPAIR.public()));
pub static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("Our blockchain"));

#[derive(NetworkBehaviour)]
pub struct BlockChainBehavior {
    pub mdns: Mdns,
    pub floodsub: Floodsub,
    #[behaviour(ignore)]
    pub blockchain: BlockChain
}

#[derive(Serialize, Deserialize)]
pub struct ChainRequest {
    pub from: String
}


#[derive(Serialize, Deserialize)]
pub struct ChainResponse {
    pub to: String,
    pub blockchain: BlockChain
}


impl NetworkBehaviourEventProcess<FloodsubEvent> for BlockChainBehavior {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(message) => {
                if let Ok(block) = serde_json::from_slice::<Block>(&message.data) {
                    println!("Received block from {}", &message.source);
                    self.blockchain.add_block(block); //почему мы не можем поменять поле miner и разослать всем наш блок, чтобы стать майнером?
                } 
                else if let Ok(transaction) = serde_json::from_slice::<Transaction>(&message.data) {
                    println!("New transaction from {}", &message.source);
                    if transaction.from == message.source.to_string() {
                        self.blockchain.add_transaction(transaction);
                    }
                    else {
                        println!("Wrong signature");
                    }
                }
                else if let Ok(msg) = serde_json::from_slice::<ChainRequest>(&message.data) {
                    println!("Chain request from {}", &msg.from);
                    self
                        .floodsub
                        .publish(Topic::new("Our blockchain"), serde_json::to_string(&ChainResponse {
                            to: msg.from,
                            blockchain: self.blockchain.clone()
                        }).unwrap())
                }
                else if let Ok(msg) = serde_json::from_slice::<ChainResponse>(&message.data) {
                    if msg.to == PEER_ID.to_string() {
                        println!("Chain response from {}", &message.source);
                        self.blockchain.choose_chain(msg.blockchain);
                    }
                }
            },
            _ => { println!("unknown message: {:#?}", event); }
        }
    }
}


impl NetworkBehaviourEventProcess<MdnsEvent> for BlockChainBehavior {
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