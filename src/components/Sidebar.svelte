<script lang="ts">
    import { onMount } from "svelte";
    import { slide, fly, fade } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import StatsModal from "./StatsModal.svelte";
    import PromptGenerator from "./PromptGenerator.svelte";
    import { Globe, MessageCircle } from "lucide-svelte";
    import {
        articles,
        activeArticleId,
        currentView,
        isSidebarOpen,
        editorDraft,
        settings,
        parsingQueue,
    } from "../lib/stores";
    import type { Article } from "../lib/types";
    import {
        Plus,
        Trash2,
        Pencil,
        X,
        Sparkles,
        RefreshCw,
        BarChart2,
        Settings2,
        CheckCircle2,
        AlertCircle,
    } from "lucide-svelte";
    import ApiConfigModal from "./ApiConfigModel.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { processQueue } from "$lib/parser";

    let showApiConfig = false;
    let showStatsModal = false;
    let showPromptGenerator = false;

    type SyncStatus = "idle" | "syncing" | "success" | "error";
    let syncStatus: SyncStatus = "idle";

    $: showSyncButton =
        $settings.syncEnabled && $settings.syncServerUrl && $settings.userId;

    let contextMenuTarget: string | null = null;
    let deleteConfirmId: string | null = null;
    let pressTimer: number;

    function handleAdd() {
        currentView.set("editor");
        isSidebarOpen.set(false);
    }

    function handleDiscover() {
        currentView.set("discover");
        isSidebarOpen.set(false);
    }

    function handleChat() {
        currentView.set("chat");
        isSidebarOpen.set(false);
    }

    function openArticle(article: Article) {
        if (article.status === "parsing" || article.status === "error") return;
        activeArticleId.set(article.id);
        currentView.set("reader");
        isSidebarOpen.set(false);
    }

    function startPress(id: string) {
        pressTimer = window.setTimeout(() => {
            contextMenuTarget = id;
            deleteConfirmId = null;
        }, 400); // Snappier interaction
    }

    function endPress() {
        clearTimeout(pressTimer);
    }

    async function deleteArticle(id: string) {
        articles.update((items) => items.filter((i) => i.id !== id));
        contextMenuTarget = null;
        activeArticleId.update((currentId) => {
            if (currentId === id) {
                currentView.set("home");
                return null;
            }
            return currentId;
        });
        await invoke("delete_article_audio", { id });
    }

    function editArticle(article: Article) {
        editorDraft.set({
            title: article.title,
            content: article.draftContent ? article.draftContent : "",
            language: article.language,
        });
        activeArticleId.set(article.id);
        currentView.set("editor");
        contextMenuTarget = null;
        isSidebarOpen.set(false);
    }

    async function retryArticle(id: string) {
        articles.update((items) =>
            items.map((i) =>
                i.id === id
                    ? { ...i, status: "parsing" as const, parsingProgress: 0 }
                    : i,
            ),
        );
        parsingQueue.update((q) => [...q, id]);
        processQueue();
    }

    async function handleSync() {
        if (syncStatus === "syncing") return;

        syncStatus = "syncing";
        try {
            const res = await invoke<string>("sync_memory", {
                serverUrl: $settings.syncServerUrl,
                userId: $settings.userId,
            });
            console.log(res);
            syncStatus = "success";
        } catch (e) {
            console.error("Sync failed:", e);
            syncStatus = "error";
        } finally {
            setTimeout(() => {
                syncStatus = "idle";
            }, 2000);
        }
    }

    let listLimit = 20;
    $: visibleArticles = $articles.slice(0, listLimit);

    function handleListScroll(e: UIEvent) {
        const target = e.currentTarget as HTMLElement;
        if (
            target.scrollHeight - target.scrollTop - 100 <
            target.clientHeight
        ) {
            if (listLimit < $articles.length) {
                listLimit += 10;
            }
        }
    }
</script>

<div
    class="flex flex-col h-full bg-[#f9fafb] border-r border-zinc-200/60 w-full md:w-[320px] dark:bg-[#0f0f11] dark:border-zinc-800/60"
