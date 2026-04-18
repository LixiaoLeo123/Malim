# Malim 🧠

**A next-generation language learning platform powered by AI and Comprehensible Input.**

Malim is designed to help you acquire languages naturally through massive comprehensible input and output, simulating a native environment. It entirely discards the traditional "flashcard memorization" approach. Instead, it tracks your vocabulary implicitly as you read and interact, building a highly personalized map of your language retention.

Currently supports **Russian** and **Korean** (Japanese planned for the future).

## 🌟 Key Features

### 📖 Intelligent Reading & Parsing

* **Asynchronous Text Parsing:** Import articles and let Malim break them down into highly detailed, interactive linguistic blocks.
* **Color-Coded Morphology:** Parts of speech are visually distinct. Verbs are red, nouns are blue (with color temperature shifts based on gender), adverbs are green, adjectives are yellow, and particles are gray.
* **Deep Russian Integration:** Automatically tags grammatical cases (1-6) and provides precise stress marking (via `ruaccent`). Click any word for translations, grammatical breakdowns, roots, tenses, and verb aspects.
* **Korean Support:** Displays Hanja (Chinese roots) to help deduce vocabulary meanings.
* **Checkpoint System:** Texts are chunked into 5-sentence blocks with "Save Points." Clicking a save point updates your daily reading volume and implicitly logs the unclicked words as "known" in your database.

### 🧠 Implicit Memory Tracking

* **Zero-Click Flashcards:** The system tracks your memory in the background. If you click a word for a definition, Malim logs that you forgot it. If you pass a checkpoint without clicking, it assumes retention.
* **Brain Visualization:** View your entire vocabulary mapped as a "Brain." Words closer to the core have higher stability, with colors (Green -> Blue -> Pink) indicating retention strength.
* **Live Statistics:** Real-time estimates of your active vocabulary size, learning rate, and a 30-day reading volume chart.

### 💬 Context-Aware AI Chat

* **Long-Term Memory Architecture:** The chat utilizes a complex RAG (Retrieval-Augmented Generation) pipeline, including memory compression (Shadow AI) and embedding models, allowing the AI to remember your past conversations.
* **Auto-Grammar Correction:** Sends your messages through a dedicated grammar-correction pipeline, giving you instant feedback on your output.
* **Instant Translation:** Type an English sentence followed by a `?` to instantly swap it for a localized Russian translation (powered by an int8-quantized `opus-mt-ru-en` model), complete with stress marks.
* **Interactive Chat:** Long-press (or right-click) any AI message to parse it just like a library article.

### 🎯 Adaptive Content Generation

* **Custom Prompts:** Malim calculates your exact memory parameters to generate a highly specific LLM prompt. Feed this prompt into any AI to generate reading materials precisely tailored to your current vocabulary level, preferred topic, and length.

### 🎧 High-Quality TTS (Text-to-Speech)

* **Multiple Engines:** Supports `edge-tts` (no config needed), `qwen3-tts` (requires API key), and `silero-tts` (self-hosted).
* **Smart Caching:** Pre-caches audio during text parsing to ensure zero-latency playback while reading.

---

## ⚙️ Architecture & Configuration

Malim utilizes a "Bring Your Own Key" (BYOK) architecture, giving you complete control over your LLM pipeline.

**Customizable AI Roles:**
You can assign different API keys, Base URLs, and Model Names to 5 distinct tasks:

* **Default AI:** For text parsing and grammar explanations.
* **Main Chat AI:** The conversational model.
* **Shadow AI:** Handles background memory compression and summarization.
* **Embedding Model:** Drives the RAG vector database (Note: Do not change this model once set, or memory will fragment).
* **Grammar Correction AI:** Dedicated to fixing user syntax.

**Performance & Sync:**

* **Concurrency Settings:** Adjust parsing and TTS concurrency to maximize speed (Note: Please respect your API provider's rate limits).
* **Remote Sync Server:** Run the included `sync_server.rs` script to push/pull your SQLite memory data across multiple platforms.
* **Local Processing Servers:** Includes `ru_accent_server.py` for highly accurate Russian stress marking and `silero_server.py` for local TTS processing.

---

## 🛠️ Data Management

Malim keeps your data local and transparent. You can easily export or import your core databases:

* `data.json`: User settings and parsed Library cache.
* `chat.db`: Complete chat histories and context memory.
* `memory.db`: Spaced repetition and implicit vocabulary tracking data.

## 🌗 UI & Accessibility

* Full support for both Light and Dark modes.
* Content Discovery module to easily import web articles for study.
* Clean, familiar interfaces designed for long study sessions without cognitive overload.
