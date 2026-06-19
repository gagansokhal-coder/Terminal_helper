import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  const baseUrl = "https://ggnmem.mytechy.in";

  return [
    {
      url: baseUrl,
      lastModified: new Date("2026-06-19"),
      changeFrequency: "weekly",
      priority: 1,
    },
  ];
}
