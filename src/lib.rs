use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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
    Gossip {
        message: HashSet<serde_json::Value>,
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
            Body::EchoOk { .. } => "echo_ok",
            Body::Gossip { .. } => "gossip",
        }
    }
}

pub enum Event {
    Message(Message),
    Action(String),
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
    pub next_send_id: u64,
    pub msg_handlers:
        HashMap<&'static str, Rc<dyn Fn(&mut Self, Event) -> Option<Result<Message>>>>,
    pub extra_data: Extra,
}

pub trait MaelstromNodeActions<Extra: Default> {
    fn handle<F>(&mut self, msg_type: &'static str, handler: F)
    where
        F: Fn(&mut Self, Event) -> Option<Result<Message>> + 'static;
    fn process_event(&mut self, event: Event) -> Option<Result<Message>>;
    fn send(&mut self, msg: Message) -> Result<()>;
    fn run(&mut self, sender: Sender<Event>, receiver: Receiver<Event>) -> Result<()>;
}

impl<Extra: Default> MaelstromNodeActions<Extra> for MaelstromNode<Extra> {
    fn handle<F>(&mut self, msg_type: &'static str, handler: F)
    where
        F: Fn(&mut Self, Event) -> Option<Result<Message>> + 'static,
    {
        self.msg_handlers.insert(msg_type, Rc::new(handler));
    }

    fn process_event(&mut self, event: Event) -> Option<Result<Message>> {
        let name = match &event {
            Event::Action(action_name) => action_name,
            Event::Message(msg) => msg.body.as_str(),
        };
        match self.msg_handlers.get(&name) {
            Some(handler) => handler.clone()(self, event),
            None => None,
        }
    }
    fn send(&mut self, msg: Message) -> Result<()> {
        println!("{}", serde_json::to_string(&msg)?);
        self.next_send_id += 1;
        Ok(())
    }

    fn run(&mut self, sender: Sender<Event>, receiver: Receiver<Event>) -> Result<()> {
        self.handle(
            "init",
            |node: &mut MaelstromNode<Extra>, event: Event| -> Option<Result<Message>> {
                match event {
                    Event::Message(msg) => {
                        let Body::Init {
                            node_id, msg_id, ..
                        } = msg.body
                        else {
                            return None;
                        };
                        node.node_id = node_id.into();
                        Some(Ok(Message {
                            src: node.node_id.as_str().expect("node id is not assigned"),
                            dst: msg.src,
                            body: Body::InitOk {
                                in_reply_to: msg_id,
                            },
                        }))
                    }
                    Event::Action(..) => None,
                }
            },
        );

        let jh = thread::spawn(move || -> Result<()> {
            for line in std::io::stdin().lock().lines() {
                let line = line.context("input from STDIN cannot be read")?;
                let req: Message = serde_json::from_str(&line)?;
                sender
                    .send(Event::Message(req))
                    .context("Error sending to sender in the STDIN thread")?;
            }
            Ok(())
        });

        for event in receiver {
            if let Some(response) = self.process_event(event) {
                self.send(response.unwrap())?;
            }
        }

        jh.join()
            .expect("STDIN thread panic")
            .context("STDIN thread error out")?;

        Ok::<(), anyhow::Error>(())
    }
}
