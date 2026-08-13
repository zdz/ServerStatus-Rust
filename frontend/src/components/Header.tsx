import { useEffect, useState } from "react";

const COLOR_MODE_KEY = "chakra-ui-color-mode";
type ColorMode = "light" | "dark";

function storedColorMode(): ColorMode {
  try {
    return localStorage.getItem(COLOR_MODE_KEY) === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function applyColorMode(mode: ColorMode) {
  document.documentElement.classList.toggle("dark", mode === "dark");
  document.documentElement.style.colorScheme = mode;
  document.body.style.backgroundColor = mode === "dark" ? "#334155" : "#f8fafc";
}

function SunIcon() {
  return (
    <svg width="24" height="24" fill="none" aria-hidden="true">
      <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" fill="currentColor" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M12 4v1M18 6l-1 1M20 12h-1M18 18l-1-1M12 19v1M7 17l-1 1M5 12H4M7 7 6 6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg width="24" height="24" fill="none" aria-hidden="true">
      <path d="M18 15.63c-.977.52-1.945.481-3.13.481A6.981 6.981 0 0 1 7.89 9.13c0-1.185-.04-2.153.481-3.13C6.166 7.174 5 9.347 5 12.018A6.981 6.981 0 0 0 11.982 19c2.67 0 4.844-1.166 6.018-3.37ZM16 5c0 2.08-.96 4-3 4 2.04 0 3 .92 3 3 0-2.08.96-3 3-3-2.04 0-3-1.92-3-4Z" fill="currentColor" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function initializeColorMode() {
  applyColorMode(storedColorMode());
}

export function Header() {
  const [mode, setMode] = useState<ColorMode>(storedColorMode);

  useEffect(() => {
    document.title = "ServerStatus";
    applyColorMode(mode);
  }, [mode]);

  const toggle = () => {
    const next = mode === "light" ? "dark" : "light";
    try {
      localStorage.setItem(COLOR_MODE_KEY, next);
    } catch {
      // localStorage can be unavailable in privacy-restricted contexts.
    }
    setMode(next);
  };

  return (
    <div className="container mx-auto flex max-w-7xl flex-row items-end justify-between pt-5">
      <h1 className="pl-2 text-2xl font-bold">ServerStatus</h1>
      <div className="pr-8">
        <button type="button" onClick={toggle} aria-label="切换深浅色主题">
          {mode === "light" ? <MoonIcon /> : <SunIcon />}
        </button>
      </div>
    </div>
  );
}
