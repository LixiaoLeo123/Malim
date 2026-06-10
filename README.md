<p align="center">
  <img src="app-icon.png" alt="Malim Logo" width="140" />
</p>

<h1 align="center">Malim</h1>

<p align="center">
  <strong>Learn languages through immersion — reading, listening, and context.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/version-0.5.1-informational" alt="Version" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

---

**Malim** is a desktop language-learning app built on the idea of **immersive contextual acquisition**. Instead of grinding flashcards and memorising grammar tables, you read real texts while the app helps you decode them — colour-coded parts of speech, instant dictionary lookups, and AI-powered analysis let your brain pick up patterns the way it naturally does: through repeated exposure in context.

<br />

##  Features

###  AI-Powered Text Analysis

Drop in any text and get it annotated by AI — part of speech, stress marks, grammatical case, and gender for every token.

- **Colour coding** — <span style="color:#ef4444">red → verbs</span>&ensp;|&ensp;<span style="color:#eab308">yellow → adjectives</span>&ensp;|&ensp;<span style="color:#6366f1">blue shades → nouns</span> (purple-blue / blue / sky-blue for masculine / neuter / feminine)&ensp;|&ensp;<span style="color:#9ca3af">grey → pronouns, particles, etc.</span>
- **Word Mode** — click a token to see its meaning, lemma (with stress marks), tense/aspect (verbs), and number (nouns)
- **Sentence Mode** — click for a full sentence translation with optional TTS playback
- **Right-click / long-press** — quick dictionary lookup on any token
- **Edit & re-parse** — the parser caches previous results and automatically fills gaps caused by network glitches
- **Concurrent parsing** with configurable batch size to balance speed and accuracy across different LLM APIs

<br />

###  Offline Dictionary

Built on the OpenRussian dataset — works entirely offline:

- Part of speech, gender, Chinese translations
- Rich example sentences
- Full conjugation & declension tables
- Synonyms

Accessible with a right-click on any token, or from the sidebar.

<br />

###  Translator Lab

Offline English → Russian translation with near-instant results. Translated output can be sent straight into the text analyser for deeper breakdown — great for quick sentence study.

<br />

###  AI Chat

A long-term-memory AI conversation partner for practising your target language:

- **Type English + `?`** — auto-translates your message on the fly so you can stay in the flow
- **Grammar correction** — every outgoing message is automatically proofread
- **Quote & reply** — right-click a message bubble to quote and respond in context
- **Parse AI replies** — right-click the bot's messages to analyse them inline, with translations shown beneath each token
- **Long-term memory** — the AI remembers past conversations via compression, RAG, and consolidation
- **Proactive messaging** — the AI may occasionally start a conversation on its own

<br />

###  Memory Tracking & Visualisation

A spaced-repetition-inspired model tracks your familiarity with every word you encounter:

- **Automatic logging** — clicking a token = "don't know"; reaching a checkpoint = "now I know everything in between"
- **Statistics dashboard** — real-time vocabulary size, learning velocity, and more
- **3D Brain** — every point is a word; red → blue → green encodes recall strength, outer → inner layers encode memory stability. Pan, zoom, and rotate freely

<br />

###  Discover — Graded Reading

Browse news in your target language with automatic difficulty & relevance scoring:

- Multiple languages and news sources
- AI scores every article based on **your** memory model — recommendation score reflects the share of words you're right on the verge of forgetting
- Tokens are coloured **green / blue / purple** by your estimated mastery; unseen words appear plain
- One-click add to drafts or parse directly

<br />

###  AI Prompt Generator

Generate writing prompts at a target difficulty, length, and topic — useful when you can't find graded reading material at the right level.

<br />

##  Tech Stack

```
┌─────────────────────────────────────────┐
│             SvelteKit 5                  │
│         (Frontend + Routing)             │
├─────────────────────────────────────────┤
│              Tauri 2                     │
│         (IPC + Native APIs)              │
├─────────────────────────────────────────┤
│            Rust Backend                  │
│  ┌──────────┬──────────┬──────────────┐  │
│  │ Morphol- │  TTS     │  Memory /    │  │
│  │ ogy      │  Engine  │  Embedding   │  │
│  │ (rsmorphy│  (Edge /  │  (ONNX)      │  │
│  │  + dict) │  Qwen /   │              │  │
│  │          │  Silero)  │              │  │
│  └──────────┴──────────┴──────────────┘  │
└─────────────────────────────────────────┘
```

