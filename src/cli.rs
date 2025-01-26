use std::process::exit;
use bincode::Error;
use bitcoincash_addr::Address;
use clap::Command;
use crate::{block::Block, 
    blockchain::*, 
    transaction::Transaction, 
    utxoset::UTXOSet, 
    wallet::{Wallet,Wallets}
    };
use crate::server::Server;
use clap::arg;
pub struct Cli {

}
impl Cli {
    pub fn new() -> Result<Cli,Box<dyn std::error::Error>> {
        Ok(Cli{

        })
    }
    
    pub async fn run(&mut self) -> Result<(),Box<dyn std::error::Error>> {
        let matches = Command::new("blockchain-rust-postquantum")
            .version("0.1")
            .author("Armo")
            .about("a blockchain for learning purpose")
            .subcommand(Command::new("printchain")).about("print all blocks in the chain")
            .subcommand(Command::new("createwallet")).about("create a wallet")
            .subcommand(Command::new("listaddresses")).about("list all addresses")
            .subcommand(Command::new("reindex")).about("update unspents transactions index")
            .subcommand(
                Command::new("getbalance")
                .about("get balance in the blockchain")
                .arg(arg!(<ADDRESS>"'The Address it get balance for'"))
            )
            .subcommand(Command::new("startnode")
            .about("start the node server")
            .arg(arg!(<WALLET_ADDR>"'wallet addresss of the node'"))
            .arg(arg!(<IP_ADDR>"'ip address of the node [specify only if it is bootsrap node]'"))
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
                .arg(arg!(-m --mine " 'the from address mine immediately'")),
            )
            .get_matches();
        
            /*******************************************************************************/
            if let Some(ref matches) = matches.subcommand_matches("startnode") {
                let ip_addr = if let Some(address) = matches.get_one::<String>("IP_ADDR") {
                    address
                } else {
                    ""
                };
                let wallet_addr = if let Some(address) = matches.get_one::<String>("WALLET_ADDR") {
                    address
                } else {
                    println!("You need to specify the wallet address !!");
                    exit(1)
                };
                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet { blockchain: bc };
                let mut server = Server::new(ip_addr, wallet_addr, utxo_set).await?;
                println!("Finish first step ===> start Server :");
                server.start_server().await;
                
            }
            /********************************************************************************/
            if let Some(_) = matches.subcommand_matches("createwallet") {
                println!("address: {}", cmd_create_wallet()?);
            }
            if let Some(_) = matches.subcommand_matches("reindex") {
                let count = cmd_reindex()?;
                println!("Done! There are {} transactions in the UTXO set.", count);
            }
    
            if let Some(_) = matches.subcommand_matches("listaddresses") {
                cmd_list_address()?;
            }
    
            if let Some(ref matches) = matches.subcommand_matches("create") {
                if let Some(address) = matches.get_one::<String>("ADDRESS") {
                    cmd_create_blockchain(address)?;
                }
            }

            if let Some(ref matches) = matches.subcommand_matches("getbalance") {
                if let Some(address) = matches.get_one::<String>("ADDRESS") {
                    let balance = cmd_get_balance(address)?;
                    println!("Balance: {}\n", balance);
                }
            }

            if let Some(ref matches) = matches.subcommand_matches("send") {
                let from = if let Some(address) = matches.get_one::<String>("FROM") {
                    address
                } else {
                    println!("from not supply!: usage");
                    exit(1)
                };
                let to = if let Some(address) = matches.get_one::<String>("TO") {
                    address
                } else {
                    println!("from not supply!: usage");
                    exit(1)
                };
                let amount: i32 = if let Some(amount) = matches.get_one::<String>("AMOUNT") {
                    amount.parse()?
                } else {
                    println!("from not supply!: usage");
                    exit(1)
                };
                cmd_send(from, to, amount, false).await?;

                /*if matches.contains_id("mine") {
                    println!("start mining now ==> ");
                    cmd_send(from, to, amount, true)?;
                } else {
                    println!("start mining later ==> ");
                    cmd_send(from, to, amount, false)?;
                }*/
            }
    
            if let Some(_) = matches.subcommand_matches("printchain") {
                cmd_print_chain()?;
            }
    
            /*if let Some(_) = matches.subcommand_matches("reindex"){
                let bc = Blockchain::new()?;
                let utxo_set = UTXOSet{blockchain : bc};
                utxo_set.reindex()?;
                let count = utxo_set.count_transactions()?;
                println!("There are {} transactions in the transactions index ",count);
            }

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
                    let bc= Blockchain::create_blockchain(address.clone())?;
                    let utxo_set = UTXOSet {blockchain : bc};
                    utxo_set.reindex()?;
                    println!("create a blockchain");
                }
            }
            
            if let Some(ref matches)= matches.subcommand_matches("getbalance"){
                if let Some(address) = matches.get_one::<String>("ADDRESS"){
                    let pub_key_hash = Address::decode(&address).unwrap().body;
                    let bc = Blockchain::new()?;
                    let utxo_set = UTXOSet { blockchain : bc};
                    let utxos = utxo_set.find_UTXO(&pub_key_hash)?;
                    let mut balance = 0;
                    for out in utxos.outputs {
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
                let mut utxo_set = UTXOSet{blockchain : bc};
                let tx = Transaction::new_UTXO(from, to, amount, &utxo_set)?;
                let cbtx = Transaction::new_coinbase(from.to_string(), String::from("reward!"))?;
                let new_block = utxo_set.blockchain.add_block(vec![cbtx,tx])?;
                utxo_set.update(&new_block)?;
                println!("succeful transaction !!!!");
            }
            
            if let Some(_) = matches.subcommand_matches("printchain") {
                self.cmd_print_chain()?;
            }*/
            Ok(())
    }
}
/***********************************************************************/
/******************************************************************/
/************************************************************************/
async fn cmd_send(from: &str, to: &str, amount: i32, mine_now: bool) -> Result<(),Box<dyn std::error::Error>> {
    let bc = Blockchain::new()?;
    let mut utxo_set = UTXOSet { blockchain: bc };
    let tx = Transaction::new_UTXO(from, to, amount, &utxo_set)?;
    Server::send_transaction(from,&tx, utxo_set).await?;
    println!("success!");
    Ok(())
}

