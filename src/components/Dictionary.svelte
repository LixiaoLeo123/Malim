<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { ArrowLeft, BookOpenText, Search, LoaderCircle, CircleAlert } from "lucide-svelte";
    import { currentView, dictionaryHistory } from "../lib/stores";
    import { notifications } from "$lib/notificationStore";
    import type { DictionaryHistoryEntry, DictionarySearchResponse } from "$lib/types";

    let query = "";
    let isSearching = false;
    let errorMessage = "";
    let response: DictionarySearchResponse | null = null;
    let isHistoryOpen = false;
    let historyPanelEl: HTMLDivElement | null = null;
    let searchInputEl: HTMLInputElement | null = null;

    const HISTORY_LIMIT = 128;

    function recordHistoryEntry(entry: DictionaryHistoryEntry) {
        dictionaryHistory.update((items) => {
            const deduped = items.filter((item) => item.normalizedQuery !== entry.normalizedQuery);
            return [entry, ...deduped].slice(0, HISTORY_LIMIT);
        });
    }

    async function searchDictionary() {
        const cleaned = query.trim();
        if (!cleaned) {
            errorMessage = "Please enter a Russian word.";
            response = null;
            return;
        }

        searchInputEl?.blur();
        isSearching = true;
        errorMessage = "";

        try {
            const result = await invoke<DictionarySearchResponse>("search_russian_dictionary", {
                query: cleaned,
            });
            // Clean up unwanted style tags from the raw HTML to prevent 404s
            if (result.results) {
                result.results = result.results.map(entry => ({
                    ...entry,
                    definition_html: entry.definition_html.replace(/<link[^>]*href="[^"]*style\.css"[^>]*>/g, '')
                }));
            }
            response = result;
            console.log("[Dictionary] Search result:", result);

            recordHistoryEntry({
                query: cleaned,
                normalizedQuery: result.normalized_query || cleaned.toLowerCase(),
                resultCount: result.results.length,
                searchedAt: Date.now(),
            });

            if (result.results.length === 0) {
                notifications.info("No matches found.");
            } else {
                notifications.success(`Found ${result.results.length} matches.`);
            }
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            errorMessage = `Search failed: ${message}`;
            response = null;
            notifications.error(errorMessage);
        } finally {
            isSearching = false;
        }
    }

    function openHome() {
        currentView.set("home");
    }

    function runHistorySearch(entry: DictionaryHistoryEntry) {
        query = entry.query;
        isHistoryOpen = false;
        searchInputEl?.blur();
        searchDictionary();
    }

    function openHistoryPanel() {
        isHistoryOpen = true;
    }

    function closeHistoryPanelSoon() {
        setTimeout(() => {
            if (!historyPanelEl?.contains(document.activeElement)) {
                isHistoryOpen = false;
            }
        }, 0);
    }

    function onKeydown(event: KeyboardEvent) {
        if (event.key === "Enter") {
            event.preventDefault();
            searchDictionary();
        }
    }
</script>

<svelte:head>
    <title>Dictionary</title>
</svelte:head>

