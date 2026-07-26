// Mimics the Protide mock server panel: header + status bar + route cards + record footer
import { tint } from "../../../lib/theme";
import { METHOD_COLORS } from "../../../lib/theme";

const ROUTES = [
  { method: "GET", path: "/api/users", status: 200 },
  { method: "POST", path: "/api/auth/login", status: 201 },
  { method: "GET", path: "/api/users/*", status: 200 },
  { method: "PUT", path: "/api/users/*", status: 200 },
  { method: "DELETE", path: "/api/users/*", status: 204 },
];

const statusColor = (status: number) =>
  status < 300
    ? "var(--color-status-2xx)"
    : status < 400
      ? "var(--color-status-3xx)"
      : "var(--color-status-4xx)";

export default function MockServerMockup() {
  const stop = "var(--color-status-4xx)";

  return (
    <div className="overflow-hidden border border-[var(--color-border)] bg-[var(--color-bg-primary)] font-mono text-xs shadow-2xl">
      <div className="flex items-center justify-between border-b border-[var(--color-border)] px-3 py-2">
        <span className="text-sm text-[var(--color-text-primary)]">Mock Server</span>
        <div
          className="cursor-default border px-3 py-1 text-sm"
          style={{ borderColor: tint(stop, 50), background: tint(stop, 10), color: stop }}
        >
          Stop
        </div>
      </div>

      <div className="border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)] px-3 py-2">
        <div className="flex items-center gap-2">
          <div className="h-2 w-2" style={{ background: "var(--color-status-2xx)" }} />
          <span className="text-xs text-[var(--color-text-secondary)]">http://localhost:3001</span>
        </div>
      </div>

      <div className="flex flex-col gap-2 p-3">
        {ROUTES.map((route) => (
          <div
            key={`${route.method} ${route.path}`}
            className="flex items-center justify-between border border-[var(--color-border)] bg-[var(--color-bg-secondary)] px-2 py-1"
          >
            <div className="flex items-center gap-2">
              <span className="text-xs font-bold" style={{ color: METHOD_COLORS[route.method] }}>
                {route.method}
              </span>
              <span className="text-sm text-[var(--color-text-primary)]">{route.path}</span>
            </div>
            <div className="flex items-center gap-2">
              <div
                className="px-2 py-px text-xs"
                style={{ background: statusColor(route.status), color: "var(--color-bg-primary)" }}
              >
                {route.status}
              </div>
              <span className="cursor-default text-xs" style={{ color: stop }}>
                x
              </span>
            </div>
          </div>
        ))}
      </div>

      <div className="border-t border-[var(--color-border)] bg-[var(--color-bg-secondary)] px-3 py-2">
        <div className="flex items-center gap-2">
          <span
            className="h-1.5 w-1.5 animate-pulse rounded-full"
            style={{ background: "var(--color-accent)" }}
          />
          <span className="text-[var(--color-text-muted)]">
            Record mode active - proxying to https://api.example.com
          </span>
        </div>
      </div>
    </div>
  );
}
