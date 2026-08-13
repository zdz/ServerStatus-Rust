import type { HostStat } from "../types";

export function PacketLoss({ host }: { host: HostStat }) {
  if (!host.online4 && !host.online6) {
    return (
      <div className="flex h-5 items-center justify-evenly bg-red-400 text-sm font-normal text-indigo-800">
        <span>-</span><span>-</span><span>-</span>
      </div>
    );
  }

  const maxLoss = Math.max(host.ping_10010, host.ping_189, host.ping_10086);
  const background = maxLoss > 70 ? "bg-red-400" : maxLoss > 20 ? "bg-yellow-400" : "bg-emerald-400";

  return (
    <div className={`h-5 w-full pr-2 text-sm lg:text-base ${background}`}>
      <div className="flex h-full w-full items-center justify-center space-x-1 pt-0.5 text-center text-[11px] leading-[15px] font-normal text-indigo-800 dark:text-indigo-900">
        <span>{host.ping_10010.toFixed(0)}%</span>
        <span className="text-amber-500">◆</span>
        <span>{host.ping_189.toFixed(0)}%</span>
        <span className="text-amber-500">◆</span>
        <span>{host.ping_10086.toFixed(0)}%</span>
      </div>
    </div>
  );
}
