use std::collections::HashMap;

use log::info;
use serde_json::error;

use crate::{block::Block, blockchain::Blockchain, tx::TXOutputs};
// it will works on unspent transactions outputs 
// to speed transactions when the blockchain becomes 
// bigger
pub struct UTXOSet {
    pub blockchain : Blockchain
}
impl UTXOSet {
    /*
    pub struct TXOutput {
        pub value : i32,
        pub pub_key_hash : Vec<u8>
    }
    pub struct TXOutputs {
        pub outputs : Vec<TXOutput>
    }
    */
    /// Reindex rebuilds the UTXO set
    pub fn reindex(&self) -> Result<(),Box<dyn std::error::Error>>{
        if let Err(e) = std::fs::remove_dir_all("data/utxos"){
            info!("no utxos index db exist to be deleted !!")
        };
        let db = sled::open("data/utxos")?;
        let utxos = self.blockchain.find_UTXO();
        for (txid,outs) in utxos {
            db.insert(txid.as_bytes(),bincode::serialize(&outs)?)?;
        }
        Ok(())
    }
    
    // Update the UTXO with transactions from the block 
    pub fn update(&self, block : &Block) -> Result<(),Box<dyn std::error::Error>>{
        let db = sled::open("data/utxos")?;
        for tx in block.get_transaction(){
            if !tx.is_coinbase(){
                for vin in &tx.vin{
                    let mut update_outputs = TXOutputs{
                        outputs : Vec::new(),
                    };
                    let outs : TXOutputs = bincode::deserialize(&db.get(&vin.txid)?.unwrap())?;
                    for out_idx in 0..outs.outputs.len(){
                        if out_idx != vin.vout as usize{
                            update_outputs.outputs.push(outs.outputs[out_idx].clone());
                        }
                    }
                    // all outputs of transaction txid have been consumed
                    if update_outputs.outputs.is_empty(){
                        db.remove(&vin.txid)?;
                    }else{
                        db.insert(vin.txid.as_bytes(), bincode::serialize(&update_outputs)?)? ;
                    }
                }
            }
            let mut new_outputs = TXOutputs{
                outputs : Vec::new(),
            };

            for out in &tx.vout{
                new_outputs.outputs.push(out.clone());
            }
            
            db.insert(tx.id.as_bytes(), bincode::serialize(&new_outputs)?)?;
        }
        Ok(())
    }
    
    // return the number of transactions in the UTXO set
    pub fn count_transactions(&self) -> Result<i32,Box<dyn std::error::Error>> {
        let mut counter : i32 = 0 ;
        let db = sled::open("data/utxos")?;
        for kv in db.iter(){
            kv?;
            counter +=1;
        }
        Ok(counter)

    }
    
    /// FindUnspentTransactions returns a list of transactions containing unspent outputs
    pub fn find_spendable_outputs(&self, address:&[u8],amount: i32)->(i32,HashMap<String, Vec<i32>>){
        let mut unspent_outputs : HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated : i32 = 0 ;
        let db = sled::open("data/utxos").unwrap();
        for kv in db.iter(){
            let (k,v) = kv.unwrap();
            let txid = String::from_utf8(k.to_vec()).unwrap();
            let outs : TXOutputs = bincode::deserialize(&v.to_vec()).unwrap();
            
            for out_idx in 0..outs.outputs.len(){
                if outs.outputs[out_idx].can_be_unlock_with(address) && accumulated < amount {
                    accumulated+=outs.outputs[out_idx].value;
                    match unspent_outputs.get_mut(&txid){
                        Some(v) => v.push(out_idx as i32),
                        None=> {
                            unspent_outputs.insert(txid.clone(), vec![out_idx as i32]);
                        }
                    }
                }
            } 
        }
        (accumulated,unspent_outputs)
    }
    /// Update updates the UTXO set with transactions from the Block
    ///
    /// The Block is considered to be the tip of a blockchain
    // it will be used to find the balance of a specific user 
    pub fn find_UTXO(&self,pub_key_hash : &[u8]) -> Result<TXOutputs,Box<dyn std::error::Error>>{
        let mut utxos = TXOutputs{
            outputs : Vec::new()
        };
        let db = sled::open("data/utxos")?;
        for kv in db.iter(){
            let (_,v) = kv?;
            let outs : TXOutputs = bincode::deserialize(&v.to_vec())?;
            
            for out in outs.outputs{
                if out.can_be_unlock_with(pub_key_hash){
                    utxos.outputs.push(out.clone());
                }
            }
        }
        Ok(utxos)
    }

}