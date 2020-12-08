//! Data structures and converter functions for dealing with AMQP frames.
//!
//! All the data types are in the `frame` module, the `codec` implements
//! the encoding and the decoding.
pub mod codec;
pub mod frame;

use std::fmt;

/// Type alias for a sync and send error.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
/// Type alias for a simplified Result with Error.
pub type Result<T> = std::result::Result<T, Error>;

/// Error struct used by the crate.
#[derive(Debug)]
pub struct FrameError {
    pub code: u16,
    pub message: String,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl std::error::Error for FrameError {}

/// Shorthand for making errors with error code and error message.
///
/// ```no_run
/// use ironmq_codec::frame_error;
/// use ironmq_codec::FrameError;
/// use ironmq_codec::frame::AMQPValue;
///
/// fn as_string(val: AMQPValue) -> Result<String, Box<dyn std::error::Error>> {
///     if let AMQPValue::SimpleString(s) = val {
///         return Ok(s.clone())
///     }
///
///     frame_error!(10, "Value cannot be converted to string")
/// }
/// ```
#[macro_export]
macro_rules! frame_error {
    ($code:expr, $message:expr) => {
        ::std::result::Result::Err(Box::new($crate::FrameError {
            code: $code,
            message: ::std::string::String::from($message),
        }))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Buf, BufMut, BytesMut};
    use codec::AMQPCodec;
    use frame::{AMQPFrame, AMQPValue, MethodFrameArgs};
    use tokio_util::codec::Encoder;

    #[test]
    fn encode_header_frame() {
        let mut encoder = AMQPCodec {};
        let mut buf = BytesMut::with_capacity(1024);

        let res = encoder.encode(AMQPFrame::Header, &mut buf);

        assert!(res.is_ok());

        let expected = b"AMQP\x00\x00\x09\x01";
        let mut current = [0u8; 8];

        buf.copy_to_slice(&mut current[..]);

        assert_eq!(expected, &current);
    }

    #[test]
    fn encode_method_frame() {
        let mut encoder = AMQPCodec {};
        let mut buf = BytesMut::with_capacity(1024);

        let args = vec![
            AMQPValue::U8(0x9A),
            AMQPValue::U16(0x1122),
            AMQPValue::U32(0x55667788),
            AMQPValue::SimpleString("test".into()),
            AMQPValue::LongString("longtest".into()),
        ];
        let res = encoder.encode(
            AMQPFrame::Method(0x0205, 0x00FF00FE, MethodFrameArgs::Other(Box::new(args))),
            &mut buf,
        );

        assert!(res.is_ok());

        let frame_header = b"\x01\x02\x05";
        let class_method = b"\x00\xFF\x00\xFE";

        let mut argbuf = BytesMut::with_capacity(256);
        argbuf.put(&class_method[..]);
        argbuf.put(&b"\x9A"[..]);
        argbuf.put(&b"\x11\x22"[..]);
        argbuf.put(&b"\x55\x66\x77\x88"[..]);
        argbuf.put(&b"\x04test"[..]);
        argbuf.put(&b"\x00\x00\x00\x08longtest"[..]);

        let mut expected = BytesMut::with_capacity(256);
        expected.put(&frame_header[..]);
        expected.put_u32(argbuf.len() as u32);
        expected.put(argbuf);
        expected.put_u8(0xCE);

        assert_eq!(expected, buf);
    }
}
