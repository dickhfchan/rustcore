# Rustcore QEMU Testing Guide

## Overview
This document provides comprehensive testing procedures for the rustcore operating system using QEMU emulation. The tests validate the actual functionality of kernel components, services, and system integration.

## Prerequisites
- QEMU installed (`qemu-system-x86_64`)
- Rust nightly toolchain
- Build dependencies for custom target

## Test Categories

### 1. Boot Sequence Testing
**Purpose**: Validate complete system boot from UEFI to kernel initialization

**Test Components**:
- UEFI loader functionality
- PVH boot protocol compliance
- Kernel entry point execution
- Memory map setup and validation
- BootInfo structure handling

**Expected Outcomes**:
- Serial output showing boot progression
- Successful kernel initialization
- Proper memory mapping
- Init service registration

### 2. Service Integration Testing
**Purpose**: Verify init service and bootfs functionality

**Test Components**:
- Bootfs parsing and validation
- Service manifest processing
- IPC channel establishment
- Bootstrap message exchange

**Expected Outcomes**:
- Successful bootfs mounting
- Service manifest validation
- IPC communication establishment
- "INIT:READY" acknowledgment

### 3. IPC Communication Testing
**Purpose**: Validate kernel-user space communication

**Test Components**:
- Channel creation and management
- Message sending/receiving
- Interrupt-based communication
- Error handling

**Expected Outcomes**:
- Successful message exchange
- Proper interrupt handling
- Error recovery mechanisms

### 4. Memory Management Testing
**Purpose**: Test paging and memory allocation

**Test Components**:
- Page table setup (PML4, PDP, PD)
- Memory region mapping
- Access permissions
- Memory protection

**Expected Outcomes**:
- Proper page table initialization
- Memory access validation
- Protection fault handling

### 5. Interrupt Handling Testing
**Purpose**: Validate timer and IPC interrupts

**Test Components**:
- Local APIC configuration
- Timer interrupt handling
- IPC interrupt processing
- Interrupt acknowledgment

**Expected Outcomes**:
- Timer tick counting
- IPC message processing
- Proper interrupt cleanup

### 6. Error Handling Testing
**Purpose**: Test fault scenarios and recovery

**Test Components**:
- General protection faults
- Invalid memory access
- Service failures
- Panic handling

**Expected Outcomes**:
- Fault detection and logging
- Graceful error recovery
- Proper system shutdown

## Test Execution Procedures

### Basic Boot Test
```bash
# Build the kernel
cargo +nightly build

# Run basic boot test
./scripts/run-qemu.sh

# Check output for:
# - "kernel: entered kernel_main"
# - "kernel: init ack received" 
# - "kernel: init complete"
# - "kernel: timer ticks observed"
```

### Comprehensive Integration Test
```bash
# Build with tests
cargo +nightly build --tests -p kernel

# Run boot smoke test
./scripts/run-qemu.sh target/x86_64-rustcore/debug/boot_smoke-*

# Validate QEMU exit code (0 = success, 1 = failure)
echo $?
```

### Service Validation Test
```bash
# Build init service
cargo +nightly build -p init

# Run with bootfs validation
./scripts/run-qemu.sh --release

# Check for service manifest validation
grep "INIT:READY" debug.log
```

## Test Validation Criteria

### Success Indicators
- QEMU exits with code 0
- Serial output shows expected boot sequence
- No general protection faults
- Timer interrupts functioning
- IPC communication established
- Services properly initialized

### Failure Indicators
- QEMU exits with code 1
- General protection faults detected
- Missing boot messages
- IPC communication failures
- Service initialization errors
- Timer interrupt failures

## Debugging Procedures

### Serial Output Analysis
```bash
# Monitor serial output in real-time
./scripts/run-qemu.sh 2>&1 | tee boot.log

# Analyze debug logs
cat debug.log | grep -E "(kernel|init|error|fault)"
```

### Memory Layout Validation
```bash
# Check memory map
objdump -h target/x86_64-rustcore/debug/kernel

# Validate boot info structure
readelf -s target/x86_64-rustcore/debug/kernel | grep boot
```

### Service Manifest Validation
```bash
# Check bootfs contents
ls -la services/init/bootfs/

# Validate manifest syntax
cat services/init/bootfs/services.manifest
```

## Performance Benchmarks

### Boot Time Measurement
- Target: < 1 second from UEFI to init service ready
- Measurement: Timer ticks from boot to "INIT:READY"

### Memory Usage
- Kernel footprint: < 1MB
- Init service: < 100KB
- Total system: < 2MB

### Interrupt Latency
- Timer interrupt: < 100μs
- IPC interrupt: < 50μs

## Continuous Integration

### Automated Testing
```bash
#!/bin/bash
# ci-test.sh
set -e

echo "Building rustcore..."
cargo +nightly build --tests -p kernel

echo "Running boot smoke test..."
./scripts/run-qemu.sh target/x86_64-rustcore/debug/boot_smoke-*

echo "Validating init service..."
cargo +nightly build -p init
./scripts/run-qemu.sh --release

echo "All tests passed!"
```

### Test Coverage Goals
- Kernel boot: 100%
- IPC communication: 100%
- Memory management: 95%
- Interrupt handling: 95%
- Service integration: 90%

## Troubleshooting Guide

### Common Issues
1. **QEMU not found**: Install qemu-system-x86_64
2. **Build failures**: Ensure nightly Rust and build-std
3. **Boot failures**: Check linker script and target configuration
4. **IPC failures**: Validate channel setup and message format

### Debug Commands
```bash
# Verbose QEMU output
QEMU_EXTRA="-d cpu_reset,int,exec" ./scripts/run-qemu.sh

# Memory debugging
QEMU_EXTRA="-d guest_errors" ./scripts/run-qemu.sh

# Interrupt debugging  
QEMU_EXTRA="-d int" ./scripts/run-qemu.sh
```

## Conclusion
This testing framework ensures comprehensive validation of rustcore's functionality, from basic boot sequence to advanced service integration. Regular execution of these tests provides confidence in system reliability and performance.
