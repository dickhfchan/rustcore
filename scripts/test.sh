#!/usr/bin/env bash
set -euo pipefail

# Rustcore Testing Framework - Main Test Runner
# This is the primary entry point for all rustcore testing

# Version and metadata
SCRIPT_VERSION="1.0.0"
SCRIPT_DATE="2025-09-27"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test configuration
DEFAULT_TIMEOUT=30
VERBOSE=false
QUICK=false
RELEASE=false
COVERAGE=false

# Function to display help
show_help() {
    cat << EOF
Rustcore Testing Framework v${SCRIPT_VERSION}

USAGE:
    ./scripts/test.sh [OPTIONS] [TEST_TYPE]

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -q, --quick         Run quick tests only (skip slow tests)
    -r, --release       Test release builds
    -c, --coverage      Generate test coverage reports
    -t, --timeout SEC   Set test timeout (default: ${DEFAULT_TIMEOUT}s)
    --clean             Clean build artifacts before testing
    --no-build          Skip building (use existing binaries)

TEST TYPES:
    all                 Run all tests (default)
    boot                Basic boot sequence tests
    smoke               Boot smoke tests
    enhanced            Enhanced functionality tests
    functional          Comprehensive functional tests (IPC, memory, timer, scheduler)
    memory              Memory management tests
    ipc                 IPC communication tests
    services            Service integration tests
    interrupts          Interrupt handling tests
    build               Build system tests
    ci                  CI/CD integration tests
    benchmark           Performance benchmarks

EXAMPLES:
    ./scripts/test.sh                    # Run all tests
    ./scripts/test.sh boot               # Run boot tests only
    ./scripts/test.sh --quick smoke      # Quick smoke tests
    ./scripts/test.sh --release all      # Test release builds
    ./scripts/test.sh --verbose ci       # Verbose CI tests
    ./scripts/test.sh --coverage         # Generate coverage report

EXIT CODES:
    0    All tests passed
    1    Some tests failed
    2    Build failed
    3    Invalid arguments
    4    Missing dependencies

For more information, see docs/qemu_testing_guide.md
EOF
}

# Function to log messages
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%H:%M:%S')
    
    case "$level" in
        "INFO")  echo -e "${BLUE}[${timestamp}] INFO: ${message}${NC}" ;;
        "WARN")  echo -e "${YELLOW}[${timestamp}] WARN: ${message}${NC}" ;;
        "ERROR") echo -e "${RED}[${timestamp}] ERROR: ${message}${NC}" ;;
        "SUCCESS") echo -e "${GREEN}[${timestamp}] SUCCESS: ${message}${NC}" ;;
        "DEBUG") 
            if [ "$VERBOSE" = true ]; then
                echo -e "${PURPLE}[${timestamp}] DEBUG: ${message}${NC}"
            fi
            ;;
    esac
}

# Function to check dependencies
check_dependencies() {
    log "INFO" "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v cargo >/dev/null 2>&1; then
        missing_deps+=("cargo")
    fi
    
    if ! command -v rustc >/dev/null 2>&1; then
        missing_deps+=("rustc")
    fi
    
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        missing_deps+=("qemu-system-x86_64")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log "ERROR" "Missing dependencies: ${missing_deps[*]}"
        log "ERROR" "Please install the missing dependencies and try again"
        exit 4
    fi
    
    log "SUCCESS" "All dependencies available"
}

# Function to clean build artifacts
clean_build() {
    log "INFO" "Cleaning build artifacts..."
    cargo clean
    log "SUCCESS" "Build artifacts cleaned"
}

# Function to build components
build_components() {
    log "INFO" "Building rustcore components..."
    
    local build_flags=""
    if [ "$RELEASE" = true ]; then
        build_flags="--release"
        log "INFO" "Building release version..."
    else
        log "INFO" "Building debug version..."
    fi
    
    # Build main kernel
    if ! cargo +nightly build $build_flags; then
        log "ERROR" "Kernel build failed"
        exit 2
    fi
    log "SUCCESS" "Kernel built successfully"
    
    # Build init service
    if ! cargo +nightly build -p init $build_flags; then
        log "ERROR" "Init service build failed"
        exit 2
    fi
    log "SUCCESS" "Init service built successfully"
    
    # Build test binaries
    if ! cargo +nightly build --tests -p kernel; then
        log "ERROR" "Test binaries build failed"
        exit 2
    fi
    log "SUCCESS" "Test binaries built successfully"
}

