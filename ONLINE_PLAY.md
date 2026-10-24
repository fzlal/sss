# Online and LAN play

**Status:** implemented (protocol version 2). Online play is host-authoritative
over plain UDP and is available from **ONLINE** in the main menu, or from the
command line.

## Playing

| Goal | How |
| --- | --- |
| Host from the game | **ONLINE → HOST GAME**, choose sport, arena, slime, score and mana mode, then **HOST MATCH**. The lobby shows the address to share. The match starts as soon as one rival joins. |
| Join a LAN game | **ONLINE → LAN GAMES**, pick a room. Hosts on the local network are discovered automatically. |
| Join by address | **ONLINE → JOIN BY IP**, type `host:port` (the port is optional; `43210` is the default), choose a slime and connect. |
| Dedicated server | `slime-soccer --server [--port 43210 ...]` starts a headless two-player server. |
| Host without menus | `slime-soccer --host [--port 43210 ...]` opens the game straight into a host lobby. |
| Join without menus | `slime-soccer --connect 192.168.1.20:43210 [--name ...]` opens the game, connects and picks the slime given by `--p1`. |

The joiner always picks their own slime in the join screen. Sport, arena,
score target and mana mode come from the host or the dedicated server's flags.
Both sides must run the same protocol version; a mismatch is rejected with
`PROTOCOL MISMATCH` instead of desyncing.

In an online match: **ESC** pauses, **Q** while paused leaves to the online
menu, **M** toggles mute and **F11** toggles fullscreen, as in local play. The
bottom bar shows the side, host name and live ping/loss. A paused online match
keeps running on the server; the pause only stops local rendering input.

### Dedicated server

```sh
slime-soccer --server --port 43210 --sport soccer --arena 0 \
             --play-to 6 --mana bonus --name "Friday Cup"
```

Each client picks its own slime on join, so `--p1`/`--p2` only provide the
initial defaults before anyone has connected.

The server prints the listening address, the LAN address and state changes.
It starts a match when two clients have joined, resets to the lobby six seconds
after a match ends (three seconds after an abort) and waits for the next pair.
Stop it with **Ctrl+C**. It has no remote console and does not need a window,
graphics or audio device.

All flags:

```
--server             headless two-player server
--host               host from the game window
--connect ADDRESS    join a host by IP or hostname (also -c)
--port PORT          UDP port, default 43210 (also -p)
--sport SPORT        soccer, volleyball or basketball
--arena NUMBER       0..10
--play-to NUMBER     1..15
--mana MODE          bonus, offensive or round
--p1 NUMBER          slime for the host or for `--connect` (0..34)
--p2 NUMBER          initial rival slime, replaced when a client joins (0..34)
--name TEXT          player or server name
-h, --help           show usage
```

`--server`, `--host` and `--connect` are mutually exclusive.

## Network model

- **The host owns the simulation.** It steps the authoritative match at a fixed
  120 Hz, sends snapshots to both clients at 30 Hz and is the only side that
  decides the score. A dedicated server is the same code without a window.
- **Clients send input at 60 Hz.** Every input packet repeats the last eight
  captured frames, so a single lost packet costs no input and needs no
  retransmission. The server drops late or duplicate input and reuses the last
  frame instead of stalling the match.
- **Snapshots are complete simulation states.** `Match` is serialized with
  postcard (no delta encoding yet) and split into 1000-byte chunks so each UDP
  datagram stays under the common 1200-byte path MTU. A client reassembles the
  chunks, ignores out-of-order/older ticks and renders between the previous and
  newest snapshot to smooth the 30 Hz stream.
- **No rollback prediction yet.** The local slime reacts after the input
  reaches the host, so responsiveness is bounded by the round trip. On a LAN
  this is imperceptible; on the internet it is noticeable above roughly 80 ms
  RTT. The protocol and snapshotting are ready for client-side prediction as a
  follow-up (see the roadmap).
- **Failure handling.** A client that goes silent for 3 seconds is dropped;
  during a match the server aborts and tells the remaining peer. The server
  drops a lobby client after 8 seconds of silence. A client that receives
  nothing for 6 seconds while connecting reports `NO RESPONSE`. Score and
  phase always come from the authoritative snapshot.

### Messages

| Message | Direction | Contents |
| --- | --- | --- |
| `Hello` | client → server | protocol version, player name, slime id |
| `Welcome` | server → client | version, side, joined/needed players, server name |
| `Reject` | server → client | full/protocol-mismatch reason |
| `Input` | client → server | side, newest sequence, up to 8 input frames |
| `Snapshot` | server → client | sequence, tick, recipient, chunk index/count, postcard state + event ids |
| `Ping` / `Pong` | both | round-trip timing for the HUD |
| `Bye` | both | leave/cancel/disconnect reason |
| `Discover` / `Offer` | both (LAN) | room discovery: name, port, sport, score target, player count, in-match state |

Packets are size-checked and rejected on protocol-version mismatch; names are
sanitized and slime ids clamped before use.

## LAN discovery

A client on the **LAN GAMES** screen broadcasts a `Discover` packet every
750 ms to `255.255.255.255:43210` and `127.0.0.1:43210`. Hosts answer with an
`Offer` containing their room name, port, sport, score target and occupancy.
Rooms disappear after four seconds without an offer and are sorted by name.
Only port 43210 is scanned; a host on a custom port must be joined by IP, and
some firewalls block UDP broadcast, in which case **JOIN BY IP** is the
fallback. Discovery works per broadcast domain, not across subnets.

## Security and NAT

- This build has **no encryption or authentication**: packets are plain UDP.
  Anyone who can reach the port and speaks protocol version 2 can attempt to
  join, and traffic can be observed on the network. Play with people you trust
  on a LAN or a private network. Do not expose the server directly to the open
  internet without a firewall, and never treat player names as trusted input.
- There is **no NAT traversal, relay or rendezvous service**. LAN play works
  without configuration; internet play requires the host to forward UDP port
  43210 (or the chosen `--port`) and share its public address. This is a
  deliberate limit of the current build, not a bug.
- The server validates packet size, protocol version, slot count and input
  shape, but it does not rate-limit misbehaving clients and has no remote
  administration interface.

## Roadmap

1. **Client-side prediction and rollback** to remove round-trip input latency,
   with state hashing and reconciliation against server snapshots.
2. **Delta snapshots** and bandwidth budgeting; the current build sends full
   states about 30 times per second.
3. **QUIC + TLS 1.3** transport for encryption, congestion control and
   connection migration, plus mDNS-based discovery instead of raw broadcast.
4. **Reconnect and rematch** in the hosted UI (the dedicated server already
   accepts the next pair after its reset).
5. **NAT traversal / relay** for internet play without port forwarding.

## Testing

`tests/network.rs` starts a real hosted server on an ephemeral loopback port and
a real UDP client, plays until the match starts, checks the assigned side,
player count, snapshot stream and rival movement, then disconnects and asserts
the server aborts. The rest of the suite covers snapshot serialization, packet
round-trips, chunk splitting/reassembly, address parsing and event ids. All
tests are offline and deterministic; no external network access is required.
