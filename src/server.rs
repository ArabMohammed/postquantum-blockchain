use super::*;
use crate::utxoset::*;
use crate::message::*;
use crate::block::*;
use crate::transaction::*;
use crate::behavior::Behavior;
use crate::behavior::Event as AgentEvent;
use crate::constants::*;
/************************/
use bincode::{deserialize, serialize};
use failure::format_err;
use std::collections::{HashMap, HashSet};
use std::io::prelude::*;
use std::ops::Mul;
use std::sync::*;
use std::{thread,time::Duration};
/*****************************/
use libp2p::kad::RoutingUpdate;
use libp2p::{
    Multiaddr,
    identity,
    PeerId,
    StreamProtocol,
    noise, 
    ping,
    Swarm,
    swarm::{NetworkBehaviour,SwarmEvent},tcp
    ,yamux,
    kad::{store::MemoryStore, Mode,Behaviour},
    kad,
    mdns
};
use libp2p::identify::{
    Config as IdentifyConfig, 
    Behaviour as IdentifyBehavior, 
    Event as IdentifyEvent
};
use libp2p::request_response::{
    Config as RequestResponseConfig, 
    ProtocolSupport as RequestResponseProtocolSupport,
    Event as RequestResponseEvent,
    Message as RequestResponseMessage
};
use libp2p::request_response::cbor::Behaviour as RequestResponseBehavior;
use tracing_subscriber::EnvFilter;
use futures::stream::StreamExt;
/****************************/
use log::{debug, info,warn,error};
use env_logger::{Env, Builder};
use std::env::args;
use get_size::GetSize;
/*****************************/
pub struct Server{
    wallet_address : String,
    swarm : Swarm<Behavior>,
    inner : Arc<Mutex<ServerInner>>
}

pub struct ServerInner{
    pub known_peers : HashSet<PeerId>,
    utxo : UTXOSet,
    blocks_in_transit : Vec<String>,
    mempool : HashMap<String,Transaction>,
}

//const BOOTSTRAP_NODE: &str = "localhost:3000";
const CMD_LEN: usize = 12;
const VERSION: i32 = 1;

