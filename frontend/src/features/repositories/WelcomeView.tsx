import { Icon } from "../../components/Icon";
import type { RecentRepository } from "../../types/settings";

interface WelcomeViewProps {
  loading: boolean;
  recentRepositories: RecentRepository[];
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onClone: () => void;
}

export function WelcomeView({
  loading,
  recentRepositories,
  onOpen,
  onOpenRecent,
  onClone,
}: WelcomeViewProps) {
  return (
    <main className="welcome-view">
      <div className="welcome-glyph" aria-hidden="true">
        <Icon name="branch" />
      </div>
      <h1>Open a local repository</h1>
      <p>
        Choose an existing Git repository to inspect its branch and working tree
        changes.
      </p>
      <div className="welcome-actions">
        <button
          className="primary-button"
          type="button"
          disabled={loading}
          title="Open repository (Ctrl+O)"
          onClick={onOpen}
        >
          <Icon name="folder" />
          {loading ? "Opening…" : "Open Repository"}
        </button>
        <button
          className="secondary-button"
          type="button"
          disabled={loading}
          title="Clone repository (Ctrl+Shift+O)"
          onClick={onClone}
        >
          <Icon name="branch" /> Clone Repository
        </button>
      </div>
      {recentRepositories.length > 0 && (
        <section className="welcome-recents" aria-labelledby="welcome-recents-heading">
          <h2 id="welcome-recents-heading">Recent repositories</h2>
          <ul>
            {recentRepositories.slice(0, 6).map((repository) => (
              <li key={repository.path}>
                <button
                  type="button"
                  disabled={loading || !repository.available}
                  title={repository.path}
                  onClick={() => onOpenRecent(repository.path)}
                >
                  <Icon name="folder" />
                  <span>
                    <strong>{repository.name}</strong>
                    <small>{repository.path}</small>
                  </span>
                  {!repository.available && <em>Unavailable</em>}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <span className="welcome-note">Your files and preferences stay on this computer.</span>
    </main>
  );
}

