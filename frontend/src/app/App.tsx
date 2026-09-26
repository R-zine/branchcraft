import { ErrorBanner } from "../components/ErrorBanner";
import { ChangesPanel } from "../features/changes/ChangesPanel";
import { useStaging } from "../features/changes/useStaging";
import { CommitForm } from "../features/commit/CommitForm";
import { DiffViewer } from "../features/diff/DiffViewer";
import { RepositoryHeader } from "../features/repositories/RepositoryHeader";
import { useRepository } from "../features/repositories/useRepository";
import { WelcomeView } from "../features/repositories/WelcomeView";
import { tauriCommitService, type CommitService } from "../services/commitService";
import { tauriDiffService, type DiffService } from "../services/diffService";
import {
  tauriRepositoryService,
  type RepositoryService,
} from "../services/repositoryService";
import {
  tauriStagingService,
  type StagingService,
} from "../services/stagingService";

interface AppProps {
  repositoryService?: RepositoryService;
  diffService?: DiffService;
  stagingService?: StagingService;
  commitService?: CommitService;
}

export function App({
  repositoryService = tauriRepositoryService,
  diffService = tauriDiffService,
  stagingService = tauriStagingService,
  commitService = tauriCommitService,
}: AppProps) {
  const state = useRepository(repositoryService);
  const staging = useStaging(
    stagingService,
    state.repository?.path ?? null,
    state.updateRepository,
  );
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
          <div className="changes-column">
            <ChangesPanel
              files={state.repository.files}
              selectedPath={state.selectedPath}
              busy={staging.busy}
              pendingPath={staging.pendingPath}
              error={staging.error}
              onSelect={state.selectPath}
              onStageFile={(path) => void staging.stageFile(path)}
              onUnstageFile={(path) => void staging.unstageFile(path)}
              onStageAll={() => void staging.stageAll()}
              onUnstageAll={() => void staging.unstageAll()}
              onDismissError={staging.dismissError}
            />
            <CommitForm
              key={state.repository.id}
              repository={state.repository}
              service={commitService}
              disabled={staging.busy}
              onCommitted={state.updateRepository}
            />
          </div>
          <DiffViewer
            repositoryPath={state.repository.path}
            file={selectedFile}
            service={diffService}
            repositoryRevision={state.revision}
          />
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

