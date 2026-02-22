<script lang="ts">
    import { slide, fly } from "svelte/transition";
    import Flag from "./Flag.svelte";
    import {
        articles,
        activeArticleId,
        currentView,
        isSidebarOpen,
        editorDraft,
    } from "../lib/stores";
    import type { Article } from "../lib/types";
    import { Plus, Trash2, Pencil, X } from "lucide-svelte";
    import ApiConfigModal from "./ApiConfigModel.svelte";
    import { settings } from "../lib/stores";
    import { invoke } from "@tauri-apps/api/core";

    let showApiConfig = false;

    let contextMenuTarget: string | null = null;
    let deleteConfirmId: string | null = null;
    let pressTimer: number;

    function handleAdd() {
        // editorDraft.set({ title: '', content: '', language: 'KR' });
        activeArticleId.set(null);
        currentView.set("editor");
        isSidebarOpen.set(false);
    }

    function openArticle(article: Article) {
        if (article.status === "parsing") return;
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
            content: article.sentences
                ? article.sentences.map((s) => s.original).join("")
                : "",
            language: article.language,
        });
        activeArticleId.set(article.id);
        currentView.set("editor");
        contextMenuTarget = null;
        isSidebarOpen.set(false);
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
        class="p-4 border-b border-zinc-200 flex justify-between items-center bg-white dark:bg-zinc-950 dark:border-zinc-800"
    >
        <h1
            class="font-bold text-xl tracking-tight text-zinc-800 dark:text-zinc-100"
        >
            Malim
        </h1>
        <button
            on:click={handleAdd}
            class="p-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full transition duration-100 ease-out dark:text-zinc-200 dark:hover:bg-zinc-800 dark:active:bg-zinc-700"
        >
            <Plus size={20} />
        </button>
    </div>

    <div
        class="flex-1 overflow-y-auto no-scrollbar p-2 space-y-2"
        on:scroll={handleListScroll}
    >
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
                on:mousedown={() => startPress(article.id)}
                on:mouseup={endPress}
                on:mouseleave={endPress}
                on:touchstart={() => startPress(article.id)}
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
                            on:click|stopPropagation={() =>
                                editArticle(article)}
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

    <div
        class="p-3 border-t border-zinc-200 bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900"
    >
        <button
            type="button"
            class="w-full flex items-center justify-center gap-2 px-3 py-2 bg-zinc-900 hover:bg-zinc-800 active:scale-[0.98] rounded-xl text-white text-sm font-medium shadow-lg transition duration-100 ease-out dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
            on:click={() => (showApiConfig = true)}
        >
            <span>
                {($settings.apiKey && $settings.apiUrl && $settings.modelName) ? "API configured ✓" : "Configure API"}
            </span>
        </button>

        <ApiConfigModal bind:open={showApiConfig} />
    </div>
</div>

<style>
    .no-scrollbar::-webkit-scrollbar {
        display: none;
    }
    .no-scrollbar {
        -ms-overflow-style: none;  /* IE and Edge */
        scrollbar-width: none;  /* Firefox */
    }
</style>