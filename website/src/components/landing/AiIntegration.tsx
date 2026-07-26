import { SectionHeader } from "./shared";
import { tint } from "../../lib/theme";
import { Bot, Terminal } from "./icons";
import { C, Line, type Tok } from "./mockups/code";

const TOOL_CALL: Tok[][] = [
  [[">", C.dim], [" "], ["send_request", C.kw], ["("], ["{", C.txt]],
  [["    "], ["method", C.fn], [": "], ['"POST"', C.str], [","]],
  [["    "], ["url", C.fn], [": "], ['"{{base_url}}/login"', C.str], [","]],
  [["    "], ["tests", C.fn], [": "], ['"expect(response.status).toBe(200)"', C.str]],
  [["});"]],
];

const EDITORS = [
  { editor: "Zed", desc: "Extension + tree-sitter grammar", color: "var(--color-proto-graphql)" },
  { editor: "VS Code", desc: "Extension in extensions/vscode", color: "var(--color-method-post)" },
  { editor: "Neovim", desc: "Any LSP client, e.g. nvim-lspconfig", color: "var(--color-proto-ws)" },
];

function Card({
  icon,
  iconColor,
  title,
  subtitle,
  body,
  children,
  delay,
}: {
  icon: React.ReactNode;
  iconColor: string;
  title: string;
  subtitle: string;
  body: string;
  children: React.ReactNode;
  delay: number;
}) {
  return (
    <div
      data-animate
      className="border p-8"
      style={
        {
          "--anim-delay": `${delay}ms`,
          borderColor: "var(--color-border)",
          background: "var(--color-bg-primary)",
        } as React.CSSProperties
      }
    >
      <div className="mb-4 flex items-center gap-3">
        <div
          className="flex h-10 w-10 items-center justify-center"
          style={{ background: tint(iconColor, 15) }}
        >
          {icon}
        </div>
        <div>
          <div className="font-semibold text-[var(--color-text-primary)]">{title}</div>
          <div className="text-xs text-[var(--color-text-muted)]">{subtitle}</div>
        </div>
      </div>
      <p className="mb-6 text-sm leading-relaxed text-[var(--color-text-secondary)]">{body}</p>
      {children}
    </div>
  );
}

export default function AiIntegration() {
  const grpc = "var(--color-proto-grpc)";
  const post = "var(--color-method-post)";

  return (
    <section
      className="border-y px-6 py-24"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
    >
      <div className="mx-auto max-w-6xl">
        <SectionHeader
          title="Built for the AI era"
          subtitle="Let your AI assistant drive requests. Get editor intelligence for .http files."
        />

        <div className="grid grid-cols-1 gap-8 lg:grid-cols-2">
          <Card
            icon={<Bot size={20} style={{ color: grpc }} />}
            iconColor={grpc}
            title="protide-mcp"
            subtitle="MCP Server · JSON-RPC 2.0 over stdio"
            body="Exposes a send_request tool to Claude, Cursor, and any MCP-capable assistant: method, URL, headers, body, environment variables, scripts, and assertions - executed by the same engine as the app."
            delay={0}
          >
            <div
              className="border p-4 font-mono text-xs"
              style={{
                borderColor: "var(--color-border)",
                background: "var(--color-bg-secondary)",
              }}
            >
              <div className="mb-2 flex items-center gap-1.5">
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ background: "var(--color-accent)" }}
                />
                <span className="text-[var(--color-text-muted)]">MCP tool call</span>
              </div>
              <div className="flex flex-col gap-1 leading-5">
                {TOOL_CALL.map((toks, i) => (
                  <Line key={i} toks={toks} />
                ))}
              </div>
            </div>
          </Card>

          <Card
            icon={<Terminal size={20} style={{ color: post }} />}
            iconColor={post}
            title="protide-lsp"
            subtitle="Language Server · tower-lsp"
            body="Language Server Protocol support for .http files: hover docs, completion, diagnostics, semantic tokens, document symbols, formatting, inlay hints, rename, and code actions."
            delay={120}
          >
            <div className="flex flex-col gap-3">
              {EDITORS.map((e) => (
                <div
                  key={e.editor}
                  className="flex items-center justify-between border px-4 py-2.5"
                  style={{ borderColor: "var(--color-border)" }}
                >
                  <span className="text-sm font-medium" style={{ color: e.color }}>
                    {e.editor}
                  </span>
                  <span className="text-xs text-[var(--color-text-muted)]">{e.desc}</span>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </section>
  );
}
