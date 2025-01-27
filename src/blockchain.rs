use std::{collections::HashMap, hash::Hash};
use bincode::{deserialize, serialize};
use failure::format_err;
use log::{info,debug};
use crate::{block::{self, *}, transaction::Transaction, tx::{TXOutput, TXOutputs}};
const TARGET_HEXT: usize = 4; 
const GENESIS_COINBASE_DATA: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";

#[derive(Debug,Clone)]
pub struct Blockchain {
    pub current_hash : String,
    pub db : sled::Db 
}
pub struct BlockchainIter<'a>{
    current_hash : String,
    bc : &'a Blockchain
}

impl Blockchain {
    
    /// NewBlockchain creates a new Blockchain db
    pub fn new() -> Result<Blockchain,Box<dyn std::error::Error>> {
        info!("open blockchain !!");
        let db = sled::open("data/blocks")?;
        let hash = db
            .get("LAST")?
            .expect("Must create a new block database fisrt");
        info!("Found block database");
        let lasthash = String::from_utf8(hash.to_vec())?;
        Ok(Blockchain{
            current_hash : lasthash.clone(),
            db
        })
    }
    
    /// CreateBlockchain creates a new blockchain DB
    pub fn create_blockchain(address : String) -> Result<Blockchain,Box<dyn std::error::Error>>{
        info!("Creating new blockchain");
        if let Err(e) = std::fs::remove_dir_all("data/blocks"){
            info!("no blockchain db exist to be deleted")
        };
        let db = sled::open("data/blocks")?;
        let cbtx = Transaction::new_coinbase(address, String::from(GENESIS_COINBASE_DATA))?;
        let genesis : Block = Block::new_genesis_block(cbtx);
        db.insert(genesis.get_hash(), bincode::serialize(&genesis)?)?;
        db.insert("LAST",genesis.get_hash().as_bytes())?;
        let bc  = Blockchain {
            current_hash : genesis.get_hash(),
            db,
        };
        //Synchronously flushes all dirty IO buffers and calls fsync.
        bc.db.flush()?;
        Ok(bc)
    }

    /// MineBlock mines a new block with the provided transactions
    pub fn mine_block(&mut self, transactions : Vec<Transaction>) -> Result<Block,Box<dyn std::error::Error>>{
        info!("mine a new block");
        for tx in &transactions{
            if !self.verify_transaction(tx)?{
                return Err(format_err!("Error: Invalid transaction").into());
            }
        }

        let lasthash = self.db.get("LAST")?.unwrap();

        let newblock = Block::new_block(
            transactions, 
            String::from_utf8(lasthash.to_vec())?, 
            self.get_best_height()?+1,
        )?;
        self.db.insert(newblock.get_hash(), serialize(&newblock)?)?;
        self.db.insert("LAST", newblock.get_hash().as_bytes())?;
        self.db.flush()?;
        self.current_hash=newblock.get_hash();
        Ok(newblock)

    }

    /// GetBestHeight returns the height of the latest block
    pub fn get_best_height(&self) -> Result<i32,Box<dyn std::error::Error>> {
        let lasthash = if let Some(h) = self.db.get("LAST")? {
            h
        } else {
            return Ok(-1);
        };
        let last_data = self.db.get(lasthash)?.unwrap();
        let last_block: Block = deserialize(&last_data.to_vec())?;
        Ok(last_block.get_height())
    }

