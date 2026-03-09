#!/bin/bash
# Generate self-signed certificate for HTTPS development

CONFIG="${1:-emukc.config.toml}"
WORKSPACE=$(grep '^workspace_root' "$CONFIG" | cut -d'"' -f2 | sed 's/^"\(.*\)"$/\1/')

if [ -z "$WORKSPACE" ]; then
  echo "Error: Cannot find workspace_root in $CONFIG"
  exit 1
fi

mkdir -p "$WORKSPACE"

openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout "$WORKSPACE/key.pem" \
  -out "$WORKSPACE/cert.pem" \
  -days 365 \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

echo "✓ Generated $WORKSPACE/cert.pem and $WORKSPACE/key.pem"
echo "Add to your config:"
echo "tls_cert = \"$WORKSPACE/cert.pem\""
echo "tls_key = \"$WORKSPACE/key.pem\""
