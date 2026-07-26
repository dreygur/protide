import { Badge } from "./shared";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];
const PROTOCOLS = ["WebSocket", "gRPC", "GraphQL", "tRPC", "Socket.IO"];

export default function ProtocolStrip() {
  return (
    <div
      className="overflow-hidden border-y py-6"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
    >
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-wrap items-center justify-center gap-3">
          <span className="mr-2 text-xs uppercase tracking-widest text-[var(--color-text-muted)]">
            Methods
          </span>
          {METHODS.map((m) => (
            <Badge key={m} label={m} type="method" />
          ))}
          <span className="mx-2 h-4 w-px" style={{ background: "var(--color-border)" }} />
          <span className="mr-2 text-xs uppercase tracking-widest text-[var(--color-text-muted)]">
            Protocols
          </span>
          {PROTOCOLS.map((p) => (
            <Badge key={p} label={p} type="protocol" />
          ))}
        </div>
      </div>
    </div>
  );
}
