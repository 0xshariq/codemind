import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ['127.0.0.1'],
  experimental: {
    // Keep the generated Next.js agent guidance and feedback workflow enabled.
    agentFeedback: true,
  },
  // Cache Components is stable in Next 16 and provides the shared app shell.
  cacheComponents: true,
  // Next 16.3+ requires this alongside Cache Components; shell prefetching is intentional.
  partialPrefetching: true,
  // The matching compiler plugin is installed; keep compiler behavior explicit.
  reactCompiler: true,
  turbopack: {
    rules: {
      // Tailwind's Turbopack loader transforms the app's Tailwind v4 CSS entry.
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
