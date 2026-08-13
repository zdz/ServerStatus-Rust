# Daily Traffic and Default Frontend Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add high-precision daily traffic reporting and reconstruct the bundled default UI as maintainable React/TypeScript source with an independent `日流量` column.

**Architecture:** vnStat-enabled Rust clients report current-day traffic through two new protobuf fields; the Server computes daily traffic from persisted cumulative-counter baselines for all other clients. A new `frontend/` source project rebuilds the current observable UI and emits static assets into `web/`, preserving the existing deployment path.

**Tech Stack:** Rust, tonic/prost protobuf, serde/chrono, React, TypeScript, Vite, Tailwind CSS.

## Global Constraints

- Work only on branch `feat/daily-traffic-frontend-rebuild`.
- Preserve existing Server configuration and deployment paths.
- Preserve protobuf backward compatibility by appending field numbers only.
- Preserve vnStat v1/v2 support.
- Existing Python/ESXi clients must work unchanged.
- Desktop shows an independent `日流量` column before `月流量`.
- Mobile shows 日流量/月流量/总流量 on separate detail rows.
- `web/` remains the production static directory.

---

### Task 1: Daily traffic protocol and vnStat reporting

**Files:**
- Modify: `common/proto/server_status.proto`
- Modify: `client/src/vnstat.rs`
- Modify: `client/src/status.rs`

**Interfaces:**
- Produces: `StatRequest.daily_network_in: u64`, `StatRequest.daily_network_out: u64`
- Produces: `get_traffic(args) -> Result<(u64, u64, u64, u64, u64, u64)>`

- [ ] **Step 1: Extend vnStat parsing tests with current-day fixtures**

Add assertions that a day record matching the selected date contributes to daily rx/tx and that interface skipping still applies.

- [ ] **Step 2: Run the vnStat tests and verify the new assertions fail before implementation**

Run: `cargo test -p stat_client vnstat -- --nocapture`

- [ ] **Step 3: Append protobuf fields**

```proto
uint64 daily_network_in = 47;
uint64 daily_network_out = 48;
```

- [ ] **Step 4: Extend vnStat calculation**

Return `(total_in, total_out, month_in, month_out, day_in, day_out)` and sum the current calendar day from `days` for v1 or `day` for v2. Preserve the existing 1024 conversion for vnStat JSON v1.

- [ ] **Step 5: Populate the new request fields in `client/src/status.rs`**

For `args.vnstat`, assign the returned daily values directly to `stat.daily_network_in/out` while keeping current monthly-baseline behavior.

- [ ] **Step 6: Run client tests**

Run: `cargo test -p stat_client`

---

### Task 2: Server daily baseline and persistence

**Files:**
- Modify: `server/src/config.rs`
- Modify: `server/src/payload.rs`
- Modify: `server/src/stats.rs`

**Interfaces:**
- Produces JSON fields: `daily_network_in`, `daily_network_out`
- Maintains runtime baseline: cumulative in/out plus local date for non-vnStat hosts

- [ ] **Step 1: Add focused tests for baseline arithmetic**

Cover same-day subtraction, date rollover, cumulative-counter reset, and old persisted JSON without daily fields.

- [ ] **Step 2: Run server tests and verify the new cases fail**

Run: `cargo test -p stat_server`

- [ ] **Step 3: Add HostStat daily output fields**

```rust
#[serde(default)]
pub daily_network_in: u64,
#[serde(default)]
pub daily_network_out: u64,
```

- [ ] **Step 4: Add non-serialized/config-safe baseline state to Host**

Store inbound/outbound daily baseline and a date key such as `YYYY-MM-DD`; use serde defaults so existing config remains valid.

- [ ] **Step 5: Extend `load_last_network` persistence restore**

Restore daily baseline/date when fields exist; accept existing `stats.json` files where they are absent.

- [ ] **Step 6: Compute daily traffic for non-vnStat hosts**

On first sample, date change, or counter regression, establish a fresh baseline. Otherwise set:

