use anyhow::{Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::rc::Rc;

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
        id: String,
    },
    Topology {
        msg_id: u64,
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {
        in_reply_to: u64,
    },
    Broadcast {
        msg_id: u64,
        message: serde_json::Value,
    },
    BroadcastOk {
        in_reply_to: u64,
    },
    Read {
        msg_id: u64,
    },
    ReadOk {
        in_reply_to: u64,
        messages: HashSet<serde_json::Value>,
    },
}

impl Body {
    fn as_str(&self) -> &'static str {
        match self {
            Body::Init { .. } => "init",
            Body::InitOk { .. } => "init_ok",
            Body::Generate { .. } => "generate",
            Body::GenerateOk { .. } => "generate_ok",
            Body::Broadcast { .. } => "broadcast",
            Body::BroadcastOk { .. } => "broadcast_ok",
            Body::Topology { .. } => "topology",
            Body::TopologyOk { .. } => "topology_ok",
            Body::Read { .. } => "read",
            Body::ReadOk { .. } => "read_ok",
            Body::Echo { .. } => "echo",
            Body::EchoOk { .. } => "echo_ok"
        }
    }
}
pub enum MaelstromNodeId {
    Unassigned,
    Assigned(String),
}

impl From<String> for MaelstromNodeId {
    fn from(value: String) -> Self {
        Self::Assigned(value)
    }
}
impl MaelstromNodeId {
    pub fn as_str(&self) -> Option<String> {
        match self {
            MaelstromNodeId::Assigned(id) => Some(id.clone()),
            MaelstromNodeId::Unassigned => None,
        }
    }
}
pub struct MaelstromNode<Extra> {
    pub node_id: MaelstromNodeId,
    pub next_res_id: u64,
    pub msg_handlers: HashMap<&'static str, Rc<dyn Fn(&mut Self, Message) -> Option<Result<Message>>>>,
    pub extra_data: Extra
}

pub trait MaelstromNodeActions<Extra: Default>
{
    fn handle<F>(&mut self, msg_type: &'static str, handler: F)
    where
        F: Fn(&mut Self, Message) -> Option<Result<Message>> + 'static;
    fn process_msg(&mut self, msg: Message) -> Option<Result<Message>>;
    fn send(&mut self, msg:Message) -> Result<()>;
    fn run(&mut self) -> Result<()>;
}

impl<Extra: Default> MaelstromNodeActions<Extra> for MaelstromNode<Extra> {
    fn handle<F>(&mut self, msg_type: &'static str, handler: F)
    where
        F: Fn(&mut Self, Message) -> Option<Result<Message>> + 'static,
    {
        self.msg_handlers.insert(msg_type, Rc::new(handler));
    }

    fn process_msg(&mut self, msg: Message) -> Option<Result<Message>> {
        let name = msg.body.as_str();
        let handler = self.msg_handlers
            .get(name)
            .expect("No action defined for message type {name}")
            .clone();
        let res = handler(self, msg);
        res
    }
    fn send(&mut self, msg: Message) -> Result<()> {
        println!("{}", serde_json::to_string(&msg)?);
        self.next_res_id += 1;
        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        self.handle("init", |node: &mut MaelstromNode<Extra>, msg: Message| -> Option<Result<Message>> {
            if let Body::Init { node_id, msg_id, .. } = msg.body
            {
                node.node_id = node_id.into();
                Some(Ok(Message {
                    src: node.node_id.as_str().expect("node id is not assigned"),
                    dst: msg.src,
                    body: Body::InitOk {
                        in_reply_to: msg_id,
                    },
                }))
            }
            else { None }
        });

        let mut stdin = std::io::stdin().lock();
        let mut buffer = String::new();
        stdin
            .read_line(&mut buffer)
            .expect("Failed reading init message");
        let init_msg: Message = serde_json::from_str(&buffer)?;

        let response = self.process_msg(init_msg);
        let response = response.unwrap()?;
        self.send(response)?;

        for line in stdin.lines() {
            let line = line?;
            let req: Message = serde_json::from_str(&line)?;
            let response = self.process_msg(req).unwrap()?;
            self.send(response)?;
        }
        Ok(())
    }
}