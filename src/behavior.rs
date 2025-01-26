use libp2p::kad::RoutingUpdate;
use libp2p::{identify, Multiaddr, PeerId};
use libp2p::swarm::NetworkBehaviour;
use libp2p::kad::{
    Behaviour as KademliaBehavior,
    Event as KademliaEvent,
    store::MemoryStore as KademliaInMemory,
};

use libp2p::identify::{
    Behaviour as IdentifyBehavior, 
    Event as IdentifyEvent,
};

use libp2p::request_response::{Event as RequestResponseEvent, OutboundRequestId, ResponseChannel as RequestResponseChannel};
use libp2p::request_response::cbor::Behaviour as RequestResponseBehavior;

use crate::message::Message;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event")]
pub(crate) struct Behavior {
    identify: IdentifyBehavior,
    kad: KademliaBehavior<KademliaInMemory>,
    rr: RequestResponseBehavior<Message, Message>
}

impl Behavior {
    pub fn new(kad: KademliaBehavior<KademliaInMemory>, identify : IdentifyBehavior,rr : RequestResponseBehavior<Message,Message>) -> Self{
        Self{ kad,identify,rr}
    }

    pub fn register_add_kad(&mut self,peer_id:&PeerId, addr : Multiaddr) -> RoutingUpdate {
        self.kad.add_address(peer_id, addr)
    }

    pub fn register_addr_rr(&mut self,peer_id: &PeerId, addr : Multiaddr) -> bool {
        self.rr.add_address(peer_id, addr)
    }
    
    pub fn send_message(&mut self, peer_id: &PeerId, message: Message) -> OutboundRequestId {
        self.rr.send_request(peer_id, message)
    }

    pub fn set_server_mode(&mut self){
        self.kad.set_mode(Some(libp2p::kad::Mode::Server));
    }
}
#[derive(Debug)]
pub(crate) enum Event {
    Identify(IdentifyEvent),
    Kad(KademliaEvent),
    RequestResponse(RequestResponseEvent<Message,Message>)
}

impl From<IdentifyEvent> for Event {
    fn from(value: IdentifyEvent) -> Self{
        Self::Identify(value)
    }
}

impl From<KademliaEvent> for Event {
    fn from(value: KademliaEvent) -> Self{
        Self::Kad(value)
    }
}

impl From<RequestResponseEvent<Message, Message>> for Event {
    fn from(value: RequestResponseEvent<Message, Message>) -> Self{
        Self::RequestResponse(value)
    }
}