import { Icon } from "../../components/Icon";

interface WelcomeViewProps {
  loading: boolean;
  onOpen: () => void;
}

export function WelcomeView({ loading, onOpen }: WelcomeViewProps) {
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
      <button
        className="primary-button"
        type="button"
        disabled={loading}
        onClick={onOpen}
      >
        <Icon name="folder" />
        {loading ? "Opening…" : "Open Repository"}
      </button>
      <span className="welcome-note">Your files stay on this computer.</span>
    </main>
  );
}

