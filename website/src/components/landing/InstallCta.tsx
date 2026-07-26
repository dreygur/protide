import { LATEST_RELEASE_URL, REPO_URL } from "../../lib/site";
import { tint } from "../../lib/theme";
import { Button } from "./shared";
import { ArrowDown } from "./icons";

export default function InstallCta() {
  return (
    <section className="relative overflow-hidden px-6 py-24 text-center">
      <div
        className="pointer-events-none absolute bottom-0 left-1/2 h-[300px] w-[500px] -translate-x-1/2 rounded-full blur-3xl"
        style={{
          background: `radial-gradient(ellipse, ${tint("var(--color-accent)", 6)} 0%, transparent 70%)`,
        }}
      />

      <div data-animate className="relative mx-auto max-w-3xl">
        <h2 className="mb-4 text-3xl font-bold tracking-tight text-[var(--color-text-primary)] sm:text-4xl">
          Ready to ditch Postman?
        </h2>
        <p className="mx-auto mb-10 max-w-xl text-lg leading-relaxed text-[var(--color-text-secondary)]">
          Native performance. File-based collections. No account required. Free and open source
          forever.
        </p>

        <div className="mb-12 flex flex-wrap items-center justify-center gap-4">
          <Button href={LATEST_RELEASE_URL} variant="primary" external>
            <ArrowDown size={16} />
            Download latest release
          </Button>
          <Button href="/docs/getting-started/installation" variant="outline">
            Installation guide
          </Button>
        </div>

        <div
          className="inline-block border p-6 text-left font-mono text-sm"
          style={{ borderColor: "var(--color-border)", background: "var(--color-bg-secondary)" }}
        >
          <div className="mb-2 text-xs text-[var(--color-text-muted)]">
            Or install the LSP server via cargo:
          </div>
          <div className="flex items-center gap-3">
            <span className="text-[var(--color-text-muted)]">$</span>
            <span>
              <span style={{ color: "var(--color-method-post)" }}>cargo install</span>
              <span className="text-[var(--color-text-secondary)]"> --git </span>
              <span style={{ color: "var(--color-method-put)" }}>{REPO_URL}</span>
              <span className="text-[var(--color-text-secondary)]"> protide-lsp</span>
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}
