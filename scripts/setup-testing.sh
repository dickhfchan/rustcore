#!/usr/bin/env bash
set -euo pipefail

# Rustcore Testing Framework Setup Script
# This script sets up the testing environment and validates the deployment

echo "🚀 Setting up Rustcore Testing Framework..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to check and install dependencies
setup_dependencies() {
    echo -e "${BLUE}📦 Checking dependencies...${NC}"
    
    local missing_deps=()
    
    # Check Rust
    if ! command -v cargo >/dev/null 2>&1; then
        echo -e "${RED}❌ Rust/Cargo not found${NC}"
        echo "Please install Rust from https://rustup.rs/"
        missing_deps+=("rust")
    else
        echo -e "${GREEN}✅ Rust/Cargo found${NC}"
    fi
    
    # Check QEMU
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        echo -e "${RED}❌ QEMU not found${NC}"
        echo "Installing QEMU..."
        
        # Try to install QEMU based on OS
        if command -v brew >/dev/null 2>&1; then
            # macOS
            brew install qemu
        elif command -v apt-get >/dev/null 2>&1; then
            # Ubuntu/Debian
            sudo apt-get update && sudo apt-get install -y qemu-system-x86
        elif command -v yum >/dev/null 2>&1; then
            # CentOS/RHEL
            sudo yum install -y qemu-system-x86
        elif command -v pacman >/dev/null 2>&1; then
            # Arch Linux
            sudo pacman -S qemu-system-x86
        else
            echo -e "${YELLOW}⚠️  Please install QEMU manually${NC}"
            missing_deps+=("qemu")
        fi
    else
        echo -e "${GREEN}✅ QEMU found${NC}"
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing dependencies: ${missing_deps[*]}${NC}"
        echo "Please install the missing dependencies and run this script again."
        exit 1
    fi
}

# Function to setup Rust toolchain
setup_rust_toolchain() {
    echo -e "${BLUE}🦀 Setting up Rust toolchain...${NC}"
    
    # Install nightly toolchain
    if ! rustup toolchain list | grep -q nightly; then
        echo "Installing Rust nightly toolchain..."
        rustup toolchain install nightly
    else
        echo -e "${GREEN}✅ Rust nightly already installed${NC}"
    fi
    
    # Add rust-src component
    rustup component add rust-src --toolchain nightly
    
    echo -e "${GREEN}✅ Rust toolchain setup complete${NC}"
}

# Function to validate project structure
validate_project() {
    echo -e "${BLUE}🔍 Validating project structure...${NC}"
    
    local required_files=(
        "Cargo.toml"
        "kernel/Cargo.toml"
        "services/init/Cargo.toml"
        "targets/x86_64-rustcore.json"
        "scripts/run-qemu.sh"
        "scripts/test.sh"
    )
    
    local missing_files=()
    
    for file in "${required_files[@]}"; do
        if [ ! -f "$file" ]; then
            missing_files+=("$file")
        fi
    done
    
    if [ ${#missing_files[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing required files:${NC}"
        for file in "${missing_files[@]}"; do
            echo "  - $file"
        done
        exit 1
    fi
    
    echo -e "${GREEN}✅ Project structure validated${NC}"
}

# Function to make scripts executable
setup_scripts() {
    echo -e "${BLUE}📜 Setting up scripts...${NC}"
    
    local scripts=(
        "scripts/test.sh"
        "scripts/run-qemu.sh"
        "scripts/comprehensive_test.sh"
        "scripts/simple_test.sh"
    )
    
    for script in "${scripts[@]}"; do
        if [ -f "$script" ]; then
            chmod +x "$script"
            echo -e "${GREEN}✅ Made $script executable${NC}"
        fi
    done
}

# Function to run initial build
initial_build() {
    echo -e "${BLUE}🔨 Running initial build...${NC}"
    
    if cargo +nightly build; then
        echo -e "${GREEN}✅ Initial build successful${NC}"
    else
        echo -e "${RED}❌ Initial build failed${NC}"
        exit 1
    fi
}

# Function to run validation tests
run_validation_tests() {
    echo -e "${BLUE}🧪 Running validation tests...${NC}"
    
    echo "Running quick smoke test..."
    if ./scripts/test.sh --quick smoke; then
        echo -e "${GREEN}✅ Validation tests passed${NC}"
    else
        echo -e "${RED}❌ Validation tests failed${NC}"
        echo "Please check the output above for issues."
        exit 1
    fi
}

# Function to create test configuration
create_test_config() {
    echo -e "${BLUE}⚙️  Creating test configuration...${NC}"
    
    # Create test configuration directory
    mkdir -p .test-config
    
    # Create default test configuration
    cat > .test-config/default.toml << EOF
[test]
timeout = 30
verbose = false
quick = false
release = false

[qemu]
machine = "q35"
cpu = "qemu64"
memory = "512M"

[coverage]
enabled = false
output_dir = "coverage"
EOF
    
    echo -e "${GREEN}✅ Test configuration created${NC}"
}

# Function to display usage instructions
show_usage() {
    echo -e "${GREEN}🎉 Testing framework setup complete!${NC}"
    echo
    echo -e "${BLUE}Usage:${NC}"
    echo "  ./scripts/test.sh                 # Run all tests"
    echo "  ./scripts/test.sh --help          # Show help"
    echo "  ./scripts/test.sh --quick         # Run quick tests"
    echo "  ./scripts/test.sh --verbose       # Verbose output"
    echo "  ./scripts/test.sh ci              # Run CI tests"
    echo
    echo -e "${BLUE}Test types:${NC}"
    echo "  boot, smoke, enhanced, services, build, ci, all"
    echo
    echo -e "${BLUE}For more information:${NC}"
    echo "  See docs/qemu_testing_guide.md"
    echo "  See docs/qemu_test_results.md"
}

# Main setup function
main() {
    echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║                Rustcore Testing Framework Setup            ║${NC}"
    echo -e "${BLUE}║                      Version 1.0.0                         ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo
    
    setup_dependencies
    setup_rust_toolchain
    validate_project
    setup_scripts
    create_test_config
    initial_build
    run_validation_tests
    
    echo
    show_usage
}

# Run main function
main "$@"
