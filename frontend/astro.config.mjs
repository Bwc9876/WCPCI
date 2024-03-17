import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";
import icon from "astro-icon";

// import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
    build: {
        format: "file"
    },
    prefetch: false,
    compressHTML: false,
    integrations: [tailwind({ nesting: true }), icon()]
});
