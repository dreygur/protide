import Image from "next/image";
import Link from "next/link";
import { asset, ISSUES_URL, RELEASES_URL, REPO_URL } from "../../lib/site";

const EXTERNAL_LINKS = [
  { label: "GitHub", href: REPO_URL },
  { label: "Releases", href: RELEASES_URL },
  { label: "Issues", href: ISSUES_URL },
];

export default function Footer() {
  return (
    <footer className="border-t px-6 py-12" style={{ borderColor: "var(--color-border)" }}>
      <div className="mx-auto max-w-6xl">
        <div className="flex flex-col items-center gap-6 text-center sm:flex-row sm:justify-between sm:text-left">
          <div className="flex items-center gap-3">
            <Image
              src={asset("/logo.png")}
              alt="Protide"
              width={24}
              height={24}
              className="h-6 w-6 opacity-60"
            />
            <span className="text-sm text-[var(--color-text-secondary)]">
              Protide &copy; {new Date().getFullYear()} &nbsp;·&nbsp; MIT License
            </span>
          </div>

          <div className="flex items-center gap-6 text-sm text-[var(--color-text-secondary)]">
            <Link href="/docs" className="no-underline transition-colors hover:text-[var(--color-accent)]">
              Docs
            </Link>
            {EXTERNAL_LINKS.map((link) => (
              <a
                key={link.label}
                href={link.href}
                target="_blank"
                rel="noopener noreferrer"
                className="no-underline transition-colors hover:text-[var(--color-accent)]"
              >
                {link.label}
              </a>
            ))}
          </div>
        </div>
      </div>
    </footer>
  );
}
