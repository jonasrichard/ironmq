extern crate ironmq_client;

use crate::ironmq_client as client;

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn channel_close_on_not_existing_exchange() -> client::Result<()> {
    let c = client::connect("127.0.0.1:5672".to_string()).await?;

    let mut flags = ironmq_codec::frame::ExchangeDeclareFlags::empty();
    flags |= ironmq_codec::frame::ExchangeDeclareFlags::PASSIVE;

    client::exchange_declare(&c, 1, "sure do not exist", "fanout", Some(flags)).await?;

    client::close(&c).await?;

    Ok(())
}