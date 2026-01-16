# Control events

## Overview

```
Peer
 ├─ Receiver PC
 │   ├─ media tracks (recvonly)
 │   └─ signal DataChannel  <-- ONLY Signaling Events
 │
 ├─ Sender PC
 │   ├─ media tracks (sendonly)
 │   └─ control DataChannel  <-- ONLY Control Events
 │
 └─ ControlBus (queue + state)
```

## State machine

Control events are never lost "silently".

Events are:

- either sent immediately
- or buffered and sent later
- Renegotiation / ICE / Track-changes don't matter
- No dependency on the media lifecycle
- A clear owner for control-events

```
NoDC
  │
  ├─ set_dc()
  ↓
DCConnecting
  │
  ├─ on_open()
  ↓
DCOpen
  │
  ├─ send()
  │
  └─ on_close()
      ↓
   NoDC (Queue stay!)
```
