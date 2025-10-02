use anyhow::Result;
use maelstrom_rust_impl::{Body, Event, MaelstromNode, MaelstromNodeActions, MaelstromNodeId, Message};
use std::collections::HashMap;

#[derive(Default)]
struct Empty;
type Echoer = MaelstromNode<Empty>;

fn main() -> Result<()> {
    let mut echoer = Echoer {
        node_id: MaelstromNodeId::Unassigned,
        next_send_id: 0,
        msg_handlers: HashMap::new(),
        extra_data: Empty,
    };

    echoer.handle("echo", |node, event| {
        if let Event::Message(msg) = event {
            if let Body::Echo { msg_id, echo } = msg.body {
                return Some(Ok(Message {
                    src: node.node_id.as_str().unwrap(),
                    dst: msg.src,
                    body: Body::EchoOk {
                        msg_id: node.next_send_id,
                        in_reply_to: msg_id,
                        echo: echo.clone(),
                    },
                }));
            } else {
                return None;
            }
        }
        None
    });
    let (sender, receiver) = std::sync::mpsc::channel();
    echoer.run(sender, receiver)?;
    Ok(())
}
