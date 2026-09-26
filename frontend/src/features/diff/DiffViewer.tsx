import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import type { DiffService } from "../../services/diffService";
import type { FileDiff } from "../../types/diff";
import type { FileStatus } from "../../types/repository";
import type { ResolvedTheme } from "../../types/settings";
import { statusLabel } from "../changes/statusPresentation";
import { languageForPath } from "./language";
import { useFileDiff } from "./useFileDiff";

const MonacoDiffEditor = lazy(() =>
  import("./MonacoDiffEditor").then((module) => ({
    default: module.MonacoDiffEditor,
  })),
);

interface DiffViewerProps {
  repositoryPath: string;
  file: FileStatus | null;
  service: DiffService;
  repositoryRevision?: number;
  theme?: ResolvedTheme;
}

type ViewMode = "split" | "inline";

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${String(bytes)} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function DiffFallback({ diff }: { diff: FileDiff }) {
  const details = `${formatBytes(diff.file.oldSize)} → ${formatBytes(diff.file.newSize)}`;
  const messages = {
    binary: {
      title: "Binary file",
      body: "Branchcraft detected non-text content, so a source diff is not available.",
    },
    tooLarge: {
      title: "File is too large to preview",
      body: "Files larger than 2 MiB per side are kept out of the editor to protect responsiveness.",
    },
    unsupported: {
      title: "Preview unavailable",
      body: "This change is not a regular text file and cannot be shown in Monaco.",
    },
    ready: { title: "", body: "" },
  } as const;
  const message = messages[diff.file.previewStatus];

  return (
    <div className="inspector-empty diff-fallback">
      <div className="file-glyph active">
        <Icon name="file" />
      </div>
      <h2>{message.title}</h2>
      <p>{message.body}</p>
      <span className="file-size-note">{details}</span>
    </div>
  );
}

export function DiffViewer({
  repositoryPath,
  file,
  service,
  repositoryRevision = 0,
  theme = "dark",
}: DiffViewerProps) {
  const { diff, error, loading, reload } = useFileDiff(
    service,
    repositoryPath,
    file?.path ?? null,
    repositoryRevision,
  );
  const container = useRef<HTMLDivElement>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("split");
  const [narrow, setNarrow] = useState(false);

  useEffect(() => {
    const element = container.current;
    if (element === null || typeof ResizeObserver === "undefined") {
      return undefined;
    }
    const observer = new ResizeObserver(([entry]) => {
      if (entry !== undefined) {
        setNarrow(entry.contentRect.width < 720);
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const language = useMemo(
    () => languageForPath(diff?.file.path ?? file?.path ?? ""),
    [diff?.file.path, file?.path],
  );
  const sideBySide = viewMode === "split" && !narrow;

  return (
    <section className="file-inspector" aria-label="Selected file" ref={container}>
      {file === null ? (
        <div className="inspector-empty">
          <div className="file-glyph">
            <Icon name="file" />
          </div>
          <h2>Select a changed file</h2>
          <p>Choose a file from the changes list to inspect its diff.</p>
        </div>
      ) : (
        <>
          <header className="inspector-header">
            <div>
              <strong>{file.path}</strong>
              {file.previousPath !== null && (
                <span>Renamed from {file.previousPath}</span>
              )}
            </div>
            <div className="diff-header-actions">
              <span className="change-pill">{statusLabel(file)}</span>
              <div className="view-toggle" aria-label="Diff layout">
                <button
                  type="button"
                  aria-pressed={sideBySide}
                  disabled={narrow}
                  title={narrow ? "Split view needs a wider panel" : "Side-by-side diff"}
                  onClick={() => setViewMode("split")}
                >
                  Split
                </button>
                <button
                  type="button"
                  aria-pressed={!sideBySide}
                  onClick={() => setViewMode("inline")}
                >
                  Inline
                </button>
              </div>
            </div>
          </header>

          {loading && (
            <div className="diff-loading" role="status">
              <span className="loading-spinner" />
              Loading diff…
            </div>
          )}
          {!loading && error !== null && (
            <div className="inspector-empty diff-error" role="alert">
              <span className="error-symbol">!</span>
              <h2>{error.summary}</h2>
              {error.detail !== null && <p>{error.detail}</p>}
              <button type="button" onClick={reload}>
                Try again
              </button>
            </div>
          )}
          {!loading && error === null && diff?.file.previewStatus !== "ready" && diff !== null && (
            <DiffFallback diff={diff} />
          )}
          {!loading &&
            error === null &&
            diff?.file.previewStatus === "ready" &&
            diff.file.oldContent !== null &&
            diff.file.newContent !== null && (
              <div className="diff-editor-shell">
                <Suspense
                  fallback={
                    <div className="diff-loading" role="status">
                      <span className="loading-spinner" />
                      Preparing editor…
                    </div>
                  }
                >
                  <MonacoDiffEditor
                    path={diff.file.path}
                    oldContent={diff.file.oldContent}
                    newContent={diff.file.newContent}
                    language={language}
                    sideBySide={sideBySide}
                    theme={theme}
                  />
                </Suspense>
              </div>
            )}
        </>
      )}
    </section>
  );
}
