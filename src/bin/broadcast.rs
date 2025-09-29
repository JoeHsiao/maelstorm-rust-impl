use anyhow::{Result};
use maelstrom_rust_impl::{Body, MaelstromNode, MaelstromNodeActions, MaelstromNodeId, Message};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct ExtraData {
    topology: HashMap<String, Vec<String>>,
    seen_messages: HashSet<serde_json::Value>,
}

type Broadcaster = MaelstromNode<ExtraData>;

fn main() -> Result<()> {
    let mut broadcaster = Broadcaster {
        node_id: MaelstromNodeId::Unassigned,
        next_res_id: 0,
        msg_handlers: HashMap::new(),
        extra_data: ExtraData {
            topology: HashMap::new(),
            seen_messages: HashSet::new(),
        },
    };

    broadcaster.handle("broadcast", |node, msg: Message| {
        if let Body::Broadcast { msg_id, message } = msg.body {
            node.extra_data.seen_messages.insert(message);
            Some(Ok(Message {
                src: node.node_id.as_str().expect("node id is not assigned"),
                dst: msg.src,
                body: Body::BroadcastOk {
                    in_reply_to: msg_id,
                },
            }))
        } else { None }
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
        } else { None }
    });
    broadcaster.handle("read", |node, msg: Message| {
        if let Body::Read { msg_id} = msg.body {
            Some(Ok(Message {
                src: node.node_id.as_str().expect("node id is not assigned"),
                dst: msg.src,
                body: Body::ReadOk {
                    in_reply_to: msg_id,
                    messages: node.extra_data.seen_messages.clone()
                },
            }))
        } else { None }
    });
    
    broadcaster.run()?;
    Ok(())
}
