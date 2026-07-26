import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import { Head } from "nextra/components";
import "nextra-theme-docs/style.css";
import "../styles/globals.css";
import { SITE_DESCRIPTION, SITE_URL, jsonLd } from "../lib/site";

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700"],
  style: ["normal", "italic"],
  variable: "--font-jetbrains-mono",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: "Protide - Native API Testing Tool",
    template: "%s - Protide",
  },
  description: SITE_DESCRIPTION,
  applicationName: "Protide",
  authors: [{ name: "dreygur", url: "https://github.com/dreygur" }],
  keywords: [
    "API testing",
    "HTTP client",
    "REST client",
    "GraphQL client",
    "gRPC client",
    "WebSocket",
    "Rust",
    "native",
    "desktop",
    "Postman alternative",
    "Bruno alternative",
  ],
  openGraph: {
    type: "website",
    siteName: "Protide",
    locale: "en_US",
    url: SITE_URL,
    images: [
      {
        url: "/screenshot.png",
        width: 1280,
        height: 800,
        alt: "Protide API testing tool screenshot",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    images: ["/screenshot.png"],
  },
  icons: {
    icon: [
      { url: "/logo.svg", type: "image/svg+xml" },
      { url: "/logo.png", type: "image/png" },
    ],
    apple: "/logo.png",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" dir="ltr" className={jetbrainsMono.variable} suppressHydrationWarning>
      <Head color={{ hue: 142, saturation: 71 }} backgroundColor={{ dark: "#0d0d0f", light: "#ffffff" }} />
      <body>
        {/* Server-rendered so the tag reaches the HTML - `Head` is a client component,
            where an inline script would never run. */}
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
        {children}
      </body>
    </html>
  );
}
