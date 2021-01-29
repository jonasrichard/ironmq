use crate::message::Message;
use ironmq_codec::frame;
use log::{debug, error};
use tokio::sync::{mpsc, oneshot};

pub(crate) type QueueCommandSink = mpsc::Sender<QueueCommand>;
pub(crate) type FrameSink = mpsc::Sender<frame::AMQPFrame>;
//pub(crate) type FrameStream = mpsc::Receiver<frame::AMQPFrame>;

#[derive(Debug)]
pub(crate) enum QueueCommand {
    Message(Message),
    Consume{ frame_sink: FrameSink, response: oneshot::Sender<()> }
}

pub(crate) async fn queue_loop(commands: &mut mpsc::Receiver<QueueCommand>) {
    let mut consumers = Vec::<FrameSink>::new();

    while let Some(command) = commands.recv().await {
        match command {
            QueueCommand::Message(message) => {
                let frames = vec![
                    frame::basic_deliver(1, "ctag".into(), 0, false, "exchange".into(), "rkey".into()),
                    frame::AMQPFrame::ContentHeader(frame::content_header(1, message.content.len() as u64)),
                    frame::AMQPFrame::ContentBody(frame::content_body(1, message.content.as_slice())),
                ];

                for consumer in &consumers {
                    for f in &frames {
                        debug!("Sending frame {:?}", f);

                        if let Err(e) = consumer.send(f.clone()).await {
                            error!("Message send error {:?}", e);
                        }
                    }
                }
            },
            QueueCommand::Consume{ frame_sink, response } => {
                consumers.push(frame_sink);

                if let Err(e) = response.send(()) {
                    error!("Send error {:?}", e);
                }
            }
        }
    }
}
