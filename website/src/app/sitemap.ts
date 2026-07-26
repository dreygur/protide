import type { MetadataRoute } from "next";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { SITE_URL } from "../lib/site";

// Required by `output: "export"` - the sitemap is generated once at build time.
export const dynamic = "force-static";

const CONTENT_DIR = join(process.cwd(), "src/content");

/** Every docs URL path, derived from the MDX files Nextra serves under /docs. */
function docPaths(dir = CONTENT_DIR, prefix = ""): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) {
      return docPaths(join(dir, entry.name), `${prefix}${entry.name}/`);
    }
    if (!entry.name.endsWith(".mdx")) return [];
    const slug = entry.name === "index.mdx" ? "" : `${entry.name.slice(0, -4)}/`;
    return [`docs/${prefix}${slug}`];
  });
}

export default function sitemap(): MetadataRoute.Sitemap {
  return [`${SITE_URL}/`, ...docPaths().map((p) => `${SITE_URL}/${p}`)].map((url) => ({
    url,
    changeFrequency: "weekly",
    priority: url.endsWith("/protide/") ? 1 : 0.7,
  }));
}
