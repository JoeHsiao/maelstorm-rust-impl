use std::collections::{HashMap, HashSet};
use anyhow::{Result, bail};
use std::io::BufRead;
use maelstrom_rust_impl::{Message, Body};
struct Broadcaster {
    node_id: String,
    topology: HashMap<String, Vec<String>>,
    seen_messages: HashSet<serde_json::Value>
}

impl Broadcaster {
    fn init(req: &Message) -> Result<(Self, Message)> {
        match &req.body {
            Body::Init {
                msg_id,
                node_id,
                node_ids: _,
            } => {
                let service = Broadcaster {
                    node_id: node_id.clone(),
                    topology: HashMap::new(),
                    seen_messages: HashSet::new()
                };
                Ok((
                    service,
                    Message {
                        src: node_id.clone(),
                        dst: req.src.clone(),
                        body: Body::InitOk {
                            in_reply_to: *msg_id,
                        },
                    },
                ))
            }
            _ => bail!("Cannot init Echoer because there was no init message!"),
        }
    }
    fn process(&mut self, req: Message) -> Result<Message> {
        match req.body {
            Body::Topology { msg_id, topology } => {
                self.topology = topology;
                Ok (Message {
                    src: self.node_id.clone(),
                    dst: req.src.clone(),
                    body: Body::TopologyOk {
                        in_reply_to: msg_id,
                    },
                })
            },
            Body::Read { msg_id} => {
                Ok(Message{
                    src: self.node_id.clone(),
                    dst: req.src.clone(),
                    body: Body::ReadOk {
                        in_reply_to: msg_id,
                        messages: self.seen_messages.clone()
                    }
                })
            },
            Body::Broadcast { msg_id, message} => {
                self.seen_messages.insert(message);
                Ok(Message{
                    src: self.node_id.clone(),
                    dst: req.src.clone(),
                    body: Body::BroadcastOk {
                        in_reply_to: msg_id
                    }
                })
            }
            _ => bail!("Received a request with unknown type!"),
        }
    }
}

fn main() -> Result<()> {
    let mut stdin = std::io::stdin().lock();
    let mut buffer = String::new();
    stdin
        .read_line(&mut buffer)
        .expect("Failed reading init message");
    let init_msg: Message = serde_json::from_str(&buffer)?;
    let (mut service, init_ok) = Broadcaster::init(&init_msg)?;
    println!("{}", serde_json::to_string(&init_ok)?);

    for line in stdin.lines() {
        let line = line?;
        let req: Message = serde_json::from_str(&line)?;
        let res = service.process(req)?;
        println!("{}", serde_json::to_string(&res)?);
    }
    Ok(())
}