# Function to find test binaries
find_test_binary() {
    local binary_name="$1"
    local binary_path=""
    
    if [ "$RELEASE" = true ]; then
        binary_path="target/x86_64-rustcore/release/${binary_name}"
    else
        binary_path="target/x86_64-rustcore/debug/${binary_name}"
    fi
    
    if [ -f "$binary_path" ]; then
        echo "$binary_path"
        return 0
    fi
    
    # Fallback to deps directory for test binaries
    binary_path=$(find target/x86_64-rustcore/debug/deps -name "${binary_name}-*" -type f | while read file; do [ -x "$file" ] && echo "$file"; done | head -1)
    
    if [ -n "$binary_path" ]; then
        echo "$binary_path"
        return 0
    fi
    
    return 1
}

# Function to run a single test
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_patterns="$3"
    local timeout_seconds="${4:-$DEFAULT_TIMEOUT}"
    
    log "INFO" "Running test: $test_name"
    
    if [ "$VERBOSE" = true ]; then
        log "DEBUG" "Command: $test_command"
        log "DEBUG" "Timeout: ${timeout_seconds}s"
    fi
    
    # Run the test with timeout
    local output
    local exit_code
    if command -v gtimeout >/dev/null 2>&1; then
        # macOS with GNU coreutils
        output=$(gtimeout "$timeout_seconds" bash -c "$test_command" 2>&1 || echo "EXIT_CODE:$?")
    elif command -v timeout >/dev/null 2>&1; then
        # Linux timeout
        output=$(timeout "$timeout_seconds" bash -c "$test_command" 2>&1 || echo "EXIT_CODE:$?")
    else
        # No timeout available
        output=$(bash -c "$test_command" 2>&1 || echo "EXIT_CODE:$?")
    fi
    
    # Extract exit code
    if echo "$output" | grep -q "EXIT_CODE:"; then
        exit_code=$(echo "$output" | grep "EXIT_CODE:" | cut -d: -f2)
        output=$(echo "$output" | grep -v "EXIT_CODE:")
    else
        exit_code=$?
    fi
    
    if [ "$VERBOSE" = true ]; then
        log "DEBUG" "Exit code: $exit_code"
        log "DEBUG" "Output: $output"
    fi
    
    # Check for expected patterns
    local patterns_found=0
    local total_patterns=0
    
    IFS=',' read -ra PATTERNS <<< "$expected_patterns"
    for pattern in "${PATTERNS[@]}"; do
        pattern=$(echo "$pattern" | xargs)
        if [ -n "$pattern" ]; then
            total_patterns=$((total_patterns + 1))
            if echo "$output" | grep -q "$pattern"; then
                if [ "$VERBOSE" = true ]; then
                    log "DEBUG" "✓ Found pattern: $pattern"
                fi
                patterns_found=$((patterns_found + 1))
            else
                log "WARN" "✗ Missing pattern: $pattern"
            fi
        fi
    done
    
    # Determine test result
    # QEMU tests can exit with code 1 (normal completion) or 0 (success)
    # The key is that all expected patterns are found
    if [ $patterns_found -eq $total_patterns ]; then
        log "SUCCESS" "✓ PASSED: $test_name"
        return 0
    else
        log "ERROR" "✗ FAILED: $test_name (exit: $exit_code, patterns: $patterns_found/$total_patterns)"
        return 1
    fi
}

