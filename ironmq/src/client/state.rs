use crate::{Context, Result, RuntimeError};
use crate::exchange;
use crate::message;
use ironmq_codec::frame::{self, AMQPFrame, Channel};
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub(crate) type MaybeFrame = Result<Option<AMQPFrame>>;

pub(crate) const NOT_FOUND: u16 = 404;
pub(crate) const PRECONDITION_FAILED: u16 = 406;
pub(crate) const CHANNEL_ERROR: u16 = 504;
pub(crate) const NOT_ALLOWED: u16 = 530;

/// All the transient data of a connection are stored here.
pub(crate) struct ConnectionState {
    context: Arc<Mutex<Context>>,
    open_channels: HashMap<Channel, ()>,
    exchanges: HashMap<String, mpsc::Sender<message::Message>>,
    queues: HashMap<String, ()>,
    /// Simple exchange-queue binding
    binding: HashMap<(String, String), ()>,
    in_flight_contents: HashMap<Channel, PublishedContent>
}

#[derive(Debug)]
struct PublishedContent {
    channel: Channel,
    exchange: String,
    length: Option<u64>,
    content: Option<Vec<u8>>
}

#[async_trait]
pub(crate) trait Connection: Sync + Send {
    async fn connection_open(&self, channel: Channel, args: frame::ConnectionOpenArgs) -> MaybeFrame;
    async fn connection_close(&self, args: frame::ConnectionCloseArgs) -> MaybeFrame;
    async fn channel_open(&mut self, channel: Channel) -> MaybeFrame;
    async fn channel_close(&mut self, channel: Channel, args: frame::ChannelCloseArgs) -> MaybeFrame;
    async fn exchange_declare(&mut self, channel: Channel, args: frame::ExchangeDeclareArgs) -> MaybeFrame;
    async fn queue_declare(&mut self, channel: Channel, args: frame::QueueDeclareArgs,) -> MaybeFrame;
    async fn queue_bind(&mut self, channel: Channel, args: frame::QueueBindArgs,) -> MaybeFrame;
    async fn basic_publish(&mut self, channel: Channel, args: frame::BasicPublishArgs) -> MaybeFrame;
    async fn basic_consume(&mut self, channel: Channel, args: frame::BasicConsumeArgs) -> MaybeFrame;
    async fn receive_content_header(&mut self, header: frame::ContentHeaderFrame) -> MaybeFrame;
    async fn receive_content_body(&mut self, body: frame::ContentBodyFrame) -> MaybeFrame;
}

pub(crate) fn new(context: Arc<Mutex<Context>>) -> Box<dyn Connection> {
    Box::new(ConnectionState {
        context: context,
        open_channels: HashMap::new(),
        exchanges: HashMap::new(),
        queues: HashMap::new(),
        binding: HashMap::new(),
        in_flight_contents: HashMap::new()
    })
}

#[async_trait]
impl Connection for ConnectionState {
    async fn connection_open(&self, channel: Channel, args: frame::ConnectionOpenArgs) -> MaybeFrame {
        if args.virtual_host != "/" {
            connection_error(NOT_ALLOWED, "Cannot connect to virtualhost", frame::CONNECTION_OPEN)
        } else {
            Ok(Some(frame::connection_open_ok(channel)))
        }
    }

    async fn connection_close(&self, args: frame::ConnectionCloseArgs) -> MaybeFrame {
        // TODO cleanup
        Ok(Some(frame::connection_close_ok(0)))
    }

    async fn channel_open(&mut self, channel: Channel) -> MaybeFrame {
        if self.open_channels.contains_key(&channel) {
            channel_error(channel, CHANNEL_ERROR, "Channel already opened", frame::CHANNEL_OPEN)
        } else {
            self.open_channels.insert(channel, ());
            Ok(Some(frame::channel_open_ok(channel)))
        }
    }

    async fn channel_close(&mut self, channel: Channel, args: frame::ChannelCloseArgs) -> MaybeFrame {
        self.open_channels.remove(&channel);
        Ok(Some(frame::channel_close_ok(channel)))
    }

    async fn exchange_declare(&mut self, channel: Channel, args: frame::ExchangeDeclareArgs) -> MaybeFrame {
        let no_wait = args.flags.contains(frame::ExchangeDeclareFlags::NO_WAIT);
        let mut ctx = self.context.lock().await;
        let result = exchange::declare(&mut ctx.exchanges, &args).await;

        match result {
            Ok(ch) => {
                self.exchanges.insert(args.exchange_name.clone(), ch);

                if no_wait {
                    Ok(None)
                } else {
                    Ok(Some(frame::exchange_declare_ok(channel)))
                }
            }
            Err(e) => match e.downcast::<RuntimeError>() {
                Ok(mut rte) => {
                    rte.channel = channel;

                    Ok(Some(AMQPFrame::from(*rte)))
                },
                Err(e2) => Err(e2),
            },
        }
    }

    async fn queue_declare(&mut self, channel: Channel, args: frame::QueueDeclareArgs,) -> MaybeFrame {
        if !self.queues.contains_key(&args.name) {
            self.queues.insert(args.name.clone(), ());
        }

        Ok(Some(frame::queue_declare_ok(channel, args.name, 0, 0)))
    }

    async fn queue_bind(&mut self, channel: Channel, args: frame::QueueBindArgs,) -> MaybeFrame {
        let binding = (args.exchange_name, args.queue_name);

        if !self.binding.contains_key(&binding) {
            self.binding.insert(binding, ());
        }

        Ok(Some(frame::queue_bind_ok(channel)))
    }

    async fn basic_publish(&mut self, channel: Channel, args: frame::BasicPublishArgs) -> MaybeFrame {
        if !self.exchanges.contains_key(&args.exchange_name) {
            channel_error(channel, NOT_FOUND, "Exchange not found", frame::BASIC_PUBLISH)
        } else {
            // TODO check if there is in flight content in the channel -> error
            self.in_flight_contents.insert(channel, PublishedContent {
                channel: channel,
                exchange: args.exchange_name,
                length: None,
                content: None
            });

            Ok(None)
        }
    }

    async fn basic_consume(&mut self, channel: Channel, args: frame::BasicConsumeArgs) -> MaybeFrame {
        Ok(None)
    }

    async fn receive_content_header(&mut self, header: frame::ContentHeaderFrame) -> MaybeFrame {
        // TODO collect info into a data struct
        info!("Receive content with length {}", header.body_size);

        if let Some(pc) = self.in_flight_contents.get_mut(&header.channel) {
            pc.length = Some(header.body_size);
        }

        Ok(None)
    }

    async fn receive_content_body(&mut self, body: frame::ContentBodyFrame) -> MaybeFrame {
        info!("Receive content with length {}", body.body.len());

        if let Some(pc) = self.in_flight_contents.remove(&body.channel) {
            let msg = message::Message {
                content: body.body,
                processed: None
            };

            match self.exchanges.get(&pc.exchange) {
                Some(ch) => {
                    ch.send(msg).await;
                    Ok(None)
                },
                None =>
                    // TODO error, exchange cannot be found
                    Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

fn channel_error(channel: Channel, code: u16, text: &str, cm_id: u32) -> MaybeFrame {
    let (cid, mid) = frame::split_class_method(cm_id);

    Ok(Some(frame::channel_close(
                channel,
                code,
                text,
                cid,
                mid)))
}

fn connection_error(code: u16, text: &str, cm_id: u32) -> MaybeFrame {
    let (cid, mid) = frame::split_class_method(cm_id);

    Ok(Some(frame::connection_close(
                0,
                code,
                text,
                cid,
                mid)))
}
