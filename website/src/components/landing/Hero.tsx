import Image from "next/image";
import { APP_VERSION, asset, LATEST_RELEASE_URL, REPO_URL } from "../../lib/site";
import { tint } from "../../lib/theme";
import { Button } from "./shared";
import { ArrowDown, GitHub } from "./icons";

const delay = (ms: number) => ({ "--anim-delay": `${ms}ms` }) as React.CSSProperties;

export default function Hero() {
  return (
    <section className="relative overflow-hidden px-6 py-24 text-center">
      {/* Dot grid background */}
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.04]"
        style={{
          backgroundImage: "radial-gradient(var(--color-text-muted) 1px, transparent 1px)",
          backgroundSize: "28px 28px",
        }}
      />

      {/* Radial glow */}
      <div
        className="pointer-events-none absolute left-1/2 top-0 h-[500px] w-[700px] -translate-x-1/2 rounded-full blur-3xl"
        style={{
          background: `radial-gradient(ellipse, ${tint("var(--color-accent)", 8)} 0%, transparent 70%)`,
        }}
      />

      <div className="relative mx-auto max-w-5xl">
        <div
          data-animate
          className="mb-8 inline-flex items-center gap-2 rounded-full border px-4 py-1.5 text-xs"
          style={{
            ...delay(0),
            borderColor: "var(--color-border)",
            background: "var(--color-bg-elevated)",
            color: "var(--color-text-secondary)",
          }}
        >
          <span className="h-1.5 w-1.5 rounded-full" style={{ background: "var(--color-accent)" }} />
          Built with Rust &amp; GPUI &nbsp;·&nbsp; MIT License &nbsp;·&nbsp; v{APP_VERSION}{" "}
          &nbsp;·&nbsp;
          <span className="font-semibold" style={{ color: "var(--color-method-put)" }}>
            alpha
          </span>
        </div>

        <h1
          data-animate
          className="mb-6 text-5xl font-bold leading-tight tracking-tight text-[var(--color-text-primary)] sm:text-6xl lg:text-7xl"
          style={delay(80)}
        >
          API Testing,
          <br />
          <span style={{ color: "var(--color-accent)" }}>Natively Fast</span>
        </h1>

        <p
          data-animate
          className="mx-auto mb-10 max-w-2xl text-lg leading-relaxed text-[var(--color-text-secondary)]"
          style={delay(160)}
        >
          A GPU-accelerated desktop API client built with Rust. HTTP, GraphQL, WebSocket, gRPC, tRPC,
          and Socket.IO - all from a single{" "}
          <code
            className="px-1.5 py-0.5 text-base"
            style={{ background: "var(--color-bg-elevated)", color: "var(--color-accent)" }}
          >
            .http
          </code>{" "}
          file.
        </p>

        <div
          data-animate
          className="flex flex-wrap items-center justify-center gap-4"
          style={delay(240)}
        >
          <Button href={LATEST_RELEASE_URL} variant="primary" external>
            <ArrowDown size={16} />
            Download
          </Button>
          <Button href="/docs" variant="outline">
            Read the docs
          </Button>
          <Button href={REPO_URL} variant="outline" external>
            <GitHub size={16} />
            View on GitHub
          </Button>
        </div>

        {/* Screenshot */}
        <div
          data-animate
          className="mt-16 overflow-hidden border shadow-2xl"
          style={{ ...delay(380), borderColor: "var(--color-border)" }}
        >
          <Image
            src={asset("/screenshot.png")}
            alt="Protide - API testing tool screenshot"
            width={1280}
            height={800}
            className="w-full"
            priority
          />
        </div>
      </div>
    </section>
  );
}
