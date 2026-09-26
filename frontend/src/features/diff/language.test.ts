import { describe, expect, it } from "vitest";
import { languageForPath } from "./language";

describe("languageForPath", () => {
  it("maps common source and special filenames", () => {
    expect(languageForPath("src/main.rs")).toBe("rust");
    expect(languageForPath("frontend/App.tsx")).toBe("typescript");
    expect(languageForPath("Dockerfile")).toBe("dockerfile");
  });

  it("falls back to plaintext", () => {
    expect(languageForPath("notes.unknown-extension")).toBe("plaintext");
  });
});
