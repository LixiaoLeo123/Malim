<script lang="ts">
    import {
        currentView,
        editorDraft,
        articles,
        activeArticleId,
        settings,
        parsingQueue,
        isProcessingQueue,
    } from "../lib/stores";
    import { ArrowLeft, Check, ChevronDown } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import { invoke } from "@tauri-apps/api/core";
    import { v4 as uuidv4 } from "uuid";
    import Flag from "./Flag.svelte";
    import type { Article } from "../lib/types";
    import { listen } from "@tauri-apps/api/event";
    import { onDestroy } from "svelte";

    let showLangSelector = false;
    let wordCount = 0;
    let unlisten: (() => void) | null = null;
    $: wordCount = $editorDraft.content
        ? $editorDraft.content.trim().split(/\s+/).length
        : 0;

    function goBack() {
        currentView.set("home");
    }
    onDestroy(() => {
        // if (unlisten) unlisten();
    });
    async function processQueue() {
        if ($isProcessingQueue || $parsingQueue.length === 0) return;
        isProcessingQueue.set(true);
        const currentId = $parsingQueue[0];
        const currentArticle = $articles.find((a) => a.id === currentId);
        if (!currentArticle) {
            parsingQueue.update((q) => q.slice(1));
            isProcessingQueue.set(false);
            processQueue();
            return;
        }
        const unlisten = await listen<any>("parsing-progress", (event) => {
            const payload = event.payload;
            if (payload.id === currentId) {
                articles.update((items) =>
                    items.map((i) => {
                        if (i.id === currentId) {
                            return {
                                ...i,
                                parsingProgress: payload.percent,
                            };
                        }
                        return i;
                    }),
                );
            }
        });

        try {
            const result: any = await invoke("parse_text", {
                id: currentId,
                text: currentArticle.draftContent,
                language: currentArticle.language,
                apiKey: $settings.apiKey,
                apiUrl: $settings.apiUrl,
                modelName: $settings.modelName,
                concurrency: $settings.concurrency,
                oldSentences: currentArticle.sentences || null,
            });

            articles.update((items) =>
                items.map((i) => {
                    if (i.id === currentId) {
                        return {
                            ...i,
                            status: "done",
                            sentences: result,
                            parsingProgress: 100,
                        };
                    }
                    return i;
                }),
            );
        } catch (e) {
            console.error("Analysis Failed:", e);
            articles.update((items) =>
                items.map((i) =>
                    i.id === currentId
                        ? { ...i, status: "error", parsingProgress: 0 }
                        : i,
                ),
            );
            alert("AI Analysis Error: " + e);
        } finally {
            unlisten();
            parsingQueue.update((q) => q.slice(1));
            isProcessingQueue.set(false);
            processQueue();
        }
    }

    async function handleConfirm() {
        const contentSnapshot = $editorDraft.content;
        const languageSnapshot = $editorDraft.language;

        if (!contentSnapshot.trim()) return;

        if (!$settings.apiKey) {
            alert("Please configure your API first.");
            return;
        }

        const isEditMode = !!$activeArticleId;
        const id = isEditMode && $activeArticleId ? $activeArticleId : uuidv4();

        const existingArticle = isEditMode ? $articles.find(a => a.id === id) : null;

        const firstSentenceEnd = contentSnapshot.search(/[.。!?]\n?/);
        const newArticle: Article = {
            id: id,
            title:
                firstSentenceEnd !== -1
                    ? contentSnapshot.slice(0, firstSentenceEnd + 1) +
                      (contentSnapshot.length > firstSentenceEnd + 1
                          ? "..."
                          : "")
                    : contentSnapshot.slice(0, 20) +
                      (contentSnapshot.length > 20 ? "..." : ""),
            preview:
                firstSentenceEnd !== -1
                    ? contentSnapshot.slice(firstSentenceEnd + 1).slice(0, 50)
                    : contentSnapshot.slice(0, 50),
            status: "parsing",
            parsingProgress: 0,
            sentences: existingArticle?.sentences || [],
            draftContent: contentSnapshot,
            language: languageSnapshot,
        };

        if (isEditMode) {
            articles.update((items) =>
                items.map((i) => (i.id === id ? newArticle : i)),
            );
        } else {
            articles.update((items) => [newArticle, ...items]);
        }

        editorDraft.set({ title: "", content: "", language: "KR" });
        currentView.set("home");
        parsingQueue.update((q) => [...q, id]);
        processQueue();
    }
</script>

<div class="flex flex-col h-full bg-white relative dark:bg-zinc-950">
    <div
        class="flex justify-between items-center p-4 border-b border-zinc-100 dark:border-zinc-800"
    >
        <button
            on:click={goBack}
            class="p-2 -ml-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full text-zinc-600 transition duration-100 ease-out dark:text-zinc-300 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
        >
            <ArrowLeft size={24} />
        </button>

        <div class="relative z-50">
            <button
                on:click={() => (showLangSelector = !showLangSelector)}
                class="flex items-center space-x-2 px-2 py-1.5 bg-zinc-100 rounded-lg text-sm font-medium hover:bg-zinc-200 transition text-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
            >
                <Flag code={$editorDraft.language} size={20} />
                <span
                    >{$editorDraft.language === "KR"
                        ? "Korean"
                        : "Russian"}</span
                >
                <ChevronDown
                    size={14}
                    class="transition-transform {showLangSelector
                        ? 'rotate-180'
                        : ''}"
                />
            </button>

            {#if showLangSelector}
                <div
                    transition:slide
                    class="absolute top-full right-0 mt-2 w-40 bg-white border border-zinc-200 rounded-xl shadow-xl overflow-hidden dark:bg-zinc-900 dark:border-zinc-700"
                >
                    <button
                        class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                        on:click={() => {
                            $editorDraft.language = "KR";
                            showLangSelector = false;
                        }}
                    >
                        <Flag code="KR" size={18} />
                        <span>Korean</span>
                    </button>
                    <button
                        class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                        on:click={() => {
                            $editorDraft.language = "RU";
                            showLangSelector = false;
                        }}
                    >
                        <Flag code="RU" size={18} />
                        <span>Russian</span>
                    </button>
                </div>
            {/if}
        </div>
    </div>

    <textarea
        class="flex-1 w-full p-6 text-lg resize-none outline-none text-zinc-800 placeholder:text-zinc-300 leading-relaxed dark:text-zinc-100 dark:placeholder:text-zinc-600 dark:bg-transparent"
        placeholder="Paste your text here..."
        bind:value={$editorDraft.content}
    ></textarea>

    <div
        class="p-4 border-t border-zinc-100 flex justify-between items-center bg-white/90 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/90"
    >
        <span class="text-xs font-mono text-zinc-400">{wordCount} words</span>
        <button
            on:click={handleConfirm}
            class="bg-zinc-900 text-white p-3 rounded-full hover:bg-black transition shadow-lg hover:shadow-xl active:scale-95 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
        >
            <Check size={24} />
        </button>
    </div>
</div>
