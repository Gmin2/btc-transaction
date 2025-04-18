#!/bin/bash
set -e

echo "Starting Bitcoin Transaction Chain Demonstration"

# Make sure docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "docker-compose is not installed. Please install it first."
    exit 1
fi

# Stop any existing containers from previous runs
echo "Cleaning up any existing containers..."
docker-compose down -v 2>/dev/null || true

# Start Bitcoin node in background
echo "Starting Bitcoin node..."
docker-compose up -d bitcoin

# Wait a moment for node to initialize
echo "Waiting for Bitcoin node to initialize..."
sleep 5

echo "Running Bitcoin transaction demo..."
docker-compose up rust-app

echo "Demonstration complete. Cleaning up..."
docker-compose down -v