fn cmd_create_wallet() -> Result<String,Box<dyn std::error::Error>> {
    let mut ws = Wallets::new()?;
    let address = ws.create_wallet();
    ws.save_all()?;
    Ok(address)
}

fn cmd_reindex() -> Result<i32,Box<dyn std::error::Error>> {
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    utxo_set.count_transactions()
}

fn cmd_create_blockchain(address: &str) -> Result<(),Box<dyn std::error::Error>> {
    let address = String::from(address);
    let bc = Blockchain::create_blockchain(address)?;

    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    println!("create blockchain");
    Ok(())
}

fn cmd_get_balance(address: &str) -> Result<i32,Box<dyn std::error::Error>> {
    let pub_key_hash = Address::decode(address).unwrap().body;
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    let utxos = utxo_set.find_UTXO(&pub_key_hash)?;

    let mut balance = 0;
    for out in utxos.outputs {
        balance += out.value;
    }
    Ok(balance)
}

fn cmd_print_chain() -> Result<(),Box<dyn std::error::Error>> {
    let bc = Blockchain::new()?;
    for b in bc.iter() {
        println!("{:#?}", b);
    }
    Ok(())
}

fn cmd_list_address() -> Result<(),Box<dyn std::error::Error>> {
    let ws = Wallets::new()?;
    let addresses = ws.get_all_addresses();
    println!("addresses: ");
    for ad in addresses {
        println!("{}", ad);
    }
    Ok(())
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