# Task: Phase 6 — LAN peer-to-peer sync (TOFU + Noise)

> **Status:** in-progress
> **Priority:** medium
> **Phase:** 6 — LAN sync
> **Created:** 2026-04-26
> **Estimated:** 6-8 weeks

## Description

Optional, opt-in sync between ditox instances on the same LAN. No
central server, modern crypto, no pre-shared password (unlike Ditto's
Friends feature).

Schema bump: v6 → v7 (peers table, sync metadata).

Decisions baked in:
- **LAN-only in v1.0** (D6). Cloud relay sketched but not built.
- **TOFU-pinned ed25519** keys, never auto-trust.
- **Noise_XX_25519_ChaChaPoly_SHA256** transport via `snow`.
- **Pull-based sync** with content-addressed dedup.

## Sub-tasks

### 6.1 mDNS-SD discovery

Service type: `_ditox._tcp.local.`

TXT records:
- `version=N` (protocol version)
- `key=<base64-pubkey-fingerprint>` (first 12 bytes of SHA-256(pubkey))
- `name=<hostname-or-user-given>`

Library: `mdns-sd` crate.

Discovery runs only when sync is enabled (`[sync] enabled = true`).
Off by default.

### 6.2 Identity & trust

On first sync-enable, generate ed25519 keypair:

```
~/.config/ditox/identity.key       (chmod 600, ed25519 private key)
~/.config/ditox/identity.pub       (ed25519 public key, ASCII)
```

`ed25519-dalek` for key generation.

Trust model: **TOFU with explicit pin**. Never auto-trust.

Schema v6 → v7:

```sql
CREATE TABLE peers (
    id            TEXT PRIMARY KEY,           -- uuid
    name          TEXT NOT NULL,              -- user-given or hostname
    public_key    BLOB NOT NULL UNIQUE,       -- ed25519 pubkey 32 bytes
    fingerprint   TEXT NOT NULL,              -- hex of SHA-256[:12]
    trust_state   TEXT NOT NULL,              -- 'untrusted' | 'pinned' | 'rejected'
    auto_send     INTEGER NOT NULL DEFAULT 0, -- broadcast new clips to this peer?
    last_seen     TEXT,
    last_sync     TEXT,
    addresses     TEXT NOT NULL,              -- JSON array of "ip:port"
    created_at    TEXT NOT NULL
);

CREATE TABLE sync_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id     TEXT NOT NULL REFERENCES peers(id) ON DELETE CASCADE,
    direction   TEXT NOT NULL,                 -- 'send' | 'receive'
    entry_id    TEXT,
    bytes       INTEGER,
    status      TEXT NOT NULL,                 -- 'ok' | 'error' | 'rejected'
    message     TEXT,
    occurred_at TEXT NOT NULL
);
```

UI flow when an unknown peer appears in mDNS:
1. Notification "Discovered ditox peer at <ip>: <fingerprint>".
2. User opens Settings → Sync → click peer → "Trust" or "Reject".
3. Optional auto-trust by hostname pattern (advanced).

### 6.3 Noise transport

Library: `snow`.

Pattern: `Noise_XX_25519_ChaChaPoly_SHA256` (mutual auth, identity hide
on initiator side acceptable for LAN).

Handshake:
1. Initiator opens TCP to peer's advertised port.
2. Sends `e` (ephemeral pub).
3. Receives `e, ee, s, es` (peer ephemeral, dh, peer static, dh).
4. Sends `s, se` (own static, dh).
5. Both verify the static key matches what's pinned in DB.
6. Transport mode begins.

After handshake: framed binary protocol.

```
[u32 length BE][protobuf message bytes]
```

### 6.4 Wire protocol

`proto/ditox-sync.proto`:

```proto
syntax = "proto3";

message Hello {
  uint32 protocol_version = 1;
  string name = 2;
  uint64 db_schema_version = 3;
}

message DigestRequest {
  // peer asks for digest of recent N entries
  uint32 limit = 1;
  optional string since_iso8601 = 2;
}

message DigestResponse {
  repeated EntryDigest entries = 1;
}

message EntryDigest {
  string id = 1;            // uuid
  string entry_hash = 2;    // SHA-256 hex
  string updated_at = 3;
  bool pinned = 4;
}

message EntryRequest {
  repeated string entry_ids = 1;
}

message EntryPayload {
  string id = 1;
  string entry_hash = 2;
  // ... mirror entries+entry_formats schema
  repeated FormatPayload formats = 3;
}

message FormatPayload {
  string format_name = 1;
  string format_hash = 2;
  oneof body {
    bytes inline = 10;       // text formats
    BlobChunk blob_chunk = 11;
  }
}

message BlobChunk {
  string blob_hash = 1;
  uint64 total_bytes = 2;
  uint64 offset = 3;
  bytes data = 4;
  bool last = 5;
}

message Bye {}
```

