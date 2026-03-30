<script lang="ts">
    import { onMount } from "svelte";
    import { slide, fly } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import StatsModal from "./StatsModal.svelte";
    import PromptGenerator from "./PromptGenerator.svelte";
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
    import { Plus, Trash2, Pencil, X, Sparkles, RefreshCw, BarChart2 } from "lucide-svelte";
    import ApiConfigModal from "./ApiConfigModel.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { processQueue } from "$lib/parser";

    let showApiConfig = false;
    let showStatsModal = false;
    let showPromptGenerator = false;

    type SyncStatus = "idle" | "syncing" | "success" | "error";
    let syncStatus: SyncStatus = "idle";

    $: showSyncButton =
        $settings.syncEnabled &&
        $settings.syncServerUrl &&
        $settings.userId;

    let contextMenuTarget: string | null = null;
    let deleteConfirmId: string | null = null;
    let pressTimer: number;

    // onMount(() => {
    //     articles.update((items) =>
    //         items.map((item) => {
    //             if (item.status === "parsing") {
    //                 return { ...item, status: "error" as const };
    //             }
    //             return item;
    //         })
    //     );
    // });

    function handleAdd() {
        activeArticleId.set(null);
        currentView.set("editor");
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
        }, 600);
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
                i.id === id ? { ...i, status: "parsing" as const, parsingProgress: 0 } : i
            )
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
    class="flex flex-col h-full bg-zinc-50 border-r border-zinc-200 w-full md:w-80 bg-zinc-50 dark:bg-zinc-900 dark:border-zinc-800"
