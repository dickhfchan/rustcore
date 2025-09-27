#!/usr/bin/env bash
set -euo pipefail

# Simple QEMU Testing Script for Rustcore
# Focuses on validating the core functionality

echo "=== Rustcore QEMU Testing ==="
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

# Function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_patterns="$3"
    
    echo -e "${BLUE}Running: $test_name${NC}"
    
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
                echo -e "${GREEN}✓ Found: $pattern${NC}"
                patterns_found=$((patterns_found + 1))
            else
                echo -e "${RED}✗ Missing: $pattern${NC}"
            fi
        fi
    done
    
    # Determine test result
    if [ $exit_code -eq 0 ] && [ $patterns_found -eq $total_patterns ]; then
        echo -e "${GREEN}✓ PASSED: $test_name${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}✗ FAILED: $test_name${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    
    echo "----------------------------------------"
    echo
}

# Check if QEMU is available
if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo -e "${RED}Error: qemu-system-x86_64 not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓ QEMU is available${NC}"

# Build components
echo -e "${BLUE}Building components...${NC}"
cargo +nightly build
cargo +nightly build -p init
cargo +nightly build --tests -p kernel
echo -e "${GREEN}✓ Build successful${NC}"
echo

# Find boot_smoke binary
BOOT_SMOKE_BINARY=$(find target/x86_64-rustcore/debug/deps -name "boot_smoke-*" -type f | while read file; do [ -x "$file" ] && echo "$file"; done | head -1)
if [ -z "$BOOT_SMOKE_BINARY" ]; then
    echo -e "${RED}Error: No boot_smoke test binary found${NC}"
    exit 1
fi
echo -e "${BLUE}Using: $BOOT_SMOKE_BINARY${NC}"
echo

# Test 1: Basic Kernel Boot
run_test "Basic Kernel Boot" \
    "./scripts/run-qemu.sh" \
    "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"

# Test 2: Boot Smoke Test
run_test "Boot Smoke Test" \
    "./scripts/run-qemu.sh $BOOT_SMOKE_BINARY" \
    "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"

# Test 3: Service Components Validation
echo -e "${BLUE}Validating service components...${NC}"

if [ -f "services/init/bootfs/services.manifest" ]; then
    echo -e "${GREEN}✓ Service manifest exists${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗ Service manifest missing${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

if [ -f "services/init/bootfs/system.toml" ]; then
    echo -e "${GREEN}✓ System configuration exists${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗ System configuration missing${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

# Test 4: Kernel Size Validation
if [ -f "target/x86_64-rustcore/debug/kernel" ]; then
    KERNEL_SIZE=$(stat -f%z "target/x86_64-rustcore/debug/kernel" 2>/dev/null || stat -c%s "target/x86_64-rustcore/debug/kernel" 2>/dev/null || echo "unknown")
    echo "Kernel size: $KERNEL_SIZE bytes"
    
    if [ "$KERNEL_SIZE" != "unknown" ] && [ "$KERNEL_SIZE" -lt 5242880 ]; then # < 5MB
        echo -e "${GREEN}✓ Kernel size is reasonable (< 5MB)${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${YELLOW}⚠ Kernel size is large or unknown${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
fi

# Test 5: Memory Layout Check
echo -e "${BLUE}Checking memory layout...${NC}"
if command -v objdump >/dev/null 2>&1; then
    echo "Kernel sections:"
    objdump -h target/x86_64-rustcore/debug/kernel 2>/dev/null | head -10 || echo "objdump not available"
    echo -e "${GREEN}✓ Memory layout check completed${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${YELLOW}⚠ objdump not available, skipping memory layout check${NC}"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi

echo "========================================"
echo -e "${BLUE}TEST SUMMARY${NC}"
echo "========================================"
TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
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
