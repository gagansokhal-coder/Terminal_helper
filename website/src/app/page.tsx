"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import Image from "next/image";

/* ══════════════════════════════════════════════════════
   CONSTANTS
   ══════════════════════════════════════════════════════ */

const GITHUB_URL = "https://github.com/gagansokhal-coder/Terminal_helper";
const DOCS_URL = `${GITHUB_URL}/blob/main/INSTALL.md`;
const RELEASES_URL = `${GITHUB_URL}/releases`;
const LICENSE_URL = `${GITHUB_URL}/blob/main/LICENSE`;
const INSTALL_CMD =
  'curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash';

const FEATURES = [
  {
    icon: "⚡",
    title: "Automatic Command Capture",
    desc: "Captures every shell command in under 10ms with zero-friction hooks for Bash, Zsh, and Fish.",
  },
  {
    icon: "🔍",
    title: "Hybrid Search (FTS5 + Semantic)",
    desc: "Combines full-text keyword matching with AI-powered semantic understanding via Reciprocal Rank Fusion.",
  },
  {
    icon: "💬",
    title: "Natural Language Search",
    desc: 'Ask in plain English — "show running containers" returns `docker ps` instantly from the built-in knowledge base.',
  },
  {
    icon: "🖥️",
    title: "Interactive TUI",
    desc: "A full-screen Ctrl+R replacement with mode cycling, command previews, and keyboard-driven navigation.",
  },
  {
    icon: "🧠",
    title: "AI-Powered Embeddings",
    desc: "Local ONNX models (MiniLM, BGE) generate vector embeddings on your CPU. No API keys required.",
  },
  {
    icon: "🔄",
    title: "Self Update",
    desc: "One command to fetch, verify checksums, swap binaries, and restart — preserving your config and database.",
  },
  {
    icon: "📦",
    title: "One-Line Installer",
    desc: "A single curl command detects your architecture, downloads pre-built binaries, and configures shell hooks.",
  },
  {
    icon: "🔒",
    title: "Local First & Private",
    desc: "Zero cloud dependencies, no telemetry, no API calls. Secret redaction scrubs keys before storage.",
  },
];

const SCREENSHOTS = [
  {
    src: "/screenshots/tui-search.png",
    alt: "Interactive TUI Search Interface",
    caption: "Interactive Terminal Search",
  },
  {
    src: "/screenshots/ask-command.png",
    alt: "Natural Language Ask Command",
    caption: "Natural Language Queries",
  },
  {
    src: "/screenshots/self-update.png",
    alt: "Self Update Process",
    caption: "Self-Updating System",
  },
];

const ARCHITECTURE_STEPS = [
  { icon: "⌨️", label: "Shell", desc: "Your terminal session" },
  { icon: "🪝", label: "Capture Hook", desc: "Zsh/Bash/Fish hooks" },
  { icon: "⚙️", label: "Daemon", desc: "Background processor" },
  {
    icon: "🗄️",
    label: "SQLite + FTS5 + Vector DB",
    desc: "Persistent storage & indexing",
  },
  {
    icon: "🤖",
    label: "Search / Semantic Search / AI",
    desc: "Hybrid retrieval engine",
  },
];

const QUICKSTART_STEPS = [
  { step: 1, label: "Install ggnmem", cmd: INSTALL_CMD },
  { step: 2, label: "Verify installation", cmd: "ggnmem doctor" },
  { step: 3, label: "Start the daemon", cmd: "ggnmem start" },
  { step: 4, label: "Search your history", cmd: "ggnmem search docker" },
  {
    step: 5,
    label: "Ask in natural language",
    cmd: 'ggnmem ask "show running containers"',
  },
];

const EXAMPLES = [
  { cmd: "ggnmem search docker", desc: "Keyword search across history" },
  {
    cmd: 'ggnmem semantic "postgres backup"',
    desc: "Search by meaning, not syntax",
  },
  {
    cmd: 'ggnmem ask "show running containers"',
    desc: "Natural language queries",
  },
  { cmd: "ggnmem self-update", desc: "One-command upgrade" },
];

const STATS = [
  { label: "Version", value: "v0.3.7-alpha" },
  { label: "License", value: "MIT" },
  { label: "Platform", value: "Linux x86_64, aarch64" },
  { label: "Status", value: "Pre-Alpha" },
];

const COMPLETED_ITEMS = [
  "Phase 22 — Self Update",
  "Phase 23 — Installer",
  "Phase 24 — Documentation",
];

const FUTURE_ITEMS = [
  "Cloud Sync",
  "Team Workspaces",
  "Plugin System",
  "Enterprise Features",
];

/* ══════════════════════════════════════════════════════
   HOOKS
   ══════════════════════════════════════════════════════ */

