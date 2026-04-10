<script lang="ts">
    import { currentView } from "../lib/stores";
    import {
        ArrowLeft,
        ChevronDown,
        Brain,
        BookmarkPlus,
        FileEdit,
        Image as ImageIcon,
        RotateCcw,
    } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { editorDraft } from "../lib/stores";
    import { notifications } from "$lib/notificationStore";
    import { v4 as uuidv4 } from "uuid";
    import type { Article as ArticleType } from "../lib/types";
    import { articles as libraryArticles, parsingQueue } from "../lib/stores";
    import { processQueue } from "../lib/parser";

    interface SourceInfo {
        id: string;
        name: string;
        language: string;
    }

    interface Article {
        source_id: string;
        source_name: string;
        language: string;
        url: string;
        title: string;
        cover_image: string;
        content: string;
        words: Array<[string, number | null]> | null;
        difficulty: number | null;
        recommendation: number | null;
    }

    let currentLang: "kr" | "ru" = "ru";
    let showLangSelector = false;
    let markFamiliarity = true;
    let sources: SourceInfo[] = [];
    let selectedSourceIds: Set<string> = new Set();
    let articles: Article[] = [];
    let layoutClasses: string[] = [];
    let isLoading = false;
    let isResetting = false;
    let selectedArticle: Article | null = null;
    let feedContainer: HTMLDivElement;

    let reachedBottom = false;
    let fetchTriggered = false;
    let touchStartY = 0;

    const CUTOFF_LARGE = 0.25;
    const CUTOFF_WIDE = 0.5;
    const CUTOFF_TALL = 0.75;
    const WEIGHTS = [
        CUTOFF_LARGE,
        CUTOFF_WIDE - CUTOFF_LARGE,
        CUTOFF_TALL - CUTOFF_WIDE,
        1 - CUTOFF_TALL,
    ];
    const CLASSES = [
        "col-span-2 row-span-2",
        "col-span-2 row-span-1",
        "col-span-1 row-span-2",
        "col-span-1 row-span-1",
    ];

    let layoutThresholds = {
        t_large: 1,
        t_wide: 1,
        t_tall: 1,
    };

    function calculateThresholds(arts: Article[]) {
        if (!arts || arts.length === 0)
            return { t_large: 1, t_wide: 1, t_tall: 1 };
        const scores = arts
            .map((a) => a.recommendation)
            .filter((s): s is number => s !== null)
            .sort((a, b) => a - b);
        if (scores.length === 0) return { t_large: 1, t_wide: 1, t_tall: 1 };

        return {
            t_large: scores[Math.floor(scores.length * CUTOFF_LARGE)],
            t_wide: scores[Math.floor(scores.length * CUTOFF_WIDE)],
            t_tall: scores[Math.floor(scores.length * CUTOFF_TALL)],
        };
    }

    function getWeightedRandomClass(weights: number[], classes: string[]): string {
        const totalWeight = weights.reduce((sum, w) => sum + w, 0);
        if (totalWeight === 0) return classes[classes.length - 1];
        let random = Math.random() * totalWeight;
        for (let i = 0; i < weights.length; i++) {
            random -= weights[i];
            if (random <= 0) return classes[i];
        }
        return classes[classes.length - 1];
    }

    function getTileSpanClass(article: Article): string {
        const score = article.recommendation;
        const { t_large, t_wide, t_tall } = layoutThresholds;
        if (score === null) {
            return getWeightedRandomClass(WEIGHTS, CLASSES);
        }
        if (score > t_large) return CLASSES[0];
        if (score > t_wide) return CLASSES[1];
        if (score > t_tall) return CLASSES[2];
        if (score < t_tall) return CLASSES[3];

        const thresholds = [t_large, t_wide, t_tall];
        let startIdx = -1;
        let span = 0;
        for (let i = 0; i < thresholds.length; i++) {
            if (score === thresholds[i]) {
                if (startIdx === -1) startIdx = i;
                else span = i - startIdx;
            }
        }
        const endIdx = startIdx + span + 2;
        return getWeightedRandomClass(
            WEIGHTS.slice(startIdx, endIdx),
            CLASSES.slice(startIdx, endIdx)
        );
    }

    function getDifficultyRingClass(diff: number | null): string {
        if (diff === null) return "ring-transparent border-transparent";
        if (diff < 0.3)
            return "ring-green-400/50 border-green-500/30 dark:ring-green-500/40 dark:border-green-400/20";
        if (diff < 0.6)
            return "ring-amber-400/50 border-amber-500/30 dark:ring-amber-500/40 dark:border-amber-400/20";
        return "ring-red-400/50 border-red-500/30 dark:ring-red-400/50 dark:border-red-400/20";
    }

    function getFamiliarityColor(fam: number | null): string {
        if (fam === null) return "";
        if (fam < 0.4) return "text-purple-600 dark:text-purple-400 font-medium";
        if (fam <= 0.7) return "text-blue-500 dark:text-blue-400 font-medium";
        return "text-green-500 dark:text-green-400 font-medium";
    }

    async function loadSources() {
        try {
            sources = await invoke("get_sources_by_language", {
                lang: currentLang,
            });
            selectedSourceIds = new Set(sources.map((s) => s.id));
            if (articles.length > 0) {
                fetchFeed(false);
            }
        } catch (error) {
            console.error("Failed to load sources:", error);
        }
    }

    async function handleReset() {
        if (isResetting) return;
        isResetting = true;
        try {
            await invoke("clear_emitted_urls");
            articles = [];
            layoutClasses = [];
            fetchFeed(false);
        } catch (error) {
            console.error("Failed to reset feed:", error);
        } finally {
            setTimeout(() => {
                isResetting = false;
            }, 500);
        }
    }

    async function fetchFeed(append = false) {
        if (isLoading || selectedSourceIds.size === 0) return;
        isLoading = true;
        try {
            const req = {
                lang: currentLang,
                selected_source_ids: Array.from(selectedSourceIds),
                limit: 15,
                mark_familiarity: markFamiliarity,
            };
            const newArticles: Article[] = await invoke("get_feed", { req });
            layoutThresholds = calculateThresholds(newArticles);
            const newClasses = newArticles.map((a) => getTileSpanClass(a));

            if (append) {
                articles = [...articles, ...newArticles];
                layoutClasses = [...layoutClasses, ...newClasses];
            } else {
                articles = newArticles;
                layoutClasses = newClasses;
            }
        } catch (error) {
            console.error("Failed to fetch feed:", error);
            fetchTriggered = false;
        } finally {
            isLoading = false;
        }
    }

    function initialFetch() {
        fetchFeed(false);
    }


    function handleScroll() {
        if (!feedContainer) return;
        const isAtBottom =
            feedContainer.scrollHeight - feedContainer.scrollTop - feedContainer.clientHeight < 5;
        
        if (isAtBottom) {
            reachedBottom = true;
        } else {
            reachedBottom = false;
            fetchTriggered = false;
        }
    }

    function triggerFetch() {
        if (reachedBottom && !fetchTriggered && !isLoading && selectedSourceIds.size > 0) {
            fetchTriggered = true;
            fetchFeed(true);
        }
    }

    function handleWheel(e: WheelEvent) {
        if (e.deltaY > 0) {
            triggerFetch();
        }
    }

    function handleTouchStart(e: TouchEvent) {
        touchStartY = e.touches[0].clientY;
    }

    function handleTouchEnd(e: TouchEvent) {
        const touchEndY = e.changedTouches[0].clientY;
        if (touchEndY - touchStartY > 50) {
            triggerFetch();
        }
    }

    function toggleSource(id: string) {
        if (selectedSourceIds.has(id)) {
            selectedSourceIds.delete(id);
        } else {
            selectedSourceIds.add(id);
        }
        selectedSourceIds = selectedSourceIds;
        if (articles.length > 0) {
            fetchFeed(false);
        }
    }

    function toggleFamiliarity() {
        markFamiliarity = !markFamiliarity;
        if (articles.length > 0) {
            fetchFeed(false);
        }
    }

    function goBack() {
        currentView.set("home");
    }

    function closeArticle() {
        selectedArticle = null;
    }

    function addToDraft() {
        if (!selectedArticle) return;
        editorDraft.set({
            title: selectedArticle.title,
            content: selectedArticle.title + ".\n" + selectedArticle.content,
            language: selectedArticle.language,
        });
        // console.log("Added to draft:", selectedArticle?.title);
        notifications.success("Article added to draft!");
        closeArticle();
    }

    function addToLibrary() {
        // console.log("Added to library:", selectedArticle?.title);
        if (!selectedArticle) return;
        const id = uuidv4();
        const newArticle: ArticleType = {
            id: id,
            title: selectedArticle?.title || "Untitled",
            preview: selectedArticle?.content.slice(0, 50),
            status: "parsing",
            parsingProgress: 0,
            sentences: [],
            draftContent: selectedArticle?.title + ".\n" + selectedArticle?.content || "",
            language: selectedArticle?.language || "RU",
            readProgress: 0,
            completedCheckpointsList: [],
            stared: false,
        };

        libraryArticles.update((items) => [newArticle, ...items]);

        parsingQueue.update((q) => [...q, id]);
        processQueue();
        notifications.success("Article added to library!");
        closeArticle();
    }

    onMount(() => {
        loadSources();
    });
