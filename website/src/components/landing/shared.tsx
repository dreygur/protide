import { METHOD_COLORS, PROTOCOL_COLORS, tint } from "../../lib/theme";

export function Badge({
  label,
  type,
  size = "md",
}: {
  label: string;
  type: "method" | "protocol";
  size?: "sm" | "md";
}) {
  const color =
    (type === "method" ? METHOD_COLORS[label] : PROTOCOL_COLORS[label]) ?? "var(--color-accent)";
  const padding = size === "sm" ? "px-2 py-0.5 text-xs" : "px-3 py-1 text-sm";

  return (
    <span
      className={`font-semibold ${padding}`}
      style={{ color, background: tint(color, 12), border: `1px solid ${tint(color, 25)}` }}
    >
      {label}
    </span>
  );
}

const BUTTON_BASE =
  "inline-flex items-center gap-2 px-6 py-3 text-sm font-semibold no-underline transition-all duration-200";

const BUTTON_VARIANTS = {
  primary: `${BUTTON_BASE} bg-[var(--color-accent)] text-[var(--color-bg-primary)] hover:bg-[var(--color-accent-hover)]`,
  outline: `${BUTTON_BASE} border border-[var(--color-border)] bg-[var(--color-bg-elevated)] text-[var(--color-text-primary)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)]`,
};

export function Button({
  href,
  variant = "outline",
  external = false,
  children,
}: {
  href: string;
  variant?: keyof typeof BUTTON_VARIANTS;
  external?: boolean;
  children: React.ReactNode;
}) {
  return (
    <a
      href={href}
      className={BUTTON_VARIANTS[variant]}
      {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
    >
      {children}
    </a>
  );
}

export function SectionHeader({
  title,
  subtitle,
  label,
  align = "center",
}: {
  title: string;
  subtitle?: string;
  label?: string;
  align?: "left" | "center";
}) {
  const centered = align === "center";

  return (
    <div data-animate className={`mb-16 ${centered ? "text-center" : ""}`}>
      {label && (
        <p className="mb-3 text-xs font-medium uppercase tracking-widest text-[var(--color-accent)]">
          {label}
        </p>
      )}
      <h2 className="mb-4 text-3xl font-bold tracking-tight text-[var(--color-text-primary)] sm:text-4xl">
        {title}
      </h2>
      {subtitle && (
        <p
          className={`leading-relaxed text-[var(--color-text-secondary)] ${
            centered ? "mx-auto max-w-xl" : "max-w-lg"
          }`}
        >
          {subtitle}
        </p>
      )}
    </div>
  );
}

/** The "→ text" bullet used by the feature sections. */
export function ArrowList({ items }: { items: string[] }) {
  return (
    <div className="flex flex-col gap-4">
      {items.map((text) => (
        <div key={text} className="flex items-start gap-3">
          <span className="mt-0.5 font-semibold text-[var(--color-accent)]">→</span>
          <span className="text-sm leading-relaxed text-[var(--color-text-secondary)]">{text}</span>
        </div>
      ))}
    </div>
  );
}
