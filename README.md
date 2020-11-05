## Build and run

## AMQP 0.9 client library

We need a client to test the server, so for that in the `client091` folder I put the client implementation.

```bash
docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management

RUST_LOG=info cargo run
```

In order to validate AMQP packages we also need a stable AMQP client implementation which is the `pika`. It uses Python, so one need to install `pipenv` to run that.

```
cd client091
pipenv run bin/basic_publish.sh
```

## AMQP server

Installation later when a stable client is implemented.
