import { ErrorBanner } from "../components/ErrorBanner";
import { ChangesPanel } from "../features/changes/ChangesPanel";
import { FileInspectorPlaceholder } from "../features/changes/FileInspectorPlaceholder";
import { RepositoryHeader } from "../features/repositories/RepositoryHeader";
import { useRepository } from "../features/repositories/useRepository";
import { WelcomeView } from "../features/repositories/WelcomeView";
import {
  tauriRepositoryService,
  type RepositoryService,
} from "../services/repositoryService";

interface AppProps {
  repositoryService?: RepositoryService;
}

export function App({
  repositoryService = tauriRepositoryService,
}: AppProps) {
  const state = useRepository(repositoryService);
  const selectedFile =
    state.repository?.files.find((file) => file.path === state.selectedPath) ??
    null;

  return (
    <div className="app-shell">
      <RepositoryHeader
        repository={state.repository}
        loading={state.loading}
        onOpen={() => void state.openRepository()}
        onRefresh={() => void state.refreshRepository()}
      />

      {state.error !== null && (
        <ErrorBanner
          error={state.error}
          onDismiss={state.dismissError}
          onRetry={() => void state.openRepository()}
        />
      )}

      {state.repository === null ? (
        <WelcomeView
          loading={state.loading}
          onOpen={() => void state.openRepository()}
        />
      ) : (
        <main className="workspace">
          <ChangesPanel
            files={state.repository.files}
            selectedPath={state.selectedPath}
            onSelect={state.selectPath}
          />
          <FileInspectorPlaceholder file={selectedFile} />
        </main>
      )}

      <footer className="status-bar">
        <span>
          {state.repository?.path ?? "Open a repository to begin"}
        </span>
        <button
          aria-label="Open repository from status bar"
          type="button"
          onClick={() => void state.openRepository()}
        >
          Open Repository
        </button>
      </footer>
    </div>
  );
}

