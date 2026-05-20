import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  site: "https://dreygur.github.io",
  base: "/protide",
  output: "static",
  vite: {
    plugins: [tailwindcss()],
  },
});
