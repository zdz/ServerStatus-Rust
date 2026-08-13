import type { HostStat } from "../types";
import { formatBytes } from "../utils/format";
import { HostLabels } from "./HostLabels";
import { LocationFlag } from "./LocationFlag";
import { PacketLoss } from "./PacketLoss";
import { StatusDot } from "./StatusDot";
import { TrafficCell } from "./TrafficCell";
import { UsageBar } from "./UsageBar";

interface ServerRowProps {
  host: HostStat;
  index: number;
  expanded: boolean;
  onToggle: () => void;
}

function OfflineIcon() {
  return (
    <div className="flex justify-center">
      <svg viewBox="0 0 24 24" className="h-6 w-6 text-red-400" fill="none" stroke="currentColor" strokeWidth="2" aria-label="offline">
        <circle cx="12" cy="12" r="9" />
        <path d="m8 8 8 8M16 8l-8 8" />
      </svg>
    </div>
  );
}

export function ServerRow({ host, index, expanded, onToggle }: ServerRowProps) {
  const online = host.online4 || host.online6;
  const dailyIn = host.daily_network_in ?? 0;
  const dailyOut = host.daily_network_out ?? 0;
  const common = "mx-auto items-stretch gap-x-4 pl-2 text-[11px] leading-[15px] md:text-sm lg:text-base";
  const border = index > 0
    ? "border-t border-dashed border-slate-400/75 font-normal dark:border-solid dark:border-white"
    : "";
  const detailVisible = online && expanded;

  return (
    <>
      <tr
        id={`r-${index}`}
        onClick={onToggle}
        className={`${common} h-[40px] ${border} cursor-pointer bg-slate-50 hover:bg-indigo-50 dark:bg-slate-700 dark:hover:bg-slate-600`}
      >
        <td className="w-20 min-w-min">
          <div className="mx-auto max-w-xs truncate pr-1">{host.alias}</div>
        </td>
        <td className="hidden lg:table-cell">
          <div className="flex justify-center">
            <StatusDot online={host.online4} label="IPv4" />
            <StatusDot online={host.online6} label="IPv6" />
          </div>
        </td>
        <td className="hidden w-24 min-w-min lg:table-cell">{online ? host.uptime : <OfflineIcon />}</td>
        <td className="w-11 min-w-min"><LocationFlag location={host.location} /></td>
        <td className="hidden w-20 min-w-min lg:table-cell">{online ? host.type : "-"}</td>
        <td className="w-20 min-w-min">{online ? host.load_1.toFixed(2) : "-"}</td>
        <td className="hidden w-32 min-w-[8rem] md:table-cell">
          <TrafficCell host={host} download={dailyIn} upload={dailyOut} />
        </td>
        <td className="hidden w-36 min-w-[9rem] md:table-cell">
          <TrafficCell
            host={host}
            download={host.network_in - host.last_network_in}
            upload={host.network_out - host.last_network_out}
          />
        </td>
        <td className="w-44 min-w-min md:w-36 md:min-w-[9rem]">
          <TrafficCell host={host} download={host.network_rx} upload={host.network_tx} />
        </td>
        <td className="hidden w-36 min-w-[9rem] md:table-cell">
          <TrafficCell host={host} download={host.network_in} upload={host.network_out} />
        </td>
        <td className="w-24 pl-2"><UsageBar host={host} kind="cpu" /></td>
        <td className="w-24 pl-2"><UsageBar host={host} kind="mem" /></td>
        <td className="w-24 pr-px pl-2 lg:pr-0"><UsageBar host={host} kind="hdd" /></td>
        <td className="hidden w-44 pr-2 pl-2 xl:table-cell"><PacketLoss host={host} /></td>
      </tr>
      <tr
        id={`r-${index}-expand`}
        className={`${common} font-mono font-normal transition-all duration-500 ease-out md:duration-[450ms] ${detailVisible ? "visible" : "collapse"}`}
      >
        <td colSpan={14}>
          <div
            className={`grid w-11/12 grid-cols-12 items-center space-x-1 overflow-hidden text-center transition-all duration-500 ease-out md:duration-[450ms] ${detailVisible ? "max-h-64" : "max-h-0"}`}
          >
            <div className="col-start-1 col-end-7 text-right">负载:</div>
            <div className="col-start-7 col-end-13 text-left">
              {host.load_1.toFixed(2)}/{host.load_5.toFixed(2)}/{host.load_15.toFixed(2)}
            </div>
            <div className="col-start-1 col-end-7 text-right">内存:</div>
            <div className="col-start-7 col-end-13 text-left">
              {formatBytes(1024 * host.memory_used)}/{formatBytes(1024 * host.memory_total)}
            </div>
            <div className="col-start-1 col-end-7 text-right">交换分区:</div>
            <div className="col-start-7 col-end-13 text-left">
              {formatBytes(host.swap_used * (host.si ? 1000 : 1024), 2, host.si)}/
              {formatBytes(host.swap_total * (host.si ? 1000 : 1024), 2, host.si)}
            </div>
            <div className="col-start-1 col-end-7 text-right">硬盘:</div>
            <div className="col-start-7 col-end-13 text-left">
              {formatBytes(host.hdd_used * (host.si ? 1_000_000 : 1_048_576), 2, host.si)}/
              {formatBytes(host.hdd_total * (host.si ? 1_000_000 : 1_048_576), 2, host.si)}
            </div>
            <div className="col-start-1 col-end-7 text-right">TCP/UDP/进/线:</div>
            <div className="col-start-7 col-end-13 text-left">
              {host.tcp_count}/{host.udp_count}/{host.process_count}/{host.thread_count}
            </div>
            <div className="col-start-1 col-end-7 text-right">时延(联/电/移):</div>
            <div className="col-start-7 col-end-13 text-left">
              {host.time_10010}ms/{host.time_189}ms/{host.time_10086}ms
            </div>
            <div className="col-start-1 col-end-7 text-right xl:hidden">丢包(联/电/移):</div>
            <div className="col-start-7 col-end-13 text-left xl:hidden">
              {host.ping_10010.toFixed(0)}%/{host.ping_189.toFixed(0)}%/{host.ping_10086.toFixed(0)}%
            </div>
            <div className="col-start-1 col-end-7 text-right md:hidden">日流量:</div>
            <div className="col-start-7 col-end-13 text-left md:hidden">
              <TrafficCell host={host} download={dailyIn} upload={dailyOut} compact />
            </div>
            <div className="col-start-1 col-end-7 text-right md:hidden">月流量:</div>
            <div className="col-start-7 col-end-13 text-left md:hidden">
              <TrafficCell
                host={host}
                download={host.network_in - host.last_network_in}
                upload={host.network_out - host.last_network_out}
                compact
              />
            </div>
            <div className="col-start-1 col-end-7 text-right md:hidden">总流量:</div>
            <div className="col-start-7 col-end-13 text-left md:hidden">
              <TrafficCell host={host} download={host.network_in} upload={host.network_out} compact />
            </div>
            <div className="col-start-2 col-end-13 flex items-center justify-center space-x-1">
              <div className="text-red-400 dark:text-white">#{index}</div>
              <div className="h-5 border-l border-slate-300 dark:border-slate-500" />
              <HostLabels host={host} />
            </div>
          </div>
        </td>
      </tr>
    </>
  );
}