    /*pub fn new() -> Result<Blockchain,Box<dyn std::error::Error>> {
        let db = sled::open("data/blocks")?;
        match db.get("LAST")?{
            Some(hash)=>{
                let lasthash = String::from_utf8(hash.to_vec())?;
                Ok(Blockchain{
                    current_hash : lasthash,
                    db,
                })
            }
            None => {
                let block = Block::new_genesis_block() ;
                db.insert(block.get_hash(), bincode::serialize(&block)?)?;
                db.insert("LAST",block.get_hash().as_bytes())?;
                let bc = Blockchain {
                    current_hash : block.get_hash(),
                    db,
                };
                bc.db.flush()?;
                Ok(bc)
            }
        }
    }*/
    fn find_unspent_transactions(&self, address : &[u8])  -> Vec<Transaction>{
        let mut spent_TXOs : HashMap<String,Vec<i32>> = HashMap::new();
        let mut unspend_TXs : Vec<Transaction> = Vec::new();

        for block  in self.iter(){
            for tx in block.get_transaction(){
                for index in 0..tx.vout.len(){
                    if let Some(ids) = spent_TXOs.get(&tx.id){
                        if ids.contains(&(index as i32)){
                            continue;
                        }
                    }
                    if tx.vout[index].can_be_unlock_with(address){
                        unspend_TXs.push(tx.to_owned())
                    }

                }
                if !tx.is_coinbase(){
                    for i in &tx.vin{
                        if i.can_unlock_output_with(address){
                            match spent_TXOs.get_mut(&i.txid){
                                Some(v) => {
                                    v.push(i.vout);
                                }
                                None => {
                                    spent_TXOs.insert(i.txid.clone(),vec![i.vout]);
                                }
                            }
                        }
                    }
                }
            }
        }
        unspend_TXs
    }

    /*pub fn find_UTXO(&self, address :&[u8]) -> Vec<TXOutput>{
        let mut utxos = Vec::<TXOutput>::new();
        let unspend_TXs = self.find_unspent_transactions(address);
        for tx in unspend_TXs {
            for out in &tx.vout{
                if out.can_be_unlock_with(&address){
                    utxos.push(out.clone());
                }
            }
        }
        utxos
    }*/

    /// FindUTXO finds and returns all unspent transaction outputs
    pub fn find_UTXO(&self) -> HashMap<String,TXOutputs>{
        let mut utxos : HashMap::<String,TXOutputs> = HashMap::new();
        let mut spend_txos : HashMap::<String,Vec<i32>> = HashMap::new(); 
        for block in self.iter(){
            for tx in block.get_transaction(){
                for index in 0..tx.vout.len(){
                    if let Some(ids) = spend_txos.get(&tx.id){
                        if ids.contains(&(index as i32)){
                            continue
                        }
                    }
                    match utxos.get_mut(&tx.id){
                        Some(v)=>{
                            v.outputs.push(tx.vout[index].clone())
                        }
                        None => {
                            utxos.insert(
                                tx.id.clone(),
                                TXOutputs{
                                    outputs : vec![tx.vout[index].clone()]
                                }
                            );
                        }
                    }
                }
                if !tx.is_coinbase(){
                    for i in &tx.vin{
                        match spend_txos.get_mut(&i.txid){
                            Some(v)=>{
                                v.push(i.vout)
                            }
                            None => {
                                spend_txos.insert(i.txid.clone(),vec![i.vout]);
                            }
                        }
                    }
                }
            }
        }
        utxos
    }
    /*     
    pub fn find_spendable_outputs(&self, address:&[u8],amount: i32)->(i32,HashMap<String, Vec<i32>>){
        let mut unspent_outputs : HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated : i32 = 0 ;
        let unspend_TXs = self.find_unspent_transactions(address);
        for tx in unspend_TXs{
            for index in 0..tx.vout.len(){
                if tx.vout[index].can_be_unlock_with(address) && accumulated < amount {
                    match unspent_outputs.get_mut(&tx.id){
                        Some(v) => v.push(index as i32),
                        None => {
                            unspent_outputs.insert(tx.id.clone(),vec![index as i32]);
                        }
                    }
                    accumulated+=tx.vout[index].value;
                    if accumulated >=amount{
                        return (accumulated,unspent_outputs);
                    }
                }
            }
        }
        (accumulated,unspent_outputs)
    }
    */
    pub fn find_transaction(&self, id: &str) -> Result<Transaction,Box<dyn std::error::Error>>{
        for b in self.iter(){
            for tx in b.get_transaction(){
                if tx.id == id {
                    return Ok(tx.clone())
                }
            }
        }
        Err(format_err!("Transaction is not found").into())
    }
    
