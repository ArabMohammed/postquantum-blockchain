
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use std::time::SystemTime;
use log::info;

use crate::transaction::Transaction;

const TARGET_HEXT: usize = 4; 
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
pub struct Block {
    timestamp : u128 ,// time when the block was created 
    //transactions : String ,
    transactions : Vec<Transaction>,
    prev_block_hash : String ,
    hash : String,
    height : usize , 
    nonce : i32 ,
}


impl Block {
    pub fn get_transaction(&self) -> &Vec<Transaction>{
        &self.transactions
    }
    pub fn new_block(transactions: Vec<Transaction>, prev_block_hash : String, height : usize) -> Result<Block,Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
        let mut block = Block {
            timestamp ,
            transactions,
            prev_block_hash,
            hash : String::new(),
            height,
            nonce : 0,
        };
        block.run_proof_of_work().unwrap();
        Ok(block)
    }
    
    pub fn new_genesis_block(coinbase : Transaction) -> Block {
        Block::new_block(vec![coinbase], String::new(),0).unwrap()
    }
    
    fn run_proof_of_work(&mut self) -> Result<(),Box<dyn std::error::Error>>{
        info!("Mining the block");
        while !self.validate()?{
            self.nonce += 1 ;
        }
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        // The input method of the Sha256 hasher (and similar functions) 
        // expects a slice (&[u8]) rather than a vector (Vec<u8>
        hasher.input(&data[..]);
        self.hash = hasher.result_str();
        Ok(())
    }
    
    fn prepare_hash_data(&self) -> Result<Vec<u8>,Box<dyn std::error::Error>> {
        let content  = (
            self.prev_block_hash.clone(),
            self.transactions.clone(),
            self.timestamp,
            TARGET_HEXT,
            self.nonce
        );
        let bytes  = bincode::serialize(&content)?;
        Ok(bytes)
    }

    fn validate(&self) -> Result<bool,Box<dyn std::error::Error>> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        let mut vec1 : Vec<u8> = vec![];
        vec1.resize(TARGET_HEXT, '0' as u8);
        println!("{:?}",vec1);
        Ok(&hasher.result_str()[0..TARGET_HEXT] == String::from_utf8(vec1)?)
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }
    
    pub fn get_prev_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

}



