# HTTPS Support

## Quick Start

1. Generate self-signed certificate:
```bash
./generate-cert.sh
```

2. Update `emukc.config.toml`:
```toml
bind = "127.0.0.1:8443"
tls_cert = "cert.pem"
tls_key = "key.pem"
```

3. Run server:
```bash
cargo run --bin emukcd serve
```

Server will start at `https://127.0.0.1:8443`

## Production

Use real certificates from Let's Encrypt or your CA:
```toml
tls_cert = "/path/to/fullchain.pem"
tls_key = "/path/to/privkey.pem"
```

## HTTP Mode

To disable HTTPS, remove or comment out `tls_cert` and `tls_key` in config.