impl Server {
    pub async fn new(ip_addr : &str, wallet_addr : &str, utxo : UTXOSet) -> Result<Server, Box<dyn std::error::Error>>{
        let local_key: identity::Keypair = libp2p::identity::Keypair::generate_ed25519();
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key.clone())
                    .with_tokio()
                    .with_tcp(
                        tcp::Config::default(), 
                        noise::Config::new, 
                        yamux::Config::default
                    )?
                    .with_behaviour(|key|{
                        let local_peer_id = PeerId::from(key.public().clone());
                        info!("Local peer ID : {local_peer_id}");
                        let mut kad_config = kad::Config::new(StreamProtocol::new("/agent/connection/1.0.0"));
                        let kad_memory = MemoryStore::new(local_peer_id);
                        let kad_mem_behaviour = kad::Behaviour::with_config(local_peer_id, kad_memory, kad_config);
                        let identity_config = IdentifyConfig::new(
                            "/agent/connection/1.0.0".to_string(), 
                            key.clone().public()
                        )
                        .with_push_listen_addr_updates(true)
                        .with_interval(Duration::from_secs(30));

                        let rr_config = RequestResponseConfig::default();
                        let rr_protocol = StreamProtocol::new("/agent/message/1.0.0");
                        let rr_behavior = RequestResponseBehavior::<Message,Message>::new([(rr_protocol,RequestResponseProtocolSupport::Full)],rr_config);

                        let identify = IdentifyBehavior::new(identity_config);
                        Behavior::new(kad_mem_behaviour,identify,rr_behavior)
                    })?
                    .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30)))
                    .build();
        let mut node_set : HashSet<PeerId> = HashSet::new();
        if ip_addr != ""{
            swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
            let remote : Multiaddr = ip_addr.parse()?;
            //Dial a known or unknown peer.
            swarm.dial(remote)?;
            info!("Dialed to: {ip_addr}");
        }else{
            info!("Act as bootstrap node");
            swarm.listen_on("/ip4/0.0.0.0/tcp/8000".parse()?)?;
        }
        /***********************************************************************************/
        /***************************connect to at least one node ***************************/
        println!("===> waiting for new connections !!! ");
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { listener_id, address } => info!("NewListenAddr: {listener_id:?} | {address:?}"),
                SwarmEvent::ConnectionEstablished { 
                    peer_id, 
                    connection_id, 
                    endpoint, 
                    num_established, 
                    concurrent_dial_errors, 
                    established_in 
                }=> {info!("ConnectionEstablished: {peer_id} | {connection_id} | {endpoint:?} | {num_established} | {concurrent_dial_errors:?} | {established_in:?}");
                    },
                SwarmEvent::Dialing { peer_id, connection_id } => info!("Dialing: {peer_id:?} | {connection_id}"),
         
                SwarmEvent::Behaviour(AgentEvent::Kad(event)) => match event {
                    kad::Event::ModeChanged { new_mode } => info!("KadEvent:ModeChanged: {new_mode}"),
                    kad::Event::RoutablePeer { peer, address } => info!("KadEvent:RoutablePeer: {peer} | {address}"),
                    kad::Event::PendingRoutablePeer { peer, address } => info!("KadEvent:PendingRoutablePeer: {peer} | {address}"),
                    kad::Event::InboundRequest { request } => info!("KadEvent:InboundRequest: {request:?}"),
                    kad::Event::RoutingUpdated { 
                        peer, 
                        is_new_peer, 
                        addresses, 
                        bucket_range, 
                        old_peer } => {
                            info!("KadEvent:RoutingUpdated: {peer} | IsNewPeer? {is_new_peer} | {addresses:?} | {bucket_range:?} | OldPeer: {old_peer:?}");
                    },
                    kad::Event::OutboundQueryProgressed { 
                        id, 
                        result, 
                        stats, 
                        step } => {
    
                        info!("KadEvent:OutboundQueryProgressed: ID: {id:?} | Result: {result:?} | Stats: {stats:?} | Step: {step:?}")
                    },
                    _ => {println!("problem in :  SwarmEvent::Behaviour(AgentEvent::Kad");}
                },
                SwarmEvent::Behaviour(AgentEvent::Identify(event)) => match event {
                    //Identification information of the local node has been sent 
                    //to a peer in response to an identification request
                    IdentifyEvent::Sent {peer_id,connection_id :_ } => info!("IdentifyEvent:Sent: {peer_id}"),
                    IdentifyEvent::Pushed { connection_id : _, peer_id, info } => info!("IdentifyEvent:Pushed: {peer_id} | {info:?}"),
                    IdentifyEvent::Received { connection_id, peer_id, info }=> {
                        info!("IdentifyEvent:Received: {peer_id} | {info:?}");
                        node_set.insert(peer_id.clone());
                        for addr in info.listen_addrs.clone(){
                            let agent_routing = swarm.behaviour_mut().register_add_kad(&peer_id, addr.clone());
                            match agent_routing {
                                RoutingUpdate::Failed => error!("IdentifyReceived: Failed to register address to Kademlia"),
                                RoutingUpdate::Pending => warn!("IdentifyReceived: Register address pending"),
                                RoutingUpdate::Success => {
                                    info!("IdentifyReceived: {addr}: Success register address");
                                } 
                            }
                            _ = swarm.behaviour_mut().register_addr_rr(&peer_id, addr.clone());
                        }
                    },
                    _ => {println!("problem in : SwarmEvent::Behaviour(AgentEvent::Identify");}
                },
                SwarmEvent::IncomingConnectionError { connection_id, local_addr, send_back_addr, error } => println!("incming connection error"),
                SwarmEvent::OutgoingConnectionError { connection_id, peer_id, error } => println!("outgoing connection error !!"),
                
                _ => {println!("problem in swarmevent !!");}
            }
            if node_set.len() > 0 {
                break;
            }
        }
        println!("return instance of the created server !!");
        /********************************************************/
        Ok(Server{
            wallet_address : wallet_addr.to_string(),
            swarm,
            inner: Arc::new(Mutex::new(ServerInner{
                known_peers : node_set,
                utxo,
                blocks_in_transit : Vec::new(),
                mempool : HashMap::new(),

            })),
        })

    }
    
    /**************************************************************************/
    /****************************inner hepler functions ***********************/
    fn get_mempool(&self) -> HashMap<String,Transaction>{
        self.inner.lock().unwrap().mempool.clone()
    }
    
    fn insert_mempool(&self, tx: Transaction){
        self.inner.lock().unwrap().mempool.insert(tx.id.clone(),tx);
    }
    
    fn get_mempool_tx(&self,addr : &str) -> Option<Transaction>{
        match self.inner.lock().unwrap().mempool.get(addr){
            Some(tx) => Some(tx.clone()),
            None => None
        }
    }
    
    fn clear_mempool(&self){
        self.inner.lock().unwrap().mempool.clear()
    }

    pub async fn start_server(&mut self){
        loop {
            self.handle_events().await.unwrap();
        }
    }
    /*******************************************************/
    /********************************************************
    fn remove_node(&self, addr : &str){
        self.inner.lock().unwrap().known_nodes.remove(addr);
    }

    fn add_node(&self, addr : &str){
        self.inner.lock().unwrap().known_nodes.insert(String::from(addr));
    }
    
    fn node_is_known(&self, addr : &str) -> bool {
        self.inner.lock().unwrap().known_nodes.get(addr).is_some()
    }
    ********************************************************/
    fn get_known_nodes(&self) -> HashSet<PeerId> {
        self.inner.lock().unwrap().known_peers.clone()
    }
    fn add_node(&self, peer_id : &PeerId){
        self.inner.lock().unwrap().known_peers.insert(peer_id.clone());
    }
    /********************************************************/
    fn get_block(&self, blokc_hash : &str) -> Result<Block,Box<dyn std::error::Error>>{
        self.inner.lock().unwrap().utxo.blockchain.get_block(blokc_hash)
    }
    
    fn add_block(&self, block: Block) {
        self.inner.lock().unwrap().utxo.blockchain.add_block(block).unwrap();
    }

    fn mine_block(&self, txs: Vec<Transaction>) -> Result<Block,Box<dyn std::error::Error>> {
        self.inner.lock().unwrap().utxo.blockchain.mine_block(txs)
    }

    fn request_blocks(&mut self) -> Result<(),Box<dyn std::error::Error>>{
        for node in self.get_known_nodes(){
            self.send_get_blocks(&node)?
        }
        Ok(())
    }
    
    fn get_block_hashs(&self) -> Vec<String>{
        self.inner.lock().unwrap().utxo.blockchain.get_block_hashs()
    }
    /******************************************************/
    fn replace_in_transit(&self,hashs : Vec<String>){
        let bit = &mut self.inner.lock().unwrap().blocks_in_transit;
        bit.clone_from(&hashs)
    }

    fn get_in_transit(&self) -> Vec<String> {
        self.inner.lock().unwrap().blocks_in_transit.clone()
    }

    fn get_best_height(&self) -> Result<i32,Box<dyn std::error::Error>>{
        self.inner.lock().unwrap().utxo.blockchain.get_best_height()
    }
    /*************************************************************************************/
    /*************************************************************************************/
    fn send_block(& mut self, peer_id : &PeerId, b: &Block) -> Result<(),Box<dyn std::error::Error>> {
        info!("send block data to: {} block hash: {}", peer_id, b.get_hash());
        let data = Message::Block(Blockmsg {
            block: b.clone(),
        });
        self.send_data(peer_id, data)
    }
    /***********************************************/
    /*fn send_addr(&self, addr: &str) -> Result<(),Box<dyn std::error::Error>> {
        info!("send address info to: {}", addr);
        let nodes = self.get_known_nodes();
        let data = serialize(&(cmd_to_bytes("addr"), nodes))?;
        self.send_data(addr, &data)
    }*/
    /***********************************************/
    fn send_version(&mut self, peer_id: &PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("send version info to: {}", peer_id);
        let data = Message::Version(Versionmsg {
            best_height: self.get_best_height()?,
            version: VERSION,
        });
        self.send_data(peer_id, data)
    }
    /***********************************************/
    fn send_inv(& mut self, peer_id: &PeerId, kind: &str, items: Vec<String>) -> Result<(),Box<dyn std::error::Error>> {
        info!(
            "send inv message to: {} kind: {} data: {:?}",
            peer_id, kind, items
        );
        let data = Message::Inv(Invmsg {
            kind: kind.to_string(),
            items,
        });
        self.send_data(peer_id, data)
    }
    /*************************************************/
    pub async fn send_transaction(wallet_addr : &str,tx : &Transaction, utxoset : UTXOSet) -> Result<(),Box<dyn std::error::Error>>{
        let mut server = Server::new("/ip4/127.0.0.1/tcp/8000",wallet_addr,utxoset).await?;
        for addr in &server.get_known_nodes(){
            server.send_tx(addr,tx)?;
        }
        server.start_server().await;
        Ok(())
    }
    /************************************************/
    pub fn send_tx(&mut self, peer_id: &PeerId, tx: &Transaction) -> Result<(),Box<dyn std::error::Error>> {
        info!("send tx to: {} txid: {}", peer_id, &tx.id);
        let data = Message::Tx(Txmsg {
            transaction: tx.clone(),
        });
        self.send_data(peer_id, data)
    }
    /************************************************/
    fn send_get_data(&mut self, peer_id: &PeerId, kind: &str, id: &str) -> Result<(),Box<dyn std::error::Error>> {
        info!(
            "send get data message to: {} kind: {} id: {}",
            peer_id, kind, id
        );
        let data = Message::GetData(GetDatamsg{
            kind: kind.to_string(),
            id: id.to_string(),
        });

        self.send_data(peer_id, data)
    }
    /************************************************/
    fn send_get_blocks(& mut self, peer_id: &PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("send get blocks message to: {}", peer_id);
        let data = Message::GetBlock ;
        self.send_data(peer_id, data)
    }
    /**********   ===================>   ***********/
    fn send_data(&mut self, peer_id: &PeerId, data: Message) -> Result<(),Box<dyn std::error::Error>> {
        if peer_id == self.swarm.local_peer_id() {
            return Ok(());
        }
        let request_id = self.swarm.behaviour_mut().send_message(&peer_id, data);
        info!("RequestID: {request_id}"); 
        // continue server running 
        info!("data send successfully");
        Ok(())
    }
    /*************************************************************************************/
    /*************************************************************************************/
    async fn handle_events(&mut self) -> Result<(),Box<dyn std::error::Error>>{
        match self.swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { listener_id, address } => info!("NewListenAddr: {listener_id:?} | {address:?}"),
            /********************************************************/
            SwarmEvent::ConnectionEstablished { 
                peer_id, 
                connection_id, 
                endpoint, 
                num_established, 
                concurrent_dial_errors, 
                established_in 
            } => info!("ConnectionEstablished: {peer_id} | {connection_id} | {endpoint:?} | {num_established} | {concurrent_dial_errors:?} | {established_in:?}"),
            /*
                A new dialing attempt has been initiated by the NetworkBehaviour
                implementation.A ConnectionEstablished event is reported if the 
                dialing attempt succeeds, otherwise a OutgoingConnectionError event 
                is reported.
            */
            SwarmEvent::Dialing { peer_id, connection_id } => info!("Dialing: {peer_id:?} | {connection_id}"),
            /*******************************************************/
            SwarmEvent::Behaviour(AgentEvent::Identify(event)) => match event {
                //Identification information of the local node has been sent 
                //to a peer in response to an identification request
                IdentifyEvent::Sent {peer_id,connection_id :_ } => info!("IdentifyEvent:Sent: {peer_id}"),
                IdentifyEvent::Pushed { connection_id : _, peer_id, info } => info!("IdentifyEvent:Pushed: {peer_id} | {info:?}"),
                IdentifyEvent::Received { connection_id, peer_id, info }=> {
                    info!("IdentifyEvent:Received: {peer_id} | {info:?}");
                    self.add_node(&peer_id);
                    for addr in info.listen_addrs.clone(){
                        let agent_routing = self.swarm.behaviour_mut().register_add_kad(&peer_id, addr.clone());
                        match agent_routing {
                            RoutingUpdate::Failed => error!("IdentifyReceived: Failed to register address to Kademlia"),
                            RoutingUpdate::Pending => warn!("IdentifyReceived: Register address pending"),
                            RoutingUpdate::Success => {
                                info!("IdentifyReceived: {addr}: Success register address");
                            } 
                        }
                        _ = self.swarm.behaviour_mut().register_addr_rr(&peer_id, addr.clone());
                    }

                    info!("Avaialable peers: {:?}",self.get_known_nodes());
                },
                _ => {}
            },
            /*********************************************************/
            SwarmEvent::Behaviour(AgentEvent::RequestResponse(event)) => match event {
                // an incoming message (req or response) , 
                // peer_id : peer who sent the message 
                RequestResponseEvent::Message { peer, connection_id, message }=> {
                    match message {
                        RequestResponseMessage::Request { request_id, request, channel } => {
                            self.handle_message(request,&peer).unwrap();
                        },
                        RequestResponseMessage::Response { request_id, response } => {
                            info!("RequestResponseEvent::Message::Response -> PeerID: {peer} | RequestID: {request_id} | Response: {response:?}")
                        }
                    }
                },
                // an outbound request failed , peer : The peer to whom the request was sent
                RequestResponseEvent::InboundFailure { peer, connection_id,request_id, error } => {
                    warn!("RequestResponseEvent::InboundFailure -> PeerID: {peer} | RequestID: {request_id} | Error: {error}")
                },
                // A response to an inbound request has been sent.
                RequestResponseEvent::ResponseSent { peer, connection_id,request_id } => {
                    info!("RequestResponseEvent::ResponseSent -> PeerID: {peer} | RequestID: {request_id}")
                },
                // An outbound request failed.
                RequestResponseEvent::OutboundFailure { peer, connection_id,request_id, error } => {
                    warn!("RequestResponseEvent::OutboundFailure -> PeerID: {peer} | RequestID: {request_id} | Error: {error}")
                },
                _ => {}
            },
            /***********************************************************/
            SwarmEvent::Behaviour(AgentEvent::Kad(event)) => match event {
                kad::Event::ModeChanged { new_mode } => info!("KadEvent:ModeChanged: {new_mode}"),
                kad::Event::RoutablePeer { peer, address } => info!("KadEvent:RoutablePeer: {peer} | {address}"),
                kad::Event::PendingRoutablePeer { peer, address } => info!("KadEvent:PendingRoutablePeer: {peer} | {address}"),
                kad::Event::InboundRequest { request } => info!("KadEvent:InboundRequest: {request:?}"),
                kad::Event::RoutingUpdated { 
                    peer, 
                    is_new_peer, 
                    addresses, 
                    bucket_range, 
                    old_peer } => {
                        info!("KadEvent:RoutingUpdated: {peer} | IsNewPeer? {is_new_peer} | {addresses:?} | {bucket_range:?} | OldPeer: {old_peer:?}");
                },
                kad::Event::OutboundQueryProgressed { 
                    id, 
                    result, 
                    stats, 
                    step } => {

                    info!("KadEvent:OutboundQueryProgressed: ID: {id:?} | Result: {result:?} | Stats: {stats:?} | Step: {step:?}")
                },
                _ => {}
            }
            /************************************************************/
            _ => {},
        }
        Ok(())
    }
    /****==================>  **********************/
    pub fn handle_message(&mut self,message : Message, peer_id : &PeerId) ->  Result<(),Box<dyn std::error::Error>> {
        println!("handle message !!");
        match message {
            Message::Tx(data) => self.handle_tx(data,peer_id)?,
            Message::Version(data) => self.handle_version(data,peer_id)?,
            //Message::Addr(data) => self.handle_addr(data)?,
            Message::Block(data) => self.handle_block(data,peer_id)?,
            Message::GetBlock => self.handle_get_blocks(peer_id)?,
            Message::GetData(data) => self.handle_get_data(data,peer_id)?,
            Message::Inv(data) => self.handle_inv(data,peer_id)?,
        }
        Ok(())
    }
    fn verify_tx(&self, tx: &Transaction) -> Result<bool,Box<dyn std::error::Error>> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .verify_transaction(tx)
    }
    /********************************/
    fn handle_tx(&mut self, msg: Txmsg,peer_id : &PeerId) -> Result<(),Box<dyn std::error::Error>> {
        println!("receive transaction {} , from :{}",peer_id, &msg.transaction.id);
        self.insert_mempool(msg.transaction.clone());

        let known_nodes = self.get_known_nodes();
        for node in &known_nodes {
            if node != self.swarm.local_peer_id() && node != peer_id {
                self.send_inv(&node, "tx", vec![msg.transaction.id.clone()])?;
            }
        }
        /*********/thread::sleep(Duration::from_secs(2));
        /********Mine the block if nb_transactions > limit *********************/
        let mut mempool = self.get_mempool();
        debug!("Current mempool: {:#?}", &mempool);
        // sent while node was mining old ones 
        let total_transactions_size: usize = mempool.values().map(|v| v.get_heap_size()).sum();
        if mempool.len() >= 2 && !self.wallet_address.is_empty()  {
            loop {
                info!("Start mining a new block !!!");
                let mut txs = Vec::new();
                for (_, tx) in &mempool {
                    if self.verify_tx(tx)? {
                        txs.push(tx.clone());
                    }
                }
                if txs.is_empty() {
                    return Ok(());
                }

                let cbtx =
                    Transaction::new_coinbase(self.wallet_address.clone(), String::new())?;
                txs.push(cbtx);

                for tx in &txs {
                    mempool.remove(&tx.id);
                }

                let new_block = self.mine_block(txs)?;
                self.utxo_reindex()?;

                for node in &self.get_known_nodes() {
                    if node != self.swarm.local_peer_id() {
                        info!("send inv msg, new_mined_block_hash {}",new_block.get_hash());
                        self.send_inv(&node, "block", vec![new_block.get_hash()])?;
                    }
                }
                // to mine a new block based on transactions 
                // sent while node was mining old ones 
                if mempool.len() < 2 {
                    break;
                }
            }
            self.clear_mempool();
        }
        Ok(())
    }
    /************************************************/
    fn handle_version(&mut self, msg: Versionmsg, peer_id : &PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("receive version msg: {:#?}", msg);
        let my_best_height = self.get_best_height()?;
        if my_best_height < msg.best_height {
            self.send_get_blocks(peer_id)?;
        } else if my_best_height > msg.best_height {
            self.send_version(peer_id)?;
        }
        Ok(())
    }
    /*************************************************/
    /*fn handle_addr(&self, msg: Vec<String>) -> Result<(),Box<dyn std::error::Error>> {
        info!("receive address msg: {:#?}", msg);
        for node in msg {
            self.add_node(&node);
        }
        //self.request_blocks()?;
        Ok(())
    }*/
    /*************************************************/
    fn handle_block(&mut self, msg: Blockmsg,peer_id: &PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!(
            "receive block msg {} from: {}",
            peer_id,
            msg.block.get_hash()
        );
        self.add_block(msg.block);

        let mut in_transit = self.get_in_transit();
        if in_transit.len() > 0 {
            let block_hash = &in_transit[0];
            self.send_get_data(peer_id, "block", block_hash)?;
            in_transit.remove(0);
            self.replace_in_transit(in_transit);
        }else{
            self.utxo_reindex().unwrap();
        }
        Ok(())
    }
    /*************************************************/
    fn handle_get_blocks(&mut self,peer_id :&PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("receive get blocks msg:");
        let block_hashs = self.get_block_hashs();
        self.send_inv(peer_id, "block", block_hashs)?;
        Ok(())
    }
    /*************************************************/
    fn handle_get_data(&mut self, msg: GetDatamsg,peer_id :&PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("receive get data msg: {:#?}", msg);
        if msg.kind == "block" {
            let block = self.get_block(&msg.id)?;
            self.send_block(peer_id ,&block)?;
        } else if msg.kind == "tx" {
            let tx = self.get_mempool_tx(&msg.id).unwrap();
            self.send_tx(peer_id, &tx)?;
        }
        Ok(())
    }
    /*************************************************/
    fn handle_inv(&mut self, msg: Invmsg,peer_id :&PeerId) -> Result<(),Box<dyn std::error::Error>> {
        info!("receive inv msg: {:#?}", msg);
        if msg.kind == "block" {
            let block_hash = &msg.items[0];
            self.send_get_data(peer_id, "block", block_hash)?;

            let mut new_in_transit = Vec::new();
            for b in &msg.items {
                if b != block_hash {
                    new_in_transit.push(b.clone());
                }
            }
            self.replace_in_transit(new_in_transit);
        } else if msg.kind == "tx" {
            let txid = &msg.items[0];
            match self.get_mempool_tx(txid) {
                Some(tx) => {
                    if tx.id.is_empty() {
                        self.send_get_data(peer_id, "tx", txid)?
                    }
                }
                None => self.send_get_data(peer_id, "tx", txid)?,
            }
        }
        Ok(())
    }
    /********************************************************************/
    /********used in handle block, handle tx******************************/
    fn utxo_reindex(&self) -> Result<(),Box<dyn std::error::Error>> {
        self.inner.lock().unwrap().utxo.reindex()
    }

}
/**************************************************************************/
/*fn cmd_to_bytes(cmd: &str) -> [u8; CMD_LEN] {
    let mut data = [0; CMD_LEN];
    for (i, d) in cmd.as_bytes().iter().enumerate() {
        data[i] = *d;
    }
    data
}*/

