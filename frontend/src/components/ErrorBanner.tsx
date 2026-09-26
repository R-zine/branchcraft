import type { GitError } from "../types/repository";

interface ErrorBannerProps {
  error: GitError;
  onDismiss: () => void;
  onRetry: () => void;
}

export function ErrorBanner({ error, onDismiss, onRetry }: ErrorBannerProps) {
  return (
    <div className="error-banner" role="alert">
      <span className="error-symbol">!</span>
      <div>
        <strong>{error.summary}</strong>
        {error.detail !== null && <span>{error.detail}</span>}
      </div>
      <button type="button" onClick={onRetry}>
        Choose another folder
      </button>
      <button aria-label="Dismiss error" type="button" onClick={onDismiss}>
        ×
      </button>
    </div>
  );
}

