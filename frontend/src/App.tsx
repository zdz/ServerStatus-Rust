import { useEffect, useState } from "react";
import { fetchStats } from "./api/stats";
import { Footer } from "./components/Footer";
import { Header } from "./components/Header";
import { ServerTable } from "./components/ServerTable";
import type { StatsResponse } from "./types";

const EMPTY_STATS: StatsResponse = { servers: [], updated: 0 };

export default function App() {
  const [stats, setStats] = useState<StatsResponse>(EMPTY_STATS);

  useEffect(() => {
    const refresh = () => {
      void fetchStats()
        .then(setStats)
        .catch((error: unknown) => console.log("错误:", error));
    };

    refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <div id="app" className="grid h-full w-full bg-transparent subpixel-antialiased dark:bg-slate-700">
      <div id="header" className="h-[80px] w-full bg-slate-700 text-green-400 md:h-[100px]">
        <Header />
      </div>
      <div id="body" className="container mx-auto -mt-[20px] h-auto w-full max-w-7xl dark:bg-slate-700 md:-mt-[32px]">
        <ServerTable stats={stats} />
      </div>
      <Footer />
    </div>
  );
}
