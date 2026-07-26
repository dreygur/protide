import type { IconProps } from "./icons";
import { Bot, FolderOpen, Globe, Network, Play, Terminal } from "./icons";
import { SectionHeader } from "./shared";

const FEATURES: {
  Icon: (p: IconProps) => React.ReactElement;
  title: string;
  desc: string;
  color: string;
}[] = [
  {
    Icon: Globe,
    title: "Multi-Protocol",
    desc: "HTTP, GraphQL, WebSocket, gRPC, tRPC, and Socket.IO - all from one tool with a unified .http file format.",
    color: "var(--color-method-get)",
  },
  {
    Icon: FolderOpen,
    title: "File-Based Collections",
    desc: "No proprietary formats. Folders are collections, .http files are requests. Commit, diff, and review with Git.",
    color: "var(--color-method-post)",
  },
  {
    Icon: Terminal,
    title: "JS Scripting & Testing",
    desc: "Pre/post-request scripts, test assertions with expect(), and request chaining via JSONPath extraction.",
    color: "var(--color-method-put)",
  },
  {
    Icon: Play,
    title: "Mock Server",
    desc: "Local HTTP mock server with route configuration and record/proxy mode to capture live traffic as static routes.",
    color: "var(--color-method-patch)",
  },
  {
    Icon: Network,
    title: "Local-First Collab",
    desc: "P2P sync via mDNS auto-discovery. No accounts, no server. Devices on your LAN find each other automatically.",
    color: "var(--color-proto-ws)",
  },
  {
    Icon: Bot,
    title: "AI Integration",
    desc: "Built-in MCP server lets Claude and other AI assistants drive requests directly. LSP for .http files in any editor.",
    color: "var(--color-proto-grpc)",
  },
];

export default function FeatureGrid() {
  return (
    <section className="px-6 py-24">
      <div className="mx-auto max-w-6xl">
        <SectionHeader
          title="Everything you need to test APIs"
          subtitle="No Electron. No slow startup. A native desktop app that starts in milliseconds."
        />

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map(({ Icon, title, desc, color }, i) => (
            <article
              key={title}
              data-animate
              className="feature-card cursor-default border p-6 transition-all duration-200"
              style={
                {
                  "--anim-delay": `${i * 75}ms`,
                  "--card-accent": color,
                  borderColor: "var(--color-border)",
                  background: "var(--color-bg-secondary)",
                } as React.CSSProperties
              }
            >
              <div className="mb-4">
                <Icon size={20} style={{ color }} />
              </div>
              <h3 className="mb-2 font-semibold text-[var(--color-text-primary)]">{title}</h3>
              <p className="text-sm leading-relaxed text-[var(--color-text-secondary)]">{desc}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
