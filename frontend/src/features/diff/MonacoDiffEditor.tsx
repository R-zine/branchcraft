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
import "./monacoEnvironment";
import type { ColorPalette, ResolvedTheme } from "../../types/settings";

loader.config({ monaco });

interface MonacoDiffEditorProps {
  path: string;
  oldContent: string;
  newContent: string;
  language: string;
  sideBySide: boolean;
  theme: ResolvedTheme;
  palette: ColorPalette;
}

const paletteThemeColors = {
  branchcraft: {
    dark: ["#0f141b", "#5fc8bf", "#243d4a", "#293340", "#245b3a80", "#6a303880", "#17382680", "#3a202580"],
    light: ["#f8fafb", "#16877e", "#d9edeb", "#d7dde1", "#81c99566", "#efa0a866", "#dff3e566", "#fae4e666"],
  },
  ocean: {
    dark: ["#0e1520", "#6ca8ff", "#203b63", "#2a3a52", "#315e9a80", "#74394580", "#1b365b80", "#40252b80"],
    light: ["#f7faff", "#2867bd", "#dbe9ff", "#cfdaea", "#72aaf466", "#ed9fa866", "#d9e8fb80", "#fae3e680"],
  },
  forest: {
    dark: ["#101812", "#71cf8c", "#204332", "#2b4036", "#2d6a4380", "#70403980", "#1b432980", "#3d292680"],
    light: ["#f7fbf8", "#2f7d46", "#dcefe2", "#d0dfd4", "#7fca9466", "#eba39d66", "#dcefe280", "#f8e4e280"],
  },
  monochrome: {
    dark: ["#151515", "#c7c7c7", "#363636", "#414141", "#8a8a8a66", "#55555580", "#343434b3", "#242424b3"],
    light: ["#f7f7f7", "#555555", "#dedede", "#d3d3d3", "#a0a0a066", "#6f6f6f55", "#e5e5e5cc", "#d8d8d8b3"],
  },
} as const satisfies Record<
  ColorPalette,
  Record<ResolvedTheme, readonly [string, string, string, string, string, string, string, string]>
>;

function defineThemes(monacoInstance: typeof monaco) {
  const palettes = Object.keys(paletteThemeColors) as ColorPalette[];
  for (const palette of palettes) {
    for (const theme of ["dark", "light"] as const) {
      const [background, accent, selection, diagonal, insertedText, removedText, insertedLine, removedLine] =
        paletteThemeColors[palette][theme];
      monacoInstance.editor.defineTheme(`branchcraft-${palette}-${theme}`, {
        base: theme === "dark" ? "vs-dark" : "vs",
        inherit: true,
        rules: [],
        colors: {
          "editor.background": background,
          "editorGutter.background": background,
          "editorCursor.foreground": accent,
          "editor.selectionBackground": selection,
          "diffEditor.insertedTextBackground": insertedText,
          "diffEditor.removedTextBackground": removedText,
          "diffEditor.insertedLineBackground": insertedLine,
          "diffEditor.removedLineBackground": removedLine,
          "diffEditor.diagonalFill": diagonal,
        },
      });
    }
  }
}

export function MonacoDiffEditor({
  path,
  oldContent,
  newContent,
  language,
  sideBySide,
  theme,
  palette,
}: MonacoDiffEditorProps) {
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
      theme={`branchcraft-${palette}-${theme}`}
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
