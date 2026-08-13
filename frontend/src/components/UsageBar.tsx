import type { HostStat } from "../types";

type UsageKind = "cpu" | "mem" | "hdd";

interface UsageBarProps {
  host: HostStat;
  kind: UsageKind;
}

function percentage(host: HostStat, kind: UsageKind): number {
  switch (kind) {
    case "cpu":
      return Math.round(host.cpu);
    case "mem":
      return host.memory_total > 0 ? Math.round((100 * host.memory_used) / host.memory_total) : -1;
    case "hdd":
      return host.hdd_total > 0 ? Math.round((100 * host.hdd_used) / host.hdd_total) : -1;
  }
}

export function UsageBar({ host, kind }: UsageBarProps) {
  if (!host.online4 && !host.online6) {
    return (
      <div className="flex h-5 items-center justify-center bg-red-400 text-sm font-normal text-indigo-800">
        -
      </div>
    );
  }

  const raw = percentage(host, kind);
  const value = raw < 0 ? 100 : Math.min(raw, 100);
  const label = raw < 0 ? "X" : `${raw}%`;
  const fillClass = raw > 90 ? "bg-red-400" : raw > 80 ? "bg-yellow-400" : "bg-indigo-400";

  return (
    <div className="relative flex h-5 w-full bg-slate-100 text-[11px] leading-[15px] lg:text-base">
      <div
        className={`h-full ${fillClass}`}
        style={{ width: `${value}%`, transition: "width 1s" }}
      />
      <div className="absolute top-1/2 left-1/4 -mt-2 text-center text-[11px] leading-[15px] font-normal text-indigo-800 md:left-1/3 md:text-xs">
        {label}
      </div>
    </div>
  );
}
