import * as React from "react";
import { loadHighlighter } from "@/lib/highlighter";

export function useHighlightedCode(value: string, language: string) {
  const code = value.endsWith("\n") ? value.slice(0, -1) : value;
  const [loaded, setLoaded] = React.useState<{
    language: string;
    highlighter: Awaited<ReturnType<typeof loadHighlighter>>;
  } | null>(null);
  const highlighter = loaded?.language === language ? loaded.highlighter : null;

  React.useEffect(() => {
    let cancelled = false;
    setLoaded(null);
    loadHighlighter(language).then((next) => {
      if (!cancelled) setLoaded({ language, highlighter: next });
    }).catch(() => {
      if (!cancelled) setLoaded({ language, highlighter: null });
    });
    return () => { cancelled = true; };
  }, [language]);

  type Renderer = NonNullable<typeof highlighter>;
  const [done, setDone] = React.useState<{
    highlighter: Renderer;
    text: string;
    lines: React.ReactNode[];
    state?: Parameters<Renderer["highlight"]>[1];
  } | null>(null);

  React.useEffect(() => {
    if (!highlighter) return;
    setDone((previous) => {
      const canContinue = previous?.highlighter === highlighter && code.startsWith(previous.text);
      const current = canContinue ? previous : { highlighter, text: "", lines: [] };
      const completeEnd = code.lastIndexOf("\n") + 1;
      if (completeEnd <= current.text.length) return current;
      const result = highlighter.highlight(code.slice(current.text.length, completeEnd - 1), current.state);
      return {
        highlighter,
        text: code.slice(0, completeEnd),
        lines: [...current.lines, ...result.lines],
        state: result.state,
      };
    });
  }, [code, highlighter]);

  const current = done?.highlighter === highlighter && code.startsWith(done.text)
    ? done
    : { highlighter, text: "", lines: [] as React.ReactNode[], state: undefined };
  const tail = code.slice(code.lastIndexOf("\n") + 1);
  return React.useMemo(() => highlighter
    ? [...current.lines, highlighter.highlight(tail, current.state).lines[0]]
    : code.split("\n"), [code, current, highlighter, tail]);
}
