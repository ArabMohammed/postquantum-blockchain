use std::{collections::HashMap};

use crypto::{digest::Digest, ed25519, sha2::Sha256};
use failure::format_err;
use sled::transaction;
use crate::{blockchain::Blockchain, tx::{self, TXInput, TXOutput}, wallet::{self, hash_pub_key, Wallets}};
use log::{error, info};

#[derive(serde::Serialize, serde::Deserialize,Debug, Clone)]
pub struct Transaction {
    // transactio identifier
    pub id : String,
    // A vector of TXInput, representing inputs 
    // to the transaction (sources of funds).
    pub vin : Vec<TXInput>,
    // A vector of TXOutput, representing outputs of 
    // the transaction (destinations for funds)
    pub vout : Vec<TXOutput>
}
impl Transaction {

    // NEWTXOTransaction creates a new transaction
    pub fn new_UTXO(from : &str, to : &str, amount : i32 , bc: &Blockchain) -> Result<Transaction,Box<dyn std::error::Error>>{
        let mut vin = Vec::new();
        
        let wallets = Wallets::new()?;
        let wallet = match wallets.get_wallet(from){
            Some(w) => w,
            None => return Err(format_err!("").into())
        };
        if let None = wallets.get_wallet(&to){
            return Err(format_err!("").into());
        };
        
        let mut pub_key_hash = wallet.public_key.clone();
        hash_pub_key(&mut pub_key_hash);

        let acc_v = bc.find_spendable_outputs(&pub_key_hash, amount);
        if acc_v.0 <amount{
            error!("Not enough balance");
            return Err(format_err!("Not enough balance: current balance {}",acc_v.0).into());
        }
        for tx in acc_v.1 {
            for out in tx.1{
                let input = TXInput{
                    txid : tx.0.clone(),
                    vout:out,
                    signature : Vec::new(),
                    pub_key : wallet.public_key.clone(),
                };
                vin.push(input);
            }
        }
        
        let mut vout = vec![TXOutput::new(amount, to.to_string())?];
        
        if acc_v.0 > amount{
            vout.push(TXOutput{
                value : acc_v.0 - amount,
                pub_key_hash : from.into(),
            });
        }
        
        let mut tx = Transaction {
            id : String::new(),
            vin ,
            vout
        };
        tx.id=tx.hash()?;
        bc.sign_transaction(&mut tx,&wallet.secret_key)?;
        Ok(tx)
    }

    pub fn new_coinbase(to : String, mut data : String) -> Result<Transaction ,Box<dyn std::error::Error>> {
        info!("new coinbase Transaction to : {}",to);
        data+= &format!("Reward to : {}",to);
        let mut tx = Transaction{
            id : String::new(),
            vin : vec![
                TXInput{
                    txid: String::new(),
                    vout : -1,
                    signature : Vec::new(),
                    pub_key : Vec::from(data.as_bytes())
                }
            ],
            vout : vec![
                TXOutput::new(100,to)?
                ]
        };
        tx.id = tx.hash()?;
        //tx.set_id()?;
        Ok(tx)
    }
    
    pub fn hash(& self)-> Result<String,Box<dyn std::error::Error>>{
        let mut copy = self.clone();
        copy.id = String::new();
        let data = bincode::serialize(&copy)?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        Ok(hasher.result_str())
    }

    fn set_id(&mut self)-> Result<(),Box<dyn std::error::Error>>{
        let mut hasher = Sha256::new();
        let data = bincode::serialize(self)?;
        hasher.input(&data);
        self.id = hasher.result_str();
        Ok(())
    }
    // IsCoiBase check whether the transaction is coinbase
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }

    pub fn sign(&mut self, private_key : &[u8], prev_TXs : HashMap<String,Transaction>,
        )-> Result<(),Box<dyn std::error::Error>>{
            if self.is_coinbase(){
                return Ok(());
            }
            for vin in &self.vin{
                if prev_TXs.get(&vin.txid).unwrap().id.is_empty(){
                    return Err(format_err!("Error: Previous transaction is not correct").into());
                }
            }
            let mut tx_copy = self.trim_copy();

            for in_id in 0..tx_copy.vin.len(){
                let prev_tx = prev_TXs.get(&tx_copy.vin[in_id].txid).unwrap();
                tx_copy.vin[in_id].signature.clear();
                tx_copy.vin[in_id].pub_key = prev_tx.vout[tx_copy.vin[in_id].vout as usize].pub_key_hash.clone();
                tx_copy.id=tx_copy.hash()?;
                tx_copy.vin[in_id].pub_key=Vec::new();
                let signature = ed25519::signature(tx_copy.id.as_bytes(), private_key);
                self.vin[in_id].signature=signature.to_vec();
            }   
            Ok(())
        }
    
    pub fn verify(&mut self, prev_TXs : HashMap<String,Transaction>)-> Result<bool,Box<dyn std::error::Error>>{
        if self.is_coinbase(){
            return Ok(true);
        }
        for vin in &self.vin{
            if prev_TXs.get(&vin.txid).unwrap().id.is_empty(){
                return Err(format_err!("").into());
            }
        }
        let mut tx_copy = self.trim_copy();

        for in_id in 0..self.vin.len() {
            let prev_Tx = prev_TXs.get(&self.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_Tx.vout[self.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();

            if !ed25519::verify(
                &tx_copy.id.as_bytes(),
                &self.vin[in_id].pub_key,
                &self.vin[in_id].signature,
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }
    
    fn trim_copy(&self) -> Transaction {
        let mut vin: Vec<_> = Vec::new();
        let mut vout: Vec<_> = Vec::new();
        for v in &self.vin{
            vin.push(
                TXInput{
                    txid : v.txid.clone(),
                    vout : v.vout.clone(),
                    signature : Vec::new(),
                    pub_key : Vec::new()
                }
            )
        }
        
        for v in &self.vout {
            vout.push(TXOutput{
                value: v.value,
                pub_key_hash : v.pub_key_hash.clone()
            })
        }
        Transaction{
            id:self.id.clone(),
            vin,
            vout,
        }
    }
        
}

// represent transaction input
