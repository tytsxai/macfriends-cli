# Design

## Web Token

`macfriends serve` generates an in-memory token at startup and injects it into the served dashboard. Mutating `/api/*` requests must send the token with `X-MacFriends-Token`. This blocks cross-site POSTs while keeping the single-user local console friction low.

## Export Path

The dashboard export action does not accept an arbitrary output path. It uses the CLI default result path so scan data remains under the configured MacFriends result directory.

## Runtime Identity

`run-state.json` records the launched executable path. Stale process checks only treat a PID as live when the PID is running and its command line still contains the recorded executable identity or currently owns the recorded socket.

## Socket Directory

The Objective-C++ agent also sets the socket parent directory permissions to `0700`, matching the Rust CLI layout guard.
