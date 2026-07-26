import type { MDXComponents } from "nextra/mdx-components";
import { useMDXComponents as getDocsMDXComponents } from "nextra-theme-docs";
import { Callout, Cards, FileTree, Steps, Table, Tabs } from "nextra/components";

const docsComponents = getDocsMDXComponents({
  // Available in every MDX page without an import
  Callout,
  Cards,
  FileTree,
  Steps,
  Table,
  Tabs,
});

export const useMDXComponents = (components?: MDXComponents) => ({
  ...docsComponents,
  ...components,
});
