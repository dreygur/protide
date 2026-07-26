// Mimics the Protide pairing flyout panel (w=260px, accent-bordered)
import { tint } from "../../../lib/theme";
import { ChevronDown, Copy, Network } from "../icons";

const ACCENT = "var(--color-accent)";
const PEERS = [
  { initials: "AL", name: "alex-macbook" },
  { initials: "SW", name: "sara-workstation" },
];

function Divider() {
  return <div className="h-px w-full" style={{ background: tint(ACCENT, 20) }} />;
}

function SectionLabel({ children }: { children: string }) {
  return <div className="text-[9px] font-semibold text-[var(--color-text-muted)]">{children}</div>;
}

export default function CollabMockup() {
  return (
    <div
      className="mx-auto overflow-hidden font-mono text-xs shadow-2xl"
      style={{
        border: `1px solid ${ACCENT}`,
        background: "var(--color-bg-secondary)",
        maxWidth: 260,
      }}
    >
      {/* Header: h=32px */}
      <div
        className="flex h-8 items-center gap-1.5 border-b bg-[var(--color-bg-primary)] px-3"
        style={{ borderColor: tint(ACCENT, 30) }}
      >
        <Network size={12} style={{ color: ACCENT }} />
        <span className="text-[11px] font-semibold text-[var(--color-text-primary)]">
          Collaboration
        </span>
        <div className="flex-1" />
        <div
          className="border px-[5px] py-px text-[9px]"
          style={{
            background: tint(ACCENT, 12),
            borderColor: tint(ACCENT, 25),
            color: ACCENT,
          }}
        >
          2 peers
        </div>
      </div>

      <div className="flex flex-col gap-1.5 px-3 pb-2.5 pt-2.5">
        <SectionLabel>YOUR CODE</SectionLabel>
        <div
          className="flex h-10 w-full items-center justify-center border bg-[var(--color-bg-primary)]"
          style={{ borderColor: tint(ACCENT, 30) }}
        >
          <span className="text-[15px] font-bold tracking-widest" style={{ color: ACCENT }}>
            brave-falcon-042
          </span>
        </div>
        <div
          className="flex h-[26px] w-full cursor-default items-center justify-center gap-1.5 border"
          style={{ background: tint(ACCENT, 12), borderColor: tint(ACCENT, 30), color: ACCENT }}
        >
          <Copy size={10} />
          <span className="text-[10px] font-semibold">Copy Code</span>
        </div>
      </div>

      <Divider />

      <div className="flex flex-col gap-1.5 px-3 pb-2.5 pt-2.5">
        <SectionLabel>JOIN PEER</SectionLabel>
        <div className="flex items-center justify-between border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-3 py-2">
          <span className="text-[var(--color-text-muted)]">enter pairing code...</span>
          <ChevronDown size={9} style={{ color: "var(--color-text-muted)" }} />
        </div>
        <div className="flex gap-2">
          <div className="flex-1 cursor-default border border-[var(--color-border)] py-2 text-center text-[11px] text-[var(--color-text-secondary)]">
            Paste &amp; Join
          </div>
          <div
            className="flex-1 cursor-default border py-2 text-center text-[11px] font-semibold"
            style={{ borderColor: ACCENT, color: ACCENT, background: tint(ACCENT, 10) }}
          >
            Connect
          </div>
        </div>
      </div>

      <Divider />

      <div className="flex flex-col gap-1 px-3 pb-2 pt-2">
        <SectionLabel>CONNECTED PEERS</SectionLabel>
        {PEERS.map((peer) => (
          <div key={peer.name} className="flex h-[22px] w-full items-center gap-1.5">
            <div
              className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full"
              style={{ background: tint(ACCENT, 15) }}
            >
              <span className="text-[7px] font-bold" style={{ color: ACCENT }}>
                {peer.initials}
              </span>
            </div>
            <span className="flex-1 text-[11px] text-[var(--color-text-primary)]">{peer.name}</span>
            <div
              className="h-[5px] w-[5px] rounded-full"
              style={{ background: "var(--color-status-2xx)" }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
