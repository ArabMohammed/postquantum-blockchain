mod block ;
mod blockchain;
mod cli ;
mod transaction;
mod message ;
use crate::cli::Cli;
mod tx ;
mod wallet ;
mod utxoset;
mod server ;
mod behavior;
mod constants;
use env_logger::{Env, Builder};
/********************
 * wallets owners rely on merkle trees to veirfy transactions 
 * Wallets ofetn operate in two modes :
 * Full nodes : download and validate the entire blockchain . 
 * 
 * Lightweight/SPV wallets : interact with the blockchain 
 * without storing the full data
 * 
 * SPV works by : if a wallet owner wants to veirfy if a specific transaction
 * belongs to a block : 
 * it retriev : - the block header, which includes the merkle root 
 *              - a merkle proof, which is a minimal subset of hashes from the merkle
 *                tree
 */
 /*******************
 * Consensus Mechanism : 
 * the network uses a consensus mechanism to decide which block will be added 
 * to the blockchain 
 * 1) Proof of work : Miners compete to solve a complex cryptographic puzzle
 * the first miner to solve it braodcasts their block and proof to the network. 
 * 
 * 2) Proof of stake (POS):
 * Validators are chosen to propose/validate blocks based on their stake
 * 
 */
#[tokio::main]
 async fn main() -> Result<(),Box<dyn std::error::Error>>{
    Builder::from_env(Env::default().default_filter_or("debug")).init();
    let mut cli = Cli::new()?;
    cli.run().await.unwrap();
    // we have a problem in the sending process 
    // if sent amount smaller than the haved one 
    // the sender = 0 
    // the receiver will recieve a correct amount 
    Ok(())
}
