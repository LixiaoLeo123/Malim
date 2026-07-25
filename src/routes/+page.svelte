<script lang="ts">
    import Sidebar from "../components/Sidebar.svelte";
    import { currentView, isSidebarOpen, pushView, articles, activeArticleId, dictionaryHistory, translatorSessions } from "../lib/stores";
    import type { AppView } from "../lib/stores";
    import { ArrowRight, BookOpenText, Languages, MessageCircle, BookMarked, Sparkles, Search, Plus, Clock3, TrendingUp, Library, ChevronRight } from "lucide-svelte";
    import { fade, fly } from "svelte/transition";
    import { cubicOut } from "svelte/easing";
    import type { Component } from "svelte";
    import type { Article } from "../lib/types";

    type ContentView = Exclude<AppView, "home">;
    type ViewComponent = Component;

    const viewLoaders: Record<ContentView, () => Promise<{ default: ViewComponent }>> = {
        editor: () => import("../components/Editor.svelte"),
        reader: () => import("../components/Reader.svelte"),
        discover: () => import("../components/Discover.svelte"),
        chat: () => import("../components/Chat.svelte"),
        translator: () => import("../components/TranslatorLab.svelte"),
        dictionary: () => import("../components/Dictionary.svelte"),
    };

    let loadedComponents: Partial<Record<ContentView, ViewComponent>> = {};
    let loadErrors: Partial<Record<ContentView, string>> = {};
    const loadingViews = new Set<ContentView>();

    async function ensureViewLoaded(view: ContentView, force = false) {
        if ((!force && loadedComponents[view]) || loadingViews.has(view)) return;
        loadingViews.add(view);
        loadErrors = { ...loadErrors, [view]: undefined };
        try {
            const module = await viewLoaders[view]();
            loadedComponents = { ...loadedComponents, [view]: module.default };
        } catch (error) {
            const message = error instanceof Error ? error.message : "Unable to load this workspace.";
            loadErrors = { ...loadErrors, [view]: message };
        } finally {
            loadingViews.delete(view);
        }
    }

    $: if ($currentView !== "home") {
        void ensureViewLoaded($currentView);
    }

    $: completedArticles = $articles.filter((article) => article.status === "done" && article.readProgress >= 100);
    $: activeArticles = $articles.filter((article) => article.status === "done" && article.readProgress > 0 && article.readProgress < 100);
    $: recentArticle = activeArticles[0] ?? $articles.find((article) => article.status === "done") ?? null;
    $: totalWords = $articles.reduce((total, article) => total + (article.sentences ?? []).reduce((count, sentence) => count + (sentence.blocks ?? []).length, 0), 0);
    $: averageProgress = $articles.length > 0 ? Math.round($articles.reduce((total, article) => total + (article.readProgress || 0), 0) / $articles.length) : 0;
    $: hasActivity = $articles.length > 0 || $dictionaryHistory.length > 0 || $translatorSessions.length > 0;

    function openArticle(article: Article) {
        activeArticleId.set(article.id);
        pushView("reader");
    }

    const quickActions = [
        {
            title: "Add a text",
            description: "Paste an article and turn it into a guided reading session.",
            view: "editor" as const,
            icon: BookOpenText,
            accent: "from-violet-500 to-fuchsia-500",
        },
        {
            title: "Practice with AI",
            description: "Start a conversation with corrections and contextual help.",
            view: "chat" as const,
            icon: MessageCircle,
            accent: "from-sky-500 to-indigo-500",
        },
        {
            title: "Translate a phrase",
            description: "Translate quickly, then send the result into the reader.",
            view: "translator" as const,
            icon: Languages,
            accent: "from-emerald-500 to-teal-500",
        },
    ];

    function handleWindowKeydown(event: KeyboardEvent) {
        if (event.key === "Escape" && $isSidebarOpen) {
            isSidebarOpen.set(false);
            return;
        }

        const target = event.target as HTMLElement | null;
        const isTyping = target?.matches("input, textarea, select, [contenteditable='true']");
        if (isTyping || !(event.metaKey || event.ctrlKey)) return;

        if (event.key.toLowerCase() === "n") {
            event.preventDefault();
            pushView("editor");
        } else if (event.key.toLowerCase() === "k") {
            event.preventDefault();
            pushView("dictionary");
        }
    }

</script>

<svelte:window on:keydown={handleWindowKeydown} />

