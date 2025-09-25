use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Body {
    Echo {
        msg_id: u64,
        echo: String,
    },
    EchoOk {
        msg_id: u64,
        in_reply_to: u64,
        echo: String,
    },
    Init {
        msg_id: u64,
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk {
        in_reply_to: u64,
    },
    Generate {
        msg_id: u64,
    },
    GenerateOk {
        in_reply_to: u64,
        id: String
    },
    Topology {
        msg_id: u64,
        topology: HashMap<String, Vec<String>>
    },
    TopologyOk {
        in_reply_to: u64,
    },
    Broadcast {
        msg_id: u64,
        message: serde_json::Value
    },
    BroadcastOk {
        in_reply_to: u64,
    },
    Read {
        msg_id: u64,
    },
    ReadOk {
        in_reply_to: u64,
        messages: HashSet<serde_json::Value>
    }

}