| Layer | Technology |
|:---|:---|
| **Desktop Shell** | Tauri 2 (Rust) |
| **Frontend** | SvelteKit 5 + Tailwind CSS 4 + TypeScript |
| **Morphology** | rsmorphy + OpenRussian dictionary |
| **TTS** | Edge TTS / Qwen3 TTS / Silero TTS |
| **Memory Model** | Custom spaced-repetition model + ONNX embeddings |
| **Offline Translation** | ONNX Runtime |
| **Database** | SQLite (rusqlite) |
| **HTTP** | reqwest (Rust), rustls TLS |

<br />

##  Getting Started

### Prerequisites

- [Node.js](https://nodejs.org) ≥ 18
- [Rust](https://www.rust-lang.org/tools/install) toolchain
- Linux system dependencies:
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
                   librsvg2-dev libssl-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

### Development

```bash
git clone https://github.com/LixiaoLeo123/Malim.git
cd Malim

npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Packaged installers land in `src-tauri/target/release/bundle/`.

<br />

##  Configuration

### 1. API Keys

Go to **Settings → AI Configuration** and add your models:

| Role | Purpose | Model Type |
|:---|:---|:---|
| **Default** | Text analysis | Text-in/text-out (e.g. DeepSeek-V4) |
| **Main Chat AI** | Chat replies | Text model |
| **Shadow AI** | Memory consolidation | Text model |
| **Grammar Correction** | Proofreading | Text model |
| **Embedding Model** | Memory retrieval | **Must be an embedding model** (e.g. `text-embedding-v4`) |

> At minimum, configure **Default** to use text analysis. Full chat features require all five.

### 2. Stress Marks (optional)

Enable `Enable Precise RU Accentuation` for higher Russian stress-mark accuracy. A community server is available, or host your own with the included `ru_accent_server.py`.

### 3. Text-to-Speech

| Engine | Setup | Notes |
|:---|:---|:---|
| **Edge TTS** | None needed | Free, decent quality |
| **Qwen3 TTS** | API Key required | Best for Korean |
| **Silero TTS** | Self-hosted server (`silero_server.py`) | Best for Russian |

### 4. Cloud Sync

Enable `Enable Remote Sync Server` to sync memory data across devices (text and settings are not synced). Provide a Sync Server URL and a unique User ID.

<br />

##  Self-Hosted Services (optional)

```bash
# Silero TTS server (Russian TTS)
python silero_server.py

# Russian stress-mark server
python ru_accent_server.py
```

<br />

##  Project Layout

```
Malim/
├── src/                      # SvelteKit frontend
│   ├── routes/               # Page routes
│   ├── components/           # UI components
│   │   ├── Brain.svelte            # 3D memory brain
│   │   ├── Chat.svelte             # AI chat
│   │   ├── Dictionary.svelte       # Offline dictionary
│   │   ├── Discover.svelte         # Graded reading
│   │   ├── Editor.svelte           # Text editor
│   │   ├── Reader.svelte           # Parsed-text reader
│   │   ├── TranslatorLab.svelte    # Translation lab
│   │   └── ...
│   └── lib/                  # Types, stores, utilities
├── src-tauri/                # Rust backend
│   └── src/
│       ├── chat/             # AI chat logic
│       ├── dict/             # Dictionary engine
│       ├── translation/      # Offline translation
│       ├── brain.rs          # Memory model
│       ├── memory.rs         # Persistent memory
│       └── ...
├── ru_accent_server.py       # Stress-mark service
├── silero_server.py          # Silero TTS service
└── ...
```

<br />

##  License

MIT — see [LICENSE](LICENSE).

All API keys are stored **locally only** and are never uploaded. The full source is open for inspection at any time.

<br />

<p align="center">
  <sub>Built by <a href="https://github.com/LixiaoLeo123">Drantiss</a></sub>
</p>
