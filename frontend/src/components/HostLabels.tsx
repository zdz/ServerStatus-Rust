import type { HostStat } from "../types";

const OS_FILES: Record<string, string> = {
  centos: "centos.svg",
  debian: "debian.svg",
  ubuntu: "ubuntu.svg",
  freebsd: "freebsd.svg",
  pi: "raspberry.svg",
  arch: "archlinux.svg",
  macos: "macos.svg",
  windows: "window.svg",
  android: "android.svg",
  linux: "linux.svg",
  alpine: "linux.svg",
};

function parseLabels(labels: string): Map<string, string> {
  const parsed = new Map<string, string>();
  for (const item of labels.split(";")) {
    if (!item) continue;
    const separator = item.indexOf("=");
    if (separator <= 0) continue;
    parsed.set(item.slice(0, separator), item.slice(separator + 1));
  }
  return parsed;
}

function ServerIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5 text-amber-500 dark:text-yellow-400" fill="none" stroke="currentColor" strokeWidth="1.5">
      <rect x="3" y="4" width="18" height="6" rx="2" />
      <rect x="3" y="14" width="18" height="6" rx="2" />
      <circle cx="17" cy="7" r="0.8" fill="currentColor" />
      <circle cx="17" cy="17" r="0.8" fill="currentColor" />
    </svg>
  );
}

function ChipIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="1.5">
      <rect x="6" y="6" width="12" height="12" rx="2" />
      <path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4" />
    </svg>
  );
}

function CalendarIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="1.5">
      <rect x="3" y="5" width="18" height="16" rx="2" />
      <path d="M7 2v6M17 2v6M3 10h18" />
    </svg>
  );
}

export function HostLabels({ host }: { host: HostStat }) {
  const labels = parseLabels(host.labels ?? "");
  const os = labels.get("os") ?? "";
  const osFile = OS_FILES[os] ?? OS_FILES.linux;

  return (
    <div className="flex items-center justify-center space-x-1">
      {os && <img className="h-5 w-5 rounded" src={`static/os/${osFile}`} alt={os} />}
      {host.gid && (
        <>
          <ServerIcon />
          <span>{host.gid}</span>
        </>
      )}
      {labels.has("spec") && (
        <>
          <ChipIcon />
          <span>{labels.get("spec")}</span>
        </>
      )}
      {labels.has("ndd") && (
        <>
          <CalendarIcon />
          <span>{labels.get("ndd")}</span>
        </>
      )}
    </div>
  );
}
