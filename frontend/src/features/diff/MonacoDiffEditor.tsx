import { DiffEditor, loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor/editor/editor.api";
import "monaco-editor/languages/definitions/cpp/register";
import "monaco-editor/languages/definitions/csharp/register";
import "monaco-editor/languages/definitions/css/register";
import "monaco-editor/languages/definitions/dockerfile/register";
import "monaco-editor/languages/definitions/go/register";
import "monaco-editor/languages/definitions/html/register";
import "monaco-editor/languages/definitions/ini/register";
import "monaco-editor/languages/definitions/java/register";
import "monaco-editor/languages/definitions/javascript/register";
import "monaco-editor/languages/definitions/kotlin/register";
import "monaco-editor/languages/definitions/less/register";
import "monaco-editor/languages/definitions/markdown/register";
import "monaco-editor/languages/definitions/php/register";
import "monaco-editor/languages/definitions/python/register";
import "monaco-editor/languages/definitions/ruby/register";
import "monaco-editor/languages/definitions/rust/register";
import "monaco-editor/languages/definitions/scss/register";
import "monaco-editor/languages/definitions/shell/register";
import "monaco-editor/languages/definitions/sql/register";
import "monaco-editor/languages/definitions/swift/register";
import "monaco-editor/languages/definitions/typescript/register";
import "monaco-editor/languages/definitions/xml/register";
import "monaco-editor/languages/definitions/yaml/register";
import { useEffect, useState } from "react";
import "./monacoEnvironment";

loader.config({ monaco });

interface MonacoDiffEditorProps {
  path: string;
  oldContent: string;
  newContent: string;
  language: string;
  sideBySide: boolean;
}

function preferredTheme(): "branchcraft-dark" | "branchcraft-light" {
  if (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: light)").matches
  ) {
    return "branchcraft-light";
  }
  return "branchcraft-dark";
}

function defineThemes(monacoInstance: typeof monaco) {
  monacoInstance.editor.defineTheme("branchcraft-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#0f141b",
      "editorGutter.background": "#0f141b",
      "diffEditor.insertedTextBackground": "#245b3a80",
      "diffEditor.removedTextBackground": "#6a303880",
      "diffEditor.insertedLineBackground": "#17382680",
      "diffEditor.removedLineBackground": "#3a202580",
      "diffEditor.diagonalFill": "#293340",
    },
  });
  monacoInstance.editor.defineTheme("branchcraft-light", {
    base: "vs",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#f8fafb",
      "editorGutter.background": "#f8fafb",
      "diffEditor.insertedTextBackground": "#81c99566",
      "diffEditor.removedTextBackground": "#efa0a866",
      "diffEditor.insertedLineBackground": "#dff3e566",
      "diffEditor.removedLineBackground": "#fae4e666",
      "diffEditor.diagonalFill": "#d7dde1",
    },
  });
}

export function MonacoDiffEditor({
  path,
  oldContent,
  newContent,
  language,
  sideBySide,
}: MonacoDiffEditorProps) {
  const [theme, setTheme] = useState(preferredTheme);

  useEffect(() => {
    if (typeof window.matchMedia !== "function") {
      return undefined;
    }
    const preference = window.matchMedia("(prefers-color-scheme: light)");
    const update = () => setTheme(preference.matches ? "branchcraft-light" : "branchcraft-dark");
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  const modelKey = encodeURIComponent(path);
  return (
    <DiffEditor
      height="100%"
      language={language}
      original={oldContent}
      modified={newContent}
      originalModelPath={`inmemory://branchcraft/original/${modelKey}`}
      modifiedModelPath={`inmemory://branchcraft/modified/${modelKey}`}
      keepCurrentOriginalModel={false}
      keepCurrentModifiedModel={false}
      theme={theme}
      beforeMount={defineThemes}
      loading={<span className="diff-loading-label">Preparing editor…</span>}
      options={{
        automaticLayout: true,
        diffAlgorithm: "advanced",
        enableSplitViewResizing: true,
        fixedOverflowWidgets: true,
        folding: false,
        fontFamily:
          '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
        fontSize: 12,
        glyphMargin: false,
        ignoreTrimWhitespace: false,
        lineDecorationsWidth: 8,
        lineNumbers: "on",
        minimap: { enabled: false },
        originalEditable: false,
        readOnly: true,
        renderIndicators: true,
        renderOverviewRuler: false,
        renderSideBySide: sideBySide,
        scrollBeyondLastLine: false,
        smoothScrolling: false,
        stickyScroll: { enabled: false },
        wordWrap: "off",
      }}
    />
  );
}
