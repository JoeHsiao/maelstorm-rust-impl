use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::io::BufRead;

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
}
struct Echoer {
    node_id: String,
    incr_id: u64,
}

impl Echoer {
    fn init(req: &Message) -> Result<(Self, Message)> {
        match &req.body {
            Body::Init {
                msg_id,
                node_id,
                node_ids: _,
            } => {
                let echoer = Echoer {
                    node_id: node_id.clone(),
                    incr_id: 0,
                };
                Ok((
                    echoer,
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
    fn echo(&mut self, req: &Message) -> Result<Message> {
        match &req.body {
            Body::Echo { msg_id, echo } => {
                let msg = Message {
                    src: self.node_id.clone(),
                    dst: req.src.clone(),
                    body: Body::EchoOk {
                        msg_id: self.incr_id,
                        in_reply_to: *msg_id,
                        echo: echo.clone(),
                    },
                };
                self.incr_id += 1;
                Ok(msg)
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
    let (mut echoer, init_ok) = Echoer::init(&init_msg)?;
    println!("{}", serde_json::to_string(&init_ok)?);

    for line in stdin.lines() {
        let line = line?;
        let req: Message = serde_json::from_str(&line)?;
        let res = echoer.echo(&req)?;
        println!("{}", serde_json::to_string(&res)?);
    }
    Ok(())
}
