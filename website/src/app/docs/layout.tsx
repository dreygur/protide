import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { getPageMap } from "nextra/page-map";
import Image from "next/image";
import { APP_VERSION, asset, LATEST_RELEASE_URL, REPO_URL } from "../../lib/site";

const logo = (
  <span className="flex items-center gap-2.5">
    <Image src={asset("/logo.png")} alt="" width={24} height={24} aria-hidden />
    <b>Protide</b>
    <span
      className="px-1.5 py-0.5 text-[10px] font-semibold"
      style={{
        color: "var(--color-method-put)",
        background: "color-mix(in srgb, var(--color-method-put) 15%, transparent)",
        border: "1px solid color-mix(in srgb, var(--color-method-put) 30%, transparent)",
      }}
    >
      alpha
    </span>
  </span>
);

export default async function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <Layout
      navbar={
        <Navbar logo={logo} logoLink="/" projectLink={REPO_URL}>
          <a
            href={LATEST_RELEASE_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="whitespace-nowrap px-3 py-1.5 text-sm font-semibold max-sm:hidden"
            style={{ background: "var(--color-accent)", color: "var(--color-bg-primary)" }}
          >
            Download
          </a>
        </Navbar>
      }
      footer={
        <Footer>
          Protide v{APP_VERSION} &nbsp;·&nbsp; MIT License &nbsp;·&nbsp; © {new Date().getFullYear()}
        </Footer>
      }
      editLink="Edit this page on GitHub"
      docsRepositoryBase={`${REPO_URL}/blob/main/website`}
      sidebar={{ defaultMenuCollapseLevel: 1 }}
      pageMap={await getPageMap("/docs")}
    >
      {children}
    </Layout>
  );
}
