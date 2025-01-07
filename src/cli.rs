

use std::process::exit;

use bincode::Error;
use bitcoincash_addr::Address;
use clap::Command;

use crate::{block::Block, blockchain::*, transaction::Transaction, wallet::Wallets};
use clap::arg;
pub struct Cli {

}
impl Cli {
    pub fn new() -> Result<Cli,Box<dyn std::error::Error>> {
        Ok(Cli{

        })
    }
    pub fn run(&mut self) -> Result<(),Box<dyn std::error::Error>> {
        let matches = Command::new("blockchain-rust-postquantum")
            .version("0.1")
            .author("Armo")
            .about("a blockchain for learning purpose")
            .subcommand(Command::new("printchain")).about("print all blocks in the chain")
            .subcommand(Command::new("createwallet")).about("create a wallet")
            .subcommand(Command::new("listaddresses")).about("list all addresses")
            .subcommand(
                Command::new("getbalance")
                .about("get balance in the blockchain")
                .arg(arg!(<ADDRESS>"'The Address it get balance for'"))
            ).subcommand(
                Command::new("create")
                .about("Create new blockchain")
                .arg(arg!(<ADDRESS>"'The address to send genesis block reward to'"))
            ).subcommand(
                Command::new("send")
                .about("send in the blockchain")
                .arg(arg!(<FROM>"'Source wallet address'"))
                .arg(arg!(<TO>"'Destination wallet address'"))
                .arg(arg!(<AMOUNT>"'amount to send'"))
            ).get_matches();

            if let Some(_) = matches.subcommand_matches("createwallet"){
                let mut ws = Wallets::new()?;
                let address = ws.create_wallet();
                ws.save_all()?;
                println!("newly created wallet with address : {}",address);
            }

            if let Some(_) = matches.subcommand_matches("listaddresses"){
                let ws = Wallets::new()?;
                let addresses = ws.get_all_addresses();
                println!("List of addresses : {:#?}",addresses);
            }
            
            if let Some(ref matches)= matches.subcommand_matches("create"){
                if let Some(address) = matches.get_one::<String>("ADDRESS"){
                    let address = String::from(address);
                    Blockchain::create_blockchain(address.clone())?;
                    println!("create a blockchain");
                }
            }
            
            if let Some(ref matches)= matches.subcommand_matches("getbalance"){
                if let Some(address) = matches.get_one::<String>("ADDRESS"){
                    let pub_key_hash = Address::decode(&address).unwrap().body;
                    let bc = Blockchain::new()?;
                    let utxos = bc.find_UTXO(&pub_key_hash);
                    let mut balance = 0;
                    for out in utxos {
                        balance+= out.value;
                    }
                    println!("Balance of '{}'; {}",address,balance);
                }
            }

            if let Some(ref matches) = matches.subcommand_matches("send"){
                let from = if let Some(address) = matches.get_one::<String>("FROM"){
                    address
                }else{
                    println!("from not supply! : usage");
                    exit(1)
                };

                let to = if let Some(address) = matches.get_one::<String>("TO"){
                    address
                }else{
                    println!("from not supply! : usage");
                    exit(1)
                };

                let amount : i32 = if let Some(amount) = matches.get_one::<String>("AMOUNT"){
                    amount.parse()?
                }else{
                    println!("from not supply! : usage");
                    exit(1)
                };
                let mut bc = Blockchain::new()?;
                let tx = Transaction::new_UTXO(from, to, amount, &bc)?;
                bc.add_block(vec![tx])?;
                println!("succeful transaction !!!!");
            }
            
            if let Some(_) = matches.subcommand_matches("printchain") {
                self.cmd_print_chain()?;
            }
            Ok(())
    }
    
    fn cmd_print_chain(&mut self) -> Result<(),Box<dyn std::error::Error>>{
        let bc = Blockchain::new()?;
        for block in bc.iter(){
            println!("{:#?}",block);
        }
        Ok(())
    }
}

/*
use bincode::Error;
use clap::Command;

use crate::{blockchain::*, transaction::Transaction};
use clap::arg;
pub struct Cli {
    bc: Blockchain,
}
impl Cli {
    pub fn new() -> Result<Cli,Box<dyn std::error::Error>> {
        Ok(Cli{
            bc : Blockchain::new()?,
        })
    }
    pub fn run(&mut self) -> Result<(),Box<dyn std::error::Error>> {
        let matches = Command::new("blockchain-rust-postquantum")
            .version("0.1")
            .author("Armo")
            .about("a blockchain for learning purpose")
            .subcommand(Command::new("printchain")).about("print all blocks in the chain")
            .subcommand(
                Command::new("addblock")
                .about("add a block in the chain")
                .arg( arg!(<DATA>" 'the blockchain data' "))
            ).get_matches();

            if let Some(ref matches) = matches.subcommand_matches("addblock"){
                if let Some(c) = matches.get_one::<String>("DATA"){
                    self.addblock(String::from(c))?;
                }else {
                    println!("No printing testing lists");
                }
            }

            if let Some(_) = matches.subcommand_matches("printchain") {
                self.print_chain()
            }
            Ok(())
    }
    
    fn addblock(&mut self, transactions : Vec<Transaction>) -> Result<(),Box< dyn std::error::Error>>{
        self.bc.add_block(transactions)
    }

    fn print_chain(&mut self){
        for b in &mut self.bc.iter(){
            println!("blocj {:#?}",b);
        }
    }
}

*/