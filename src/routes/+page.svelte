<script lang="ts">
    import Sidebar from "../components/Sidebar.svelte";
    import Editor from "../components/Editor.svelte";
    import Reader from "../components/Reader.svelte";
    import { currentView, isSidebarOpen } from "../lib/stores";
    import { fade, fly } from "svelte/transition";
    import { cubicOut } from "svelte/easing";
</script>

<main class="flex h-screen w-screen overflow-hidden bg-white text-zinc-900">
    <div
        class="hidden md:block h-full w-80 shrink-0 z-20 border-r border-zinc-200"
    >
        <Sidebar />
    </div>

    {#if $isSidebarOpen}
        <div
            role="button"
            tabindex="0"
            class="absolute inset-0 z-40 bg-black/50 md:hidden outline-none"
            on:click={() => isSidebarOpen.set(false)}
            on:keydown={(e) => {
                if (e.key === "Escape" || e.key === "Enter")
                    isSidebarOpen.set(false);
            }}
            transition:fade
        ></div>

        <div
            class="absolute inset-y-0 left-0 z-50 w-3/4 md:hidden shadow-2xl bg-white"
            transition:fly={{
                x: -300,
                duration: 300,
                opacity: 1,
                easing: cubicOut,
            }}
        >
            <Sidebar />
        </div>
    {/if}

    <div class="flex-1 h-full relative z-0 bg-white">
        {#if $currentView === "home"}
            <div
                class="absolute top-0 left-0 w-full h-14 px-4 flex items-center z-10 md:hidden"
            >
                <button
                    class="p-2 -ml-2 rounded-full hover:bg-zinc-100 text-zinc-600 transition-colors"
                    on:click={() => isSidebarOpen.set(true)}
                    aria-label="Open sidebar"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <line x1="4" x2="20" y1="12" y2="12" />
                        <line x1="4" x2="20" y1="6" y2="6" />
                        <line x1="4" x2="20" y1="18" y2="18" />
                    </svg>
                </button>
            </div>
            <div
                class="flex flex-col items-center justify-center h-full text-zinc-300 select-none pt-14"
            >
                <p class="md:hidden text-sm">Tap menu to start</p>
                <p class="hidden md:block">Select a text to read</p>
            </div>
        {:else if $currentView === "editor"}
            <Editor />
        {:else if $currentView === "reader"}
            <Reader />
        {/if}
    </div>
</main>
