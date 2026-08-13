import type { HostStat } from "../types";
import { formatTrafficPair } from "../utils/format";

interface TrafficCellProps {
  host: HostStat;
  download: number;
  upload: number;
  compact?: boolean;
}

export function TrafficCell({ host, download, upload, compact = false }: TrafficCellProps) {
  const pair = formatTrafficPair(host, download, upload);
  if (!pair) {
    return <span>-</span>;
  }

  if (compact) {
    return <>{pair[0]}|{pair[1]}</>;
  }

  return (
    <div className="inline-grid grid-cols-11 items-center space-x-1">
      <div className="col-span-5 min-w-min text-right">{pair[0]}</div>
      <div>|</div>
      <div className="col-span-5 min-w-min text-left">{pair[1]}</div>
    </div>
  );
}
