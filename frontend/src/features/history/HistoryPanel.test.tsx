import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { HistoryService } from "../../services/historyService";
import { HistoryPanel } from "./HistoryPanel";

describe("HistoryPanel", () => {
  it("renders commit metadata in newest-first service order", async () => {
    const getHistory = vi.fn<HistoryService["getHistory"]>().mockResolvedValue({
      commits: [
        {
          id: "1234567890abcdef",
          shortId: "12345678",
          summary: "Add branch picker",
          description: "Keep branch operations safe.",
          authorName: "Branchcraft User",
          authorEmail: "user@branchcraft.local",
          authoredAt: 1_700_000_000,
          authorOffsetMinutes: 0,
          parentIds: ["abcdef"],
        },
      ],
      truncated: false,
    });

    render(
      <HistoryPanel
        repositoryPath="/work/branchcraft"
        repositoryRevision={0}
        service={{ getHistory }}
      />,
    );

    expect(await screen.findByText("Add branch picker")).toBeDefined();
    expect(screen.getByText("Keep branch operations safe.")).toBeDefined();
    expect(screen.getByText("12345678")).toBeDefined();
    expect(getHistory).toHaveBeenCalledWith("/work/branchcraft");
  });

  it("shows an empty repository state", async () => {
    const getHistory = vi.fn<HistoryService["getHistory"]>().mockResolvedValue({
      commits: [],
      truncated: false,
    });

    render(
      <HistoryPanel
        repositoryPath="/work/branchcraft"
        repositoryRevision={0}
        service={{ getHistory }}
      />,
    );

    expect(await screen.findByText("No commits yet")).toBeDefined();
  });
});