</script>

<div class="flex flex-col h-full bg-white relative dark:bg-zinc-950 overflow-hidden">
    <div
        class="flex flex-col border-b border-zinc-100 dark:border-zinc-800 z-40 bg-white/90 dark:bg-zinc-950/90 backdrop-blur"
    >
        <div class="flex justify-between items-center p-4 pb-2">
            <button
                on:click={goBack}
                class="p-2 -ml-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full text-zinc-600 transition duration-100 ease-out dark:text-zinc-300 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <ArrowLeft size={24} />
            </button>
            <div class="flex items-center space-x-2">
                <button
                    on:click={handleReset}
                    class="flex items-center justify-center p-2 rounded-lg transition bg-zinc-100 text-zinc-500 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-700"
                    title="Reset Feed"
                >
                    <RotateCcw
                        size={18}
                        class="transition-transform duration-500 {isResetting
                            ? '-rotate-180'
                            : ''}"
                    />
                </button>
                <button
                    on:click={toggleFamiliarity}
                    class="flex items-center justify-center p-2 rounded-lg transition {markFamiliarity
                        ? 'bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900'
                        : 'bg-zinc-100 text-zinc-500 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-700'}"
                    title="Toggle Familiarity Highlighting"
                >
                    <Brain size={18} />
                </button>
                <div class="relative w-[110px]">
                    <button
                        on:click={() => (showLangSelector = !showLangSelector)}
                        class="flex items-center justify-between w-full px-3 py-1.5 bg-zinc-100 rounded-lg text-sm font-medium hover:bg-zinc-200 transition text-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
                    >
                        <div class="flex items-center space-x-2">
                            <Flag code={currentLang.toUpperCase()} size={20} />
                            <span>{currentLang === "kr" ? "KR" : "RU"}</span>
                        </div>
                        <ChevronDown
                            size={14}
                            class="transition-transform {showLangSelector
                                ? 'rotate-180'
                                : ''}"
                        />
                    </button>
                    {#if showLangSelector}
                        <div
                            transition:slide={{ duration: 200 }}
                            class="absolute top-full right-0 mt-2 w-40 bg-white border border-zinc-200 rounded-xl shadow-xl overflow-hidden dark:bg-zinc-900 dark:border-zinc-700 z-50"
                        >
                            <button
                                class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                on:click={() => {
                                    currentLang = "kr";
                                    showLangSelector = false;
                                    loadSources();
                                }}
                            >
                                <Flag code="KR" size={18} /><span>Korean</span>
                            </button>
                            <button
                                class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                on:click={() => {
                                    currentLang = "ru";
                                    showLangSelector = false;
                                    loadSources();
                                }}
                            >
                                <Flag code="RU" size={18} /><span>Russian</span>
                            </button>
                        </div>
                    {/if}
                </div>
            </div>
        </div>

        {#if sources.length > 0}
            <div
                class="flex overflow-x-auto whitespace-nowrap scrollbar-hide px-4 py-2 gap-2 pb-3"
            >
                {#each sources as source}
                    <button
                        on:click={() => toggleSource(source.id)}
                        class="px-3 py-1 text-xs rounded-full border transition-colors {selectedSourceIds.has(
                            source.id
                        )
                            ? 'bg-zinc-800 border-zinc-800 text-white dark:bg-zinc-200 dark:border-zinc-200 dark:text-zinc-900'
                            : 'bg-transparent border-zinc-200 text-zinc-500 hover:border-zinc-400 dark:border-zinc-700 dark:text-zinc-400 dark:hover:border-zinc-500'}"
                    >
                        {source.name}
                    </button>
                {/each}
            </div>
        {/if}
    </div>

    <div
        bind:this={feedContainer}
        on:scroll={handleScroll}
        on:wheel={handleWheel}
        on:touchstart={handleTouchStart}
        on:touchend={handleTouchEnd}
        class="flex-1 overflow-y-auto p-4 scroll-smooth"
    >
        <div
            class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 auto-rows-[140px] grid-flow-dense"
        >
            {#each articles as article, i (article.url)}
                <button
                    type="button"
                    on:click={() => (selectedArticle = article)}
                    class="group relative overflow-hidden rounded-2xl bg-zinc-100 dark:bg-zinc-900 transition-all duration-300 hover:scale-[1.02] hover:shadow-lg cursor-pointer border ring-1 ring-inset {getDifficultyRingClass(
                        article.difficulty
                    )} {layoutClasses[i]} text-left"
                >
                    <div
                        class="absolute top-2 right-2 flex gap-1.5 z-10 opacity-90 group-hover:opacity-100 transition-opacity"
                    >
                        {#if article?.recommendation != null}
                            <div
                                class="px-2 py-0.5 bg-black/60 backdrop-blur text-amber-300 text-[10px] font-bold rounded-md tracking-wide"
                            >
                                ★ {article.recommendation.toFixed(2)}
                            </div>
                        {/if}
                        {#if article?.difficulty != null}
                            <div
                                class="px-2 py-0.5 bg-black/60 backdrop-blur text-white/90 text-[10px] font-medium rounded-md tracking-wide"
                            >
                                Diff {article.difficulty.toFixed(2)}
                            </div>
                        {/if}
                    </div>
                    {#if article.cover_image}
                        <img
                            src={article.cover_image}
                            alt={article.title}
                            loading="lazy"
                            decoding="async"
                            class="absolute inset-0 w-full h-full object-cover opacity-60 group-hover:opacity-40 transition-opacity"
                        />
                    {:else}
                        <div
                            class="absolute inset-0 flex items-center justify-center opacity-10 dark:opacity-5"
                        >
                            <ImageIcon size={64} />
                        </div>
                    {/if}
                    <div
                        class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/30 to-transparent p-4 flex flex-col justify-end"
                    >
                        <span
                            class="text-[10px] font-bold text-white/70 uppercase tracking-wider mb-1 line-clamp-1"
                        >
                            {article.source_name}
                        </span>
                        <h3
                            class="text-white font-semibold text-sm md:text-base leading-tight line-clamp-3 drop-shadow-md"
                        >
                            {article.title}
                        </h3>
                    </div>
                </button>
            {/each}

            {#if isLoading}
                <div class="col-span-full py-8 flex justify-center items-center">
                    <div class="animate-pulse flex space-x-2">
                        <div class="w-3 h-3 bg-zinc-400 rounded-full"></div>
                        <div class="w-3 h-3 bg-zinc-400 rounded-full"></div>
                        <div class="w-3 h-3 bg-zinc-400 rounded-full"></div>
                    </div>
                </div>
            {/if}

            {#if articles.length === 0 && !isLoading}
                <div
                    class="col-span-full flex flex-col items-center justify-center py-24 text-zinc-400 dark:text-zinc-600"
                >
                    <Brain size={48} class="mb-4 opacity-30" />
                    <p class="text-sm mb-6 font-medium">Ready</p>
                    <button
                        on:click={initialFetch}
                        class="px-6 py-2.5 bg-zinc-900 text-white rounded-full text-sm font-medium hover:bg-black transition active:scale-95 shadow-lg dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
                    >
                        Load Articles
                    </button>
                </div>
            {/if}
        </div>
    </div>
</div>

{#if selectedArticle}
    <div
        transition:slide={{ duration: 300 }}
        class="absolute inset-0 z-50 flex flex-col bg-white dark:bg-zinc-950"
    >
        <div
            class="flex justify-between items-center p-4 border-b border-zinc-100 dark:border-zinc-800 bg-white/90 backdrop-blur dark:bg-zinc-950/90 sticky top-0 z-10"
        >
            <button
                on:click={closeArticle}
                class="p-2 -ml-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full text-zinc-600 transition duration-100 ease-out dark:text-zinc-300 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <ArrowLeft size={24} />
            </button>
            <div class="flex items-center gap-2">
                <span
                    class="flex items-center gap-1.5 text-xs font-mono font-medium text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-500/10 border border-amber-200 dark:border-amber-500/20 px-2.5 py-1 rounded-md"
                >
                    ★
                    {selectedArticle.recommendation != null
                        ? selectedArticle.recommendation.toFixed(2)
                        : "N/A"}
                </span>
                <span
                    class="text-xs font-mono font-medium text-zinc-600 dark:text-zinc-300 border border-zinc-200 dark:border-zinc-700 bg-zinc-50 dark:bg-zinc-800 px-2.5 py-1 rounded-md"
                >
                    Diff:
                    {selectedArticle.difficulty != null
                        ? selectedArticle.difficulty.toFixed(2)
                        : "N/A"}
                </span>
            </div>
        </div>

        <div class="flex-1 overflow-y-auto p-6 md:px-12 pb-32 scroll-smooth">
            {#if selectedArticle.cover_image}
                <div
                    class="w-full mb-6 bg-zinc-50 dark:bg-zinc-900/50 rounded-2xl flex items-center justify-center overflow-hidden shadow-sm ring-1 ring-zinc-100 dark:ring-zinc-800"
                >
                    <img
                        src={selectedArticle.cover_image}
                        alt={selectedArticle.title}
                        class="w-full h-auto max-h-[40vh] object-contain"
                    />
                </div>
            {/if}

            <h1
                class="text-2xl md:text-3xl font-bold text-zinc-900 dark:text-zinc-50 mb-4 leading-snug"
            >
                {selectedArticle.title}
            </h1>
            <div
                class="flex items-center space-x-2 text-sm text-zinc-500 dark:text-zinc-400 mb-8"
            >
                <span class="font-medium text-zinc-700 dark:text-zinc-300"
                    >{selectedArticle.source_name}</span
                >
                <span>•</span>
                <a
                    href={selectedArticle.url}
                    target="_blank"
                    rel="noreferrer"
                    class="underline hover:text-zinc-800 dark:hover:text-zinc-200 transition-colors"
                >
                    Original Link
                </a>
            </div>

            <div
                class="text-lg text-zinc-800 dark:text-zinc-100 leading-relaxed font-serif flex flex-wrap gap-x-1"
            >
                {#if selectedArticle.words && selectedArticle.words.length > 0}
                    {#each selectedArticle.words as [word, fam]}
                        <span class={getFamiliarityColor(fam)}>{word}</span>
                    {/each}
                {:else}
                    {selectedArticle.content}
                {/if}
            </div>
        </div>

        <div
            class="absolute bottom-0 left-0 right-0 p-4 border-t border-zinc-100 bg-white/90 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/90 flex justify-end gap-3 shadow-[0_-10px_30px_rgba(0,0,0,0.05)]"
        >
            <button
                on:click={addToDraft}
                class="flex items-center gap-2 px-5 py-3 rounded-full bg-zinc-100 text-zinc-800 font-medium hover:bg-zinc-200 transition active:scale-95 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
            >
                <FileEdit size={18} />
                <span>Draft</span>
            </button>
            <button
                on:click={addToLibrary}
                class="flex items-center gap-2 px-5 py-3 rounded-full bg-zinc-900 text-white font-medium hover:bg-black transition shadow-lg active:scale-95 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
            >
                <BookmarkPlus size={18} />
                <span>Library</span>
            </button>
        </div>
    </div>
{/if}

<style>
    .scrollbar-hide::-webkit-scrollbar {
        display: none;
    }
    .scrollbar-hide {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
