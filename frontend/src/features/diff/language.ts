import { gitFilename } from "../../utils/gitPath";

const filenameLanguages: Record<string, string> = {
  Dockerfile: "dockerfile",
  Makefile: "shell",
  "Cargo.lock": "ini",
};

const extensionLanguages: Record<string, string> = {
  c: "c",
  cc: "cpp",
  cpp: "cpp",
  cs: "csharp",
  css: "css",
  go: "go",
  h: "c",
  hpp: "cpp",
  html: "html",
  java: "java",
  js: "javascript",
  json: "javascript",
  jsx: "javascript",
  kt: "kotlin",
  less: "less",
  md: "markdown",
  php: "php",
  py: "python",
  rb: "ruby",
  rs: "rust",
  scss: "scss",
  sh: "shell",
  sql: "sql",
  swift: "swift",
  toml: "ini",
  ts: "typescript",
  tsx: "typescript",
  xml: "xml",
  yaml: "yaml",
  yml: "yaml",
};

export function languageForPath(path: string): string {
  const filename = gitFilename(path);
  const exact = filenameLanguages[filename];
  if (exact !== undefined) {
    return exact;
  }
  const extension = filename.includes(".")
    ? filename.slice(filename.lastIndexOf(".") + 1).toLowerCase()
    : "";
  return extensionLanguages[extension] ?? "plaintext";
}
