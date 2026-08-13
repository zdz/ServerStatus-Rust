import type { StatsResponse } from "../types";

export async function fetchStats(): Promise<StatsResponse> {
  const response = await fetch("json/stats.json");
  if (!response.ok) {
    throw new Error(`stats request failed: ${response.status}`);
  }
  return response.json() as Promise<StatsResponse>;
}
