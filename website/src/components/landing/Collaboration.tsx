import type { IconProps } from "./icons";
import { Copy, EyeOff, HardDrive, Replace } from "./icons";
import { SectionHeader } from "./shared";
import CollabMockup from "./mockups/CollabMockup";

const ITEMS: {
  Icon: (p: IconProps) => React.ReactElement;
  title: string;
  desc: string;
  color: string;
}[] = [
  {
    Icon: Copy,
    title: "Code-based pairing",
    desc: "Every session gets a unique code. Share it with a teammate and connect in seconds.",
    color: "var(--color-accent)",
  },
  {
    Icon: EyeOff,
    title: "PAKE secure pairing",
    desc: "Password-authenticated key exchange - encrypted from first contact, no certificates needed.",
    color: "var(--color-proto-grpc)",
  },
  {
    Icon: Replace,
    title: "CRDT-based sync",
    desc: "Last-writer-wins registers with logical clocks. Changes always converge, no merge conflicts.",
    color: "var(--color-method-put)",
  },
  {
    Icon: HardDrive,
    title: "BYOB remote sync",
    desc: "Use Dropbox, Google Drive, or a Git repo as the sync backend for distributed teams.",
    color: "var(--color-method-post)",
  },
];

export default function Collaboration() {
  return (
    <section className="px-6 py-24">
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div data-animate>
            <CollabMockup />
          </div>

          <div data-animate style={{ "--anim-delay": "150ms" } as React.CSSProperties}>
            <SectionHeader
              title="Team sync without a server"
              subtitle="Share a pairing code, connect instantly. No accounts, no cloud, no internet required."
              align="left"
              label="Local-First Collaboration"
            />

            <div className="mb-8 flex flex-col gap-4">
              {ITEMS.map(({ Icon, title, desc, color }) => (
                <div
                  key={title}
                  className="flex items-start gap-3 border p-4"
                  style={{
                    borderColor: "var(--color-border)",
                    background: "var(--color-bg-secondary)",
                  }}
                >
                  <Icon size={16} className="mt-0.5 shrink-0" style={{ color }} />
                  <div>
                    <div className="mb-1 text-sm font-semibold text-[var(--color-text-primary)]">
                      {title}
                    </div>
                    <div className="text-sm leading-relaxed text-[var(--color-text-secondary)]">
                      {desc}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
