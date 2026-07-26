export const SITE_URL = "https://dreygur.js.org/protide";

/** Set from `basePath` in next.config.mjs. next/image skips it for unoptimized images. */
const BASE_PATH = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

/** Absolute URL for a file in public/, including the base path. */
export const asset = (path: string) => `${BASE_PATH}${path}`;
export const REPO_URL = "https://github.com/dreygur/protide";
export const RELEASES_URL = `${REPO_URL}/releases`;
export const LATEST_RELEASE_URL = `${RELEASES_URL}/latest`;
export const ISSUES_URL = `${REPO_URL}/issues`;
export const APP_VERSION = "0.1.0-alpha.4";

export const SITE_DESCRIPTION =
  "A fast, GPU-accelerated desktop API testing tool built with Rust. HTTP, GraphQL, WebSocket, gRPC, tRPC, and Socket.IO - all from a single .http file. No Electron, no slow startup.";

export const jsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "Protide",
  applicationCategory: "DeveloperApplication",
  operatingSystem: "macOS, Linux",
  description: SITE_DESCRIPTION,
  url: SITE_URL,
  softwareVersion: APP_VERSION,
  license: "https://opensource.org/licenses/MIT",
  author: { "@type": "Person", name: "dreygur", url: "https://github.com/dreygur" },
  codeRepository: REPO_URL,
  offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
};