    /// SignTransaction signs inputs of a Transaction
    pub fn sign_transaction(&self, tx : &mut Transaction, private_key : &[u8]) -> Result<(),Box<dyn std::error::Error>> {
        let prev_TXs = self.get_prev_TXs(tx)?;
        tx.sign(private_key,prev_TXs)?;
        Ok(())
    }

    /// VerifyTransaction verifies transaction input signatures
    pub fn verify_transaction(&self,tx : &Transaction) -> Result<bool,Box<dyn std::error::Error>>{
        if tx.is_coinbase(){
            return Ok(true);
        }
        let prev_TXs = self.get_prev_TXs(tx)?;
        tx.verify(prev_TXs)
    }
  
    /// GetBlockHashes returns a list of hashes of all the blocks in the chain
    pub fn get_block_hashs(&self) -> Vec<String> {
        let mut list = Vec::new();
        for b in self.iter() {
            list.push(b.get_hash());
        }
        list
    }
    /*pub fn add_block(&mut self, transactions : Vec<Transaction>) -> Result<Block,Box<dyn std::error::Error>> {
        /*
            The .into() method converts the string into 
            a boxed dynamic error (Box<dyn std::error::Error>).
        */
        let lasthash = self.db.get("LAST")?.unwrap();
        let new_block = Block::new_block(transactions, String::from_utf8(lasthash.to_vec())?, 10)?;
        self.db.insert(new_block.get_hash(), bincode::serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.current_hash = new_block.get_hash();
        Ok(new_block)
    }*/

    /// AddBlock saves the block into the blockchain
    pub fn add_block(&mut self, block: Block) -> Result<(),Box<dyn std::error::Error>> {
        let data = bincode::serialize(&block)?;
        if let Some(_) = self.db.get(block.get_hash())? {
            return Ok(());
        }
        self.db.insert(block.get_hash(), data)?;

        let lastheight = self.get_best_height()?;
        if block.get_height() > lastheight {
            self.db.insert("LAST", block.get_hash().as_bytes())?;
            self.current_hash = block.get_hash();
            self.db.flush()?;
        }
        Ok(())
    }
     
    fn get_prev_TXs(&self,tx : &Transaction) -> Result<HashMap<String,Transaction>,Box<dyn std::error::Error>>{
        let mut prev_TXs = HashMap::new();
        for vin in &tx.vin{
            let prev_tx = self.find_transaction(&vin.txid)?;
            prev_TXs.insert(prev_tx.id.clone(),prev_tx);
        }
        Ok(prev_TXs)
    }
    
    pub fn get_block(&self, block_hash :&str)-> Result<Block,Box<dyn std::error::Error>>{
        let data = self.db.get(block_hash)?.unwrap();
        let block = deserialize(&data.to_vec())?;
        Ok(block)
    }

    pub fn iter(&self) -> BlockchainIter {
        BlockchainIter {
            current_hash : self.current_hash.clone(),
            bc : &self,
        }
    }

}

impl <'a> Iterator for BlockchainIter<'a> {
    type Item = Block ;
    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(encode_block) = self.bc.db.get(&self.current_hash){
            return match encode_block{
                Some(b) =>{
                    if let Ok(block) = bincode::deserialize::<Block>(&b){
                        self.current_hash = block.get_prev_hash();
                        Some(block)
                        
                    }else{
                        None
                    }
                }
                None => None
            };
        }
        None   
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blockchain(){
        let mut b = Blockchain::new().unwrap();
        //b.add_block("data".to_string()).unwrap();
        //b.add_block("data1".to_string()).unwrap();
        //b.add_block("data2".to_string()).unwrap();
        dbg!(b);
    }
}
