#!/usr/bin/env bash
set -euo pipefail

# Comprehensive QEMU Testing Script for Rustcore
# This script validates the complete boot sequence and system functionality

echo "=== Rustcore Comprehensive QEMU Testing ==="
echo "Timestamp: $(date)"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Function to run a test and capture results
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_patterns="$3"
    
    echo -e "${BLUE}Running Test: $test_name${NC}"
    echo "Command: $test_command"
    echo
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Run the test and capture output
    local output
    local exit_code
    output=$(eval "$test_command" 2>&1 || echo "EXIT_CODE:$?")
    exit_code=$?
    
    # Check if we got an exit code from the test
    if echo "$output" | grep -q "EXIT_CODE:"; then
        exit_code=$(echo "$output" | grep "EXIT_CODE:" | cut -d: -f2)
        output=$(echo "$output" | grep -v "EXIT_CODE:")
    fi
    
    echo "Exit Code: $exit_code"
    echo "Output:"
    echo "$output"
    echo
    
    # Check for expected patterns
    local patterns_found=0
    local total_patterns=0
    
    IFS=',' read -ra PATTERNS <<< "$expected_patterns"
    for pattern in "${PATTERNS[@]}"; do
        pattern=$(echo "$pattern" | xargs) # trim whitespace
        if [ -n "$pattern" ]; then
            total_patterns=$((total_patterns + 1))
            if echo "$output" | grep -q "$pattern"; then
                echo -e "${GREEN}✓ Found expected pattern: $pattern${NC}"
                patterns_found=$((patterns_found + 1))
            else
                echo -e "${RED}✗ Missing expected pattern: $pattern${NC}"
            fi
        fi
    done
    
    # Determine test result
    if [ $exit_code -eq 0 ] && [ $patterns_found -eq $total_patterns ]; then
        echo -e "${GREEN}✓ Test PASSED: $test_name${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}✗ Test FAILED: $test_name${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    echo "----------------------------------------"
    echo
}

# Function to check if QEMU is available
check_qemu() {
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        echo -e "${RED}Error: qemu-system-x86_64 not found${NC}"
        echo "Please install QEMU to run tests"
        exit 1
    fi
    echo -e "${GREEN}✓ QEMU is available${NC}"
}

# Function to build required components
build_components() {
    echo -e "${BLUE}Building rustcore components...${NC}"
    
    echo "Building kernel..."
    cargo +nightly build
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Kernel build successful${NC}"
    else
        echo -e "${RED}✗ Kernel build failed${NC}"
        exit 1
    fi
    
    echo "Building init service..."
    cargo +nightly build -p init
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Init service build successful${NC}"
    else
        echo -e "${RED}✗ Init service build failed${NC}"
        exit 1
    fi
    
    echo "Building test binaries..."
    cargo +nightly build --tests -p kernel
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Test binaries build successful${NC}"
    else
        echo -e "${RED}✗ Test binaries build failed${NC}"
        exit 1
    fi
    
    echo
}

# Function to find the latest boot_smoke test binary
find_latest_boot_smoke() {
    local latest_binary
    # Find executable files (on macOS, check for files with execute permission)
    latest_binary=$(find target/x86_64-rustcore/debug/deps -name "boot_smoke-*" -type f | while read file; do [ -x "$file" ] && echo "$file"; done | head -1)
    if [ -z "$latest_binary" ]; then
        echo -e "${RED}Error: No boot_smoke test binary found${NC}"
        exit 1
    fi
    echo "$latest_binary"
}

# Main test execution
main() {
    echo "Checking prerequisites..."
    check_qemu
    echo
    
    build_components
    
    local boot_smoke_binary
    boot_smoke_binary=$(find_latest_boot_smoke)
    echo -e "${BLUE}Using boot_smoke binary: $boot_smoke_binary${NC}"
    echo
    
    # Test 1: Basic Kernel Boot
    run_test "Basic Kernel Boot" \
        "./scripts/run-qemu.sh" \
        "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"
    
    # Test 2: Boot Smoke Test
    run_test "Boot Smoke Test" \
        "./scripts/run-qemu.sh $boot_smoke_binary" \
        "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"
    
    # Test 3: Release Build Test
    echo -e "${BLUE}Building release version...${NC}"
    cargo +nightly build --release
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Release build successful${NC}"
        
        run_test "Release Build Test" \
            "./scripts/run-qemu.sh --release" \
            "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"
    else
        echo -e "${RED}✗ Release build failed${NC}"
    fi
    
    # Test 4: Service Validation
    echo -e "${BLUE}Validating service components...${NC}"
    
    # Check if bootfs files exist
    if [ -f "services/init/bootfs/services.manifest" ]; then
        echo -e "${GREEN}✓ Service manifest exists${NC}"
    else
        echo -e "${RED}✗ Service manifest missing${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi
    
    if [ -f "services/init/bootfs/system.toml" ]; then
        echo -e "${GREEN}✓ System configuration exists${NC}"
    else
        echo -e "${RED}✗ System configuration missing${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi
    
    # Test 5: Memory Layout Validation
    echo -e "${BLUE}Validating memory layout...${NC}"
    
    if [ -f "target/x86_64-rustcore/debug/kernel" ]; then
        local kernel_size
        kernel_size=$(stat -f%z "target/x86_64-rustcore/debug/kernel" 2>/dev/null || stat -c%s "target/x86_64-rustcore/debug/kernel" 2>/dev/null || echo "unknown")
        echo "Kernel size: $kernel_size bytes"
        
        if [ "$kernel_size" != "unknown" ] && [ "$kernel_size" -lt 2097152 ]; then # < 2MB
            echo -e "${GREEN}✓ Kernel size is reasonable (< 2MB)${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            echo -e "${YELLOW}⚠ Kernel size is large or unknown${NC}"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi
    
    # Print final results
    echo "========================================"
    echo -e "${BLUE}TEST SUMMARY${NC}"
    echo "========================================"
    echo "Total Tests: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
    echo
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}🎉 ALL TESTS PASSED! 🎉${NC}"
        echo "Rustcore is functioning correctly in QEMU"
        exit 0
    else
        echo -e "${RED}❌ SOME TESTS FAILED ❌${NC}"
        echo "Please review the failed tests above"
        exit 1
    fi
}

# Run main function
main "$@"
