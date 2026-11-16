---
paths:
  - "crates/ash-protocol/**"
---

# Review checklist: protocol crate

Rules: `.claude/rules/protocol.md`. Each "no" is a finding.

- Does `ash-protocol` still depend on `ash-core` only?
- Does the message format stay independent of the transport?
- Does a protocol change ship the server and the client in the same PR?
- Is the generated WASM package absent from the diff?
- Does the change avoid settling the wire format before ADR 0003?
- Once a message decoder exists, does protocol fuzzing cover it?
