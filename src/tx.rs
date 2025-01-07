use bitcoincash_addr::Address;
use log::debug;

use crate::wallet::hash_pub_key;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TXInput {
    // the id of the transaction that contains the output 
    // being spent
    pub txid : String,
    //the index of the output in the previous transaction
    pub vout : i32,
    // signature
    //pub script_sig : String
    pub signature : Vec<u8>,
    pub pub_key : Vec<u8>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TXOutput {
    // amount of funds this output represents
    pub value : i32,
    // an address that specifies who will recieve this money
    pub pub_key_hash : Vec<u8>
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TXOutputs {
    pub outputs : Vec<TXOutput>
}


impl TXInput {
    // CanUlockOutputWith checks whether the address initiated the transaction
    pub fn can_unlock_output_with(&self,unlocking_data : &[u8]) -> bool {
        let mut pubkeyhash = self.pub_key.clone();
        hash_pub_key(& mut pubkeyhash);
        pubkeyhash == unlocking_data
    }

}

impl TXOutput {
    // CanBeUnlockedWith checks if the output can be unlocked with the provided data
    pub fn can_be_unlock_with(&self,unlocking_data : &[u8]) -> bool {
        self.pub_key_hash == unlocking_data
    }

    fn lock(&mut self, address : &str) -> Result<(),Box<dyn std::error::Error>>{
        let pub_key_hash = Address::decode(address).unwrap().body;
        debug!("lock: {}",address);
        self.pub_key_hash = pub_key_hash ;
        Ok(())
    }

    pub fn new(value : i32, address : String) -> Result<Self,Box<dyn std::error::Error>>{
        let mut txo = TXOutput{
            value,
            pub_key_hash : Vec::new(),
        };
        txo.lock(&address)?;
        Ok(txo)
    }
}