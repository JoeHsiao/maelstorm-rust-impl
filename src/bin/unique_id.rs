use std::collections::HashMap;
use anyhow::Result;
use maelstrom_rust_impl::{Message, Body, MaelstromNode, MaelstromNodeId, MaelstromNodeActions, Event};
#[derive(Default)]
struct Empty {}
type GUIDGenerator = MaelstromNode<Empty>;

fn main() -> Result<()> {
    let mut id_generator = GUIDGenerator {
        node_id: MaelstromNodeId::Unassigned,
        next_send_id: 0,
        msg_handlers: HashMap::new(),
        extra_data: Empty{},
    };

    id_generator.handle("generate", |node, event| {
        if let Event::Message(msg) = event {
            if let Body::Generate { msg_id } = msg.body {
                return Some(Ok(Message {
                    src: node.node_id.as_str().unwrap(),
                    dst: msg.src,
                    body: Body::GenerateOk {
                        in_reply_to: msg_id,
                        id: format!("{}_{}", node.node_id.as_str().unwrap(), node.next_send_id)
                    },
                }));
            } else {
                return None;
            }
        }
        None
    });

    let (sender, receiver) = std::sync::mpsc::channel();
    id_generator.run(sender, receiver)?;
    Ok(())
}
