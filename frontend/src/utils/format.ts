export interface TrafficHostState {
  online4: boolean;
  online6: boolean;
  si: boolean;
}

const UNITS = ["B", "K", "M", "G", "T", "P"] as const;

export function formatBytes(value: number, decimals = 2, si = false): string {
  const base = si ? 1000 : 1024;
  for (let index = UNITS.length - 1; index >= 1; index -= 1) {
    const divisor = base ** index;
    if (value >= divisor) {
      return `${(value / divisor).toFixed(decimals)}${UNITS[index]}`;
    }
  }
  return `${value.toFixed(decimals)}B`;
}

export function formatTrafficPair(
  host: TrafficHostState,
  download: number,
  upload: number,
): [string, string] | null {
  if (!host.online4 && !host.online6) {
    return null;
  }

  return [
    formatBytes(Math.max(download, 0), 1, host.si),
    formatBytes(Math.max(upload, 0), 1, host.si),
  ];
}

export function formatTimestamp(timestampSeconds: number): string {
  const date = new Date(timestampSeconds * 1000);
  const pad = (value: number) => `${value}`.padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}
