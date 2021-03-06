# IronMQ

[![Crates.io][crates-badge]][crates-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/ironmq.svg
[crates-url]: https://crates.io/crates/ironmq
[actions-badge]: https://github.com/jonasrichard/ironmq/workflows/CI/badge.svg
[actions-url]: https://github.com/jonasrichard/ironmq/actions?query=workflow%3ACI

**Project is moved/renamed** go to [metalmq](https://github.com/jonasrichard/metalmq) repo

## Build and run

`ironmq` is under development, it is not feature complete but you can try and run with cargo run.

```bash
cargo run --bin ironmq
## or to enable logs
RUST_LOG=debug cargo run --bin ironmq
```

There are some examples in the `examples` directory, they implement simple scenarios of the
`ironmq-client` library. To run execute

```bash
cargo run --example publish-consume
cargo test
cd ironmq
cargo test --features integration-tests
```

for example.

### Checklist

* Connection
  * Login
    - [x] guest password
    - [ ] user management
* Channel
* Exchange
* Queue
* Basic

## AMQP 0.9 client library

We need a client to test the server, so for that in the `client091` folder I put the client implementation.

```bash
docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management

RUST_LOG=info cargo run
```

In order to validate AMQP packages we also need a stable AMQP client implementation which is the `pika`. It uses Python, so one need to install `pipenv` to run that.

```
cd ironmq-client
pipenv run bin/basic_publish.sh
```

## AMQP server

Installation later when a stable client is implemented.
