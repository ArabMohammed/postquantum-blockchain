mod block ;
mod blockchain;
mod cli ;
mod transaction;
use crate::cli::Cli;
mod tx ;
mod wallet ;
mod utxoset;
fn main() -> Result<(),Box<dyn std::error::Error>>{
    let mut cli = Cli::new()?;
    cli.run()?;
    // we have a problem in the sending process 
    // if sent amount smaller than the haved one 
    // the sender = 0 
    // the receiver will recieve a correct amount 
    Ok(())
}
