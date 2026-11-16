---
paths:
  - "crates/ash-protocol/**"
---

# Protocol crate: `ash-protocol`

Decided in `docs/plans/0000-methodology.md`, sections 1, 4 and 10.

- `ash-protocol` holds the messages and the codec. It depends on `ash-core`
  only.
- It compiles to WASM for the web client. `xtask` builds the WASM package
  into a git-ignored directory: never commit it.
- The message format is independent of the transport. The transport is
  WebSocket, with binary messages.
- The server is authoritative. The client displays the player's area.
- A protocol change ships server and client in one PR.
- The wire format and the codec belong to ADR 0003, not decided yet. Phase 1
  uses a minimal, replaceable codec. The lead under study, not a decision:
  this crate in WASM on the client, one codec, a version handshake.
- Protocol fuzzing starts with the first message decoder.