<main class="flex h-screen w-screen overflow-hidden bg-[#fbfaff] text-zinc-900 dark:bg-[#15131a] dark:text-zinc-100 pt-[env(safe-area-inset-top)] transition-colors duration-300">
    <div class="hidden md:block h-full w-[21rem] shrink-0 z-20 border-r border-violet-100/80 bg-white/60 dark:border-white/10 dark:bg-black/10">
        <Sidebar />
    </div>

    {#if $isSidebarOpen}
        <button
            type="button"
            aria-label="Close navigation"
            class="absolute inset-0 z-40 bg-black/50 outline-none md:hidden"
            on:click={() => isSidebarOpen.set(false)}
            transition:fade
        ></button>

        <aside
            aria-label="Main navigation"
            class="absolute inset-y-0 left-0 z-50 w-[min(82vw,20rem)] md:hidden shadow-2xl bg-white dark:bg-zinc-950"
            transition:fly={{
                x: -300,
                duration: 300,
                opacity: 1,
                easing: cubicOut,
            }}
        >
            <Sidebar />
        </aside>
    {/if}

    <div class="flex-1 h-full relative z-0 overflow-hidden bg-transparent" inert={$isSidebarOpen}>
        <div
            class="absolute inset-0 z-0 transition-opacity duration-500 ease-out {$currentView === 'home' ? 'view-active opacity-100' : 'pointer-events-none opacity-0'}"
            inert={$currentView !== 'home'}
            aria-hidden={$currentView !== 'home'}
        >
            <div class="absolute top-0 left-0 w-full h-16 px-5 flex items-center z-10 md:hidden">
                <button
                    class="rounded-2xl border border-violet-100 bg-white/80 p-2.5 text-zinc-600 shadow-sm backdrop-blur hover:bg-white dark:border-white/10 dark:bg-white/5 dark:text-zinc-300 dark:hover:bg-white/10 transition-colors"
                    on:click={() => isSidebarOpen.set(true)}
                    aria-label="Open sidebar"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="4" x2="20" y1="12" y2="12" />
                        <line x1="4" x2="20" y1="6" y2="6" />
                        <line x1="4" x2="20" y1="18" y2="18" />
                    </svg>
                </button>
            </div>

            <div class="dashboard-scroll relative h-full overflow-y-auto px-5 pb-12 pt-20 md:px-10 md:pb-16 md:pt-10 xl:px-14">
                <div class="dashboard-shell mx-auto max-w-6xl">
                    <div class="mb-8 flex items-center justify-between gap-4 md:mb-10">
                        <div>
                            <p class="eyebrow">PERSONAL LANGUAGE STUDIO</p>
                            <p class="mt-2 text-sm text-[#766f80] dark:text-zinc-400">A calmer way to turn curiosity into fluency.</p>
                        </div>
                        <button type="button" on:click={() => pushView("editor")} class="new-reading-button hidden items-center gap-2 rounded-2xl bg-violet-600 px-4 py-2.5 text-sm font-semibold text-white shadow-lg shadow-violet-500/20 transition hover:-translate-y-0.5 hover:bg-violet-700 sm:flex">
                            <Plus size={16} strokeWidth={2.5} aria-hidden="true" />
                            New reading
                        </button>
                    </div>

                    <section class="dashboard-hero relative overflow-hidden rounded-[2rem] border border-violet-200/70 bg-[#211934] px-6 py-7 text-white shadow-[0_28px_80px_-35px_rgba(76,29,149,0.75)] md:px-10 md:py-10">
                        <div class="hero-grid pointer-events-none absolute inset-0 opacity-40"></div>
                        <div class="hero-glow pointer-events-none absolute -right-24 -top-32 h-80 w-80 rounded-full bg-fuchsia-400/30 blur-3xl"></div>
                        <div class="relative z-[1] max-w-2xl">
                            <div class="mb-5 inline-flex items-center gap-2 rounded-full border border-white/15 bg-white/10 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.18em] text-violet-100 backdrop-blur">
                                <Sparkles size={13} aria-hidden="true" />
                                {hasActivity ? "Keep your momentum" : "Your language journey starts here"}
                            </div>
                            <h1 class="dashboard-title text-3xl font-semibold leading-[1.08] tracking-[-0.045em] md:text-6xl">
                                {#if recentArticle}
                                    Pick up where<br class="hidden md:block" /> you left off.
                                {:else}
                                    Make every text<br class="hidden md:block" /> a lesson.
                                {/if}
                            </h1>
                            <p class="mt-5 max-w-xl text-sm leading-6 text-violet-100/75 md:text-base">
                                Read authentic language, notice the words that matter, and build a learning rhythm that feels like your own.
                            </p>
                            <div class="mt-7 flex flex-wrap items-center gap-3">
                                {#if recentArticle}
                                    <button type="button" on:click={() => openArticle(recentArticle)} class="group inline-flex items-center gap-2 rounded-2xl bg-white px-4 py-3 text-sm font-semibold text-[#211934] shadow-xl shadow-black/10 transition hover:-translate-y-0.5 hover:bg-violet-50">
                                        Continue reading
                                        <ArrowRight size={16} class="transition group-hover:translate-x-0.5" aria-hidden="true" />
                                    </button>
                                {:else}
                                    <button type="button" on:click={() => pushView("editor")} class="group inline-flex items-center gap-2 rounded-2xl bg-white px-4 py-3 text-sm font-semibold text-[#211934] shadow-xl shadow-black/10 transition hover:-translate-y-0.5 hover:bg-violet-50">
                                        Add your first text
                                        <ArrowRight size={16} class="transition group-hover:translate-x-0.5" aria-hidden="true" />
                                    </button>
                                {/if}
                                <button type="button" on:click={() => pushView("discover")} class="inline-flex items-center gap-2 rounded-2xl border border-white/15 bg-white/5 px-4 py-3 text-sm font-semibold text-violet-100 transition hover:bg-white/10">
                                    Explore stories
                                </button>
                            </div>
                        </div>
                        <div class="hero-orbit pointer-events-none absolute -bottom-28 -right-16 hidden h-80 w-80 rounded-full border border-white/10 md:block"></div>
                        <div class="hero-orbit hero-orbit-small pointer-events-none absolute bottom-4 right-16 hidden h-32 w-32 rounded-full border border-fuchsia-300/20 md:block"></div>
                    </section>

                    {#if recentArticle}
                        <section class="mt-5 grid gap-5 lg:grid-cols-[minmax(0,1.55fr)_minmax(250px,0.8fr)]">
                            <button type="button" on:click={() => openArticle(recentArticle)} class="continue-card group relative overflow-hidden rounded-[1.65rem] border border-violet-100/90 bg-white/80 p-5 text-left shadow-[0_18px_45px_-30px_rgba(76,29,149,0.45)] backdrop-blur-xl transition hover:-translate-y-0.5 hover:border-violet-200 hover:shadow-[0_24px_55px_-28px_rgba(76,29,149,0.5)] dark:border-white/10 dark:bg-white/[0.06]">
                                <div class="flex items-start justify-between gap-5">
                                    <div>
                                        <div class="mb-4 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.16em] text-violet-500 dark:text-violet-300"><BookMarked size={14} aria-hidden="true" /> Continue learning</div>
                                        <h2 class="max-w-lg text-xl font-semibold tracking-[-0.03em] text-[#211934] dark:text-white md:text-2xl">{recentArticle.title || "Untitled reading"}</h2>
                                        <p class="mt-2 line-clamp-2 max-w-xl text-sm leading-6 text-[#766f80] dark:text-zinc-400">{recentArticle.preview || "Your next reading session is waiting."}</p>
                                    </div>
                                    <span class="hidden h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-violet-50 text-violet-600 transition group-hover:bg-violet-600 group-hover:text-white dark:bg-violet-500/10 dark:text-violet-300 sm:flex"><ChevronRight size={19} aria-hidden="true" /></span>
                                </div>
                                <div class="mt-7 flex items-center justify-between gap-4">
                                    <div class="flex items-center gap-3 text-xs font-medium text-[#766f80] dark:text-zinc-400"><Clock3 size={15} aria-hidden="true" /> {recentArticle.readProgress}% complete</div>
                                    <span class="text-xs font-semibold text-violet-600 dark:text-violet-300">Resume session</span>
                                </div>
                                <div class="mt-3 h-2 overflow-hidden rounded-full bg-violet-100/80 dark:bg-white/10"><div class="h-full rounded-full bg-gradient-to-r from-violet-500 to-fuchsia-400 transition-all" style="width: {Math.max(4, recentArticle.readProgress)}%"></div></div>
                            </button>
                            <div class="stats-card rounded-[1.65rem] border border-violet-100/90 bg-white/70 p-5 shadow-[0_18px_45px_-30px_rgba(76,29,149,0.35)] dark:border-white/10 dark:bg-white/[0.04]">
                                <div class="flex items-center justify-between"><span class="text-[11px] font-bold uppercase tracking-[0.16em] text-[#766f80] dark:text-zinc-500">Your rhythm</span><TrendingUp size={17} class="text-emerald-500" aria-hidden="true" /></div>
                                <div class="mt-5 flex items-end gap-2"><span class="text-4xl font-semibold tracking-[-0.06em] text-[#211934] dark:text-white">{averageProgress}%</span><span class="mb-1 text-xs font-medium text-[#766f80] dark:text-zinc-400">average progress</span></div>
                                <div class="mt-5 grid grid-cols-3 gap-2 border-t border-violet-100/80 pt-4 dark:border-white/10">
                                    <div><div class="text-lg font-semibold text-[#211934] dark:text-white">{$articles.length}</div><div class="mt-1 text-[10px] uppercase tracking-wider text-[#766f80]">texts</div></div>
                                    <div><div class="text-lg font-semibold text-[#211934] dark:text-white">{completedArticles.length}</div><div class="mt-1 text-[10px] uppercase tracking-wider text-[#766f80]">finished</div></div>
                                    <div><div class="text-lg font-semibold text-[#211934] dark:text-white">{$dictionaryHistory.length}</div><div class="mt-1 text-[10px] uppercase tracking-wider text-[#766f80]">lookups</div></div>
                                </div>
                            </div>
                        </section>
                    {/if}

                    <section class="mt-8 md:mt-10">
                        <div class="mb-4 flex items-end justify-between"><div><p class="eyebrow">WORKBENCH</p><h2 class="mt-1 text-xl font-semibold tracking-[-0.03em] text-[#211934] dark:text-white">Choose your next move</h2></div><span class="hidden text-xs text-[#766f80] dark:text-zinc-500 sm:block">Shortcuts for your daily practice</span></div>
                        <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
                            {#each quickActions as action, index}
                                <button type="button" on:click={() => pushView(action.view)} class="workbench-card group relative overflow-hidden rounded-[1.45rem] border border-violet-100/90 bg-white/75 p-5 text-left shadow-[0_14px_36px_-26px_rgba(76,29,149,0.35)] backdrop-blur-xl transition duration-300 hover:-translate-y-1 hover:border-violet-200 hover:shadow-[0_22px_46px_-24px_rgba(76,29,149,0.45)] dark:border-white/10 dark:bg-white/[0.05] dark:hover:border-violet-400/30">
                                    <span class="card-number">0{index + 1}</span>
                                    <span class="mb-8 flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-br text-white shadow-lg shadow-violet-500/20 {action.accent}"><svelte:component this={action.icon} size={19} aria-hidden="true" /></span>
                                    <span class="block text-[15px] font-semibold text-[#211934] dark:text-white">{action.title}</span>
                                    <span class="mt-2 block text-xs leading-5 text-[#766f80] dark:text-zinc-400">{action.description}</span>
                                    <span class="mt-5 flex items-center gap-1 text-xs font-semibold text-violet-600 dark:text-violet-300">Open tool <ArrowRight size={13} class="transition group-hover:translate-x-1" aria-hidden="true" /></span>
                                </button>
                            {/each}
                            <button type="button" on:click={() => pushView("dictionary")} class="workbench-card group relative overflow-hidden rounded-[1.45rem] border border-violet-100/90 bg-white/75 p-5 text-left shadow-[0_14px_36px_-26px_rgba(76,29,149,0.35)] backdrop-blur-xl transition duration-300 hover:-translate-y-1 hover:border-violet-200 hover:shadow-[0_22px_46px_-24px_rgba(76,29,149,0.45)] dark:border-white/10 dark:bg-white/[0.05] dark:hover:border-violet-400/30">
                                <span class="card-number">04</span><span class="mb-8 flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-br from-amber-400 to-orange-500 text-white shadow-lg shadow-orange-500/20"><Search size={19} aria-hidden="true" /></span><span class="block text-[15px] font-semibold text-[#211934] dark:text-white">Look up a word</span><span class="mt-2 block text-xs leading-5 text-[#766f80] dark:text-zinc-400">Search a definition, check usage, and keep moving.</span><span class="mt-5 flex items-center gap-1 text-xs font-semibold text-violet-600 dark:text-violet-300">Open tool <ArrowRight size={13} class="transition group-hover:translate-x-1" aria-hidden="true" /></span>
                            </button>
                        </div>
                    </section>

                    {#if !recentArticle}
                        <section class="mt-8 grid gap-4 md:grid-cols-3">
                            <div class="insight-tile md:col-span-2"><Library size={18} aria-hidden="true" /><div><p class="text-sm font-semibold text-[#211934] dark:text-white">Your library is ready for its first story.</p><p class="mt-1 text-xs leading-5 text-[#766f80] dark:text-zinc-400">Start with something you already enjoy—an article, a dialogue, or a page from a book.</p></div></div>
                            <div class="insight-tile"><Clock3 size={18} aria-hidden="true" /><div><p class="text-sm font-semibold text-[#211934] dark:text-white">A small habit wins.</p><p class="mt-1 text-xs leading-5 text-[#766f80] dark:text-zinc-400">Ten focused minutes is enough for today.</p></div></div>
                        </section>
                    {/if}
                </div>
            </div>
        </div>

        {#each Object.entries(loadedComponents) as [view, component] (view)}
            <div
                class="absolute inset-0 z-10 bg-white dark:bg-zinc-950 transition-opacity duration-500 ease-out {$currentView === view ? 'view-active opacity-100' : 'pointer-events-none opacity-0'}"
                inert={$currentView !== view}
                aria-hidden={$currentView !== view}
            >
                <svelte:component this={component} />
            </div>
        {/each}

        {#if $currentView !== "home" && !loadedComponents[$currentView]}
            <div class="absolute inset-0 z-20 flex items-center justify-center bg-white/90 px-6 dark:bg-zinc-950/90" aria-live="polite">
                {#if loadErrors[$currentView]}
                    <div class="max-w-sm rounded-2xl border border-red-200 bg-white p-5 text-center shadow-lg dark:border-red-950 dark:bg-zinc-900">
                        <p class="text-sm font-semibold text-zinc-900 dark:text-zinc-100">Workspace failed to load</p>
                        <p class="mt-2 text-xs leading-5 text-zinc-500 dark:text-zinc-400">{loadErrors[$currentView]}</p>
                        <button
                            type="button"
                            class="mt-4 rounded-full bg-zinc-900 px-4 py-2 text-xs font-semibold text-white transition hover:bg-black dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-white"
                            on:click={() => ensureViewLoaded($currentView as ContentView, true)}
                        >
                            Try again
                        </button>
                    </div>
                {:else}
                    <div class="flex items-center gap-3 rounded-full border border-zinc-200 bg-white px-4 py-2 text-sm text-zinc-500 shadow-sm dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-400">
                        <span class="h-2 w-2 animate-pulse rounded-full bg-violet-500"></span>
                        Loading workspace…
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</main>

<style>
    .dashboard-scroll {
        scrollbar-width: thin;
        scrollbar-color: rgba(139, 92, 246, 0.3) transparent;
    }

    .dashboard-shell {
        animation: dashboardReveal 0.65s cubic-bezier(0.22, 1, 0.36, 1) both;
    }

    .eyebrow {
        color: #8b5cf6;
        font-size: 0.68rem;
        font-weight: 800;
        letter-spacing: 0.18em;
        line-height: 1;
    }

    .dashboard-title {
        text-wrap: balance;
    }

    .hero-grid {
        background-image: linear-gradient(rgba(255,255,255,0.065) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.065) 1px, transparent 1px);
        background-size: 32px 32px;
        mask-image: linear-gradient(to bottom right, black, transparent 72%);
    }

    .hero-orbit {
        animation: orbitDrift 12s ease-in-out infinite;
        box-shadow: 0 0 80px rgba(216, 180, 254, 0.16), inset 0 0 50px rgba(216, 180, 254, 0.08);
    }

    .hero-orbit-small {
        animation-delay: -4s;
        animation-duration: 9s;
    }

    .card-number {
        position: absolute;
        top: 1.15rem;
        right: 1.25rem;
        color: rgba(124, 58, 237, 0.28);
        font-size: 0.68rem;
        font-weight: 800;
        letter-spacing: 0.12em;
    }

    .insight-tile {
        display: flex;
        align-items: flex-start;
        gap: 0.75rem;
        border: 1px solid rgba(139, 92, 246, 0.14);
        border-radius: 1.25rem;
        background: rgba(255, 255, 255, 0.58);
        padding: 1rem 1.1rem;
    }

    .insight-tile :global(svg) {
        flex: 0 0 auto;
        color: #8b5cf6;
        margin-top: 0.1rem;
    }

    @media (prefers-color-scheme: dark) {
        .eyebrow { color: #c4b5fd; }
        .card-number { color: rgba(196, 181, 253, 0.38); }
        .insight-tile {
            border-color: rgba(255, 255, 255, 0.1);
            background: rgba(255, 255, 255, 0.04);
        }
    }

    @keyframes dashboardReveal {
        from { opacity: 0; transform: translateY(14px); }
        to { opacity: 1; transform: translateY(0); }
    }

    @keyframes orbitDrift {
        0%, 100% { transform: translate3d(0, 0, 0) rotate(0deg); }
        50% { transform: translate3d(-12px, -8px, 0) rotate(8deg); }
    }

    .view-active {
        animation: viewFadeIn 0.5s ease-out both;
    }

    @keyframes viewFadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }

</style>
