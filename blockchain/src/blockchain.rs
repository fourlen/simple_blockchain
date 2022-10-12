use serde::{Serialize, Deserialize};
use serde_json;
use sha2::{Digest, Sha256};
use chrono;
use crate::transactions::{Transaction};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub timestamp: i64,
    pub nonce: u64,
    pub previous_block_hash: String,
    pub hash: String,
    pub miner: String,
    pub transactions: Vec<Transaction>
    // pub data: String
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
    pub mempool: Vec<Transaction>
}


pub fn calculate_hash(data: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("0x{:x}", hasher.finalize())
}

pub fn calculate_block_hash(id: u64, timestamp: i64, nonce: u64, previous_block_hash: &String, miner: &String, transactions: &Vec<Transaction>) -> String {
    calculate_hash(serde_json::json!({
        "id": id,
        "timestamp": timestamp,
        "nonce": nonce,
        "previous_block_hash": previous_block_hash,
        "miner": miner,
        "transactions": transactions
    }).to_string())
}

impl Block {
    pub fn mine(id: u64, previous_block_hash: String, miner: String, mut transactions: Vec<Transaction>) -> Self {
        println!("Mining block...");
        let mut nonce = 0u64;
        transactions.push(Transaction { from: "BAZA".to_string(), to: miner.clone(), value: 1000 });

        loop {

            let timestamp = chrono::offset::Utc::now().timestamp();

            let hash = calculate_block_hash(
                id,
                timestamp,
                nonce,
                &previous_block_hash,
                &miner,
                &transactions
            );
            print!("\r{}", hash);
            if hash.starts_with("0x000") {
                println!();
                return Self {
                    id,
                    timestamp,
                    nonce, 
                    previous_block_hash,
                    hash,
                    miner, 
                    transactions
                };
            }
            nonce += 1;
        }
    }
}


impl BlockChain {
    pub fn new() -> Self {
        let mut blockchain = Self { blocks: vec![], mempool: Vec::new() };

        let previous_block_hash = String::from("0x0000000000000000000000000000000000000000000000000000000000000000");
        let miner = String::from("BAZA");
        let transactions = Vec::new();
        // let data = String::from("GENESIS");

        let hash = calculate_block_hash(
            0,
            0,
            0,
            &previous_block_hash,
            &miner,
            &transactions
        );

        let genesis_block = Block {
            id: 0,
            timestamp: 0, //изначально было chrono::offset::Utc::now().timestamp(), почему это не подходит?
            nonce: 0,
            previous_block_hash: String::from("0x0000000000000000000000000000000000000000000000000000000000000000"),
            hash: hash,
            miner: miner, 
            transactions: transactions
        };

        blockchain.blocks.push(genesis_block);
        blockchain
    }


    pub fn is_block_valid(&self, new_block: &Block, latest_block: &Block) -> bool {
       if new_block.id != latest_block.id + 1 {
            println!("Block #{} has wrong block id", new_block.id);
            if new_block.id == 1 && latest_block.id == 0 {
                return true;
            }
            return false;
        }
        else if new_block.previous_block_hash != latest_block.hash {
            println!("Block #{} has wrong prev block hash", new_block.id);
            return false;
        }
        else if !new_block.hash.starts_with("0x000") {
            println!("Block #{} has wrong block difficulty", new_block.id);
            return false;
        }
        else if new_block.hash != calculate_block_hash(new_block.id, 
                                new_block.timestamp, 
                                new_block.nonce, 
                                &new_block.previous_block_hash, 
                                &new_block.miner, 
                                &new_block.transactions)
        {
            println!("Block #{} has wrong hash", new_block.id);
            return false;
        }
        for transaction in &new_block.transactions {
            if !self.is_transaction_valid(&transaction) {
                return false;
            }
        }
        true
    }

    pub fn is_chain_valid(&self) -> bool {
        for i in 1..self.blocks.len() {
            if !self.is_block_valid(self.blocks.get(i).expect("Can't get block"),
                             self.blocks.get(i - 1).expect("Can't get block"))
            {
                return false;
            }
        }
        true
    }

    pub fn add_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("no blocks yet");
        if self.is_block_valid(&block, latest_block) {
            println!("Block #{} added to the blockchain", {block.id});
            self.blocks.push(block);
            self.mempool = Vec::new();
        }
    }

    pub fn choose_chain(&mut self, remote_chain: BlockChain) {
        let is_local_chain_valid = self.is_chain_valid();
        let is_remote_chain_valid = remote_chain.is_chain_valid();

        if is_local_chain_valid && is_remote_chain_valid {
            if remote_chain.blocks.len() > self.blocks.len() {
                self.blocks = remote_chain.blocks;
                println!("Remote chain better");
            }
        }
        else if !is_local_chain_valid && is_remote_chain_valid {
            self.blocks = remote_chain.blocks;
            println!("Remote chain better");
        }
        println!("Our chain better");
    }

    pub fn get_balance(&self, address: &String) -> u64 {
        let mut balance = 0u64;
        for block in &self.blocks {
            for transaction in &block.transactions {
                if &transaction.to == address {
                    balance += transaction.value;
                }
                else if &transaction.from == address {
                    balance -= transaction.value;
                }
            }
        }
        balance
    }

    pub fn is_transaction_valid(&self, transaction: &Transaction) -> bool {
        if &transaction.from == "BAZA" && &transaction.value == &1000u64 ||
        self.get_balance(&transaction.from) >= transaction.value {
            return true;
        }
        false
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {
        if self.is_transaction_valid(&transaction) {
            self.mempool.push(transaction);
        }
        else {
            println!("Transaction invalid");
        }
    }
}