import { describe, expect, it } from "vitest";
import { gitFilename, presentGitPath } from "./gitPath";

describe("Git-relative path presentation", () => {
  it("presents nested paths containing spaces, Unicode, and punctuation", () => {
    expect(presentGitPath("Mixed Case/Гілка/résumé + [draft]!.txt")).toEqual({
      filename: "résumé + [draft]!.txt",
      parent: "Mixed Case/Гілка",
    });
  });

  it("uses a dot parent for a root-level Git path", () => {
    expect(presentGitPath("Cargo.toml")).toEqual({
      filename: "Cargo.toml",
      parent: ".",
    });
  });

  it("does not reinterpret a backslash as a Windows separator", () => {
    expect(gitFilename(String.raw`literal\name.rs`)).toBe(
      String.raw`literal\name.rs`,
    );
  });
});