Pull-based: each peer periodically (or on user action) sends `DigestRequest`.
Receiving side replies with `DigestResponse`. Caller diffs against own
DB and sends `EntryRequest` for missing ids. Receiver streams
`EntryPayload`s, with images split into multiple `BlobChunk`s.

### 6.5 Conflict resolution

Entries are content-addressed via SHA-256. If both sides have an entry
with the same hash → already identical, no transfer.

For mutable fields (`pinned`, `last_used`, `notes`, `tags`,
`collection_id`):
- Last-write-wins on RFC3339 timestamp, with a `client_id` tiebreak.
- A `mutations` table tracks edits with monotonic vector clocks (per
  peer): `(entry_id, field, value, lamport_ts, peer_id)`.
- On merge, max lamport_ts wins.

This is the most subtle part of the phase. Allocate a full week.

### 6.6 Image blob transfer

- Source side computes blob SHA-256 (already stored).
- Splits into 64 KiB chunks.
- Sends as `BlobChunk { offset, data, last }`.
- Receiver writes to `<images>/.tmp-recv/{blob_hash}.{ext}.partial`.
- On `last = true`: verify SHA-256 matches; rename to final location.
- Resume: receiver tracks last received offset per blob_hash; can
  request `EntryRequest` with `start_offset = N`.

### 6.7 Sync settings UI

Settings → Sync page:

- Toggle "Enable sync" (off by default).
- Show local fingerprint + QR code (for easy mobile-aware comparison
  later).
- Discovered peers list with trust state, last sync timestamp, error
  count.
- Per-peer:
  - Trust / Reject buttons.
  - Auto-send toggle.
  - "Sync now" button.
  - Recent activity log (`sync_log` rows).

### 6.8 Firewall hint

Inno Setup post-install option (Windows): "Add firewall rule for
ditox sync (port 9001-9100)". Calls `netsh advfirewall firewall add
rule name=Ditox dir=in action=allow program=<exe> profile=private`.

Default port: 9001 (configurable). If 9001 in use, scan up to 9100 for
a free port and advertise the actual port via mDNS.

### 6.9 Tests

- Two `ditox-core` library instances in one test process; they
  discover each other via in-memory mDNS, exchange Hello, sync 100
  entries, assert convergence.
- Conflict test: both sides modify `pinned` flag at different times;
  later timestamp wins.
- Trust test: untrusted peer is silently dropped; rejected peer never
  tried again until manually un-rejected.
- Resume test: kill receiver mid-blob-transfer; restart; assert blob
  completes correctly.

## Acceptance criteria

- [ ] Two ditox instances on the same LAN discover each other via
      mDNS.
- [ ] First-time pairing requires explicit trust on both sides.
- [ ] After pairing, copying a clip on machine A appears on machine B
      within 5 seconds (auto-send mode).
- [ ] Image clips sync correctly with content-addressed blobs.
- [ ] Sync survives transient network blips (resumable).
- [ ] Mutual TLS-like authentication via Noise (a third machine
      pretending to be peer's IP can't sync).
- [ ] Disabling sync leaves no leftover network activity.
- [ ] Sync log accessible via UI and `ditox sync log --json`.

## Implementation Notes

Cloud relay sketch (not built):
- Same Noise protocol, but the relay is a forwarding-only TCP server.
- Relay sees only encrypted bytes (no key material).
- Schema-version pinned at the relay; it can refuse mismatched peers.
- Self-hosted; we may publish a Docker image post-v1.0.

Mobile sync future: a Flutter / React Native client implementing the
same wire protocol. Not v1.0 scope but the protocol is designed to
support it.

## Risks

- **Risk:** Conflict resolution edge cases corrupt data.
  Mitigation: `proptest` properties for "merge is idempotent",
  "later-timestamp wins under all orderings".
- **Risk:** mDNS leaks the user's hostname on shared networks.
  Mitigation: configurable `[sync] name = "..."` (default = hostname,
  can be set to "ditox-anon").
- **Risk:** ed25519 key compromise.
  Mitigation: keys are local-machine-only; user can revoke + regenerate;
  peers must re-trust the new key.
- **Risk:** Protocol incompatibility between versions.
  Mitigation: `Hello.protocol_version`; refuse mismatched peers with
  clear error message.

## Work Log

### 2026-04-26
- Task file created (epic).

### 2026-04-27
- Started Phase 6 groundwork.
- Added `[sync]` config (`enabled = false` default, port range, name,
  digest limit).
- Added `ditox_core::sync` identity/protocol primitives: ed25519 local
  identity generation/persistence, public-key fingerprint helper, peer
  trust/log models, protocol constants.
- Bumped schema v6 → v7 with `peers` and `sync_log` tables plus indexes.
- Added DB helpers for discovered-peer upsert, trust/auto-send updates,
  last-sync marking, sync log append/list.
- Added `ditox sync peers [--json]` and `ditox sync log [--json]`.
- Added tests for config defaults/parsing, identity round-trip,
  fingerprinting, v7 migration, and peer trust preservation.
