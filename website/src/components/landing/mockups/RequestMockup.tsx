// Mimics the Protide request panel: mode dropdown + URL bar + tabs + KV headers table
import { tint } from "../../../lib/theme";
import { Check, ChevronDown } from "../icons";

const TABS = [
  { label: "Params", count: 0 },
  { label: "Headers", count: 2, active: true },
  { label: "Body", count: 0 },
  { label: "Auth", count: 0 },
  { label: "Scripts", count: 0 },
  { label: "Data", count: 0 },
  { label: "Settings", count: 0 },
];

const HEADER_ROWS = [
  { enabled: true, key: "Authorization", value: "Bearer {{token}}" },
  { enabled: true, key: "Accept", value: "application/json" },
  { enabled: false, key: "X-Request-Id", value: "{{$uuid}}" },
];

const GET = "var(--color-method-get)";

export default function RequestMockup() {
  return (
    <div className="overflow-hidden border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-xs shadow-2xl">
      {/* URL bar: h=48px — mode + method + url + send */}
      <div className="flex h-12 items-center gap-2 border-b border-[var(--color-border)] bg-[var(--color-bg-primary)] px-3">
        <div className="flex h-7 w-[88px] shrink-0 cursor-default items-center justify-between gap-1 border border-[var(--color-border)] bg-[var(--color-bg-secondary)] px-2">
          <span className="text-[12px] font-medium text-[var(--color-text-primary)]">HTTP</span>
          <ChevronDown size={9} style={{ color: "var(--color-text-muted)" }} />
        </div>
        <div
          className="flex h-7 w-[64px] shrink-0 cursor-default items-center justify-center gap-1 border px-2 text-[11px] font-bold"
          style={{ color: GET, background: tint(GET, 10), borderColor: tint(GET, 20) }}
        >
          GET
          <ChevronDown size={9} style={{ opacity: 0.7 }} />
        </div>
        <div className="flex h-7 min-w-0 flex-1 items-center overflow-hidden border border-[var(--color-border)] bg-[var(--color-bg-tertiary)] px-2.5 text-[12px]">
          <span className="text-[var(--color-text-muted)]">https://</span>
          <span className="text-[var(--color-text-primary)]">api.example.com/users</span>
        </div>
        <div
          className="flex h-7 shrink-0 cursor-default items-center justify-center px-3 text-[12px] font-semibold"
          style={{ background: "var(--color-accent)", color: "var(--color-bg-primary)" }}
        >
          Send
        </div>
      </div>

      {/* Tab bar: h=40px */}
      <div className="flex h-10 w-full items-center border-b border-[var(--color-border)] bg-[var(--color-bg-primary)]">
        {TABS.map((tab) => (
          <div
            key={tab.label}
            className="flex h-full cursor-default items-center gap-1.5 border-b-2 px-3"
            style={
              tab.active
                ? {
                    color: "var(--color-text-primary)",
                    borderColor: "var(--color-accent)",
                    fontWeight: 500,
                  }
                : { color: "var(--color-text-secondary)", borderColor: "transparent" }
            }
          >
            <span className="text-[12px]">{tab.label}</span>
            {tab.count > 0 && (
              <span
                className="px-[5px] py-px text-[10px] font-medium"
                style={{
                  background: tint("var(--color-accent)", 15),
                  color: "var(--color-accent)",
                }}
              >
                {tab.count}
              </span>
            )}
          </div>
        ))}
      </div>

      {/* KV table header */}
      <div className="mb-1 flex w-full items-center gap-2 border-b border-[var(--color-border)] px-0.5 py-1.5">
        <div className="w-3 shrink-0" />
        <div className="h-4 w-4 shrink-0" />
        <div
          className="w-[150px] shrink-0 text-[10px] font-semibold"
          style={{ color: tint("var(--color-accent)", 70) }}
        >
          HEADER
        </div>
        <div className="w-px self-stretch" style={{ background: tint("var(--color-border)", 60) }} />
        <div className="flex flex-1 items-center justify-between">
          <span className="text-[10px] font-semibold text-[var(--color-text-secondary)]">VALUE</span>
          <span
            className="border px-[6px] py-0.5 text-[10px] font-medium"
            style={{
              background: tint("var(--color-accent)", 12),
              borderColor: tint("var(--color-accent)", 35),
              color: "var(--color-accent)",
            }}
          >
            2 active
          </span>
        </div>
        <div className="h-7 w-7 shrink-0" />
      </div>

      {/* Header rows */}
      <div className="flex flex-col gap-0.5 pb-2">
        {HEADER_ROWS.map((row) => (
          <div
            key={row.key}
            className="flex items-center gap-2 px-0.5 py-1"
            style={row.enabled ? undefined : { opacity: 0.4 }}
          >
            <div
              className="flex h-7 w-3 shrink-0 items-center justify-center text-[8px] text-[var(--color-text-muted)]"
              style={{ opacity: 0.3, letterSpacing: "-1px" }}
            >
              ⠿
            </div>
            <div
              className={`flex h-4 w-4 shrink-0 items-center justify-center border ${
                row.enabled ? "border-[var(--color-accent)]" : "border-[var(--color-border)]"
              }`}
              style={row.enabled ? { background: "var(--color-accent)" } : undefined}
            >
              {row.enabled && <Check size={9} style={{ color: "var(--color-bg-primary)" }} />}
            </div>
            <div className="flex h-7 w-[150px] shrink-0 items-center overflow-hidden border border-transparent bg-[var(--color-bg-tertiary)] px-2 text-[12px] text-[var(--color-text-primary)]">
              {row.key}
            </div>
            <div
              className="w-1 self-stretch"
              style={{ background: tint("var(--color-border)", 60) }}
            />
            <div className="flex h-7 flex-1 items-center overflow-hidden border border-transparent bg-[var(--color-bg-tertiary)] px-2 text-[12px] text-[var(--color-text-primary)]">
              {row.value}
            </div>
            <div className="flex h-7 w-7 shrink-0 items-center justify-center text-[14px] text-[var(--color-text-muted)]">
              ×
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
