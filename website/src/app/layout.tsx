import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

const SITE_URL = "https://ggnmem.mytechy.in";
const SITE_TITLE = "ggnmem — Semantic Terminal Memory Engine";
const SITE_DESCRIPTION =
  "AI-powered terminal memory for developers. Search commands using keywords, semantic search, and natural language. Local-first, private, and offline.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: SITE_TITLE,
    template: "%s | ggnmem",
  },
  description: SITE_DESCRIPTION,
  keywords: [
    "terminal",
    "shell history",
    "command line",
    "semantic search",
    "AI",
    "developer tools",
    "Linux",
    "WSL",
    "CLI",
    "open source",
    "ggnmem",
    "terminal memory",
    "command search",
    "natural language",
    "local AI",
    "DevOps",
  ],
  authors: [{ name: "Gagan Rajput" }],
  creator: "Gagan Rajput",
  publisher: "Gagan Rajput",
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  openGraph: {
    type: "website",
    locale: "en_US",
    url: SITE_URL,
    siteName: "ggnmem",
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "ggnmem — Semantic Terminal Memory Engine",
        type: "image/png",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    images: ["/og-image.png"],
  },
  alternates: {
    canonical: SITE_URL,
  },
  icons: {
    icon: "/logo.png",
    apple: "/logo.png",
  },
};

/* JSON-LD Structured Data */
const jsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "ggnmem",
  description: SITE_DESCRIPTION,
  applicationCategory: "DeveloperApplication",
  operatingSystem: "Linux",
  offers: {
    "@type": "Offer",
    price: "0",
    priceCurrency: "USD",
  },
  author: {
    "@type": "Person",
    name: "Gagan Rajput",
  },
  url: SITE_URL,
  license: "https://opensource.org/licenses/MIT",
  softwareVersion: "0.3.7-alpha",
  downloadUrl:
    "https://github.com/gagansokhal-coder/Terminal_helper/releases",
  codeRepository: "https://github.com/gagansokhal-coder/Terminal_helper",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} antialiased`}
    >
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </head>
      <body className="min-h-screen flex flex-col">{children}</body>
    </html>
  );
}
