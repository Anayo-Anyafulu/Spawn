# Spawn (RustTorrent)

**Spawn** is a BitTorrent-inspired peer-to-peer (P2P) file distribution system written in Rust. This is a **learning-focused implementation** designed to explore the intricacies of low-level networking, concurrent systems, and data integrity in a distributed environment.

*Note: This project is intended for educational purposes and is not a production-ready BitTorrent client.*

### Core Concepts Explored
- **File Chunking & Hashing:** Implementing the logic to split files into pieces, generate SHA-1 hashes, and verify data integrity during assembly.
- **TCP Peer Communication:** Handling raw TCP streams to implement the BitTorrent wire protocol (Handshakes, Bitfields, Choke/Unchoke, Have/Request messages).
- **Concurrency in Rust:** Utilizing `tokio` for asynchronous I/O and managing shared state across multiple peer connections safely using Mutexes and Channels.
- **Leecher Logic:** The orchestration of requesting specific pieces from different peers simultaneously to maximize throughput.

### Project Goals
- Understand the "Handshake to Completion" lifecycle of a P2P download.
- Master Rusts ownership and borrowing rules in a highly concurrent, networked environment.
- Experiment with custom protocol parsing (Bencode decoding/encoding).

### Technical Overview
- **Runtime:** `tokio` for async networking.
- **Serialization:** `serde` with custom Bencode support.
- **Hashing:** `sha1` for piece verification.
