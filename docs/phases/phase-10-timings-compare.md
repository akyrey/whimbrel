# Phase 10: Timing breakdown and response comparison

Goal: see where a request spent its time (FR-3.1 "timing breakdown where available"), and what
changed between two responses.

Design: [PRD.md](../PRD.md) FR-3.1; D-H10 (superseded in part by D-H20).

## Tasks

- [x] 10.1 Timing breakdown in `wbl-http` (D-H20): DNS and connect (TCP and TLS together) for a
      new connection, measured by a timing DNS resolver and a reqwest connector layer and
      attributed to the send through a task-local; both are absent on a reused connection.
      Old history entries without them still read.
- [x] 10.2 Timing view: a "Timing" tab in the response with a waterfall (DNS, connect,
      waiting, download) and "reused connection" when there was no connect.
- [ ] 10.3 Compare responses: a tab keeps the response before the current one; a "Compare"
      tab shows the status, the header changes, and a line diff of the (pretty) bodies.

## Acceptance

Each task's tests pass; `wbl-http` and the frontend keep 80% line coverage.
