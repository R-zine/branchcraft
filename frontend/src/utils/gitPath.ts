export interface GitPathPresentation {
  filename: string;
  parent: string;
}

/**
 * Splits a backend-normalized Git-relative path for display only.
 *
 * This deliberately understands Git's `/` separator, not native filesystem
 * paths. Repository roots returned by the directory picker stay opaque in the
 * frontend and must never be passed through this helper.
 */
export function presentGitPath(path: string): GitPathPresentation {
  const separator = path.lastIndexOf("/");
  if (separator === -1) {
    return { filename: path, parent: "." };
  }
  return {
    filename: path.slice(separator + 1),
    parent: path.slice(0, separator),
  };
}

export function gitFilename(path: string): string {
  return presentGitPath(path).filename;
}
