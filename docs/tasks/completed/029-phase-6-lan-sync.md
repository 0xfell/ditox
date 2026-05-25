# Task: LAN Peer-To-Peer Sync

## Status

completed

## Completed

2026-04-27

## Summary

Added opt-in LAN sync with local identity, peer discovery, trust controls, and
trusted pull sessions.

## Work Log

- Added schema v7 peer and sync-log tables.
- Added ed25519 local identity.
- Added mDNS discovery.
- Added explicit trust, reject, untrust, and auto-send controls.
- Added Noise transport with signed identity proof.
- Added trusted TCP pull sessions.
- Added metadata sync for notes, collections, tags, favorites, and last-used.
- Added chunked image transfer with hash verification.
- Added CLI commands for discovery, peers, pull, trust, reject, untrust,
  auto-send, and logs.

