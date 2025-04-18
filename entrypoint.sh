#!/bin/bash
set -e

echo "Waiting for Bitcoin node to be available..."
for i in {1..60}; do
    if curl -s -u alice:password -d '{"jsonrpc":"1.0","method":"getblockchaininfo","params":[]}' -H 'content-type:text/plain;' http://bitcoin:18443/ | grep -q "blocks"; then
        echo "Bitcoin node is up and running!"
        # Add extra wait time for full initialization
        sleep 5
        break
    fi
    echo "Waiting for Bitcoin node to start... ($i/60)"
    sleep 2
    if [ $i -eq 60 ]; then
        echo "Timed out waiting for Bitcoin node!"
        exit 1
    fi
done

echo "Starting Bitcoin transaction demonstration..."
cargo run --release

exit 0