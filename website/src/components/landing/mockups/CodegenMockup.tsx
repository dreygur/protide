// Mimics the Protide codegen side panel: language tabs + line-numbered code + copy button
import { tint } from "../../../lib/theme";
import { Copy } from "../icons";
import { C, NumberedLines, type Tok } from "./code";

const LANGS = ["cURL", "Python", "JS", "Go", "Rust"];

const CURL: Tok[][] = [
  [["curl", C.fn], [" \\"]],
  [["  "], ["-X GET", C.str], [" \\"]],
  [["  "], ["'https://api.example.com/users'", C.str], [" \\"]],
  [["  "], ["-H", C.kw], [" "], ["'Authorization: Bearer eyJhbGc...'", C.str], [" \\"]],
  [["  "], ["-H", C.kw], [" "], ["'Accept: application/json'", C.str]],
];

export default function CodegenMockup() {
  return (
    <div className="overflow-hidden border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-xs shadow-2xl">
      {/* Toolbar: h=40px */}
      <div className="flex h-10 items-center gap-1.5 border-b border-[var(--color-border)] px-3">
        <div className="flex flex-1 items-center gap-0.5">
          {LANGS.map((label, i) => (
            <div
              key={label}
              className="cursor-default px-2 py-[3px] text-[11px] font-medium"
              style={
                i === 0
                  ? {
                      background: tint("var(--color-accent)", 15),
                      color: "var(--color-accent)",
                      border: `1px solid ${tint("var(--color-accent)", 30)}`,
                    }
                  : { color: "var(--color-text-secondary)" }
              }
            >
              {label}
            </div>
          ))}
        </div>
        <div className="flex h-7 cursor-default items-center gap-1 border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-[10px] text-[11px] text-[var(--color-text-secondary)]">
          <Copy size={10} />
          Copy
        </div>
        <div className="flex h-7 w-7 cursor-default items-center justify-center text-[11px] text-[var(--color-text-muted)]">
          ×
        </div>
      </div>

      <div className="p-4 leading-5">
        <NumberedLines lines={CURL} />
      </div>
    </div>
  );
}
