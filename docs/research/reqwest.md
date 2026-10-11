# Reqwest Client & Async HTTP Research

Access Date: 2026-10-11
Sources:
- docs.rs: https://docs.rs/reqwest/latest/reqwest/

## Best Practices
- Client reuse: Instantiate `reqwest::Client` once per pipeline session to benefit from connection pooling and keepalive.
- Timeouts: Configure per-request timeouts using `.timeout(Duration::from_millis(timeout_ms))` to guarantee hard SLA bounds.
- Error handling: Check `err.is_timeout()` and `err.is_connect()` for network-level retry decisions.
- Headers: Always include `Authorization`, `Content-Type: application/json`, and `User-Agent: Mozilla/5.0`.