>
    <div
        class="p-4 border-b border-zinc-200 flex justify-between items-center bg-white dark:bg-zinc-950 dark:border-zinc-800 pt-[env(safe-area-inset-top)]"
    >
        <div class="font-bold text-xl tracking-tight dark:text-zinc-200">
            Malim
        </div>

        <div class="flex items-center gap-2">
            {#if showSyncButton}
                <button
                    on:click={handleSync}
                    disabled={syncStatus === 'syncing'}
                    class="p-2 rounded-full transition duration-100 ease-out focus:outline-none
                        {syncStatus === 'syncing'
                            ? 'bg-zinc-100 dark:bg-zinc-800 cursor-wait animate-spin'
                            : syncStatus === 'success'
                            ? 'text-emerald-500 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 dark:hover:bg-zinc-800'
                            : syncStatus === 'error'
                            ? 'text-red-500 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 dark:hover:bg-zinc-800'
                            : 'hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 dark:text-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700'}"
                    title={syncStatus === 'syncing'
                        ? 'Syncing...'
                        : syncStatus === 'success'
                        ? 'Sync Successful'
                        : syncStatus === 'error'
                        ? 'Sync Failed'
                        : 'Sync Data'}
                >
                    <RefreshCw size={20} />
                </button>
            {/if}

            <button
                on:click={handleAdd}
                class="p-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full transition duration-100 ease-out dark:text-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
                title="New Article"
            >
                <Plus size={20} />
            </button>
        </div>
    </div>

    <div
        class="flex-1 overflow-y-auto no-scrollbar p-2"
        on:scroll={handleListScroll}
    >
        <div class="rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-950 overflow-hidden divide-y divide-zinc-200 dark:divide-zinc-800">
            <button
                on:click={() => (showPromptGenerator = true)}
                class="w-full flex items-center p-3 text-left transition-colors duration-100 ease-out hover:bg-zinc-100 active:bg-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <Sparkles class="text-zinc-500 dark:text-zinc-400" size={20} />
                <span class="ml-3 font-medium text-sm text-zinc-700 dark:text-zinc-200">Generate Article Prompt</span>
            </button>
            <button
                on:click={handleAdd}
                class="w-full flex items-center p-3 text-left transition-colors duration-100 ease-out hover:bg-zinc-100 active:bg-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <Plus class="text-zinc-500 dark:text-zinc-400" size={20} />
                <span class="ml-3 font-medium text-sm text-zinc-700 dark:text-zinc-200">New Article</span>
            </button>
            <button
                on:click={() => (showStatsModal = true)}
                class="w-full flex items-center p-3 text-left transition-colors duration-100 ease-out hover:bg-zinc-100 active:bg-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
            >
                <BarChart2 class="text-zinc-500 dark:text-zinc-400" size={20} />
                <span class="ml-3 font-medium text-sm text-zinc-700 dark:text-zinc-200">View Statistics</span>
            </button>
        </div>

        <hr class="border-zinc-200 dark:border-zinc-800 my-3"/>

        <div class="space-y-2">
            {#each visibleArticles as article (article.id)}
                <div
                    class="relative group rounded-xl bg-white border border-zinc-100 shadow-sm overflow-hidden select-none touch-manipulation hover:shadow-md active:scale-[0.98] active:bg-zinc-100 transition duration-100 ease-out dark:bg-zinc-950 dark:border-zinc-800 dark:hover:border-zinc-700 dark:active:bg-zinc-800"
                    class:opacity-50={article.status === "parsing"}
                    role="button"
                    tabindex="0"
                    on:click={() => {
                        if (contextMenuTarget) return;
                        openArticle(article);
                    }}
                    on:mousedown={() => article.status !== "parsing" && startPress(article.id)}
                    on:mouseup={endPress}
                    on:mouseleave={endPress}
                    on:touchstart={() => article.status !== "parsing" && startPress(article.id)}
                    on:touchend={endPress}
                    on:keydown={(e) => e.key === "Enter" && openArticle(article)}
                >
                    <div class="p-4">
                        <div class="flex justify-between items-start mb-1">
                            <h3
                                class="font-semibold text-zinc-800 truncate pr-2 leading-tight dark:text-zinc-100"
                            >
                                {article.title || "Untitled"}
                            </h3>
                            <div class="shrink-0">
                                <Flag code={article.language} size={18} />
                            </div>
                        </div>

                        {#if article.status === "parsing"}
                            <div
                                class="h-1 w-full bg-zinc-100 mt-2 rounded-full overflow-hidden dark:bg-zinc-800"
                            >
                                <div
                                    class="h-full bg-zinc-900 transition-all duration-300 dark:bg-zinc-200"
                                    style="width: {article.parsingProgress}%"
                                ></div>
                            </div>
                        {:else if article.status === "error"}
                            <div class="flex items-center gap-2 mt-2">
                                <div class="flex-1 h-1 w-full bg-zinc-100 rounded-full overflow-hidden dark:bg-zinc-800">
                                    <div
                                        class="h-full bg-red-500 dark:bg-red-400 transition-all duration-300"
                                        style="width: {article.parsingProgress}%"
                                    ></div>
                                </div>
                                <button
                                    class="shrink-0 text-[10px] font-medium text-zinc-400 dark:text-zinc-500 hover:text-red-500 dark:hover:text-red-400 px-1.5 py-0.5 rounded transition-colors"
                                    on:click|stopPropagation={() => retryArticle(article.id)}
                                >
                                    Retry
                                </button>
                            </div>
                        {:else}
                            <p class="text-sm text-zinc-500 line-clamp-2">
                                {article.preview}
                            </p>
                        {/if}
                    </div>

                    {#if contextMenuTarget === article.id}
                        <div
                            transition:slide={{ axis: "y", duration: 200 }}
                            class="bg-zinc-900 text-white p-2 flex items-center justify-around absolute inset-0 z-10"
                        >
                            <div
                                class="relative flex items-center overflow-hidden h-full"
                            >
                                {#if deleteConfirmId === article.id}
                                    <button
                                        transition:fly={{ x: 20, duration: 200 }}
                                        class="flex items-center space-x-2 bg-red-600 rounded-lg px-3 py-1 h-full cursor-pointer hover:bg-red-500"
                                        on:click|stopPropagation={() =>
                                            deleteArticle(article.id)}
                                    >
                                        <span class="text-sm font-bold"
                                            >Confirm?</span
                                        >
                                    </button>
                                {:else}
                                    <button
                                        on:click|stopPropagation={() =>
                                            (deleteConfirmId = article.id)}
                                        class="flex flex-col items-center justify-center w-16 hover:text-red-400 transition-colors"
                                    >
                                        <Trash2 size={18} />
                                        <span class="text-[10px] mt-1">Delete</span>
                                    </button>
                                {/if}
                            </div>

                            <button
                                on:click|stopPropagation={() => editArticle(article)}
                                class="flex flex-col items-center justify-center w-16 hover:text-blue-400 transition-colors"
                            >
                                <Pencil size={18} />
                                <span class="text-[10px] mt-1">Edit</span>
                            </button>

                            <button
                                on:click|stopPropagation={() =>
                                    (contextMenuTarget = null)}
                                class="flex flex-col items-center justify-center w-10 text-zinc-500 hover:text-white transition-colors"
                            >
                                <X size={18} />
                            </button>
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    </div>

    <div
        class="p-3 border-t border-zinc-200 bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900"
    >
        <button
            type="button"
            class="w-full flex items-center justify-center gap-2 px-3 py-2 bg-zinc-900 hover:bg-zinc-800 active:scale-[0.98] rounded-xl text-white text-sm font-medium shadow-lg transition duration-100 ease-out dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
            on:click={() => (showApiConfig = true)}
        >
            <span>
                {$settings.apiKey && $settings.apiUrl && $settings.modelName
                    ? "API configured ✓"
                    : "Configure API"}
            </span>
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