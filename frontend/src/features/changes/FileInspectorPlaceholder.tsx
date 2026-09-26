import { Icon } from "../../components/Icon";
import type { FileStatus } from "../../types/repository";
import { statusLabel } from "./statusPresentation";

interface FileInspectorPlaceholderProps {
  file: FileStatus | null;
}

export function FileInspectorPlaceholder({ file }: FileInspectorPlaceholderProps) {
  return (
    <section className="file-inspector" aria-label="Selected file">
      {file === null ? (
        <div className="inspector-empty">
          <div className="file-glyph">
            <Icon name="file" />
          </div>
          <h2>Select a changed file</h2>
          <p>Choose a file from the changes list to inspect it.</p>
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
            <span className="change-pill">{statusLabel(file)}</span>
          </header>
          <div className="inspector-empty">
            <div className="file-glyph active">
              <Icon name="file" />
            </div>
            <h2>Diff viewer arrives next</h2>
            <p>
              Branchcraft has identified this change. Monaco diff rendering is
              intentionally scoped to Milestone 2.
            </p>
          </div>
        </>
      )}
    </section>
  );
}

