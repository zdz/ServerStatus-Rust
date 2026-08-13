import { useState } from "react";
import type { StatsResponse } from "../types";
import { formatTimestamp } from "../utils/format";
import { ServerRow } from "./ServerRow";

export function ServerTable({ stats }: { stats: StatsResponse }) {
  const [expanded, setExpanded] = useState<number[]>([]);
  const toggle = (index: number) => {
    setExpanded((current) =>
      current.includes(index) ? current.filter((item) => item !== index) : [...current, index],
    );
  };

  const common = "mx-auto items-stretch gap-x-4 pl-2 text-[11px] leading-[15px] md:text-sm lg:text-base";

  return (
    <div className="mx-auto min-w-min flex-1">
      <div className="overflow-x-auto">
        {stats.servers.length > 0 ? (
          <table id="ssr-table" className="w-full items-end pt-1 text-center text-indigo-800 dark:text-white">
            <thead>
              <tr className={`${common} h-8 border-b border-indigo-100 bg-slate-100 text-center font-bold dark:border-white dark:bg-slate-700`}>
                <th>节点</th>
                <th className="hidden lg:table-cell">协议</th>
                <th className="hidden w-20 lg:table-cell">在线</th>
                <th className="w-11 min-w-min">位置</th>
                <th className="hidden w-20 lg:table-cell">类型</th>
                <th className="w-20">负载</th>
                <th className="hidden md:table-cell">日流量 ↓|↑</th>
                <th className="hidden md:table-cell">月流量 ↓|↑</th>
                <th>网络 ↓|↑</th>
                <th className="hidden md:table-cell">总流量 ↓|↑</th>
                <th>处理器</th>
                <th>内存</th>
                <th>硬盘</th>
                <th className="hidden w-44 pr-5 pl-3 xl:table-cell">联通 | 电信 | 移动</th>
              </tr>
            </thead>
            <tbody>
              {stats.servers.map((host, index) => (
                <ServerRow
                  key={`${host.name}-${index}`}
                  host={host}
                  index={index}
                  expanded={expanded.includes(index)}
                  onToggle={() => toggle(index)}
                />
              ))}
            </tbody>
          </table>
        ) : (
          <div className="mx-auto mt-8 flex min-w-min justify-center bg-transparent">
            <div className="h-8 w-8 animate-spin rounded-full border-4 border-slate-300 border-t-blue-500" aria-label="loading" />
          </div>
        )}
      </div>
      <div className="mt-2 pl-6 text-xs text-slate-400">最后更新：{formatTimestamp(stats.updated)}</div>
    </div>
  );
}
