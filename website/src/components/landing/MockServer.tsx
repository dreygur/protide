import { ArrowList, SectionHeader } from "./shared";
import { tint } from "../../lib/theme";
import MockServerMockup from "./mockups/MockServerMockup";

const POINTS = [
  "Configurable routes with custom status codes, headers, and response bodies",
  "Record/proxy mode: forward to the real server, capture every response as a route",
  "Per-route latency simulation with a configurable delay",
  "Test frontend apps against mocked backends during development",
];

export default function MockServer() {
  return (
    <section
      className="border-y px-6 py-24"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
    >
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div data-animate>
            <SectionHeader
              title="Mock APIs before they exist"
              subtitle="Run a local HTTP server alongside your requests. Configure static routes or record real traffic automatically."
              align="left"
              label="Mock Server"
            />

            <div className="mb-8">
              <ArrowList items={POINTS} />
            </div>

            <div
              className="border-l-2 p-4"
              style={{
                borderColor: "var(--color-accent)",
                background: tint("var(--color-accent)", 6),
              }}
            >
              <p className="mb-1 text-sm font-semibold" style={{ color: "var(--color-accent)" }}>
                Record mode
              </p>
              <p className="text-sm leading-relaxed text-[var(--color-text-secondary)]">
                Point Protide at your real API, flip the switch, and every request is captured as a
                static mock route. Ship without hitting production.
              </p>
            </div>
          </div>

          <div data-animate style={{ "--anim-delay": "150ms" } as React.CSSProperties}>
            <MockServerMockup />
          </div>
        </div>
      </div>
    </section>
  );
}
