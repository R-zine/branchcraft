import { Icon } from "../../components/Icon";
import type { FileStatus } from "../../types/repository";
import { statusCode, statusLabel, statusTone } from "./statusPresentation";

interface ChangesPanelProps {
  files: FileStatus[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

export function ChangesPanel({
  files,
  selectedPath,
  onSelect,
}: ChangesPanelProps) {
  return (
    <aside className="changes-panel" aria-label="Working tree changes">
      <div className="panel-heading">
        <div>
          <h2>Changes</h2>
          <span>
            {files.length} {files.length === 1 ? "file" : "files"}
          </span>
        </div>
      </div>

      {files.length === 0 ? (
        <div className="clean-state">
          <span className="clean-check">✓</span>
          <strong>No local changes</strong>
          <span>Your working tree is clean.</span>
        </div>
      ) : (
        <ul className="file-list">
          {files.map((file) => (
            <li key={file.path}>
              <button
                className={file.path === selectedPath ? "selected" : undefined}
                type="button"
                onClick={() => onSelect(file.path)}
              >
                <span className={`status-code ${statusTone(file)}`}>
                  {statusCode(file)}
                </span>
                <Icon name="file" />
                <span className="file-copy">
                  <strong>{file.path.split("/").at(-1)}</strong>
                  <span>{file.path.includes("/") ? file.path.slice(0, file.path.lastIndexOf("/")) : "."}</span>
                </span>
                <span className="visually-hidden">{statusLabel(file)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}

