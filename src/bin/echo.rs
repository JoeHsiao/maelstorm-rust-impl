use anyhow::Result;
use maelstrom_rust_impl::{Body, MaelstromNode, MaelstromNodeActions, MaelstromNodeId, Message};
use std::collections::HashMap;

#[derive(Default)]
struct Empty;
type Echoer = MaelstromNode<Empty>;

fn main() -> Result<()> {
    let mut echoer = Echoer {
        node_id: MaelstromNodeId::Unassigned,
        next_res_id: 0,
        msg_handlers: HashMap::new(),
        extra_data: Empty,
    };

    echoer.handle("echo", |node, msg| {
        if let Body::Echo { msg_id, echo } = msg.body {
            Some(Ok(Message {
                src: node.node_id.as_str().unwrap(),
                dst: msg.src,
                body: Body::EchoOk {
                    msg_id: node.next_res_id,
                    in_reply_to: msg_id,
                    echo: echo.clone(),
                },
            }))
        } else {
            None
        }
    });
    echoer.run()?;
    Ok(())
}
