import { Badge, SectionHeader } from "./shared";
import RequestMockup from "./mockups/RequestMockup";

const PROTOCOLS = [
  { label: "WebSocket", desc: "Real-time bidirectional messaging with history" },
  { label: "gRPC", desc: "Proto loading, all streaming types, metadata" },
  { label: "GraphQL", desc: "Query/variables editor with syntax highlighting" },
  { label: "tRPC", desc: "Query and mutation procedures" },
  { label: "Socket.IO", desc: "Events, namespaces, and acknowledgements" },
];

export default function MultiProtocol() {
  return (
    <section
      className="border-y px-6 py-24"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
    >
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div data-animate>
            <SectionHeader
              title="One tool. Every protocol."
              subtitle="Switch between HTTP, GraphQL, WebSocket, gRPC and more without changing your workflow or file format."
              align="left"
              label="Multi-Protocol"
            />

            <div className="flex flex-col gap-4">
              {PROTOCOLS.map((p) => (
                <div key={p.label} className="flex items-start gap-3">
                  <Badge label={p.label} type="protocol" size="sm" />
                  <span className="pt-0.5 text-sm leading-relaxed text-[var(--color-text-secondary)]">
                    {p.desc}
                  </span>
                </div>
              ))}
            </div>

            <div
              className="mt-8 border p-4"
              style={{ borderColor: "var(--color-border)", background: "var(--color-bg-elevated)" }}
            >
              <p className="font-mono text-xs leading-relaxed text-[var(--color-text-muted)]">
                <span className="text-[var(--color-text-secondary)]">
                  # Switch protocol with a single annotation:
                </span>
                <br />
                <span style={{ color: "var(--color-method-put)" }}># @protocol</span>
                <span style={{ color: "var(--color-accent)" }}> websocket</span>
              </p>
            </div>
          </div>

          <div data-animate style={{ "--anim-delay": "150ms" } as React.CSSProperties}>
            <RequestMockup />
          </div>
        </div>
      </div>
    </section>
  );
}
