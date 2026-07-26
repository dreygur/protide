import Image from "next/image";
import Link from "next/link";
import { asset, ISSUES_URL, LATEST_RELEASE_URL, REPO_URL } from "../../lib/site";
import { tint } from "../../lib/theme";
import { GitHub, Plus } from "./icons";

const OUTLINE_LINK =
  "flex items-center gap-2 border border-[var(--color-border)] px-3 py-2 text-sm text-[var(--color-text-secondary)] transition-colors hover:border-[var(--color-accent)] hover:text-[var(--color-accent)]";

export default function Nav() {
  return (
    <nav className="sticky top-0 z-50 border-b border-[var(--color-border)] bg-[var(--color-bg-primary)]/90 backdrop-blur-md">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <Link href="/" className="flex items-center gap-3 no-underline">
          <Image src={asset("/logo.png")} alt="Protide" width={32} height={32} className="h-8 w-8" />
          <span className="text-base font-semibold tracking-tight text-[var(--color-text-primary)]">
            Protide
          </span>
          <span
            className="rounded-full px-2 py-0.5 text-xs font-semibold"
            style={{
              color: "var(--color-method-put)",
              background: tint("var(--color-method-put)", 15),
              border: `1px solid ${tint("var(--color-method-put)", 30)}`,
            }}
          >
            alpha
          </span>
        </Link>

        <div className="flex items-center gap-3">
          <Link
            href="/docs"
            className={`${OUTLINE_LINK} max-sm:hidden no-underline`}
            aria-label="Read the documentation"
          >
            Docs
          </Link>
          <a
            href={ISSUES_URL + "/new"}
            target="_blank"
            rel="noopener noreferrer"
            className={`${OUTLINE_LINK} max-sm:hidden`}
            aria-label="Open an issue"
          >
            <Plus size={16} />
            Open Issue
          </a>
          <a
            href={LATEST_RELEASE_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="px-4 py-2 text-sm font-semibold text-[var(--color-bg-primary)] transition-all max-sm:hidden"
            style={{ background: "var(--color-accent)" }}
          >
            Download
          </a>
          <a
            href={REPO_URL}
            target="_blank"
            rel="noopener noreferrer"
            className={OUTLINE_LINK}
            aria-label="View on GitHub"
          >
            <GitHub size={18} />
            <span className="max-sm:hidden">GitHub</span>
          </a>
        </div>
      </div>
    </nav>
  );
}
