import { SectionHeader } from "./shared";
import { tint } from "../../lib/theme";
import CodegenMockup from "./mockups/CodegenMockup";
import { C, Line, type Tok } from "./mockups/code";

const LANGS = [
  { name: "cURL", color: "var(--color-method-get)" },
  { name: "Python", color: "var(--color-method-post)" },
  { name: "JavaScript", color: "var(--color-method-put)" },
  { name: "Go", color: "var(--color-proto-ws)" },
  { name: "Rust", color: "var(--color-method-delete)" },
];

const RUST: Tok[][] = [
  [["let", C.kw], [" resp = client"]],
  [["    ."], ["get", C.fn], ["("], ['"https://api.example.com/users"', C.str], [")"]],
  [["    ."], ["bearer_auth", C.fn], ["(token)"]],
  [["    ."], ["send", C.fn], ["()."], ["await", C.kw], ["?;"]],
];

export default function CodeGen() {
  return (
    <section className="px-6 py-24">
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div data-animate>
            <CodegenMockup />
          </div>

          <div data-animate style={{ "--anim-delay": "150ms" } as React.CSSProperties}>
            <SectionHeader
              title="Export to any language"
              subtitle="Convert any request to ready-to-paste code with one click."
              align="left"
              label="Code Generation"
            />

            <div className="mb-8 flex flex-wrap gap-3">
              {LANGS.map((lang) => (
                <div
                  key={lang.name}
                  className="border px-4 py-2 text-sm font-semibold"
                  style={{
                    color: lang.color,
                    borderColor: tint(lang.color, 30),
                    background: tint(lang.color, 8),
                  }}
                >
                  {lang.name}
                </div>
              ))}
            </div>

            <p className="mb-6 text-sm leading-relaxed text-[var(--color-text-secondary)]">
              Every request in your collection can be exported as production-ready client code.
              Environment variables are resolved before export - no placeholders in the output.
            </p>

            <div
              className="border p-4 font-mono text-xs"
              style={{
                borderColor: "var(--color-border)",
                background: "var(--color-bg-secondary)",
              }}
            >
              <div className="mb-2 text-[var(--color-text-muted)]">Generated Rust (reqwest):</div>
              <div className="flex flex-col gap-0.5 leading-5">
                {RUST.map((toks, i) => (
                  <Line key={i} toks={toks} />
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
