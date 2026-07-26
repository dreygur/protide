// Mimics the Protide scripts tab: accordion sections for Pre-request, Post-response, Tests
import { tint } from "../../../lib/theme";
import { Check, ChevronDown, ChevronLeft, ChevronRight } from "../icons";
import { C, Line, type Tok } from "./code";

const DIVIDER = tint("var(--color-border)", 50);

const POST_SCRIPT: Tok[][] = [
  [["const", C.kw], [" body = response."], ["json", C.fn], ["();"]],
  [[""]],
  [["env", C.fn], ["."], ["set", C.kw], ["("], ['"user_id"', C.str], [", body."], ["id", C.fn], [");"]],
  [
    ["env", C.fn],
    ["."],
    ["set", C.kw],
    ["("],
    ['"token"', C.str],
    [", body."],
    ["access_token", C.fn],
    [");"],
  ],
];

const TESTS: Tok[][] = [
  [["expect", C.fn], ["(response."], ["status", C.fn], [")."], ["toBe", C.kw], ["("], ["200", C.str], [");"]],
  [
    ["expect", C.fn],
    ["(response."],
    ["json", C.fn],
    ["()."],
    ["id", C.fn],
    [")."],
    ["toBeTruthy", C.kw],
    ["();"],
  ],
  [["expect", C.fn], ["(response."], ["time", C.fn], [")."], ["toBeLessThan", C.kw], ["("], ["2000", C.str], [");"]],
];

function SectionRow({
  icon,
  iconColor,
  title,
  hint,
  open,
}: {
  icon: React.ReactNode;
  iconColor: string;
  title: string;
  hint: string;
  open: boolean;
}) {
  return (
    <div
      className="flex h-9 w-full cursor-default items-center gap-2 border-b px-3"
      style={{ borderColor: DIVIDER }}
    >
      {open ? (
        <ChevronDown size={10} style={{ color: "var(--color-text-muted)" }} />
      ) : (
        <ChevronRight size={10} style={{ color: "var(--color-text-muted)" }} />
      )}
      <div
        className="flex h-[18px] w-[18px] shrink-0 items-center justify-center"
        style={{ background: tint(iconColor, 15) }}
      >
        {icon}
      </div>
      <span className="text-[12px] font-semibold text-[var(--color-text-primary)]">{title}</span>
      <span className="text-[10px] text-[var(--color-text-muted)]">{hint}</span>
    </div>
  );
}

export default function ScriptingMockup() {
  const post = "var(--color-method-post)";
  const ok = "var(--color-status-2xx)";
  const accent = "var(--color-accent)";

  return (
    <div className="overflow-hidden border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-xs shadow-2xl">
      <SectionRow
        icon={<ChevronRight size={9} style={{ color: post }} />}
        iconColor={post}
        title="Pre-request Script"
        hint="Runs before sending the request"
        open={false}
      />

      <SectionRow
        icon={<ChevronLeft size={9} style={{ color: ok }} />}
        iconColor={ok}
        title="Post-response Script"
        hint="Runs after receiving response"
        open
      />
      <div
        className="border-b px-4 py-3 leading-5"
        style={{ borderColor: DIVIDER, height: 132, overflow: "hidden" }}
      >
        <div className="flex flex-col gap-1">
          {POST_SCRIPT.map((toks, i) => (
            <Line key={i} toks={toks} />
          ))}
        </div>
      </div>

      <SectionRow
        icon={<Check size={9} style={{ color: accent }} />}
        iconColor={accent}
        title="Tests"
        hint="Test assertions using expect()"
        open
      />
      <div className="px-4 py-3 leading-5">
        <div className="flex flex-col gap-1">
          {TESTS.map((toks, i) => (
            <Line key={i} toks={toks} />
          ))}
        </div>
      </div>
    </div>
  );
}
