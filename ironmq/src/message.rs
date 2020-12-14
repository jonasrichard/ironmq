//! Messages are sent to exhchanges and forwarded to queues. There is a
//! possibility to state that a message is processed via an oneshot channel.

pub(crate) type MessageId = String

pub(crate) struct Message {
    content: Vec<u8>,
    processed: Option<oneshot::Channel<MessageId>>
}
