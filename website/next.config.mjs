import nextra from "nextra";

// GitHub Pages serves the site from /protide. Exposed to the client too, because
// next/image does not prefix basePath when images are unoptimized.
const BASE_PATH = "/protide";

const withNextra = nextra({
  // Docs MDX lives in src/content and is served under /docs
  contentDirBasePath: "/docs",
  defaultShowCopyCode: true,
  search: { codeblocks: false },
});

export default withNextra({
  // GitHub Pages: static HTML in out/, served from /protide
  output: "export",
  basePath: BASE_PATH,
  env: { NEXT_PUBLIC_BASE_PATH: BASE_PATH },
  trailingSlash: true,
  images: { unoptimized: true },
  reactStrictMode: true,
});