<div class="flex flex-col h-full bg-zinc-50 relative dark:bg-zinc-950 overflow-hidden text-zinc-900 dark:text-zinc-100">
    <div class="flex flex-col border-b border-zinc-200 dark:border-zinc-800 z-40 bg-white/90 dark:bg-zinc-950/90 backdrop-blur">
        <div class="flex justify-between items-center p-4 pb-3">
            <button
                on:click={openHome}
                class="p-2 -ml-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full text-zinc-600 transition duration-100 ease-out dark:text-zinc-300 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <ArrowLeft size={24} />
            </button>
            <div class="flex-1 px-3 sm:px-6">
                <div class="relative mx-auto w-full max-w-3xl">
                    <form
                        class="relative flex w-full"
                        on:submit|preventDefault={searchDictionary}
                        on:focusin={openHistoryPanel}
                        on:focusout={closeHistoryPanelSoon}
                    >
                        <Search size={18} class="absolute left-4 top-1/2 -translate-y-1/2 text-zinc-400" />
                        <input
                            bind:this={searchInputEl}
                            bind:value={query}
                            on:keydown={onKeydown}
                            on:click={openHistoryPanel}
                            lang="ru"
                            autocomplete="off"
                            spellcheck="false"
                            inputmode="text"
                            placeholder="Search OpenRussian dictionary..."
                            class="w-full rounded-2xl border border-zinc-200 bg-zinc-100/50 px-11 py-2.5 text-[0.95rem] font-medium text-zinc-900 outline-none transition-all placeholder:text-zinc-400 focus:border-[#660874] focus:bg-white focus:ring-4 focus:ring-[#660874]/10 dark:border-zinc-800 dark:bg-zinc-900/50 dark:text-white dark:placeholder:text-zinc-500 dark:focus:border-[#9a2eb0] dark:focus:bg-zinc-900"
                        />
                        <button
                            type="submit"
                            disabled={isSearching}
                            class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded-xl text-zinc-400 hover:text-[#660874] hover:bg-[#660874]/5 dark:hover:text-[#9a2eb0] dark:hover:bg-[#9a2eb0]/10 transition-colors disabled:opacity-50"
                        >
                            {#if isSearching}
                                <LoaderCircle size={18} class="animate-spin" />
                            {:else}
                                <Search size={18} />
                            {/if}
                        </button>
                    </form>

                    {#if isHistoryOpen}
                        <div
                            bind:this={historyPanelEl}
                            class="absolute left-0 right-0 top-full z-50 mt-1.5 overflow-hidden rounded-2xl border border-zinc-200 bg-white shadow-lg shadow-zinc-950/10 dark:border-zinc-800 dark:bg-zinc-950"
                        >
                            {#if $dictionaryHistory.length > 0}
                                <div class="max-h-64 overflow-y-auto p-1.5">
                                    <div class="grid gap-1">
                                        {#each $dictionaryHistory as entry (entry.normalizedQuery + entry.searchedAt)}
                                            <button
                                                type="button"
                                                on:mousedown|preventDefault
                                                on:click={() => runHistorySearch(entry)}
                                                class="group flex w-full items-center justify-between gap-2 rounded-xl border border-transparent px-3 py-2 text-left transition-colors hover:border-[#660874]/20 hover:bg-[#660874]/5 dark:hover:border-[#9a2eb0]/20 dark:hover:bg-[#9a2eb0]/10"
                                            >
                                                <div class="min-w-0 truncate text-[0.92rem] font-semibold text-zinc-900 dark:text-zinc-100">{entry.query}</div>
                                                <span class="shrink-0 rounded-full border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[0.7rem] font-medium text-zinc-500 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-400">
                                                    {entry.resultCount} matches
                                                </span>
                                            </button>
                                        {/each}
                                    </div>
                                </div>
                            {:else}
                                <div class="px-3 py-5 text-center text-sm text-zinc-500 dark:text-zinc-400">
                                    No search history yet.
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
            </div>
            <div class="flex items-center space-x-2 text-[#660874] dark:text-[#9a2eb0] font-medium text-sm hidden sm:flex">
                 <BookOpenText size={22} />
            </div>
        </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 sm:p-6 lg:p-8 scroll-smooth">
        <div class="mx-auto max-w-4xl">
            {#if errorMessage}
                <div class="mb-6 inline-flex items-start gap-2 rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-500/20 dark:bg-red-500/10 dark:text-red-200">
                    <CircleAlert size={18} class="mt-0.5 shrink-0" />
                    <span>{errorMessage}</span>
                </div>
            {/if}

            {#if response && response.results.length > 0}
                <div class="mb-4 flex flex-wrap items-center gap-2 text-xs font-semibold text-zinc-500 dark:text-zinc-400">
                    <span class="rounded-full border border-zinc-200 bg-white px-3 py-1 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">Query: {response.query}</span>
                    <span class="rounded-full border border-zinc-200 bg-white px-3 py-1 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">Matches: {response.results.length}</span>
                </div>

                <section class="space-y-8 pb-12">
                    {#each response.results as entry}
                        <article class="overflow-hidden rounded-3xl border border-zinc-200 bg-white shadow-sm dark:border-zinc-800 dark:bg-zinc-950/80">
                            <div class="px-5 py-6 sm:px-8 sm:py-8">
                                <div class="definition-html max-w-none">
                                    {@html entry.definition_html}
                                </div>
                            </div>
                        </article>
                    {/each}
                </section>
            {:else if response && response.results.length === 0}
                <div class="mt-12 flex flex-col items-center justify-center text-center">
                    <div class="flex h-20 w-20 items-center justify-center rounded-full bg-zinc-100 text-zinc-400 dark:bg-zinc-900 dark:text-zinc-600 mb-6">
                        <Search size={32} />
                    </div>
                    <h3 class="text-lg font-bold text-zinc-900 dark:text-zinc-100 mb-2">No matches found</h3>
                    <p class="max-w-sm text-sm text-zinc-500 dark:text-zinc-400">Try searching for the lemma, an inflected form, or the alternate ё / е spelling.</p>
                </div>
            {:else if !isSearching}
                <div class="mt-16 flex flex-col items-center justify-center text-center opacity-60">
                    <BookOpenText size={48} class="text-zinc-300 dark:text-zinc-700 mb-6" />
                    <p class="text-sm font-medium text-zinc-500 dark:text-zinc-400">Enter a Russian word to dive in.</p>
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
    .definition-html {
        --th-purple: #660874;
        --th-purple-muted: rgba(102, 8, 116, 0.08);
        --th-purple-light: #9a2eb0;
        --th-purple-dark: #4d0658;
        --th-border: rgba(102, 8, 116, 0.15);
        color: #374151;
        font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
        line-height: 1.6;
    }

    @media (prefers-color-scheme: dark) {
        .definition-html {
            --th-purple: #8857e1;
            --th-purple-muted: rgba(226, 160, 248, 0.15);
            --th-border: rgba(226, 160, 248, 0.25);
            color: #d1d5db;
        }
    }

    .definition-html :global(.page.word .section.basics h1) {
        font-size: 2rem;
        font-weight: 900;
        line-height: 1.1;
        letter-spacing: -0.03em;
        color: var(--th-purple);
        margin: 0 0 1.25rem 0;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding-bottom: 0.75rem;
        border-bottom: 2px solid var(--th-border);
    }
    @media (max-width: 640px) {
        .definition-html :global(.page.word .section.basics h1) {
            font-size: 1.75rem;
        }
    }

    .definition-html :global(.page.word .overview) {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
        margin-bottom: 1.5rem;
    }
    .definition-html :global(.page.word .overview p) {
        margin: 0;
        padding: 0.25rem 0.75rem;
        background: var(--th-purple-muted) !important;
        color: var(--th-purple) !important;
        border-radius: 9999px;
        font-size: 0.75rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }

    .definition-html :global(.page.word .overview p:empty) {
        display: none !important;
    }


    .definition-html :global(.page.word .section) {
        margin-top: 1.75rem;
        margin-bottom: 1.75rem;
    }
    .definition-html :global(.page.word .section > h2) {
        font-size: 0.95rem;
        font-weight: 800;
        color: #9ca3af;
        text-transform: uppercase;
        letter-spacing: 0.15em;
        margin-top: 0;
        margin-bottom: 1rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }
    .definition-html :global(.page.word .section > h2::before) {
        content: "";
        display: block;
        width: 1rem;
        height: 3px;
        background: var(--th-purple);
        border-radius: 2px;
    }


    .definition-html :global(.section.partner) {
        text-transform: lowercase;
    }
    .definition-html :global(.section.partner > h2) {
        text-transform: uppercase;
    }
    .definition-html :global(.section.partner a), .definition-html :global(.section.partner span) {
        text-transform: lowercase;
        color: var(--th-purple);
    }


    .definition-html :global(.section.translations ul) {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .definition-html :global(.section.translations li) {
        background: #fcfafb;
        border: 1px solid var(--th-border);
        border-radius: 0.75rem;
        padding: 0.75rem 1rem;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.translations li) {
            background: rgba(226, 160, 248, 0.05) !important;
        }
    }
    .definition-html :global(.section.translations .tl) {
        font-size: 1rem;
        font-weight: 600;
        color: #111827;
        margin: 0;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.translations .tl) {
            color: #f3f4f6 !important;
        }
    }


    .definition-html :global(.section.sentences ul.sentences),
    .definition-html :global(.section.expressions ul) {
        list-style: none;
        padding: 0;
        margin: 0;
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.5rem;
    }
    .definition-html :global(.section.sentences li.sentence),
    .definition-html :global(.section.expressions li) {
        padding: 0.75rem 1rem;
        background: #f9fafb;
        border-radius: 0.75rem;
        border: 1px solid rgba(0,0,0,0.03);
        transition: all 0.2s ease;
    }
    .definition-html :global(.section.sentences li.sentence:hover),
    .definition-html :global(.section.expressions li:hover) {
        background: #fff;
        border-color: rgba(0,0,0,0.05);
        box-shadow: 0 2px 4px -1px rgba(0,0,0,0.02);
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.sentences li.sentence),
        .definition-html :global(.section.expressions li) {
            background: rgba(255,255,255,0.02) !important;
            border-color: rgba(255,255,255,0.03) !important;
        }
        .definition-html :global(.section.sentences li.sentence:hover),
        .definition-html :global(.section.expressions li:hover) {
            background: rgba(255,255,255,0.04) !important;
            border-color: rgba(255,255,255,0.06) !important;
        }
    }
    .definition-html :global(.section.sentences .ru),
    .definition-html :global(.section.expressions .ru) {
        display: block;
        font-weight: 700;
        font-size: 1rem;
        color: #1f2937;
        margin-bottom: 0.25rem;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.sentences .ru),
        .definition-html :global(.section.expressions .ru) {
            color: #e5e7eb !important;
        }
    }
    .definition-html :global(.section.sentences .tl),
    .definition-html :global(.section.expressions .tl) {
        display: block;
        font-size: 0.9rem;
        color: #6b7280;
        font-style: italic;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.sentences .tl),
        .definition-html :global(.section.expressions .tl) {
            color: #9ca3af !important;
        }
    }

    .definition-html :global(.table-container) {
        width: 100%;
        overflow-x: auto;
        -webkit-overflow-scrolling: touch;
        border-radius: 0.75rem;
        border: 1px solid rgba(0,0,0,0.08);
        background: #fff;
        margin-bottom: 1.5rem;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.table-container) {
            border-color: rgba(255,255,255,0.1) !important;
            background: rgba(255,255,255,0.02) !important;
        }
    }
    .definition-html :global(table) {
        width: 100%;
        border-collapse: separate;
        border-spacing: 0;
        text-align: left;
        background: transparent !important;
    }
    .definition-html :global(th), .definition-html :global(td), .definition-html :global(tr) {
        background: transparent !important;
    }
    .definition-html :global(th), .definition-html :global(td) {
        padding: 0.5rem 0.75rem;
        border-bottom: 1px solid rgba(0,0,0,0.05);
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(th), .definition-html :global(td) {
            border-bottom-color: rgba(255,255,255,0.05) !important;
        }
    }
    .definition-html :global(th) {
        background-color: #fafafa !important;
        font-weight: 700;
        color: #4b5563 !important;
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        white-space: nowrap;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(th) {
            background-color: rgba(255,255,255,0.03) !important;
            color: #9ca3af !important;
        }
    }
    .definition-html :global(td) {
        font-size: 0.9rem;
        color: #111827 !important;
        font-weight: 500;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(td) {
            color: #e5e7eb !important;
        }
    }
    .definition-html :global(tbody tr:last-child th), .definition-html :global(tbody tr:last-child td) {
        border-bottom: none;
    }
    .definition-html :global(tbody tr:hover td), .definition-html :global(tbody tr:hover th) {
        background-color: var(--th-purple-muted) !important;
    }
    
    @media (prefers-color-scheme: dark) {
        .definition-html :global(td *) {
            color: #e5e7eb !important;
        }
    }

    .definition-html :global(.section.relateds2 ul), .definition-html :global(.section.synonyms ul) {
        list-style: none;
        padding: 0;
        margin: 0;
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 0.5rem;
    }
    .definition-html :global(.section.relateds2 li), .definition-html :global(.section.synonyms li) {
        display: flex;
        flex-direction: column;
        background: #f9fafb;
        padding: 0.75rem 1rem;
        border-radius: 0.75rem;
        font-weight: 700;
        color: #1f2937;
        font-size: 0.9rem;
        border: 1px solid rgba(0,0,0,0.03);
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.relateds2 li), .definition-html :global(.section.synonyms li) {
            background: rgba(255,255,255,0.02) !important;
            color: #e5e7eb !important;
            border-color: rgba(255,255,255,0.04) !important;
        }
    }
    .definition-html :global(.section.relateds2 li span), .definition-html :global(.section.synonyms li span) {
        font-weight: 400;
        color: #6b7280;
        font-size: 0.8rem;
        margin-top: 0.2rem;
    }
    @media (prefers-color-scheme: dark) {
        .definition-html :global(.section.relateds2 li span), .definition-html :global(.section.synonyms li span) {
            color: #9ca3af !important;
        }
    }
</style>
