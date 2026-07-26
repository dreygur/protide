import { ArrowList, SectionHeader } from "./shared";
import { tint } from "../../lib/theme";
import { ChevronDown, ChevronRight, ExternalLink, File, FolderClosed, FolderOpen } from "./icons";

type Row =
  | { type: "folder"; depth: number; name: string; open: boolean }
  | {
      type: "file";
      depth: number;
      name: string;
      badge: string;
      badgeColor: string;
      selected?: boolean;
    };

const ROWS: Row[] = [
  { type: "folder", depth: 0, name: "e2e", open: true },
  { type: "folder", depth: 1, name: "graphql", open: false },
  {
    type: "file",
    depth: 2,
    name: "graphql-tests",
    badge: "GQL",
    badgeColor: "var(--color-proto-graphql)",
  },
  { type: "folder", depth: 1, name: "grpc", open: false },
  {
    type: "file",
    depth: 2,
    name: "grpc-employee",
    badge: "gRPC",
    badgeColor: "var(--color-proto-grpc)",
    selected: true,
  },
  { type: "folder", depth: 1, name: "http", open: true },
  {
    type: "file",
    depth: 2,
    name: "http-api-tests",
    badge: "GET",
    badgeColor: "var(--color-method-get)",
  },
  {
    type: "file",
    depth: 2,
    name: "http-scripting",
    badge: "POST",
    badgeColor: "var(--color-method-post)",
  },
  { type: "folder", depth: 1, name: "socketio", open: false },
  {
    type: "file",
    depth: 2,
    name: "socketio-echo",
    badge: "SIO",
    badgeColor: "var(--color-proto-sio)",
  },
  { type: "folder", depth: 1, name: "trpc", open: false },
  {
    type: "file",
    depth: 2,
    name: "trpc-example",
    badge: "tRPC",
    badgeColor: "var(--color-proto-trpc)",
  },
  { type: "folder", depth: 1, name: "websocket", open: false },
  {
    type: "file",
    depth: 2,
    name: "websocket-echo",
    badge: "WS",
    badgeColor: "var(--color-proto-ws)",
  },
];

const POINTS = [
  "Folders are collections - open any folder as a workspace",
  "Commit and diff requests in Git like any other code",
  "Environment variables with {{variable}} substitution",
  "Per-environment config: development, production, or your own",
  "Import from Postman, Bruno, OpenAPI, or cURL commands",
];

const MUTED = { color: "var(--color-text-muted)", flexShrink: 0 };

function TreeRow({ row }: { row: Row }) {
  const selected = row.type === "file" && row.selected;

  return (
    <div
      className="flex h-[22px] w-full items-center gap-1"
      style={{
        paddingLeft: 8 + row.depth * 16,
        background: selected ? tint("var(--color-accent)", 15) : "transparent",
      }}
    >
      {row.type === "folder" ? (
        <>
          {row.open ? (
            <ChevronDown size={10} style={MUTED} />
          ) : (
            <ChevronRight size={10} style={MUTED} />
          )}
          {row.open ? <FolderOpen size={13} style={MUTED} /> : <FolderClosed size={13} style={MUTED} />}
          <span className="text-[12px] text-[var(--color-text-primary)]">{row.name}</span>
        </>
      ) : (
        <>
          <span className="w-2.5 shrink-0" />
          <File size={13} style={MUTED} />
          <span
            className="shrink-0 px-[5px] py-px text-[9px] font-bold"
            style={{ background: tint(row.badgeColor, 15), color: row.badgeColor }}
          >
            {row.badge}
          </span>
          <span
            className="truncate text-[12px]"
            style={{ color: selected ? "var(--color-accent)" : "var(--color-text-primary)" }}
          >
            {row.name}
          </span>
          {selected && (
            <ExternalLink
              size={11}
              className="ml-auto mr-1 shrink-0"
              style={{ color: "var(--color-accent)" }}
            />
          )}
        </>
      )}
    </div>
  );
}

export default function Collections() {
  return (
    <section className="px-6 py-24">
      <div className="mx-auto max-w-6xl">
        <div className="grid grid-cols-1 items-center gap-16 lg:grid-cols-2">
          <div
            data-animate
            className="order-2 lg:order-1"
            style={{ "--anim-delay": "100ms" } as React.CSSProperties}
          >
            <div
              className="overflow-hidden border font-mono text-xs shadow-2xl"
              style={{
                borderColor: "var(--color-border)",
                background: "var(--color-bg-secondary)",
              }}
            >
              {/* Explorer header: h=32px */}
              <div
                className="flex h-8 items-center justify-between border-b bg-[var(--color-bg-primary)] px-3"
                style={{ borderColor: "var(--color-border)" }}
              >
                <span className="text-[11px] font-semibold text-[var(--color-text-secondary)]">
                  Explorer
                </span>
                <div className="flex items-center gap-1">
                  <FolderClosed size={13} style={MUTED} />
                  <File size={13} style={MUTED} />
                </div>
              </div>

              <div className="bg-[var(--color-bg-primary)] py-1 leading-none">
                {ROWS.map((row) => (
                  <TreeRow key={`${row.depth}-${row.name}`} row={row} />
                ))}
              </div>

              {/* Env selector */}
              <div
                className="flex items-center gap-2 border-t bg-[var(--color-bg-secondary)] px-3 py-2"
                style={{ borderColor: "var(--color-border)" }}
              >
                <span className="text-[10px] font-semibold text-[var(--color-text-muted)]">ENV</span>
                <div
                  className="flex items-center gap-1 border bg-[var(--color-bg-elevated)] px-2 py-0.5 text-[11px]"
                  style={{ color: "var(--color-accent)", borderColor: "var(--color-border)" }}
                >
                  Development
                  <ChevronDown size={9} />
                </div>
              </div>
            </div>
          </div>

          <div className="order-1 lg:order-2">
            <SectionHeader
              title="Collections that live in your filesystem"
              subtitle="No proprietary formats, no cloud lock-in. Your requests are plain .http files in folders."
              align="left"
              label="Collections"
            />
            <ArrowList items={POINTS} />
          </div>
        </div>
      </div>
    </section>
  );
}
