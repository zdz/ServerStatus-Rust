import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ServerTable } from "./ServerTable";
import type { StatsResponse } from "../types";

const stats: StatsResponse = {
  updated: 1_786_608_000,
  servers: [
    {
      name: "jp01",
      alias: "JP-01",
      type: "KVM",
      location: "jp",
      notify: true,
      vnstat: true,
      online4: true,
      online6: true,
      uptime: "10 天",
      load_1: 0.12,
      load_5: 0.18,
      load_15: 0.2,
      ping_10010: 0,
      ping_189: 1,
      ping_10086: 2,
      time_10010: 45,
      time_189: 50,
      time_10086: 55,
      tcp_count: 10,
      udp_count: 3,
      process_count: 90,
      thread_count: 150,
      network_rx: 2_048,
      network_tx: 1_024,
      network_in: 10_000_000,
      network_out: 5_000_000,
      last_network_in: 7_000_000,
      last_network_out: 4_000_000,
      daily_network_in: 500_000,
      daily_network_out: 200_000,
      cpu: 20,
      memory_total: 1_048_576,
      memory_used: 524_288,
      swap_total: 0,
      swap_used: 0,
      hdd_total: 20_000,
      hdd_used: 10_000,
      labels: "os=ubuntu;spec=2C2G",
      custom: "",
      gid: "",
      weight: 0,
      latest_ts: 1_786_608_000,
      si: false,
    },
  ],
};

describe("ServerTable", () => {
  it("places daily traffic before monthly traffic", () => {
    const html = renderToStaticMarkup(<ServerTable stats={stats} />);
    expect(html).toContain("日流量 ↓|↑");
    expect(html.indexOf("日流量 ↓|↑")).toBeLessThan(html.indexOf("月流量 ↓|↑"));
  });

  it("renders separate daily, monthly and total traffic detail labels", () => {
    const html = renderToStaticMarkup(<ServerTable stats={stats} />);
    expect(html).toContain("日流量:");
    expect(html).toContain("月流量:");
    expect(html).toContain("总流量:");
  });
});
