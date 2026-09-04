# ADR-0009: HTTP Client Architecture, Advanced Networking, and Response Inspection

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

PacketSmith requires a production-grade HTTP client capable of handling complex REST and raw payloads, multi-part form data, automatic URL query synchronization, redirect policy enforcement, proxy routing (HTTP, HTTPS, SOCKS5), custom CA bundles, and streaming response inspection without freezing the user interface or leaking credentials.

## Considered Options

1. **Custom Raw TCP/TLS Socket Client:** Maximum control, but requires re-implementing HTTP/1.1 chunking, HTTP/2 multiplexing, connection pooling, and decompression from scratch.
2. **Reqwest with Modular Wrapper (`ps-http`):** Built on Hyper, Tokio, and Rustls. Supports HTTP/1.1, HTTP/2, connection pooling, proxy chains, and streaming responses with memory efficiency.

## Decision Outcome

Chosen option: **Reqwest with Modular Wrapper (`ps-http`)**.

### Positive Consequences
- Robust async HTTP/1.1 and HTTP/2 transport with native connection pooling.
- Complete bidirectional synchronization between the raw URL and parsed query parameters table rows.
- Sensitive header masking (`Authorization`, `Cookie`, `X-Api-Key`) prevents credential exposure in execution logs.
- Multiple response inspection modes: Pretty (JSON/XML), Raw, and byte-level Hex dump.

### Negative Consequences
- Advanced features like HTTP/3 require tracking upstream quiche/h3 development in later phases.