```rust
stat_t.daily_network_in = stat_t.network_in.saturating_sub(info.daily_network_in_base);
stat_t.daily_network_out = stat_t.network_out.saturating_sub(info.daily_network_out_base);
```

For `vnstat=true`, preserve the Client-reported daily values.

- [ ] **Step 7: Ensure daily baselines are emitted in the persisted stats snapshot**

Keep the existing 60-second persistence cadence and startup recovery behavior.

- [ ] **Step 8: Run server tests**

Run: `cargo test -p stat_server`

---

### Task 3: Reconstruct maintainable default frontend source

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/tsconfig.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tailwind.config.js`
- Create: `frontend/postcss.config.js`
- Create: `frontend/index.html`
- Create: `frontend/src/main.tsx`
- Create: `frontend/src/App.tsx`
- Create: `frontend/src/index.css`
- Create: `frontend/src/types.ts`
- Create: `frontend/src/api/stats.ts`
- Create: `frontend/src/utils/format.ts`
- Create: `frontend/src/components/Header.tsx`
- Create: `frontend/src/components/ServerTable.tsx`
- Create: `frontend/src/components/ServerRow.tsx`
- Create: `frontend/src/components/TrafficCell.tsx`
- Create focused supporting components only where required by the reconstructed bundle behavior.

**Interfaces:**
- Consumes: `json/stats.json`
- Consumes: `HostStat.daily_network_in/out`
- Produces: production static build under `web/`

- [ ] **Step 1: Extract observable UI behavior from the existing bundle**

Record table headers, responsive breakpoints, polling interval, row expansion behavior, traffic formatting semantics, status indicators, labels, flags, usage bars, light/dark behavior and footer/header text.

- [ ] **Step 2: Scaffold React/TypeScript/Vite/Tailwind source**

Pin dependency versions in `package.json`; configure Vite `outDir: "../web"`, `emptyOutDir: false`, and root-relative asset paths compatible with current deployment.

- [ ] **Step 3: Define typed stats API**

`fetchStats()` fetches `json/stats.json`; `App` refreshes every 1000 ms and cleans up the interval on unmount.

- [ ] **Step 4: Rebuild traffic formatting helpers**

Preserve SI-vs-binary behavior from the host `si` flag, direction indicators, rate suffixes, and compact values.

- [ ] **Step 5: Rebuild desktop table**

Match the current table ordering and responsive visibility. Insert `日流量` immediately before `月流量`, with separate inbound/outbound values.

- [ ] **Step 6: Rebuild mobile expanded details**

Preserve row expansion and show separate 日流量, 月流量 and 总流量 rows.

- [ ] **Step 7: Rebuild remaining visible default-theme behavior**

Restore online/offline state, OS/location display, load, CPU/memory/disk bars, uptime, labels, grouping/order, header/footer and light/dark presentation using existing assets.

- [ ] **Step 8: Add frontend tests for formatting and daily labels**

Use lightweight unit/component tests only for logic that can regress independently.

---

### Task 4: Production build integration and verification

**Files:**
- Modify generated assets under: `web/`
- Modify: `README.md` only if a build-from-source instruction is required for maintainers

**Interfaces:**
- Produces: Server-embeddable static site under `web/`

- [ ] **Step 1: Install dependencies from declared lockfile/package metadata**

Use the local package manager normally; do not execute arbitrary downloaded scripts outside package lifecycle requirements.

- [ ] **Step 2: Run frontend tests**

Run the package test command declared in `frontend/package.json`.

- [ ] **Step 3: Build frontend into `web/`**

Run the package build command and verify `web/index.html` references the new generated assets.

- [ ] **Step 4: Run Rust formatting and tests**

Run: `cargo fmt --all -- --check`

Run: `cargo test --workspace`

Run: `cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 5: Inspect the final diff**

Verify no unrelated refactor, no generated secrets, no accidental config change, protobuf field numbers are append-only, and the deployment path remains `web/`.

- [ ] **Step 6: Record deployment files**

Deployment must include the rebuilt Server binary, updated Rust Client binary for vnStat hosts that need exact daily traffic, and the `web/` directory. Existing Python/ESXi clients can remain unchanged.
