# api-gate — Zero-Trust HTTP-to-Frame Proxy

HTTP gatekeeper for inference servers: enforces 4KB payload cap, 2000ms hard timeout, per-request credit burn, and blank error responses.

## Architecture

- **Synchronous** `std::net::TcpListener` + thread-per-connection (no tokio/async)
- **Frame protocol** forward to upstream (u32-BE-length + payload, matching gemma-sidecar)
- **Credit ledger** atomic CAS-burn, 402 rejection on zero balance
- **Strict limits**: 413 on body >4096 bytes, 400 on malformed JSON, 504 on timeout
- **Zero error details** in responses (blank bodies only)

## Running

```bash
export API_GATE_BIND="127.0.0.1:8080"      # Gate listen addr (default)
export API_GATE_CREDITS="1000"              # Initial credit balance (default)
./target/debug/api-gate.exe
```

Gate listens on `API_GATE_BIND` and proxies to `127.0.0.1:13017` (gemma-sidecar).

## Request Flow

1. Accept HTTP connection
2. Burn 1 credit or return 402 Payment Required
3. Read body capped to 4096 bytes or return 413 Payload Too Large
4. Validate JSON shape (balanced braces/quotes) or return 400 Bad Request
5. Forward request body as frame to upstream (u32-BE-length + payload)
6. Read response frame or timeout after 2000ms total elapsed
7. Return 200 OK with upstream reply, or 504 Gateway Timeout

## Testing

```bash
# Unit tests (credit ledger, JSON shape validation)
cargo test -p api-gate --lib

# Build binary
cargo build -p api-gate --bin api-gate
```

## Known Limitations

- **Deadline reactor not wired**: The 2000ms timeout is enforced at read/connect time per individual I/O, not as a hard wall-clock deadline. Real deadline (park on viximesh::Reactor, fire at 2000ms elapsed) is deferred until `viximesh::Reactor::new()` is exported or a thread-safe clock primitive lands.
- **JSON extraction**: Currently forwards entire request body verbatim to upstream. Real JSON field extraction (e.g., extracting `{"prompt": "..."}` to send `INFER ...`) is a follow-up.
- **Upstream discovery**: Hardcoded to `127.0.0.1:13017`. Environment-variable config is deferred.

## Wire Protocol (Upstream)

Frame format (u32-BE-length + UTF-8 payload):
- **Request**: `<len><payload>` where payload is the query text
- **Response**: `<len><reply>` where reply is the inference output or `ERR <message>` on failure

Example:
```
Send:   0x0000000B "test query"     (11 bytes)
Recv:   0x00000016 "REPLY: test query"  (22 bytes)
```
