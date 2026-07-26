import { ArrowList, SectionHeader } from "./shared";
import ScriptingMockup from "./mockups/ScriptingMockup";

const POINTS = [
  "Pre-request scripts for auth setup, token refresh, or dynamic headers",
  "Post-response assertions with an expect() API and .not chaining",
  "JSONPath extraction - pull values from responses into variables",
  "@set annotations chain requests: use response data in the next call",
  "Console output and per-assertion pass/fail results next to the response",
];

export default function Scripting() {
  return (
    <section
      className="border-y px-6 py-24"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
    >
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div data-animate>
            <SectionHeader
              title="Test as you go"
              subtitle="JavaScript pre/post-request scripts let you automate, validate, and chain requests without leaving the app."
              align="left"
              label="Scripting & Testing"
            />

            <div className="mb-8">
              <ArrowList items={POINTS} />
            </div>

            <div
              className="border p-4 font-mono text-xs"
              style={{ borderColor: "var(--color-border)", background: "var(--color-bg-elevated)" }}
            >
              <div className="mb-1 text-[var(--color-text-muted)]">Request chaining:</div>
              <div>
                <span style={{ color: "var(--color-method-put)" }}># @set</span>
                <span className="text-[var(--color-text-secondary)]"> token = </span>
                <span style={{ color: "var(--color-accent)" }}>$.access_token</span>
              </div>
            </div>
          </div>

          <div data-animate style={{ "--anim-delay": "150ms" } as React.CSSProperties}>
            <ScriptingMockup />
          </div>
        </div>
      </div>
    </section>
  );
}
