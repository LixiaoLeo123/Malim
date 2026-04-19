<script lang="ts">
    import { onMount } from "svelte";
    import { slide, fly, fade } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import StatsModal from "./StatsModal.svelte";
    import PromptGenerator from "./PromptGenerator.svelte";
    import { Globe, MessageCircle, Star } from "lucide-svelte";
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
        ListChecks,
        CheckSquare,
        Square
    } from "lucide-svelte";
    import ApiConfigModal from "./ApiConfigModel.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { processQueue } from "$lib/parser";
    import Brain from "./Brain.svelte";

    let showApiConfig = false;
    let showStatsModal = false;
    let showPromptGenerator = false;
    let showBrain = false;

    type SyncStatus = "idle" | "syncing" | "success" | "error";
    let syncStatus: SyncStatus = "idle";

    $: showSyncButton =
        $settings.syncEnabled && $settings.syncServerUrl && $settings.userId;

    let contextMenuTarget: string | null = null;
    let deleteConfirmId: string | null = null;
    let pressTimer: number;

    let isSelectMode = false;
    let selectedArticleIds = new Set<string>();
    let showMultiDeleteConfirm = false;

    function handleAdd() {
        activeArticleId.set(null);
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
        if (isSelectMode) return;
        pressTimer = window.setTimeout(() => {
            contextMenuTarget = id;
            deleteConfirmId = null;
        }, 400);
    }

    function endPress() {
        if (isSelectMode) return;
        clearTimeout(pressTimer);
    }

    function toggleSelect(id: string) {
        if (selectedArticleIds.has(id)) {
            selectedArticleIds.delete(id);
        } else {
            selectedArticleIds.add(id);
        }
        selectedArticleIds = new Set(selectedArticleIds);
    }

    async function deleteSelected() {
        if (selectedArticleIds.size === 0) return;
        
        const idsToDelete = new Set(selectedArticleIds);
        articles.update((items) => items.filter((i) => !idsToDelete.has(i.id)));
        
        let activeIdWasDeleted = false;
        activeArticleId.update((currentId) => {
            if (currentId && idsToDelete.has(currentId)) {
                activeIdWasDeleted = true;
                return null;
            }
            return currentId;
        });

        if (activeIdWasDeleted) {
            currentView.set("home");
        }

        isSelectMode = false;
        showMultiDeleteConfirm = false;
        selectedArticleIds.clear();
        selectedArticleIds = new Set();
        
        for (const id of idsToDelete) {
            await invoke("delete_article_audio", { id }).catch(console.error);
        }
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

    function toggleStar(id: string) {
        articles.update((items) =>
            items.map((i) =>
                i.id === id ? { ...i, stared: !i.stared } : i,
            ),
        );
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
        class="px-5 py-4 flex justify-between items-center bg-[#f9fafb]/90 dark:bg-[#0f0f11]/90 backdrop-blur-md sticky top-0 z-20 pt-[calc(env(safe-area-inset-top)+1rem)] border-b border-zinc-100 dark:border-zinc-900/50 shadow-sm transition-all"
    >
        <div
            class="group flex items-center gap-3 cursor-pointer select-none"
            on:click={() => (showBrain = true)}
        >
            <div
                class="w-[30px] h-[30px] bg-zinc-900 dark:bg-white rounded-[10px] flex items-center justify-center flex-shrink-0 transition-transform duration-400 ease-out group-hover:scale-[1.05] group-hover:rotate-3 shadow-sm active:scale-95"
            >
                <span
                    class="text-white dark:text-zinc-900 text-[16px] font-black pointer-events-none transform transition-transform group-hover:-rotate-3"
                    >말</span
                >
            </div>
            <span class="font-extrabold text-[22px] tracking-tight text-zinc-900 dark:text-zinc-50 transition-colors duration-300">
                Malim
            </span>
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
                on:click={() => { isSelectMode = !isSelectMode; selectedArticleIds.clear(); selectedArticleIds = new Set(); contextMenuTarget = null; showMultiDeleteConfirm = false; }}
                class="p-2 ml-1 {isSelectMode ? 'bg-indigo-100 text-indigo-600 dark:bg-indigo-500/20 dark:text-indigo-400' : 'text-zinc-500 hover:bg-zinc-200/50 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-white'} active:scale-95 rounded-xl transition-all duration-200 shadow-sm"
                title="Select Mode"
            >
                <ListChecks size={18} strokeWidth={2.5} />
            </button>

            <button
                on:click={handleAdd}
                class="p-2 ml-1 text-white bg-zinc-900 hover:bg-zinc-800 active:scale-95 rounded-xl transition-all duration-200 shadow-sm dark:bg-zinc-100 dark:text-zinc-950 dark:hover:bg-white dark:hover:shadow-md"
                title="New Article"
            >
                <Plus size={18} strokeWidth={2.5} />
            </button>
        </div>
    </div>

    {#if isSelectMode}
        <div 
            class="px-4 py-2.5 flex items-center justify-between border-b border-zinc-200/60 dark:border-zinc-800/60 bg-white dark:bg-zinc-950 sticky top-[calc(env(safe-area-inset-top)+4.2rem)] z-10 shadow-sm transition-all" 
            transition:slide={{axis: 'y'}}
        >
            {#if showMultiDeleteConfirm}
                <span class="text-xs font-bold text-red-500 tracking-wide uppercase shrink-0">
                    Delete {selectedArticleIds.size}?
                </span>
                <div class="flex items-center gap-1.5" in:fade={{ duration: 150 }}>
                    <button
                        on:click={() => showMultiDeleteConfirm = false}
                        class="flex items-center justify-center w-8 h-8 md:w-auto md:px-2.5 md:py-1.5 md:gap-1 bg-zinc-100 text-zinc-500 hover:bg-zinc-200 hover:text-zinc-700 dark:bg-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-700 dark:hover:text-zinc-200 rounded-md text-[10px] font-bold uppercase tracking-wider transition-colors"
                    >
                        <X size={15} strokeWidth={2.5} /> <span class="hidden md:inline">Cancel</span>
                    </button>
                    <button
                        on:click={() => { showMultiDeleteConfirm = false; deleteSelected(); }}
                        class="flex items-center justify-center w-8 h-8 md:w-auto md:px-2.5 md:py-1.5 md:gap-1 bg-red-500 hover:bg-red-600 text-white rounded-md text-[10px] font-bold uppercase tracking-wider transition-colors shadow-sm"
                    >
                        <CheckCircle2 size={15} strokeWidth={2.5} /> <span class="hidden md:inline">Confirm</span>
                    </button>
                </div>
            {:else}
                <span class="text-sm font-semibold text-zinc-600 dark:text-zinc-300">
                    {selectedArticleIds.size} Selected
                </span>
                <button
                    on:click={() => showMultiDeleteConfirm = true}
                    disabled={selectedArticleIds.size === 0}
                    class="flex items-center gap-1.5 px-3 py-1.5 {selectedArticleIds.size > 0 ? 'bg-red-500 hover:bg-red-600 text-white' : 'bg-zinc-200 text-zinc-400 dark:bg-zinc-800 dark:text-zinc-600'} rounded-lg text-xs font-semibold uppercase tracking-wider transition-colors disabled:cursor-not-allowed"
                >
                    <Trash2 size={14} /> Delete
                </button>
            {/if}
        </div>
    {/if}

    <div
        class="flex-1 overflow-y-auto no-scrollbar pb-6"
        on:scroll={handleListScroll}
    >
        <div class="px-3 pt-3 mb-2 space-y-[2px]">
            <div class="px-2 pt-1 pb-2 text-[10px] font-bold tracking-[0.15em] text-zinc-400 dark:text-zinc-500 uppercase">
                Menu
            </div>
            
            <button on:click={handleDiscover} class="w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-zinc-700 dark:text-zinc-300 hover:bg-white dark:hover:bg-zinc-900 transition-all hover:shadow-sm border border-transparent hover:border-zinc-200/50 dark:hover:border-zinc-800 active:scale-[0.98] group">
                <div class="flex items-center gap-3">
                    <Globe size={18} class="text-blue-500/80 group-hover:text-blue-500 transition-colors" opacity={0.9} />
                    <span class="font-semibold text-[14px]">Discover</span>
                </div>
            </button>
            <button on:click={handleChat} class="w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-zinc-700 dark:text-zinc-300 hover:bg-white dark:hover:bg-zinc-900 transition-all hover:shadow-sm border border-transparent hover:border-zinc-200/50 dark:hover:border-zinc-800 active:scale-[0.98] group">
                <div class="flex items-center gap-3">
                    <MessageCircle size={18} class="text-emerald-500/80 group-hover:text-emerald-500 transition-colors" opacity={0.9} />
                    <span class="font-semibold text-[14px]">AI Chat</span>
                </div>
            </button>
            <button on:click={() => (showPromptGenerator = true)} class="w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-zinc-700 dark:text-zinc-300 hover:bg-white dark:hover:bg-zinc-900 transition-all hover:shadow-sm border border-transparent hover:border-zinc-200/50 dark:hover:border-zinc-800 active:scale-[0.98] group">
                <div class="flex items-center gap-3">
                    <Sparkles size={18} class="text-indigo-500/80 group-hover:text-indigo-500 transition-colors" opacity={0.9} />
                    <span class="font-semibold text-[14px]">AI Prompt</span>
                </div>
            </button>
            <button on:click={() => (showStatsModal = true)} class="w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-zinc-700 dark:text-zinc-300 hover:bg-white dark:hover:bg-zinc-900 transition-all hover:shadow-sm border border-transparent hover:border-zinc-200/50 dark:hover:border-zinc-800 active:scale-[0.98] group">
                <div class="flex items-center gap-3">
                    <BarChart2 size={18} class="text-orange-500/80 group-hover:text-orange-500 transition-colors" opacity={0.9} />
                    <span class="font-semibold text-[14px]">Statistics</span>
                </div>
            </button>
        </div>

        <div class="px-3 mt-4">
            <div class="flex items-center justify-between px-2 pt-2 pb-2">
                <div class="text-[10px] font-bold tracking-[0.15em] text-zinc-400 dark:text-zinc-500 uppercase">
                    Your Library
                </div>
                <div class="text-[10px] font-medium text-zinc-400/80 dark:text-zinc-600 bg-zinc-200/50 dark:bg-zinc-800/50 px-1.5 py-0.5 rounded-md">
                    {$articles.length}
                </div>
            </div>

            <div class="space-y-[3px]">
                {#each visibleArticles as article (article.id)}
                    <div
                        class="relative group rounded-xl bg-transparent select-none touch-manipulation hover:bg-white dark:hover:bg-zinc-900 overflow-hidden transition-all duration-200 ease-out active:scale-[0.99] active:bg-zinc-100 dark:active:bg-zinc-800 border border-transparent hover:border-zinc-200/50 dark:hover:border-zinc-800 hover:shadow-sm"
                        class:opacity-60={article.status === "parsing"}
                        class:bg-amber-50_30={article.stared}
                        class:dark:bg-amber-500_5={article.stared}
                        role="button"
                        tabindex="0"
                        on:click={() => {
                            if (isSelectMode) {
                                toggleSelect(article.id);
                                return;
                            }
                            if (contextMenuTarget) return;
                            openArticle(article);
                        }}
                        on:contextmenu|preventDefault={(e) => {
                            if (isSelectMode || article.status === "parsing") return;
                            contextMenuTarget = article.id;
                            deleteConfirmId = null;
                        }}
                        on:mousedown={() =>
                            article.status !== "parsing" &&
                            !isSelectMode &&
                            startPress(article.id)}
                        on:mouseup={endPress}
                        on:mouseleave={endPress}
                        on:touchstart={() =>
                            article.status !== "parsing" &&
                            !isSelectMode &&
                            startPress(article.id)}
                        on:touchend={endPress}
                        on:keydown={(e) =>
                            e.key === "Enter" && openArticle(article)}
                    >
                        {#if isSelectMode && article.status !== "parsing"}
                            <div class="absolute right-3 top-3 z-10">
                                {#if selectedArticleIds.has(article.id)}
                                    <div class="w-5 h-5 bg-indigo-500 rounded-[5px] flex items-center justify-center shadow-sm">
                                        <CheckSquare size={14} class="text-white" />
                                    </div>
                                {:else}
                                    <div class="w-5 h-5 bg-white border border-zinc-300 dark:border-zinc-700 dark:bg-zinc-800 rounded-[5px] shadow-sm"></div>
                                {/if}
                            </div>
                        {/if}

                        <div class="absolute -left-10 -bottom-10 opacity-[0.08] dark:opacity-[0.12] pointer-events-none transition-all duration-300 group-hover:scale-110 group-hover:rotate-6 group-hover:opacity-[0.15] dark:group-hover:opacity-[0.2]">
                            <Flag code={article.language} size={96} />
                        </div>

                        {#if article.stared}
                            <div class="absolute left-0 top-0 bottom-0 w-[3px] bg-orange-400 rounded-l-xl z-10"></div>
                        {/if}
                        
                        {#if article.status !== "parsing" && article.status !== "error" && article.readProgress > 0}
                            <div class="absolute top-0 left-0 right-0 h-0.5 bg-zinc-100 dark:bg-zinc-800/80 overflow-hidden opacity-80 group-hover:opacity-100 transition-opacity z-10">
                                <div 
                                    class="h-full bg-emerald-500 rounded-r-full transition-all duration-500" 
                                    style="width: {article.readProgress}%"
                                ></div>
                            </div>
                        {/if}

                        <div class="px-3 py-2.5 flex flex-col gap-1.5 z-10 relative {article.stared ? 'pl-4' : ''}">
                            <div class="flex items-start justify-between gap-3">
                                <h3
                                    class="font-semibold text-zinc-800 text-[13.5px] leading-snug dark:text-zinc-200 line-clamp-2 {isSelectMode ? 'pr-8' : 'pr-1'}"
                                >
                                    {article.title || "Untitled"}
                                </h3>
                                
                                {#if !isSelectMode}
                                    <div class="flex items-center gap-1.5 shrink-0 pt-0.5">
                                        {#if article.stared}
                                            <button 
                                                class="p-1 rounded-md bg-orange-50/50 dark:bg-orange-500/10 hover:bg-orange-100 dark:hover:bg-orange-500/20 transition-colors"
                                                on:click|stopPropagation={() => toggleStar(article.id)}
                                            >
                                                <Star size={13} class="fill-orange-400 text-orange-400" />
                                            </button>
                                        {:else}
                                            <button 
                                                class="p-1 rounded-md opacity-100 sm:opacity-0 sm:group-hover:opacity-100 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-all text-zinc-300 hover:text-zinc-500 dark:text-zinc-500 dark:hover:text-zinc-400"
                                                on:click|stopPropagation={() => toggleStar(article.id)}
                                            >
                                                <Star size={13} />
                                            </button>
                                        {/if}
                                    </div>
                                {/if}
                            </div>

                            {#if article.status === "parsing"}
                                <div class="flex items-center gap-2 pt-0.5">
                                    <div
                                        class="flex-1 h-1 bg-zinc-200/60 rounded-full overflow-hidden dark:bg-zinc-800"
                                    >
                                        <div
                                            class="h-full bg-indigo-500 transition-all duration-300 rounded-full"
                                            style="width: {article.parsingProgress}%"
                                        ></div>
                                    </div>
                                    <span
                                        class="text-[9px] font-bold text-indigo-500 uppercase tracking-wide animate-pulse"
                                        >Parsing</span
                                    >
                                </div>
                            {:else if article.status === "error"}
                                <div class="flex items-center justify-between pt-0.5">
                                    <div
                                        class="flex items-center gap-1.5 text-red-500 leading-none"
                                    >
                                        <AlertCircle
                                            size={12}
                                            strokeWidth={2.5}
                                        />
                                        <span class="text-xs font-semibold"
                                            >Failed</span
                                        >
                                    </div>
                                    <button
                                        class="text-[9px] font-bold text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-500/20 px-2 py-1 rounded transition-colors uppercase tracking-wider"
                                        on:click|stopPropagation={() =>
                                            retryArticle(article.id)}
                                    >
                                        Retry
                                    </button>
                                </div>
                            {:else}
                                <p
                                    class="text-[11.5px] text-zinc-400/90 dark:text-zinc-500/90 line-clamp-1 select-text"
                                >
                                    {article.preview}
                                </p>
                            {/if}
                        </div>

                        {#if contextMenuTarget === article.id}
                            <div
                                in:fade={{ duration: 100 }}
                                out:fade={{ duration: 100 }}
                                class="absolute inset-0 z-[15] bg-white/90 dark:bg-zinc-900/95 backdrop-blur-md p-1.5 flex items-stretch outline outline-1 outline-zinc-200/80 dark:outline-zinc-700/80 rounded-xl"
                            >
                                {#if deleteConfirmId === article.id}
                                    <div
                                        class="flex items-stretch w-full gap-1.5"
                                        in:slide={{ axis: "x", duration: 150 }}
                                    >
                                        <button
                                            class="flex-1 flex flex-col items-center justify-center gap-1 bg-red-500 hover:bg-red-600 text-white rounded-lg text-[13px] font-bold uppercase tracking-wider transition-colors shadow-sm"
                                            on:click|stopPropagation={() =>
                                                deleteArticle(article.id)}
                                        >
                                            <Trash2 size={18} strokeWidth={2.5} /> Confirm
                                        </button>
                                        <button
                                            class="w-14 shrink-0 flex flex-col items-center justify-center gap-1 text-zinc-500 bg-zinc-200/60 hover:bg-zinc-300/80 dark:bg-zinc-800 dark:hover:bg-zinc-700 rounded-lg transition-colors text-[11px] font-bold uppercase tracking-wider shadow-sm"
                                            on:click|stopPropagation={() =>
                                                (deleteConfirmId = null)}
                                        >
                                            <X size={18} strokeWidth={2.5} />
                                        </button>
                                    </div>
                                {:else}
                                    <div
                                        class="flex items-stretch w-full gap-1.5"
                                        in:fly={{ y: 5, duration: 100 }}
                                    >
                                        <button
                                            on:click|stopPropagation={() =>
                                                (deleteConfirmId = article.id)}
                                            class="flex-1 flex flex-col items-center justify-center gap-1 rounded-lg text-red-500 bg-red-50/50 hover:bg-red-500 hover:text-white dark:bg-red-500/10 dark:hover:bg-red-500 dark:hover:text-white transition-all shadow-sm"
                                            title="Delete"
                                        >
                                            <Trash2 size={18} />
                                            <span class="text-[11px] font-bold tracking-wider">Delete</span>
                                        </button>

                                        <button
                                            on:click|stopPropagation={() =>
                                                editArticle(article)}
                                            class="flex-1 flex flex-col items-center justify-center gap-1 rounded-lg text-zinc-600 bg-zinc-100/80 hover:bg-zinc-800 hover:text-white dark:bg-zinc-800/80 dark:text-zinc-300 dark:hover:bg-zinc-200 dark:hover:text-zinc-900 transition-all shadow-sm"
                                            title="Edit"
                                        >
                                            <Pencil size={18} />
                                            <span class="text-[11px] font-bold tracking-wider">Edit</span>
                                        </button>

                                        <button
                                            on:click|stopPropagation={() =>
                                                (contextMenuTarget = null)}
                                            class="w-12 shrink-0 flex flex-col items-center justify-center gap-1 rounded-lg text-zinc-400 bg-white hover:bg-zinc-200/80 dark:bg-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-white transition-all outline outline-1 outline-zinc-200 dark:outline-zinc-700 shadow-sm"
                                            title="Cancel"
                                        >
                                            <X size={20} strokeWidth={2} />
                                        </button>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                {/each}
                {#if visibleArticles.length === 0}
                    <div class="py-8 text-center px-4">
                        <div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-zinc-100 dark:bg-zinc-800 mb-3 text-zinc-400">
                            <Plus size={24} />
                        </div>
                        <h4 class="text-[13px] font-semibold text-zinc-700 dark:text-zinc-300 mb-1">No articles yet</h4>
                        <p class="text-[12px] text-zinc-400 dark:text-zinc-500">Tap the + button to add your first article.</p>
                    </div>
                {/if}
            </div>
        </div>
    </div>

    <div
        class="border-t border-zinc-200/60 bg-[#f9fafb] dark:border-zinc-800/60 dark:bg-[#0f0f11] shrink-0 pb-[env(safe-area-inset-bottom)]"
    >
        <button
            type="button"
            class="w-full flex items-center justify-between px-4 py-2.5 hover:bg-zinc-200/40 dark:hover:bg-zinc-900 transition-colors group"
            on:click={() => (showApiConfig = true)}
        >
            <div class="flex items-center gap-2.5">
                <Settings2
                    size={16}
                    class="text-zinc-500 dark:text-zinc-400 group-hover:text-zinc-800 dark:group-hover:text-zinc-200 transition-colors"
                />
                <span
                    class="text-[13.5px] font-semibold text-zinc-600 group-hover:text-zinc-900 dark:text-zinc-300 dark:group-hover:text-zinc-100 transition-colors"
                >
                    Settings
                </span>
            </div>

            {#if $settings.defaultAiConfigId && $settings.embedAiConfigId && $settings.grammarAiConfigId && $settings.shadowAiConfigId && $settings.mainAiConfigId}
                <div class="flex items-center text-emerald-500">
                    <CheckCircle2 size={14} class="mr-1" />
                    <span class="text-[10px] font-bold uppercase tracking-wider"
                        >Ready</span
                    >
                </div>
            {:else}
                <div class="flex items-center text-amber-500">
                    <AlertCircle size={14} class="mr-1" />
                    <span class="text-[10px] font-bold uppercase tracking-wider"
                        >Setup needed</span
                    >
                </div>
            {/if}
        </button>
        <ApiConfigModal bind:opened={showApiConfig} />
    </div>
</div>

<StatsModal bind:open={showStatsModal} />
<PromptGenerator bind:open={showPromptGenerator} />
<Brain bind:open={showBrain} />

<style>
    .no-scrollbar::-webkit-scrollbar {
        display: none;
    }
    .no-scrollbar {
        -ms-overflow-style: none; /* IE and Edge */
        scrollbar-width: none; /* Firefox */
    }
</style>
