<script lang="ts">
    import {
        articles,
        activeArticleId,
        isSidebarOpen,
        settings,
    } from "../lib/stores";
    import { Menu, BookOpen, Type } from "lucide-svelte";
    import { fade, fly } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import type { Sentence, Block } from "../lib/types";
    import { convertFileSrc } from "@tauri-apps/api/core";

    $: article = $articles.find((a) => a.id === $activeArticleId) as any;

    let activeBlock: Block | null = null;
    let activeBlockEl: HTMLElement | null = null;
    let activeSentence: Sentence | null = null;
    let player: HTMLAudioElement | null = null;

    let viewMode: "word" | "sentence" = "word";

    let popoverPosition = { top: 0, left: 0, align: "bottom", arrowLeft: 0 };
    let isGrammarExpanded = false;

    const colorMap: Record<string, string> = {
        noun: "bg-blue-50 text-blue-700 active:bg-blue-100 dark:bg-blue-950/40 dark:text-blue-200 dark:active:bg-blue-900/55",
        pronoun:
            "bg-indigo-50 text-indigo-700 active:bg-indigo-100 dark:bg-indigo-950/40 dark:text-indigo-200 dark:active:bg-indigo-900/55",
        verb: "bg-red-50 text-red-700 active:bg-red-100 dark:bg-red-950/40 dark:text-red-200 dark:active:bg-red-900/55",
        adjective:
            "bg-rose-50 text-rose-700 active:bg-rose-100 dark:bg-rose-950/40 dark:text-rose-200 dark:active:bg-rose-900/55",
        adverb: "bg-emerald-50 text-emerald-700 active:bg-emerald-100 dark:bg-emerald-950/40 dark:text-emerald-200 dark:active:bg-emerald-900/55",
        determiner:
            "bg-sky-50 text-sky-700 active:bg-sky-100 dark:bg-sky-950/40 dark:text-sky-200 dark:active:bg-sky-900/55",
        number: "bg-violet-50 text-violet-700 active:bg-violet-100 dark:bg-violet-950/40 dark:text-violet-200 dark:active:bg-violet-900/55",
        particle:
            "bg-zinc-100 text-zinc-600 active:bg-zinc-200 dark:bg-zinc-800/60 dark:text-zinc-200 dark:active:bg-zinc-700/70",
        ending: "bg-gray-100 text-gray-600 active:bg-gray-200 dark:bg-gray-800/60 dark:text-gray-200 dark:active:bg-gray-700/70",
        suffix: "bg-slate-100 text-slate-600 active:bg-slate-200 dark:bg-slate-800/60 dark:text-slate-200 dark:active:bg-slate-700/70",
        interjection:
            "bg-amber-50 text-amber-700 active:bg-amber-100 dark:bg-amber-950/35 dark:text-amber-200 dark:active:bg-amber-900/55",
        punctuation:
            "bg-transparent text-zinc-400 cursor-default dark:text-zinc-500",
        unknown:
            "bg-slate-50 text-slate-500 active:bg-slate-100 dark:bg-slate-800/50 dark:text-slate-200 dark:active:bg-slate-700/65",
    };

    function stopAudio() {
        console.log("1");
        if (player) {
            player.pause();
            player.currentTime = 0;
            player = null;
        }
    }

    function playAudio(localPath?: string | null) {
        stopAudio();
        if (!localPath) return;

        const normalized = localPath.replace(/\\/g, "/"); // Windows path normalize
        const src = convertFileSrc(normalized);

        player = new Audio(src);

        player.onended = () => {
            player = null;
        };

        player.onerror = (e) => {
            console.error("audio error:", e);
            player = null;
        };

        player.play().catch((e) => {
            console.error("audio play failed:", e, { localPath, src });
            player = null;
        });
    }

    function getLanguageName(code: string | undefined): string {
        if (code === "KR") return "Korean";
        if (code === "RU") return "Russian";
        return "Unknown";
    }

    function handleBlockClick(
        event: MouseEvent,
        block: Block,
        sentence: Sentence,
    ) {
        event.stopPropagation();

        if (viewMode === "word") {
            activeSentence = null;
            if (activeBlock === block) {
                closePopover();
                return;
            }
            activeBlock = block;
            activeBlockEl = event.currentTarget as HTMLElement;
            isGrammarExpanded = false;
            calculatePosition();
            if ($settings.autoSpeak) {
                playAudio(block.audio_path);
            }
        } else {
            closePopover();
            activeSentence = sentence;
            if ($settings.autoSpeak) {
                playAudio(sentence.audio_path);
            }
        }
    }

    function calculatePosition() {
        if (!activeBlockEl) return;
        const rect = activeBlockEl.getBoundingClientRect();
        const screenW = window.innerWidth;
        const screenH = window.innerHeight;
        const popoverWidth = 300;
        const arrowSize = 8;
        let popoverLeft = rect.left;
        if (popoverLeft + popoverWidth > screenW - 20)
            popoverLeft = screenW - popoverWidth - 20;
        if (popoverLeft < 10) popoverLeft = 10;
        const blockCenter = rect.left + rect.width / 2;
        let arrowLeft = blockCenter - popoverLeft - arrowSize;
        arrowLeft = Math.max(8, Math.min(popoverWidth - 24, arrowLeft));
        const spaceBelow = screenH - rect.bottom;
        const showOnTop = spaceBelow < 250;
        popoverPosition = {
            left: popoverLeft,
            top: showOnTop ? rect.top - 10 : rect.bottom + 10,
            align: showOnTop ? "top" : "bottom",
            arrowLeft: arrowLeft,
        };
    }

    function closePopover() {
        activeBlock = null;
        activeBlockEl = null;
    }

    function handleGlobalClick(e: MouseEvent) {
        const target = e.target as HTMLElement;
        if (
            !target.closest(".interactive-block") &&
            !target.closest(".reader-popover") &&
            !target.closest(".sentence-panel")
        ) {
            closePopover();
            if (viewMode === "sentence") {
                activeSentence = null;
            }
            stopAudio();
        }
    }
