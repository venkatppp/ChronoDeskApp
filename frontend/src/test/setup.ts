// Shared vitest setup: register jest-dom matchers and Tauri API mocks.
import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";

// The `@tauri-apps/api/core` invoke is mocked per-test via explicit mocks;
// provide a default no-op so components that import it render in tests
// without a running Tauri webview.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => async () => {}),
}));