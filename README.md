<div align="Center">
<h1>avail-light-bootstrap</h1>
<h3>Bootstrap Node for the Avail blockchain Light client</h3>
</div>

<br>

## Introduction

`avail-light-bootstrap` is called a Bootstrap node. Bootstrappers act as the initial point of contact for other Avail clients to find other peers in the network.

These network entry points alow other newly joined nodes to discover new peers and to connect to them.

To start a Bootstrap node, run:

```bash
cargo run -- -c config.yaml  
```

## Config reference

```yaml
# Set the Log Level
log_level = "info"
# If set to true, logs are displayed in JSON format, which is used for structured logging. Otherwise, plain text format is used (default: false).
log_format_json = false
# Secret key used to generate keypair. Can be either set to `seed` or to `key`.
# If set to seed, keypair will be generated from that seed.
# If set to key, a valid ed25519 private key must be provided, else the client will fail
# If `secret_key` is not set, random seed will be used.
secret_key = { seed="1" }
# P2P service port (default: 37000).
port = 3700
# Sets application-specific version of the protocol family used by the peer. (default: "/avail_kad/id/1.0.0")
identify_protocol = "/avail_kad/id/1.0.0"
# Sets agent version that is sent to peers. (default: "avail-light-client/rust-client")
identify_agent = "avail-light-client/rust-client"
# Sets the amount of time to keep Kademlia connections alive when they're idle. (default: 30s).
kad_connection_idle_timeout = 30
# Sets the timeout for a single Kademlia query. (default: 60s).
kad_query_timeout = 60
```
