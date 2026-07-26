/** Tiny syntax-coloured code renderer for the UI mockups: one span per token. */
export const C = {
  txt: "var(--color-text-secondary)",
  str: "var(--color-method-put)",
  fn: "var(--color-method-post)",
  kw: "var(--color-proto-graphql)",
  dim: "var(--color-text-muted)",
};

export type Tok = [text: string, color?: string];

export function Line({ toks }: { toks: Tok[] }) {
  return (
    <div>
      {toks.map(([text, color], i) => (
        <span key={i} style={{ color: color ?? C.txt }}>
          {text}
        </span>
      ))}
    </div>
  );
}

/** Same, with a gutter of line numbers. */
export function NumberedLines({ lines, gutter = "w-4" }: { lines: Tok[][]; gutter?: string }) {
  return (
    <div className="flex flex-col gap-1">
      {lines.map((toks, i) => (
        <div key={i} className="flex gap-3">
          <span
            className={`${gutter} shrink-0 select-none text-right text-[11px]`}
            style={{ color: C.dim }}
          >
            {i + 1}
          </span>
          <Line toks={toks} />
        </div>
      ))}
    </div>
  );
}
