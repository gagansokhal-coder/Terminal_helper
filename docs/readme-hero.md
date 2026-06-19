<div align="center">

# ggnmem

**The Semantic Terminal Memory Engine: Your shell history, understood—not just stored.**

</div>

---

## What is ggnmem?

**The Problem with Traditional History**  
Every developer has spent hours crafting the perfect multi-flag command—only to forget it weeks later. Traditional shell history is just a flat, literal log. When you press `Ctrl+R`, it searches for keystrokes, not intent. If you can't remember the exact syntax you used to extract a tarball or query a database, your shell history can't help you. 

**The Semantic Solution**  
ggnmem replaces your "dumb" history file with an intelligent, searchable memory engine. By running silently in the background, it captures every command and uses AI to generate vector embeddings. This means you can search your history by meaning instead of memorization. If you type "check git changes," ggnmem knows you're looking for `git status` or `git diff`. It fuses blazing-fast keyword search with local AI embeddings to deliver exactly what you meant to find.

**Local-First and Private**  
Your terminal history contains highly sensitive data, from server addresses to project names and secret tokens. We believe that intelligence shouldn't require compromising your privacy. That's why ggnmem is built on a strict local-first architecture. All AI inference runs directly on your CPU using lightweight, offline models. There are no API keys, no subscriptions, and no telemetry. Your data never leaves your machine.

---

### Key Benefits

*   **Never forget commands:** Automatically capture and index every shell operation in milliseconds.
*   **Search by meaning:** Find commands using natural language, not just exact syntax.
*   **AI-powered recall:** Leverage lightweight ONNX models (MiniLM, BGE) for intelligent intent matching.
*   **Local and private:** Zero cloud dependencies. No telemetry, no API calls, complete privacy.
*   **Works offline:** Fully functional on air-gapped systems after the initial model setup.

---

### See it in Action

```bash
# Blazing-fast keyword and hybrid search
ggnmem search docker

# Ask for help using natural language
ggnmem ask "show running containers"

# Semantic search for complex workflows
ggnmem semantic "postgres backup"
```
