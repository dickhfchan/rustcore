# Rustcore AWS S3-Compatible Service Requirements

Goal: Provide an S3-style object storage service on top of Rustcore without relying on non-Rust libraries or external stacks. The implementation will add kernel subsystems, runtime services, and an S3 API layer so that clients can perform standard S3 operations (bucket/object CRUD, listings, multipart uploads) over TLS using API keys for authentication.

## Core Constraints
- Entire stack must be implemented in Rust (no C libraries, no external OS stacks).
- Integration points must remain `#![no_std]` compatible unless explicitly isolated in user services.
- Every milestone must pass `cargo +nightly test -p kernel --test boot_smoke` under QEMU.

## Functional Requirements

### Storage Layer
1. **Block Device Abstraction (`storage/block`)**
   - Provide async-capable read/write interfaces over hardware drivers (NVMe, virtio).
   - Handle alignment, error propagation, and queue management.
2. **Object Store Layer (`storage/object`)**
   - Organize data in fixed-size chunks with metadata records.
   - Maintain checksums and size attributes for each object part.
   - Offer streaming reads/writes to avoid buffering entire objects.
3. **Optional Versioning (`storage/version`)**
   - Track version IDs per object.
   - Provide garbage collection hooks for expired versions.

### Metadata & Filesystem
4. **Catalog Service (`fs/catalog`)**
   - Maintain bucket and object namespaces, ACL metadata, quotas.
   - Support bucket-level operations: create, delete, list, rename.
5. **Write-Ahead Journal (`fs/journal`)**
   - Record catalog/object mutations for crash consistency.
   - Include replay logic during boot.
6. **Indexing (`fs/index`)**
   - Support prefix and delimiter-based listings.
   - Maintain object iterators for pagination.

### Networking & TLS
7. **Network Stack**
   - `net/ethernet`: frame parsing, MAC addressing, ARP.
   - `net/ip`: IPv4/IPv6 routing, fragmentation.
   - `net/tcp`: connection establishment, retransmission, congestion control.
8. **TLS Termination (`net/tls`)**
   - Pure-Rust TLS implementation or ported rustls subset.
   - Certificate management and renegotiation support.

### Runtime Infrastructure
9. **Async Executor (`runtime/executor`)**
   - Cooperative multitasking for drivers and services.
10. **Timers (`runtime/timers`)**
   - High-resolution timers for TCP/TLS and housekeeping.
11. **Kernel Allocator (`runtime/alloc`)**
   - Scalable heap tuned for long-lived allocations.

### Security & Authentication
12. **API Key Store (`security/keystore`)**
   - Persist hashed API keys per bucket/application.
13. **API Key Middleware (`security/apikey`)**
   - Validate key (header/query) per request.
   - Enforce bucket-level capabilities (read/write/list).

### S3 Service Layer (`services/s3`)
14. **HTTP Layer**
   - Minimal HTTP/1.1 server supporting chunked transfer.
   - Integrate with TLS and executor.
15. **Request Routing & Auth**
   - Decode S3 REST routes, apply API key middleware.
16. **Operation Handlers**
   - Bucket ops: `CreateBucket`, `DeleteBucket`, `ListBuckets`.
   - Object ops: `PutObject`, `GetObject`, `DeleteObject`, `HeadObject`.
   - Listing: `ListObjectsV2` with prefix/delimiter support.
   - Multipart: `CreateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, `AbortMultipartUpload`.
17. **Error Handling**
   - Return S3-compatible XML errors and HTTP status codes.
18. **Event Hooks (Optional)**
   - Emit notifications for object changes (internal queue/log).

### Observability & Tooling
19. **Logging (`services/log`)**
   - Structured logging for network/storage/S3 events.
20. **Metrics (`services/metrics`)**
   - Counters for requests, errors, latency, storage usage.
21. **Integration Tests**
   - Boot-time harness using bootfs configs to run S3 scenarios in QEMU.

## Delivery Milestones
1. **Document & Skeletons**
   - Requirements doc (this file).
   - Stub modules with feature flags; ensure existing tests pass.
2. **Storage + Catalog Foundations**
   - Implement `storage/block`, `storage/object`, `fs/catalog` minimal features.
3. **Networking & TLS Skeleton**
   - Basic Ethernet/IP/TCP loopback with TLS handshake stub.
4. **Runtime Enhancements**
   - Executor/timers integrated; run simple async tasks in QEMU.
5. **API Key Auth + HTTP**
   - HTTP parser, API key enforcement, simple request routing.
6. **S3 Core Operations**
   - Implement basic bucket/object CRUD with catalog/object store.
7. **Listings & Multipart**
   - Add prefix listings and multipart upload flows.
8. **Observability + Tests**
   - Logging/metrics integration, full QEMU integration tests.

Each milestone must conclude with `cargo +nightly test -p kernel --test boot_smoke`. Additional integration tests will be added once the HTTP layer is operational.