function useScrollReveal() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("visible");
          }
        });
      },
      { threshold: 0.1, rootMargin: "0px 0px -40px 0px" }
    );

    const items = el.querySelectorAll(".fade-in-up");
    items.forEach((item) => observer.observe(item));

    return () => observer.disconnect();
  }, []);

  return ref;
}

/* ══════════════════════════════════════════════════════
   COMPONENTS
   ══════════════════════════════════════════════════════ */

function CopyButton({ text, className = "" }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* fallback: select the text */
    }
  }, [text]);

  return (
    <button
      onClick={copy}
      className={`copy-btn ${className}`}
      aria-label="Copy to clipboard"
    >
      {copied ? "✓ Copied" : "Copy"}
    </button>
  );
}

function TerminalBlock({
  children,
  copyText,
}: {
  children: React.ReactNode;
  copyText?: string;
}) {
  return (
    <div className="terminal-block relative">
      <div className="terminal-dots">
        <span className="bg-red-500/80" />
        <span className="bg-yellow-500/80" />
        <span className="bg-green-500/80" />
      </div>
      {copyText && <CopyButton text={copyText} />}
      <div className="px-5 pb-5 pt-1 text-sm leading-relaxed overflow-x-auto">
        {children}
      </div>
    </div>
  );
}

/* ── Navbar ── */
function Navbar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 inset-x-0 z-50 transition-all duration-300 ${
        scrolled ? "nav-blur shadow-lg" : ""
      }`}
    >
      <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
        <a
          href="#hero"
          className="flex items-center gap-3 text-lg font-bold tracking-tight"
        >
          <Image
            src="/logo.png"
            alt="ggnmem logo"
            width={32}
            height={32}
            className="rounded-lg"
          />
          <span className="gradient-text">ggnmem</span>
        </a>

        <div className="hidden md:flex items-center gap-6 text-sm text-[var(--text-secondary)]">
          <a href="#features" className="hover:text-white transition-colors">
            Features
          </a>
          <a href="#screenshots" className="hover:text-white transition-colors">
            Screenshots
          </a>
          <a href="#quickstart" className="hover:text-white transition-colors">
            Quick Start
          </a>
          <a href="#roadmap" className="hover:text-white transition-colors">
            Roadmap
          </a>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-secondary !py-2 !px-4 !text-sm"
          >
            <GithubIcon />
            GitHub
          </a>
        </div>
      </div>
    </nav>
  );
}

function GithubIcon() {
  return (
    <svg viewBox="0 0 24 24" className="w-4 h-4" fill="currentColor">
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}

/* ══════════════════════════════════════════════════════
   SECTIONS
   ══════════════════════════════════════════════════════ */

/* ── Section 1: Hero ── */
function HeroSection() {
  const [showInstall, setShowInstall] = useState(false);

  return (
    <section
      id="hero"
      className="relative min-h-screen flex items-center justify-center bg-grid"
    >
      <div className="hero-bg" />

      <div className="relative z-10 max-w-4xl mx-auto px-6 text-center py-32">
        {/* Logo */}
        <div className="mb-8 animate-float">
          <Image
            src="/logo.png"
            alt="ggnmem logo"
            width={96}
            height={96}
            className="mx-auto rounded-2xl shadow-2xl"
            priority
          />
        </div>

        {/* Title */}
        <h1 className="text-6xl md:text-8xl font-black tracking-tighter mb-4">
          <span className="gradient-text">ggnmem</span>
        </h1>

        {/* Subtitle */}
        <p className="text-xl md:text-2xl font-semibold text-[var(--text-secondary)] mb-3">
          Semantic Terminal Memory Engine
        </p>

        {/* Tagline */}
        <p className="text-lg text-[var(--accent-cyan)] font-medium mb-6">
          Never forget terminal commands again.
        </p>

        {/* Description */}
        <p className="text-[var(--text-secondary)] text-base md:text-lg max-w-2xl mx-auto mb-10 leading-relaxed">
          Search your command history using keywords, semantic search, and
          natural language. Local-first, AI-powered, and completely private.
        </p>

        {/* Buttons */}
        <div className="flex flex-wrap items-center justify-center gap-4 mb-12">
          <button
            onClick={() => setShowInstall(true)}
            className="btn-primary text-base"
            id="install-btn"
          >
            <span>Install Now</span>
          </button>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-secondary text-base"
          >
            <GithubIcon />
            GitHub
          </a>
          <a
            href={DOCS_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-secondary text-base"
          >
            Documentation
          </a>
          <a
            href={RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="btn-secondary text-base"
          >
            Releases
          </a>
        </div>

        {/* Install Command Preview */}
        <div className="max-w-2xl mx-auto">
          <TerminalBlock copyText={INSTALL_CMD}>
            <code className="text-[var(--accent-green)] text-xs md:text-sm break-all">
              <span className="text-[var(--text-muted)]">$</span>{" "}
              {INSTALL_CMD}
            </code>
          </TerminalBlock>
        </div>
      </div>

      {/* Install Modal */}
      {showInstall && (
        <div
          className="install-modal-overlay"
          onClick={() => setShowInstall(false)}
        >
          <div
            className="install-modal"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-xl font-bold gradient-text">
                Install ggnmem
              </h3>
              <button
                onClick={() => setShowInstall(false)}
                className="text-[var(--text-muted)] hover:text-white transition-colors text-2xl leading-none"
                aria-label="Close"
              >
                ×
              </button>
            </div>
            <p className="text-[var(--text-secondary)] text-sm mb-4">
              Run this command in your terminal:
            </p>
            <TerminalBlock copyText={INSTALL_CMD}>
              <code className="text-[var(--accent-green)] text-xs md:text-sm break-all">
                <span className="text-[var(--text-muted)]">$</span>{" "}
                {INSTALL_CMD}
              </code>
            </TerminalBlock>
            <p className="text-[var(--text-muted)] text-xs mt-4">
              Supports Linux x86_64 and aarch64. Detects your architecture
              automatically.
            </p>
          </div>
        </div>
      )}
    </section>
  );
}

/* ── Section 2: Features ── */
function FeaturesSection() {
  const ref = useScrollReveal();

  return (
    <section id="features" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-7xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Features
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            Everything you need to make your terminal history intelligent,
            searchable, and private.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
          {FEATURES.map((f, i) => (
            <div
              key={f.title}
              className={`glass-card glow-border p-6 fade-in-up stagger-${i + 1}`}
            >
              <div className="text-3xl mb-4">{f.icon}</div>
              <h3 className="font-bold text-lg mb-2 text-white">{f.title}</h3>
              <p className="text-sm text-[var(--text-secondary)] leading-relaxed">
                {f.desc}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Section 3: Screenshots ── */
function ScreenshotsSection() {
  const ref = useScrollReveal();
  const [lightbox, setLightbox] = useState<string | null>(null);

  return (
    <section id="screenshots" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-7xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Screenshots
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            See ggnmem in action. Click any image to enlarge.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {SCREENSHOTS.map((s, i) => (
            <div
              key={s.src}
              className={`fade-in-up stagger-${i + 1}`}
              onClick={() => setLightbox(s.src)}
            >
              <div className="screenshot-card">
                <Image
                  src={s.src}
                  alt={s.alt}
                  width={600}
                  height={400}
                  className="w-full h-auto"
                />
              </div>
              <p className="text-center text-sm text-[var(--text-secondary)] mt-3 font-medium">
                {s.caption}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Lightbox */}
      {lightbox && (
        <div className="lightbox-overlay" onClick={() => setLightbox(null)}>
          <Image
            src={lightbox}
            alt="Screenshot enlarged"
            width={1200}
            height={800}
            className="object-contain"
          />
        </div>
      )}
    </section>
  );
}

/* ── Section 4: How It Works ── */
function HowItWorksSection() {
  const ref = useScrollReveal();

  return (
    <section id="architecture" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-3xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            How It Works
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            A modular architecture designed for speed, privacy, and reliability.
          </p>
        </div>

        <div className="space-y-0">
          {ARCHITECTURE_STEPS.map((step, i) => (
            <div key={step.label}>
              <div
                className={`glass-card p-6 flex items-center gap-5 fade-in-up stagger-${i + 1}`}
              >
                <div className="text-3xl flex-shrink-0 w-12 h-12 flex items-center justify-center rounded-xl bg-[rgba(56,189,248,0.08)]">
                  {step.icon}
                </div>
                <div>
                  <h3 className="font-bold text-white text-base">
                    {step.label}
                  </h3>
                  <p className="text-sm text-[var(--text-secondary)]">
                    {step.desc}
                  </p>
                </div>
              </div>
              {i < ARCHITECTURE_STEPS.length - 1 && (
                <div className="flow-connector" />
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Section 5: Quick Start ── */
function QuickStartSection() {
  const ref = useScrollReveal();

  return (
    <section id="quickstart" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-3xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Quick Start
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            From zero to semantic search in under a minute.
          </p>
        </div>

        <div className="space-y-6">
          {QUICKSTART_STEPS.map((s, i) => (
            <div
              key={s.step}
              className={`fade-in-up stagger-${i + 1}`}
            >
              <div className="flex items-center gap-4 mb-3">
                <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[var(--accent-cyan)] to-[var(--accent-purple)] flex items-center justify-center text-sm font-bold text-white flex-shrink-0">
                  {s.step}
                </div>
                <span className="font-semibold text-white">{s.label}</span>
              </div>
              <div className="ml-12">
                <TerminalBlock copyText={s.cmd}>
                  <code className="text-[var(--accent-green)] text-sm">
                    <span className="text-[var(--text-muted)]">$</span>{" "}
                    {s.cmd}
                  </code>
                </TerminalBlock>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Section 6: Examples ── */
function ExamplesSection() {
  const ref = useScrollReveal();

  return (
    <section id="examples" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-5xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Examples
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            Real commands you can run right after installing.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {EXAMPLES.map((ex, i) => (
            <div
              key={ex.cmd}
              className={`glass-card p-6 fade-in-up stagger-${i + 1}`}
            >
              <p className="text-sm text-[var(--text-secondary)] mb-3">
                {ex.desc}
              </p>
              <TerminalBlock copyText={ex.cmd}>
                <code className="text-[var(--accent-green)] text-sm">
                  <span className="text-[var(--text-muted)]">$</span> {ex.cmd}
                </code>
              </TerminalBlock>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Section 7: Project Stats ── */
function StatsSection() {
  const ref = useScrollReveal();

  return (
    <section id="stats" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-5xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Project Stats
          </h2>
        </div>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
          {STATS.map((s, i) => (
            <div
              key={s.label}
              className={`glass-card p-6 text-center fade-in-up stagger-${i + 1}`}
            >
              <p className="text-xs uppercase tracking-wider text-[var(--text-muted)] mb-2">
                {s.label}
              </p>
              <p className="stat-value gradient-text">{s.value}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

/* ── Section 8: Roadmap ── */
function RoadmapSection() {
  const ref = useScrollReveal();

  return (
    <section id="roadmap" className="py-24 md:py-32 relative" ref={ref}>
      <div className="max-w-5xl mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="section-title gradient-text fade-in-up mb-4">
            Roadmap
          </h2>
          <p className="section-subtitle fade-in-up stagger-1">
            Where we&apos;ve been and where we&apos;re going.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
          {/* Completed */}
          <div className="glass-card p-8 fade-in-up stagger-1">
            <h3 className="font-bold text-lg mb-6 flex items-center gap-2">
              <span className="text-[var(--accent-green)]">✓</span>
              Completed
            </h3>
            <ul className="space-y-4">
              {COMPLETED_ITEMS.map((item) => (
                <li
                  key={item}
                  className="flex items-center gap-3 text-[var(--text-secondary)]"
                >
                  <span className="roadmap-check text-sm">✓</span>
                  {item}
                </li>
              ))}
            </ul>
          </div>

          {/* Future */}
          <div className="glass-card p-8 fade-in-up stagger-2">
            <h3 className="font-bold text-lg mb-6 flex items-center gap-2">
              <span className="text-[var(--accent-purple)]">→</span>
              Coming Next
            </h3>
            <ul className="space-y-4">
              {FUTURE_ITEMS.map((item) => (
                <li
                  key={item}
                  className="flex items-center gap-3 text-[var(--text-secondary)]"
                >
                  <span className="roadmap-future text-sm">→</span>
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ── Section 9: Footer ── */
function Footer() {
  return (
    <footer className="border-t border-[var(--border-card)] py-12 mt-auto">
      <div className="max-w-7xl mx-auto px-6">
        <div className="flex flex-col md:flex-row items-center justify-between gap-6">
          {/* Links */}
          <div className="flex flex-wrap items-center justify-center gap-6 text-sm text-[var(--text-secondary)]">
            <a
              href={GITHUB_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--accent-cyan)] transition-colors"
            >
              GitHub
            </a>
            <a
              href={RELEASES_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--accent-cyan)] transition-colors"
            >
              Releases
            </a>
            <a
              href={LICENSE_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--accent-cyan)] transition-colors"
            >
              License
            </a>
            <a
              href={DOCS_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--accent-cyan)] transition-colors"
            >
              Documentation
            </a>
          </div>

          {/* Credit */}
          <p className="text-sm text-[var(--text-muted)]">
            Built by{" "}
            <span className="gradient-text font-semibold">Gagan Rajput</span>
          </p>
        </div>
      </div>
    </footer>
  );
}

/* ══════════════════════════════════════════════════════
   PAGE
   ══════════════════════════════════════════════════════ */

export default function Home() {
  return (
    <>
      <Navbar />
      <main>
        <HeroSection />
        <FeaturesSection />
        <ScreenshotsSection />
        <HowItWorksSection />
        <QuickStartSection />
        <ExamplesSection />
        <StatsSection />
        <RoadmapSection />
      </main>
      <Footer />
    </>
  );
}
