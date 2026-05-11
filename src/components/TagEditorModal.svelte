<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { X, Tag, Plus, Hash } from "lucide-svelte";
    import { articles } from "../lib/stores";

    export let opened = false;
    export let articleId: string | null = null;
    export let allTags: string[] = [];

    let inputText = "";
    let showSuggestions = false;

    $: article = $articles.find((a) => a.id === articleId);
    $: currentTags = article?.tags ?? [];
    $: suggestions = allTags
        .filter(
            (t) =>
                t.toLowerCase().startsWith(inputText.toLowerCase()) &&
                !currentTags.includes(t),
        )
        .slice(0, 8);
    $: remainingTags = allTags.filter((t) => !currentTags.includes(t));

    function addTag(tag: string) {
        const trimmed = tag.trim();
        if (!trimmed || !articleId) return;
        if (currentTags.includes(trimmed)) {
            inputText = "";
            showSuggestions = false;
            return;
        }
        articles.update((items) =>
            items.map((a) =>
                a.id === articleId ? { ...a, tags: [...a.tags, trimmed] } : a,
            ),
        );
        inputText = "";
        showSuggestions = false;
    }

    function removeTag(tag: string) {
        if (!articleId) return;
        articles.update((items) =>
            items.map((a) =>
                a.id === articleId
                    ? { ...a, tags: a.tags.filter((t) => t !== tag) }
                    : a,
            ),
        );
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            if (inputText.trim()) {
                addTag(inputText);
            }
        } else if (e.key === "Escape") {
            opened = false;
        }
    }

    function close() {
        opened = false;
        inputText = "";
        showSuggestions = false;
    }
</script>

{#if opened}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        on:click={close}
    >
        <!-- Backdrop -->
        <div
            class="absolute inset-0 bg-black/40 backdrop-blur-sm"
            transition:fade={{ duration: 150 }}
        ></div>

        <!-- Modal -->
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
            class="relative w-full max-w-md bg-white dark:bg-zinc-900 rounded-2xl shadow-2xl border border-zinc-200/80 dark:border-zinc-700/80 overflow-hidden"
            transition:fly={{ y: 20, duration: 200 }}
            on:click|stopPropagation
        >
            <!-- Header -->
            <div
                class="flex items-center justify-between px-5 py-4 border-b border-zinc-100 dark:border-zinc-800"
            >
                <div class="flex items-center gap-2.5">
                    <div
                        class="w-8 h-8 rounded-lg bg-indigo-50 dark:bg-indigo-500/10 flex items-center justify-center"
                    >
                        <Tag size={16} class="text-indigo-500" />
                    </div>
                    <h2
                        class="text-[15px] font-bold text-zinc-900 dark:text-white"
                    >
                        Edit Tags
                    </h2>
                </div>
                <button
                    on:click={close}
                    class="p-1.5 rounded-lg hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
                >
                    <X size={18} />
                </button>
            </div>

            <!-- Content -->
            <div class="p-5 space-y-4">
                <!-- Article title -->
                <p
                    class="text-xs text-zinc-400 dark:text-zinc-500 line-clamp-1 italic"
                >
                    {article?.title || "Untitled"}
                </p>

                <!-- Current tags -->
                <div>
                    {#if currentTags.length > 0}
                        <div class="flex flex-wrap gap-1.5">
                            {#each currentTags as tag}
                                <span
                                    class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-[11px] font-semibold bg-indigo-50 text-indigo-600 dark:bg-indigo-500/10 dark:text-indigo-400 transition-all group hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-500/10 dark:hover:text-red-400 cursor-default"
                                >
                                    <Hash size={10} />
                                    <span>{tag}</span>
                                    <button
                                        on:click={() => removeTag(tag)}
                                        class="ml-0.5 opacity-50 group-hover:opacity-100 transition-opacity"
                                    >
                                        <X size={12} />
                                    </button>
                                </span>
                            {/each}
                        </div>
                    {:else}
                        <p class="text-[12px] text-zinc-400 dark:text-zinc-500">
                            No tags yet. Start typing below to add one.
                        </p>
                    {/if}
                </div>

                <!-- Input with autocomplete -->
                <div class="relative">
                    <div
                        class="flex items-center gap-2 px-3 py-2.5 bg-zinc-50 dark:bg-zinc-800/50 border border-zinc-200 dark:border-zinc-700 rounded-xl focus-within:border-indigo-400 dark:focus-within:border-indigo-500 focus-within:ring-2 focus-within:ring-indigo-500/20 transition-all"
                    >
                        <Hash size={15} class="text-zinc-400 shrink-0" />
                        <input
                            type="text"
                            bind:value={inputText}
                            on:keydown={handleKeydown}
                            on:focus={() => (showSuggestions = true)}
                            on:blur={() =>
                                setTimeout(
                                    () => (showSuggestions = false),
                                    180,
                                )}
                            placeholder="Add a tag..."
                            class="flex-1 bg-transparent text-sm text-zinc-900 dark:text-white placeholder-zinc-400 dark:placeholder-zinc-500 outline-none min-w-0"
                        />
                        {#if inputText.trim()}
                            <button
                                on:mousedown|preventDefault={() =>
                                    addTag(inputText)}
                                class="p-1 rounded-md bg-indigo-500 hover:bg-indigo-600 text-white transition-colors shrink-0"
                            >
                                <Plus size={14} />
                            </button>
                        {/if}
                    </div>

                    <!-- Suggestions dropdown -->
                    {#if showSuggestions && suggestions.length > 0}
                        <div
                            class="absolute top-full left-0 right-0 mt-1.5 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-xl shadow-lg overflow-hidden z-10 max-h-48 overflow-y-auto no-scrollbar"
                        >
                            {#each suggestions as tag}
                                <button
                                    on:mousedown|preventDefault={() =>
                                        addTag(tag)}
                                    class="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-zinc-50 dark:hover:bg-zinc-700/50 transition-colors"
                                >
                                    <Hash size={13} class="text-zinc-400" />
                                    <span
                                        class="text-[13px] text-zinc-700 dark:text-zinc-300"
                                        >{tag}</span
                                    >
                                </button>
                            {/each}
                        </div>
                    {/if}
                </div>

                <!-- Existing tags quick-add -->
                {#if remainingTags.length > 0 && !inputText}
                    <div>
                        <p
                            class="text-[10px] font-bold tracking-[0.12em] uppercase text-zinc-400 dark:text-zinc-500 mb-2"
                        >
                            All Tags
                        </p>
                        <div class="flex flex-wrap gap-1.5">
                            {#each remainingTags.slice(0, 20) as tag}
                                <button
                                    on:click={() => addTag(tag)}
                                    class="px-2.5 py-1 rounded-lg text-[11px] font-medium bg-zinc-100 text-zinc-600 hover:bg-indigo-50 hover:text-indigo-600 dark:bg-zinc-800 dark:text-zinc-400 dark:hover:bg-indigo-500/10 dark:hover:text-indigo-400 transition-colors"
                                >
                                    {tag}
                                </button>
                            {/each}
                        </div>
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    .no-scrollbar::-webkit-scrollbar {
        display: none;
    }
    .no-scrollbar {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
