use serde::{Deserialize, Serialize};
use crate::block::*;
use crate::transaction::*;

#[derive(Serialize,Deserialize, Debug,Clone)]
pub enum Message {
    //Addr(Vec<String>), 
    Version(Versionmsg),
    Tx(Txmsg),
    GetData(GetDatamsg),
    GetBlock,
    Inv(Invmsg),
    Block(Blockmsg)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockmsg{
    pub block : Block,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetDatamsg{
    pub kind : String,
    pub id : String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Invmsg{
    pub kind : String,
    pub items : Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Txmsg{
    pub transaction : Transaction
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Versionmsg{
    pub version : i32,
    pub best_height : i32
}