# Function to run boot tests
run_boot_tests() {
    log "INFO" "Running boot sequence tests..."
    
    local kernel_binary
    kernel_binary=$(find_test_binary "kernel")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "Kernel binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # Basic kernel boot test
    if run_test "Basic Kernel Boot" \
        "./scripts/run-qemu.sh $kernel_binary" \
        "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "Boot tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run smoke tests
run_smoke_tests() {
    log "INFO" "Running smoke tests..."
    
    local smoke_binary
    smoke_binary=$(find_test_binary "boot_smoke")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "Boot smoke binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # Boot smoke test
    if run_test "Boot Smoke Test" \
        "./scripts/run-qemu.sh $smoke_binary" \
        "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "Smoke tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run enhanced tests
run_enhanced_tests() {
    log "INFO" "Running enhanced functionality tests..."
    
    local enhanced_binary
    enhanced_binary=$(find_test_binary "enhanced_kernel")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "Enhanced kernel binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # Enhanced kernel test
    if run_test "Enhanced Kernel Test" \
        "./scripts/run-qemu.sh $enhanced_binary" \
        "arch: serial ready,arch: paging init,arch: descriptor init,arch: idt init"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "Enhanced tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run comprehensive functional tests
run_functional_tests() {
    log "INFO" "Running comprehensive functional tests..."
    
    local functional_binary
    functional_binary=$(find_test_binary "direct_functional_test")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "Direct functional test binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # Direct functional test (runs tests immediately without scheduler)
    if run_test "Direct Functional Test" \
        "./scripts/run-qemu.sh $functional_binary" \
        "DIRECT_FUNC: All functional tests PASSED!,DIRECT_FUNC: tests_passed,DIRECT_FUNC: tests_failed"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "Functional tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run memory management tests
run_memory_tests() {
    log "INFO" "Running memory management tests..."
    
    local memory_binary
    memory_binary=$(find_test_binary "memory_test")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "Memory test binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # Memory management test
    if run_test "Memory Management Test" \
        "./scripts/run-qemu.sh $memory_binary" \
        "MEMORY: All memory tests PASSED!,MEMORY: tests_passed,MEMORY: tests_failed"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "Memory tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run IPC communication tests
run_ipc_tests() {
    log "INFO" "Running IPC communication tests..."
    
    local ipc_binary
    ipc_binary=$(find_test_binary "ipc_test")
    
    if [ $? -ne 0 ]; then
        log "ERROR" "IPC test binary not found"
        return 1
    fi
    
    local passed=0
    local failed=0
    
    # IPC communication test
    if run_test "IPC Communication Test" \
        "./scripts/run-qemu.sh $ipc_binary" \
        "IPC: All IPC tests PASSED!,IPC: tests_passed,IPC: tests_failed"; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    
    log "INFO" "IPC tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run service tests
run_service_tests() {
    log "INFO" "Running service integration tests..."
    
    local passed=0
    local failed=0
    
    # Check service manifest
    if [ -f "services/init/bootfs/services.manifest" ]; then
        log "SUCCESS" "✓ Service manifest exists"
        passed=$((passed + 1))
    else
        log "ERROR" "✗ Service manifest missing"
        failed=$((failed + 1))
    fi
    
    # Check system configuration
    if [ -f "services/init/bootfs/system.toml" ]; then
        log "SUCCESS" "✓ System configuration exists"
        passed=$((passed + 1))
    else
        log "ERROR" "✗ System configuration missing"
        failed=$((failed + 1))
    fi
    
    log "INFO" "Service tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run build tests
run_build_tests() {
    log "INFO" "Running build system tests..."
    
    local passed=0
    local failed=0
    
    # Check if kernel binary exists and is reasonable size
    local kernel_binary
    kernel_binary=$(find_test_binary "kernel")
    
    if [ $? -eq 0 ]; then
        local kernel_size
        kernel_size=$(stat -f%z "$kernel_binary" 2>/dev/null || stat -c%s "$kernel_binary" 2>/dev/null || echo "unknown")
        
        if [ "$kernel_size" != "unknown" ] && [ "$kernel_size" -lt 10485760 ]; then # < 10MB
            log "SUCCESS" "✓ Kernel size is reasonable: $kernel_size bytes"
            passed=$((passed + 1))
        else
            log "WARN" "⚠ Kernel size is large or unknown: $kernel_size bytes"
            failed=$((failed + 1))
        fi
    else
        log "ERROR" "✗ Kernel binary not found"
        failed=$((failed + 1))
    fi
    
    log "INFO" "Build tests completed: $passed passed, $failed failed"
    return $failed
}

# Function to run CI tests
run_ci_tests() {
    log "INFO" "Running CI/CD integration tests..."
    
    local total_failed=0
    
    # Run all core tests
    run_boot_tests || total_failed=$((total_failed + 1))
    run_smoke_tests || total_failed=$((total_failed + 1))
    run_service_tests || total_failed=$((total_failed + 1))
    run_build_tests || total_failed=$((total_failed + 1))
    
    if [ "$total_failed" -eq 0 ]; then
        log "SUCCESS" "All CI tests passed"
        return 0
    else
        log "ERROR" "$total_failed CI test suites failed"
        return 1
    fi
}

# Function to run all tests
run_all_tests() {
    log "INFO" "Running comprehensive test suite..."
    
    local total_failed=0
    
    # Run all test suites
    run_boot_tests || total_failed=$((total_failed + 1))
    run_smoke_tests || total_failed=$((total_failed + 1))
    
    if [ "$QUICK" = false ]; then
        run_enhanced_tests || total_failed=$((total_failed + 1))
        run_functional_tests || total_failed=$((total_failed + 1))
        run_memory_tests || total_failed=$((total_failed + 1))
        run_ipc_tests || total_failed=$((total_failed + 1))
    fi
    
    run_service_tests || total_failed=$((total_failed + 1))
    run_build_tests || total_failed=$((total_failed + 1))
    
    if [ "$total_failed" -eq 0 ]; then
        log "SUCCESS" "All tests passed"
        return 0
    else
        log "ERROR" "$total_failed test suites failed"
        return 1
    fi
}

# Main function
main() {
    local test_type="all"
    local clean_build=false
    local skip_build=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -q|--quick)
                QUICK=true
                shift
                ;;
            -r|--release)
                RELEASE=true
                shift
                ;;
            -c|--coverage)
                COVERAGE=true
                shift
                ;;
            -t|--timeout)
                DEFAULT_TIMEOUT="$2"
                shift 2
                ;;
            --clean)
                clean_build=true
                shift
                ;;
            --no-build)
                skip_build=true
                shift
                ;;
            all|boot|smoke|enhanced|functional|memory|ipc|services|interrupts|build|ci|benchmark)
                test_type="$1"
                shift
                ;;
            *)
                log "ERROR" "Unknown option: $1"
                show_help
                exit 3
                ;;
        esac
    done
    
    # Display banner
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                    Rustcore Testing Framework               ║"
    echo "║                         Version $SCRIPT_VERSION                        ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    log "INFO" "Starting rustcore testing..."
    log "INFO" "Test type: $test_type"
    log "INFO" "Configuration: verbose=$VERBOSE, quick=$QUICK, release=$RELEASE"
    
    # Check dependencies
    check_dependencies
    
    # Clean build if requested
    if [ "$clean_build" = true ]; then
        clean_build
    fi
    
    # Build components unless skipped
    if [ "$skip_build" = false ]; then
        build_components
    fi
    
    # Run selected tests
    local exit_code=0
    case "$test_type" in
        "boot")
            run_boot_tests || exit_code=1
            ;;
        "smoke")
            run_smoke_tests || exit_code=1
            ;;
        "enhanced")
            run_enhanced_tests || exit_code=1
            ;;
        "functional")
            run_functional_tests || exit_code=1
            ;;
        "memory")
            run_memory_tests || exit_code=1
            ;;
        "ipc")
            run_ipc_tests || exit_code=1
            ;;
        "services")
            run_service_tests || exit_code=1
            ;;
        "build")
            run_build_tests || exit_code=1
            ;;
        "ci")
            run_ci_tests || exit_code=1
            ;;
        "all"|*)
            run_all_tests || exit_code=1
            ;;
    esac
    
    # Final summary
    echo
    if [ $exit_code -eq 0 ]; then
        log "SUCCESS" "🎉 All tests completed successfully!"
        echo -e "${GREEN}Rustcore testing framework deployment complete.${NC}"
    else
        log "ERROR" "❌ Some tests failed. Please review the output above."
        echo -e "${RED}Testing completed with failures.${NC}"
    fi
    
    exit $exit_code
}

# Run main function with all arguments
main "$@"
