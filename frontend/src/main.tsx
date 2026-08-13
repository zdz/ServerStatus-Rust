import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { initializeColorMode } from "./components/Header";
import "./index.css";

initializeColorMode();

const root = document.getElementById("root");
if (!root) {
  throw new Error("root element not found");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
