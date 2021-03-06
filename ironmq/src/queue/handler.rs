use crate::message::Message;
use ironmq_codec::frame;
use log::{debug, error};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

pub(crate) type QueueCommandSink = mpsc::Sender<QueueCommand>;
pub(crate) type FrameSink = mpsc::Sender<frame::AMQPFrame>;
//pub(crate) type FrameStream = mpsc::Receiver<frame::AMQPFrame>;

#[derive(Debug)]
pub(crate) enum QueueCommand {
    Message(Message),
    Consume{ consumer_tag: String, frame_sink: FrameSink, response: oneshot::Sender<()> },
    Cancel{ consumer_tag: String, response: oneshot::Sender<()> }
}

pub(crate) async fn queue_loop(commands: &mut mpsc::Receiver<QueueCommand>) {
    let mut consumers = HashMap::<String, FrameSink>::new();

    while let Some(command) = commands.recv().await {
        match command {
            QueueCommand::Message(message) => {
                let frames = vec![
                    frame::basic_deliver(1, "ctag".into(), 0, false, "exchange".into(), "rkey".into()),
                    frame::AMQPFrame::ContentHeader(frame::content_header(1, message.content.len() as u64)),
                    frame::AMQPFrame::ContentBody(frame::content_body(1, message.content.as_slice())),
                ];

                'consumer: for (_, consumer) in &consumers {
                    for f in &frames {
                        debug!("Sending frame {:?}", f);

                        if let Err(e) = consumer.send(f.clone()).await {
                            error!("Message send error {:?}", e);
                            break 'consumer;
                        }
                    }
                }
            },
            QueueCommand::Consume{ consumer_tag, frame_sink, response } => {
                consumers.insert(consumer_tag, frame_sink);

                if let Err(e) = response.send(()) {
                    error!("Send error {:?}", e);
                }
            },
            QueueCommand::Cancel{ consumer_tag, response } => {
                consumers.remove(&consumer_tag);

                if let Err(e) = response.send(()) {
                    error!("Send error {:?}", e);
                }
            }
        }
    }
}
