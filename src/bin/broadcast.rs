use anyhow::{Context, Result};
use maelstrom_rust_impl::{
    Body, Event, MaelstromNode, MaelstromNodeActions, MaelstromNodeId, Message,
};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

#[derive(Default)]
struct ExtraData {
    topology: HashMap<String, Vec<String>>,
    seen_messages: HashSet<serde_json::Value>,
}

type Broadcaster = MaelstromNode<ExtraData>;

fn main() -> Result<()> {
    let mut broadcaster = Broadcaster {
        node_id: MaelstromNodeId::Unassigned,
        next_send_id: 0,
        msg_handlers: HashMap::new(),
        extra_data: ExtraData {
            topology: HashMap::new(),
            seen_messages: HashSet::new(),
        },
    };

    broadcaster.handle("gossip", |node, event: Event| {
        if let Event::Message(msg) = event {
            if let Body::Gossip { message } = msg.body {
                node.extra_data.seen_messages.extend(message);
            };
            return None;
        }
        None
    });

    broadcaster.handle("do_gossip", |node, event: Event| {
        if let Event::Action(name) = event {
            if name == "do_gossip" {
                let node_id = node.node_id.as_str().expect("node_id is not assigned");
                let neighbors = node
                    .extra_data
                    .topology
                    .clone()
                    .remove(&node_id)
                    .expect(&format!("topology missing entry for node id {}", node_id));
                neighbors.iter().for_each(|n| {
                    let gossip = Message {
                        src: node_id.clone(),
                        dst: n.clone(),
                        body: Body::Gossip {
                            message: node.extra_data.seen_messages.clone(),
                        },
                    };
                    let _ = node.send(gossip);
                })
            }
            return None;
        }
        None
    });

    broadcaster.handle("broadcast", |node, event: Event| {
        if let Event::Message(msg) = event {
            if let Body::Broadcast { msg_id, message } = msg.clone().body {
                let node_id = node.node_id.as_str().expect("node id is not assigned");
                node.extra_data.seen_messages.insert(message);
                return Some(Ok(Message {
                    src: node_id.clone(),
                    dst: msg.src,
                    body: Body::BroadcastOk {
                        in_reply_to: msg_id,
                    },
                }));
            } else {
                return None;
            }
        }
        None
    });

    broadcaster.handle("topology", |node, event: Event| {
        if let Event::Message(msg) = event {
            if let Body::Topology { msg_id, topology } = msg.body {
                node.extra_data.topology = topology;
                return Some(Ok(Message {
                    src: node.node_id.as_str().expect("node id is not assigned"),
                    dst: msg.src,
                    body: Body::TopologyOk {
                        in_reply_to: msg_id,
                    },
                }));
            } else {
                return None;
            }
        }
        None
    });
    broadcaster.handle("read", |node, event: Event| {
        if let Event::Message(msg) = event {
            if let Body::Read { msg_id } = msg.body {
                return Some(Ok(Message {
                    src: node.node_id.as_str().expect("node id is not assigned"),
                    dst: msg.src,
                    body: Body::ReadOk {
                        in_reply_to: msg_id,
                        messages: node.extra_data.seen_messages.clone(),
                    },
                }));
            } else {
                return None;
            }
        };
        None
    });

    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn({
        let sender = sender.clone();
        move || -> Result<()> {
            loop {
                sleep(Duration::from_millis(300));
                sender
                    .send(Event::Action("do_gossip".into()))
                    .context("Cannot send do_gossip")?;
            }
        }
    });
    broadcaster.run(sender, receiver)?;
    Ok(())
}
