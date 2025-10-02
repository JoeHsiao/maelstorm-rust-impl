use anyhow::Result;
use maelstrom_rust_impl::{Body, MaelstromNode, MaelstromNodeActions, MaelstromNodeId, Message};
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

    broadcaster.handle("gossip", |node, msg: Message| {
        if let Body::Gossip { message } = msg.body {
            node.extra_data.seen_messages.extend(message);
        };
        None
    });
    broadcaster.handle("do_gossip", |node, msg: Message| {
        if let Body::DoGossip = msg.body {
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
        None
    });

    broadcaster.handle("broadcast", |node, msg: Message| {
        if let Body::Broadcast { msg_id, message } = msg.clone().body {
            let node_id = node.node_id.as_str().expect("node id is not assigned");
            node.extra_data.seen_messages.insert(message);
            Some(Ok(Message {
                src: node_id.clone(),
                dst: msg.src,
                body: Body::BroadcastOk {
                    in_reply_to: msg_id,
                },
            }))
        } else {
            None
        }
    });

    broadcaster.handle("topology", |node, msg: Message| {
        if let Body::Topology { msg_id, topology } = msg.body {
            node.extra_data.topology = topology;

            thread::spawn({
                let node_id = node.node_id.as_str().expect("node id is not assigned");
                move || {
                    loop {
                        sleep(Duration::from_millis(300));
                        let gossip_reminder = Message {
                            src: node_id.clone(),
                            dst: node_id.clone(),
                            body: Body::DoGossip,
                        };
                        println!("{}", serde_json::to_string(&gossip_reminder).expect("Failed to serialize gossip reminder"));
                    }
                }
            });

            Some(Ok(Message {
                src: node.node_id.as_str().expect("node id is not assigned"),
                dst: msg.src,
                body: Body::TopologyOk {
                    in_reply_to: msg_id,
                },
            }))
        } else {
            None
        }
    });
    broadcaster.handle("read", |node, msg: Message| {
        if let Body::Read { msg_id } = msg.body {
            Some(Ok(Message {
                src: node.node_id.as_str().expect("node id is not assigned"),
                dst: msg.src,
                body: Body::ReadOk {
                    in_reply_to: msg_id,
                    messages: node.extra_data.seen_messages.clone(),
                },
            }))
        } else {
            None
        }
    });

    broadcaster.run()?;
    Ok(())
}
