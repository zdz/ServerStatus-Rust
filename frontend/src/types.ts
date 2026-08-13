export interface HostStat {
  name: string;
  alias: string;
  type: string;
  location: string;
  notify: boolean;
  vnstat: boolean;
  online4: boolean;
  online6: boolean;
  uptime: string;
  load_1: number;
  load_5: number;
  load_15: number;
  ping_10010: number;
  ping_189: number;
  ping_10086: number;
  time_10010: number;
  time_189: number;
  time_10086: number;
  tcp_count: number;
  udp_count: number;
  process_count: number;
  thread_count: number;
  network_rx: number;
  network_tx: number;
  network_in: number;
  network_out: number;
  last_network_in: number;
  last_network_out: number;
  daily_network_in: number;
  daily_network_out: number;
  cpu: number;
  memory_total: number;
  memory_used: number;
  swap_total: number;
  swap_used: number;
  hdd_total: number;
  hdd_used: number;
  labels: string;
  custom: string;
  gid: string;
  weight: number;
  latest_ts: number;
  si: boolean;
}

export interface StatsResponse {
  updated: number;
  servers: HostStat[];
}
