import { describe, expect, it } from "vitest";
import { formatBytes, formatTrafficPair } from "./format";

const onlineHost = { online4: true, online6: false, si: false };

describe("formatBytes", () => {
  it("uses binary units by default", () => {
    expect(formatBytes(1024, 1, false)).toBe("1.0K");
    expect(formatBytes(1024 * 1024, 1, false)).toBe("1.0M");
  });

  it("uses SI units when requested", () => {
    expect(formatBytes(1000, 1, true)).toBe("1.0K");
    expect(formatBytes(1_000_000, 1, true)).toBe("1.0M");
  });
});

describe("formatTrafficPair", () => {
  it("keeps download and upload separate and clamps negative values", () => {
    expect(formatTrafficPair(onlineHost, -1, 2048)).toEqual(["0.0B", "2.0K"]);
  });

  it("returns no values for an offline host", () => {
    expect(formatTrafficPair({ ...onlineHost, online4: false }, 100, 200)).toBeNull();
  });
});