</script>

<svelte:window on:click={handleGlobalClick} />

<div class="flex flex-col h-full bg-white relative dark:bg-zinc-950">
    <div
        class="h-14 border-b border-zinc-100 flex items-center justify-between px-4 sticky top-0 bg-white/95 backdrop-blur z-30 dark:border-zinc-800 dark:bg-zinc-950/95"
    >
        <div class="flex items-center space-x-2">
            <button
                class="md:hidden p-2 -ml-2 rounded-full hover:bg-zinc-100 text-zinc-600 transition-colors dark:hover:bg-zinc-800 dark:text-zinc-300"
                on:click={() => isSidebarOpen.update((v) => !v)}
            >
                <Menu size={24} />
            </button>

            <div
                class="relative inline-grid grid-cols-2 bg-zinc-100 rounded-lg p-0.5 dark:bg-zinc-800/50"
            >
                <!-- sliding indicator -->
                <div
                    class="absolute top-0.5 bottom-0.5 left-0.5 w-1/2 bg-white rounded-md shadow-sm
                           transition-transform duration-200 ease-out dark:bg-zinc-700"
                    style="transform: translateX({viewMode === 'sentence'
                        ? '100%'
                        : '0%'});"
                ></div>

                <!-- Word -->
                <button
                    class="relative z-10 w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
           active:scale-95 transition-all
           {viewMode === 'word'
                        ? 'text-zinc-800 dark:text-zinc-100'
                        : 'text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200'}"
                    on:click={() => {
                        viewMode = "word";
                        activeSentence = null;
                    }}
                >
                    <Type size={14} />
                    <span>Word</span>
                </button>

                <!-- Sentence -->
                <button
                    class="relative z-10 w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
                           active:scale-95 transition-all
                           {viewMode === 'sentence'
                           ? 'text-zinc-800 dark:text-zinc-100'
                           : 'text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200'}"
                    on:click={() => {
                        viewMode = "sentence";
                        activeBlock = null;
                    }}
                >
                    <BookOpen size={14} />
                    <span>Sentence</span>
                </button>
            </div>
        </div>

        <div class="flex items-center space-x-2">
            <span class="text-sm font-medium text-zinc-500 dark:text-zinc-400">
                {getLanguageName(article?.language)}
            </span>
            <Flag code={article?.language} size={20} />
        </div>
    </div>

    <div
        class="flex-1 overflow-y-auto p-6 md:p-10 leading-loose text-lg md:text-xl font-medium text-zinc-800 dark:text-zinc-200 pb-32"
    >
        {#if article && article.sentences}
            {#each article.sentences as sentence}
                <div
                    class="mb-4 flex flex-wrap gap-y-2 items-end rounded-lg transition-colors duration-200
                    {activeSentence === sentence && viewMode === 'sentence'
                        ? 'bg-zinc-100/80 -mx-2 px-2 dark:bg-zinc-800/50'
                        : ''}"
                >
                    {#each sentence.blocks as block}
                        <button
                            class="interactive-block px-1 py-0 mx-[2px] rounded
                            transition-transform duration-75 ease-out
                            active:scale-95
                            {colorMap[block.pos] || colorMap['unknown']}"
                            on:click={(e) =>
                                handleBlockClick(e, block, sentence)}
                        >
                            {block.text}
                        </button>
                    {/each}
                </div>
            {/each}
        {:else}
            <div class="text-zinc-400 text-center mt-20 dark:text-zinc-500">
                No content loaded.
            </div>
        {/if}
    </div>

    {#if activeBlock && viewMode === "word"}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="reader-popover fixed z-50 w-[300px] bg-zinc-900/95 backdrop-blur text-white rounded-xl shadow-2xl p-4"
            style="left: {popoverPosition.left}px; top: {popoverPosition.top}px; transform: translateY({popoverPosition.align ===
            'top'
                ? '-100%'
                : '0'});"
            transition:fade={{ duration: 150 }}
            on:click|stopPropagation
        >
            <div class="flex flex-col space-y-2">
                {#if activeBlock.chinese_root}
                    <div
                        class="inline-block self-start bg-yellow-500/20 text-yellow-300 text-xs px-1.5 py-0.5 rounded border border-yellow-500/50 mb-1"
                    >
                        [{activeBlock.chinese_root}]
                    </div>
                {/if}

                <div class="text-lg font-bold text-white">
                    {activeBlock.definition}
                </div>

                {#if activeBlock.grammar_note}
                    <div
                        class="text-zinc-400 text-sm border-t border-zinc-700 pt-2 mt-1"
                    >
                        {#if isGrammarExpanded || activeBlock.grammar_note.length < 50}
                            {activeBlock.grammar_note}
                        {:else}
                            {activeBlock.grammar_note.slice(0, 50)}...
                            <button
                                class="text-blue-400 ml-1 hover:underline"
                                on:click|stopPropagation={() =>
                                    (isGrammarExpanded = true)}>more</button
                            >
                        {/if}
                    </div>
                {/if}
            </div>
            <div
                class="absolute w-4 h-4 bg-zinc-900/95 rotate-45"
                style="left: {popoverPosition.arrowLeft}px; {popoverPosition.align ===
                'top'
                    ? 'bottom: -6px;'
                    : 'top: -6px;'}"
            ></div>
        </div>
    {/if}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    {#if activeSentence && viewMode === "sentence"}
        <div
            class="sentence-panel absolute bottom-0 left-0 right-0 z-50
            bg-white border-t border-zinc-200 shadow-[0_-4px_20px_-4px_rgba(0,0,0,0.1)]
            rounded-t-2xl px-6 md:px-10 pt-3 pb-4
            dark:bg-zinc-900 dark:border-zinc-800
            flex flex-col"
            style="max-height: 30vh;"
            transition:fly={{ y: 50, duration: 250 }}
            on:click|stopPropagation
        >
            <div class="space-y-2 min-h-0 flex-1 flex flex-col">
                <div
                    class="flex items-center space-x-2 text-xs font-bold uppercase tracking-wider text-zinc-400 dark:text-zinc-500"
                >
                    <BookOpen size={14} />
                    <span>Sentence Translation</span>
                </div>
                <div
                    class="border-t border-zinc-100 dark:border-zinc-800"
                ></div>
                <div
                    class="flex-1 min-h-0 overflow-y-auto text-lg md:text-xl text-zinc-800 leading-relaxed dark:text-zinc-100"
                >
                    <span class="italic text-zinc-400 dark:text-zinc-500">
                        {#if activeSentence.translation}
                            {activeSentence.translation}
                        {:else}
                            Translation not available.
                        {/if}
                    </span>
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    :global(.dark) ::-webkit-scrollbar {
        width: 4px;
        height: 4px;
    }
    :global(.dark) ::-webkit-scrollbar-track {
        background: transparent;
    }
    :global(.dark) ::-webkit-scrollbar-thumb {
        background: rgba(255, 255, 255, 0.2);
        border-radius: 2px;
    }

    @media (prefers-color-scheme: dark) {
        :global(::-webkit-scrollbar) {
            width: 4px;
            height: 4px;
        }
        :global(::-webkit-scrollbar-track) {
            background: transparent;
        }
        :global(::-webkit-scrollbar-thumb) {
            background: rgba(255, 255, 255, 0.2);
            border-radius: 2px;
        }
    }

    :global(.dark) * {
        scrollbar-width: thin;
        scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
    }
    @media (prefers-color-scheme: dark) {
        :global(*) {
            scrollbar-width: thin;
            scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
        }
    }
</style>