/*fn bytes_to_cmd(bytes: &[u8]) -> Result<Message,Box<dyn std::error::Error>> {

    let mut cmd = Vec::new();
    let cmd_bytes = &bytes[..CMD_LEN];
    let data = &bytes[CMD_LEN..];
    for b in cmd_bytes {
        if 0 as u8 != *b {
            cmd.push(*b);
        }
    }
    info!("cmd: {}", String::from_utf8(cmd.clone())?);

    if cmd == "addr".as_bytes() {
        let data: Vec<String> = deserialize(data)?;
        Ok(Message::Addr(data))
    } else if cmd == "block".as_bytes() {
        let data: Blockmsg = deserialize(data)?;
        Ok(Message::Block(data))
    } else if cmd == "inv".as_bytes() {
        let data: Invmsg = deserialize(data)?;
        Ok(Message::Inv(data))
    } else if cmd == "getblocks".as_bytes() {
        let data: GetBlocksmsg = deserialize(data)?;
        Ok(Message::GetBlock(data))
    } else if cmd == "getdata".as_bytes() {
        let data: GetDatamsg = deserialize(data)?;
        Ok(Message::GetData(data))
    } else if cmd == "tx".as_bytes() {
        let data: Txmsg = deserialize(data)?;
        Ok(Message::Tx(data))
    } else if cmd == "version".as_bytes() {
        let data: Versionmsg = deserialize(data)?;
        Ok(Message::Version(data))
    } else {
        Err(format_err!("Unknown command in the server").into())
    }
}*/