>
    <div
        class="px-5 py-4 flex justify-between items-center bg-transparent sticky top-0 z-20 pt-[calc(env(safe-area-inset-top)+1rem)]"
    >
        <div
            class="font-bold text-2xl tracking-tight text-zinc-900 dark:text-white flex items-center gap-2"
        >
            <div
                class="w-6 h-6 bg-zinc-900 dark:bg-white rounded-md flex items-center justify-center"
            >
                <span
                    class="text-white dark:text-zinc-900 text-[15px] font-black"
                    >말</span
                >
            </div>
            Malim
        </div>

        <div class="flex items-center gap-1">
            {#if showSyncButton}
                <button
                    on:click={handleSync}
                    disabled={syncStatus === "syncing"}
                    class="p-2 rounded-xl transition-all duration-200 ease-out focus:outline-none
                        {syncStatus === 'syncing'
                        ? 'cursor-wait animate-spin text-zinc-400'
                        : syncStatus === 'success'
                          ? 'text-emerald-500 bg-emerald-50 dark:bg-emerald-500/10'
                          : syncStatus === 'error'
                            ? 'text-red-500 bg-red-50 dark:bg-red-500/10'
                            : 'text-zinc-500 hover:bg-zinc-200/50 hover:text-zinc-900 active:scale-95 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-white'}"
                >
                    <RefreshCw size={18} strokeWidth={2.5} />
                </button>
            {/if}

            <button
                on:click={handleAdd}
                class="p-2 ml-1 text-white bg-zinc-900 hover:bg-zinc-800 active:scale-95 rounded-xl transition-all duration-200 shadow-sm dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-200"
                title="New Article"
            >
                <Plus size={18} strokeWidth={2.5} />
            </button>
        </div>
    </div>

    <div
        class="flex-1 overflow-y-auto no-scrollbar pb-6"
        on:scroll={handleListScroll}
    >
        <div class="px-4 py-2 grid grid-cols-2 gap-3 mb-4">
            <button
                on:click={() => (showPromptGenerator = true)}
                class="flex flex-col items-start p-3.5 rounded-2xl border border-zinc-200/50 bg-white shadow-sm transition-all duration-200 hover:shadow-md hover:border-indigo-200 active:scale-[0.98] dark:bg-zinc-900/50 dark:border-zinc-800/50 dark:hover:border-indigo-500/30 group"
            >
                <Sparkles
                    size={20}
                    class="text-indigo-500 dark:text-indigo-400 mb-2 group-hover:scale-110 transition-transform"
                />
                <span
                    class="font-semibold text-sm text-zinc-700 dark:text-zinc-200"
                    >AI Prompt</span
                >
            </button>

            <button
                on:click={() => (showStatsModal = true)}
                class="flex flex-col items-start p-3.5 rounded-2xl border border-zinc-200/50 bg-white shadow-sm transition-all duration-200 hover:shadow-md hover:border-orange-200 active:scale-[0.98] dark:bg-zinc-900/50 dark:border-zinc-800/50 dark:hover:border-orange-500/30 group"
            >
                <BarChart2
                    size={20}
                    class="text-orange-500 dark:text-orange-400 mb-2 group-hover:scale-110 transition-transform"
                />
                <span
                    class="font-semibold text-sm text-zinc-700 dark:text-zinc-200"
                    >Statistics</span
                >
            </button>
            <button
                on:click={handleDiscover}
                class="flex flex-col items-start p-3.5 rounded-2xl border border-zinc-200/50 bg-white shadow-sm transition-all duration-200 hover:shadow-md hover:border-blue-200 active:scale-[0.98] dark:bg-zinc-900/50 dark:border-zinc-800/50 dark:hover:border-blue-500/30 group"
            >
                <Globe
                    size={20}
                    class="text-blue-500 dark:text-blue-400 mb-2 group-hover:scale-110 transition-transform"
                />
                <span
                    class="font-semibold text-sm text-zinc-700 dark:text-zinc-200"
                    >Discover</span
                >
            </button>

            <button
                on:click={handleChat}
                class="flex flex-col items-start p-3.5 rounded-2xl border border-zinc-200/50 bg-white shadow-sm transition-all duration-200 hover:shadow-md hover:border-emerald-200 active:scale-[0.98] dark:bg-zinc-900/50 dark:border-zinc-800/50 dark:hover:border-emerald-500/30 group"
            >
                <MessageCircle
                    size={20}
                    class="text-emerald-500 dark:text-emerald-400 mb-2 group-hover:scale-110 transition-transform"
                />
                <span
                    class="font-semibold text-sm text-zinc-700 dark:text-zinc-200"
                    >AI Chat</span
                >
            </button>
        </div>

        <div class="px-2">
            <div
                class="px-3 text-[11px] font-bold tracking-widest text-zinc-400 dark:text-zinc-500 uppercase mb-2"
            >
                Your Library
            </div>

            <div class="space-y-[2px]">
                {#each visibleArticles as article (article.id)}
                    <div
                        class="relative group rounded-xl bg-transparent select-none touch-manipulation hover:bg-white dark:hover:bg-zinc-900 transition-all duration-150 ease-out active:scale-[0.99] active:bg-zinc-100 dark:active:bg-zinc-800"
                        class:opacity-60={article.status === "parsing"}
                        role="button"
                        tabindex="0"
                        on:click={() => {
                            if (contextMenuTarget) return;
                            openArticle(article);
                        }}
                        on:mousedown={() =>
                            article.status !== "parsing" &&
                            startPress(article.id)}
                        on:mouseup={endPress}
                        on:mouseleave={endPress}
                        on:touchstart={() =>
                            article.status !== "parsing" &&
                            startPress(article.id)}
                        on:touchend={endPress}
                        on:keydown={(e) =>
                            e.key === "Enter" && openArticle(article)}
                    >
                        <div class="py-3 px-3 flex flex-col gap-1.5">
                            <div class="flex justify-between items-start">
                                <h3
                                    class="font-medium text-zinc-900 truncate pr-3 text-[15px] dark:text-zinc-100"
                                >
                                    {article.title || "Untitled"}
                                </h3>
                                <div
                                    class="shrink-0 opacity-80 group-hover:opacity-100 transition-opacity"
                                >
                                    <Flag code={article.language} size={16} />
                                </div>
                            </div>

                            {#if article.status === "parsing"}
                                <div class="flex items-center gap-3">
                                    <div
                                        class="flex-1 h-1 bg-zinc-200/60 rounded-full overflow-hidden dark:bg-zinc-800"
                                    >
                                        <div
                                            class="h-full bg-indigo-500 transition-all duration-300"
                                            style="width: {article.parsingProgress}%"
                                        ></div>
                                    </div>
                                    <span
                                        class="text-[10px] font-bold text-indigo-500 uppercase tracking-wide animate-pulse"
                                        >Parsing</span
                                    >
                                </div>
                            {:else if article.status === "error"}
                                <div class="flex items-center justify-between">
                                    <div
                                        class="flex items-center gap-1.5 text-red-500"
                                    >
                                        <AlertCircle
                                            size={12}
                                            strokeWidth={2.5}
                                        />
                                        <span class="text-xs font-medium"
                                            >Parsing Failed</span
                                        >
                                    </div>
                                    <button
                                        class="text-[11px] font-bold text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-500/20 px-2 py-0.5 rounded transition-colors"
                                        on:click|stopPropagation={() =>
                                            retryArticle(article.id)}
                                    >
                                        Retry
                                    </button>
                                </div>
                            {:else}
                                <p
                                    class="text-[13px] text-zinc-500 dark:text-zinc-400 line-clamp-1"
                                >
                                    {article.preview}
                                </p>
                            {/if}
                        </div>

                        {#if contextMenuTarget === article.id}
                            <div
                                in:fade={{ duration: 100 }}
                                out:fade={{ duration: 100 }}
                                class="absolute inset-0 z-10 bg-white/70 dark:bg-zinc-900/80 backdrop-blur-md rounded-xl p-1 flex items-center justify-center border border-zinc-200/50 dark:border-zinc-700/50"
                            >
                                {#if deleteConfirmId === article.id}
                                    <div
                                        class="flex items-center gap-2"
                                        in:slide={{ axis: "x", duration: 200 }}
                                    >
                                        <button
                                            class="flex items-center gap-1.5 bg-red-500 hover:bg-red-600 text-white rounded-lg px-3 py-1.5 text-sm font-semibold transition-colors shadow-sm"
                                            on:click|stopPropagation={() =>
                                                deleteArticle(article.id)}
                                        >
                                            <Trash2 size={14} /> Confirm
                                        </button>
                                        <button
                                            class="p-1.5 text-zinc-500 hover:bg-zinc-200/50 dark:hover:bg-zinc-800 rounded-lg transition-colors"
                                            on:click|stopPropagation={() =>
                                                (deleteConfirmId = null)}
                                        >
                                            <X size={18} />
                                        </button>
                                    </div>
                                {:else}
                                    <div
                                        class="flex items-center gap-1 bg-zinc-900 dark:bg-zinc-100 rounded-lg p-1 shadow-xl"
                                        in:fly={{ y: 5, duration: 150 }}
                                    >
                                        <button
                                            on:click|stopPropagation={() =>
                                                (deleteConfirmId = article.id)}
                                            class="flex items-center justify-center w-10 h-8 rounded-md text-red-400 hover:bg-red-500 hover:text-white transition-all"
                                            title="Delete"
                                        >
                                            <Trash2 size={16} />
                                        </button>

                                        <div
                                            class="w-[1px] h-4 bg-zinc-700 dark:bg-zinc-300 mx-1"
                                        ></div>

                                        <button
                                            on:click|stopPropagation={() =>
                                                editArticle(article)}
                                            class="flex items-center justify-center w-10 h-8 rounded-md text-zinc-300 dark:text-zinc-600 hover:bg-zinc-800 dark:hover:bg-zinc-200 hover:text-white dark:hover:text-zinc-900 transition-all"
                                            title="Edit"
                                        >
                                            <Pencil size={16} />
                                        </button>

                                        <div
                                            class="w-[1px] h-4 bg-zinc-700 dark:bg-zinc-300 mx-1"
                                        ></div>

                                        <button
                                            on:click|stopPropagation={() =>
                                                (contextMenuTarget = null)}
                                            class="flex items-center justify-center w-10 h-8 rounded-md text-zinc-400 hover:text-white dark:hover:text-zinc-900 transition-all"
                                            title="Cancel"
                                        >
                                            <X size={18} />
                                        </button>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                {/each}
            </div>
        </div>
    </div>

    <div
        class="border-t border-zinc-200/60 bg-[#f9fafb] dark:border-zinc-800/60 dark:bg-[#0f0f11] shrink-0"
    >
        <button
            type="button"
            class="w-full flex items-center justify-between px-5 py-3 hover:bg-zinc-200/30 dark:hover:bg-zinc-900 transition-colors group"
            on:click={() => (showApiConfig = true)}
        >
            <div class="flex items-center gap-2">
                <Settings2
                    size={16}
                    class="text-zinc-400 group-hover:text-zinc-700 dark:group-hover:text-zinc-300 transition-colors"
                />
                <span
                    class="text-xs font-semibold text-zinc-500 group-hover:text-zinc-800 dark:text-zinc-400 dark:group-hover:text-zinc-200 transition-colors"
                >
                    API Settings
                </span>
            </div>

            {#if $settings.apiKey && $settings.apiUrl && $settings.modelName}
                <div class="flex items-center gap-1 text-emerald-500">
                    <CheckCircle2 size={14} />
                    <span class="text-[10px] font-bold uppercase tracking-wider"
                        >Ready</span
                    >
                </div>
            {:else}
                <div class="flex items-center gap-1 text-amber-500">
                    <AlertCircle size={14} />
                    <span class="text-[10px] font-bold uppercase tracking-wider"
                        >Setup Needed</span
                    >
                </div>
            {/if}
        </button>
        <ApiConfigModal bind:open={showApiConfig} />
    </div>
</div>

<StatsModal bind:open={showStatsModal} />
<PromptGenerator bind:open={showPromptGenerator} />

<style>
    .no-scrollbar::-webkit-scrollbar {
        display: none;
    }
    .no-scrollbar {
        -ms-overflow-style: none; /* IE and Edge */
        scrollbar-width: none; /* Firefox */
    }
</style>
