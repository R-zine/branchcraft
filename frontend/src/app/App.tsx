import { useCallback, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { AccountMenu } from "../features/auth/AccountMenu";
import { BranchPicker } from "../features/branches/BranchPicker";
import { CloneView } from "../features/cloning/CloneView";
import { ChangesPanel } from "../features/changes/ChangesPanel";
import { useStaging } from "../features/changes/useStaging";
import { CommitForm } from "../features/commit/CommitForm";
import { DiffViewer } from "../features/diff/DiffViewer";
import { HistoryPanel } from "../features/history/HistoryPanel";
import { RepositoryHeader } from "../features/repositories/RepositoryHeader";
import { RemoteControls } from "../features/remotes/RemoteControls";
import { useRepository } from "../features/repositories/useRepository";
import { WelcomeView } from "../features/repositories/WelcomeView";
import { SettingsPanel } from "../features/settings/SettingsPanel";
import { useKeyboardShortcuts } from "../features/settings/useKeyboardShortcuts";
import { useSettings } from "../features/settings/useSettings";
import { tauriAuthService, type AuthService } from "../services/authService";
import { tauriCommitService, type CommitService } from "../services/commitService";
import { tauriCloneService, type CloneService } from "../services/cloneService";
import { tauriBranchService, type BranchService } from "../services/branchService";
import { tauriDiffService, type DiffService } from "../services/diffService";
import { tauriHistoryService, type HistoryService } from "../services/historyService";
import {
  tauriGitHubRepositoryService,
  type GitHubRepositoryService,
} from "../services/githubRepositoryService";
import {
  tauriRepositoryService,
  type RepositoryService,
} from "../services/repositoryService";
import { tauriRemoteService, type RemoteService } from "../services/remoteService";
import {
  tauriSettingsService,
  type SettingsService,
} from "../services/settingsService";
import {
  tauriStagingService,
  type StagingService,
} from "../services/stagingService";

interface AppProps {
  authService?: AuthService;
  repositoryService?: RepositoryService;
  diffService?: DiffService;
  stagingService?: StagingService;
  commitService?: CommitService;
  branchService?: BranchService;
  historyService?: HistoryService;
  remoteService?: RemoteService;
  cloneService?: CloneService;
  githubRepositoryService?: GitHubRepositoryService;
  settingsService?: SettingsService;
}

export function App({
  authService = tauriAuthService,
  repositoryService = tauriRepositoryService,
  diffService = tauriDiffService,
  stagingService = tauriStagingService,
  commitService = tauriCommitService,
  branchService = tauriBranchService,
  historyService = tauriHistoryService,
  remoteService = tauriRemoteService,
  cloneService = tauriCloneService,
  githubRepositoryService = tauriGitHubRepositoryService,
  settingsService = tauriSettingsService,
}: AppProps) {
  const [workspaceView, setWorkspaceView] = useState<"changes" | "history">(
    "changes",
  );
  const [startView, setStartView] = useState<"welcome" | "clone">("welcome");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const state = useRepository(repositoryService);
  const settings = useSettings(settingsService);
  const staging = useStaging(
    stagingService,
    state.repository?.path ?? null,
    state.updateRepository,
  );
  const selectedFile =
    state.repository?.files.find((file) => file.path === state.selectedPath) ??
    null;
  const openRepository = useCallback(() => {
    setStartView("welcome");
    void state.openRepository();
  }, [state.openRepository]);
  const openRecentRepository = useCallback(
    (path: string) => {
      setStartView("welcome");
      void state.openPath(path);
    },
    [state.openPath],
  );
  const showClone = useCallback(() => setStartView("clone"), []);
  const refreshRepository = useCallback(() => {
    void state.refreshRepository();
  }, [state.refreshRepository]);
  const showChanges = useCallback(() => setWorkspaceView("changes"), []);
  const showHistory = useCallback(() => setWorkspaceView("history"), []);
  const openSettings = useCallback(() => setSettingsOpen(true), []);
  const closeSettings = useCallback(() => setSettingsOpen(false), []);

  useKeyboardShortcuts({
    openRepository,
    cloneRepository: showClone,
    refreshRepository,
    showChanges,
    showHistory,
    openSettings,
    closeSettings,
    settingsOpen,
  });

  return (
    <div className="app-shell">
      <RepositoryHeader
        repository={state.repository}
        loading={state.loading}
        branchControl={
          <BranchPicker
            repository={state.repository}
            service={branchService}
            onUpdated={state.updateRepository}
          />
        }
        remoteControl={
          <RemoteControls
            repository={state.repository}
            repositoryRevision={state.revision}
            service={remoteService}
            onUpdated={state.updateRepository}
          />
        }
        accountControl={<AccountMenu service={authService} />}
        onSettings={openSettings}
        onOpen={openRepository}
        onRefresh={refreshRepository}
      />

      {state.error !== null && (
        <ErrorBanner
          error={state.error}
          onDismiss={state.dismissError}
          onRetry={() => void state.openRepository()}
        />
      )}

      {startView === "clone" ? (
        <CloneView
          githubService={githubRepositoryService}
          cloneService={cloneService}
          onCloned={(repository) => {
            state.updateRepository(repository);
            setStartView("welcome");
          }}
          onBack={() => setStartView("welcome")}
        />
      ) : state.repository === null ? (
        <WelcomeView
          loading={state.loading}
          recentRepositories={settings.settings.recentRepositories}
          onOpen={openRepository}
          onOpenRecent={openRecentRepository}
          onClone={showClone}
        />
      ) : (
        <main className="workspace">
          <div className="changes-column">
            <div className="workspace-tabs" role="tablist" aria-label="Repository view">
              <button
                type="button"
                role="tab"
                aria-selected={workspaceView === "changes"}
                onClick={() => setWorkspaceView("changes")}
              >
                Changes <span>{state.repository.files.length}</span>
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={workspaceView === "history"}
                onClick={() => setWorkspaceView("history")}
              >
                History
              </button>
            </div>
            {workspaceView === "changes" ? (
              <>
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
              </>
            ) : (
              <HistoryPanel
                repositoryPath={state.repository.path}
                repositoryRevision={state.revision}
                service={historyService}
              />
            )}
          </div>
          <DiffViewer
            repositoryPath={state.repository.path}
            file={selectedFile}
            service={diffService}
            repositoryRevision={state.revision}
            theme={settings.resolvedTheme}
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
          title="Open repository (Ctrl+O)"
          onClick={openRepository}
        >
          Open Repository
        </button>
      </footer>

      {settingsOpen && <SettingsPanel state={settings} onClose={closeSettings} />}
    </div>
  );
}

