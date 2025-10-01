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

trait BroadcasterActions {
    fn broadcast_to_neighbors(&mut self, msg: Message);
}

impl BroadcasterActions for Broadcaster {
    fn broadcast_to_neighbors(&mut self, msg: Message) {
        if let Body::ContinuousBroadcast { message, .. } = msg.body {
            let node_id = self.node_id.as_str().expect("node id is not assigned");

            self.extra_data
                .topology
                .get(&node_id)
                .expect("Cannot find node id in topology")
                .iter()
                .filter(|neighbor| **neighbor != msg.src)
                .cloned()
                .collect::<Vec<String>>()
                .iter()
                .for_each(|neighbor| {
                    let broadcast_msg = Message {
                        src: node_id.clone(),
                        dst: neighbor.into(),
                        body: Body::Broadcast {
                            msg_id: self.next_send_id,
                            message: message.clone(),
                        },
                    };
                    let _ = self.send(broadcast_msg);
                });
        }
    }
}
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

    broadcaster.handle("continuous_broadcast", |node, msg: Message| {
        node.broadcast_to_neighbors(msg);
        None
    });

    broadcaster.handle("broadcast", |node, msg: Message| {
        if let Body::Broadcast { msg_id, message } = msg.clone().body {
            let node_id = node.node_id.as_str().expect("node id is not assigned");

            if !node.extra_data.seen_messages.contains(&message) {
                thread::spawn({
                    let mut msg = msg.clone();
                    let message = message.clone();
                    msg.body = Body::ContinuousBroadcast { message };
                    move || {
                        loop {
                            sleep(Duration::from_millis(300));
                            println!(
                                "{}",
                                serde_json::to_string(&msg).expect("message cannot be serialized")
                            );
                        }
                    }});
            }

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
