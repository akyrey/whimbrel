import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";

async function start() {
  // Playwright runs the frontend in a plain browser against a mocked backend. The condition is
  // a build-time constant, so the mock is not bundled in real builds.
  if (import.meta.env.VITE_E2E === "true") {
    const { installMockBackend, externalEdit } = await import("./test/mockBackend");
    installMockBackend();
    // Playwright simulates another program changing a file through this.
    Object.assign(window, { whimbrelMock: { externalEdit } });
  }
  const root = document.getElementById("root");
  if (!root) return;
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void start();
