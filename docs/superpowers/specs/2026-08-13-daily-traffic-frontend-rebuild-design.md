# Daily Traffic and Default Frontend Rebuild Design

## Goal

Add an independent daily traffic column to the default ServerStatus-Rust UI and restore the bundled default React UI as maintainable source code that builds into `web/`.

## Daily traffic model

The server exposes two new JSON fields for every host:

- `daily_network_in`
- `daily_network_out`

For `vnstat=true` Rust clients, the client reads the current calendar day's traffic from vnStat and reports the two fields through newly appended protobuf fields. This preserves full-day traffic when the Server process was stopped across midnight.

For other clients, including existing Python and ESXi clients, the Server calculates daily traffic from cumulative `network_in/network_out` using a persisted daily baseline. The baseline includes the local calendar date and inbound/outbound cumulative counters. On a date change or cumulative counter reset, the Server establishes a new baseline. Arithmetic uses saturating subtraction.

Old clients remain compatible because protobuf additions use new field numbers and decode to zero. When a host does not report daily traffic through vnStat, Server-side baseline calculation supplies the values.

## Persistence

The existing `stats.json` persistence mechanism is extended to retain the daily baseline and baseline date for non-vnStat nodes. Startup restores both monthly and daily baselines when present. Existing `stats.json` files without daily fields continue to load using defaults.

## Frontend reconstruction

Create a standalone `frontend/` React + TypeScript project. Reconstruct the current default UI from the committed production bundle, preserving observable behavior and visual structure rather than minified variable names or original private source layout.

The reconstructed frontend preserves:

- one-second polling of `json/stats.json`
- light/dark behavior
- host grouping and ordering
- desktop table and mobile expanded rows
- online/offline state
- load, traffic, CPU, memory, disk, uptime and labels display
- existing static assets under `web/`

The build output remains `web/`, so Server deployment behavior stays unchanged.

## Daily traffic UI

Desktop adds an independent `日流量` column immediately before `月流量`:

`负载 | 日流量 | 月流量 | 实时流量 | 总流量 | CPU | 内存 | 硬盘 | ...`

Each traffic cell keeps separate download/upload values using the same formatting and direction semantics as the existing monthly and total traffic cells.

Mobile expanded details display daily, monthly and total traffic on separate rows.

## Compatibility

- Existing Server configuration remains valid.
- Existing Rust clients can connect; missing daily protobuf fields default to zero.
- Existing Python/ESXi clients require no update because Server-side daily baselines cover them.
- `vnstat` v1/v2 parsing behavior remains supported.
- The current `web/` deployment path and HTTP endpoints remain unchanged.

## Tests

Rust tests cover vnStat daily parsing, daily baseline rollover, counter reset handling, persistence compatibility, and protobuf defaults. Frontend tests cover traffic formatting and the presence of the daily traffic column/expanded-row labels. Production build must complete successfully and Rust workspace tests/clippy must pass.
