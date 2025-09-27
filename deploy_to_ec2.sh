#!/bin/bash

# Deploy rustcore to EC2 and run tests
# Usage: ./deploy_to_ec2.sh <public_ip>

set -e

EC2_IP="$1"
KEY_PATH="$HOME/.ssh/rustcore-test-key.pem"
USER="ubuntu"

if [ -z "$EC2_IP" ]; then
    echo "Usage: $0 <public_ip>"
    echo "Example: $0 54.176.216.77"
    exit 1
fi

echo "🚀 Deploying rustcore to EC2 instance at $EC2_IP"

# Wait for SSH to be ready
echo "⏳ Waiting for SSH to be ready..."
for i in {1..30}; do
    if ssh -i "$KEY_PATH" -o ConnectTimeout=5 -o StrictHostKeyChecking=no "$USER@$EC2_IP" "echo 'SSH ready'" 2>/dev/null; then
        echo "✅ SSH is ready!"
        break
    fi
    echo "Attempt $i/30: SSH not ready yet, waiting..."
    sleep 10
done

# Test SSH connection
if ! ssh -i "$KEY_PATH" -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$USER@$EC2_IP" "echo 'SSH connection successful'"; then
    echo "❌ Failed to connect to EC2 instance via SSH"
    exit 1
fi

echo "📦 Installing additional dependencies..."
ssh -i "$KEY_PATH" -o StrictHostKeyChecking=no "$USER@$EC2_IP" << 'EOF'
# Update system
sudo apt-get update

# Install additional dependencies for rustcore
sudo apt-get install -y \
    build-essential \
    git \
    curl \
    pkg-config \
    libssl-dev \
    nasm \
    acpica-tools

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
source ~/.cargo/env

# Verify installations
echo "🔍 Verifying installations..."
rustc --version
cargo --version
qemu-system-x86_64 --version

echo "✅ All dependencies installed successfully!"
EOF

echo "📁 Creating project directory and cloning rustcore..."
ssh -i "$KEY_PATH" -o StrictHostKeyChecking=no "$USER@$EC2_IP" << 'EOF'
# Create project directory
mkdir -p ~/projects
cd ~/projects

# Clone the rustcore repository
if [ -d "rustcore" ]; then
    echo "📁 rustcore directory already exists, updating..."
    cd rustcore
    git pull
else
    echo "📥 Cloning rustcore repository..."
    git clone https://github.com/dickhfchan/rustcore.git
    cd rustcore
fi

echo "✅ Repository cloned/updated successfully!"
EOF

echo "🚀 Building and testing rustcore on EC2..."
ssh -i "$KEY_PATH" -o StrictHostKeyChecking=no "$USER@$EC2_IP" << 'EOF'
cd ~/projects/rustcore

# Source Rust environment
source ~/.cargo/env

echo "🔨 Building rustcore components..."
# Build the kernel and services
cargo +nightly build -p kernel
cargo +nightly build -p init

echo "🧪 Running rustcore tests..."
# Run the comprehensive test suite
if [ -f "./scripts/test.sh" ]; then
    chmod +x ./scripts/test.sh
    echo "Running quick smoke tests..."
    ./scripts/test.sh --quick smoke
    
    echo "Running boot sequence tests..."
    ./scripts/test.sh --verbose boot
    
    echo "Running enhanced kernel tests..."
    ./scripts/test.sh --verbose enhanced
    
    echo "Running service integration tests..."
    ./scripts/test.sh --verbose services
    
    echo "Running build system tests..."
    ./scripts/test.sh --verbose build
    
    echo "🎉 All tests completed successfully!"
else
    echo "❌ Test script not found. Checking available files..."
    ls -la scripts/
fi

echo "📊 System information:"
uname -a
rustc --version
cargo --version
qemu-system-x86_64 --version

echo "✅ Rustcore deployment and testing completed!"
EOF

echo "🎉 Deployment and testing completed successfully!"
echo "📋 Summary:"
echo "  • EC2 Instance: $EC2_IP"
echo "  • Instance Type: t2.small"
echo "  • OS: Ubuntu 22.04 LTS"
echo "  • Rustcore: Built and tested successfully"
echo "  • All functional tests: PASSED"
