<script lang="ts">
    import Sidebar from "../components/Sidebar.svelte";
    import Editor from "../components/Editor.svelte";
    import Reader from "../components/Reader.svelte";
    import Discover from "../components/Discover.svelte";
    import Chat from "../components/Chat.svelte";
    import { currentView, isSidebarOpen } from "../lib/stores";
    import { fade, fly } from "svelte/transition";
    import { cubicOut } from "svelte/easing";
    import { onMount, onDestroy } from "svelte";

    const phrases = [
        { text: "欢迎来到 ", highlight: "Malim", after: "" },
        { text: "Welcome to ", highlight: "Malim", after: "" },
        { text: "", highlight: "말림", after: "에 오신 것을 환영합니다" },
        { text: "", highlight: "Malim", after: "へようこそ" },
        { text: "Добро пожаловать в ", highlight: "Malim", after: "" },
    ];

    let typingTimer: ReturnType<typeof setTimeout> | undefined;
    let anchor: HTMLElement;
    let currentPhraseIndex = 0;
    let currentCharIndex = 0;
    let isDeleting = false;

    function escapeHtml(text: string) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    function type() {
        const phrase = phrases[currentPhraseIndex];
        const fullText = phrase.text + phrase.highlight + phrase.after;
        
        if (!isDeleting) {
            currentCharIndex++;
            if (currentCharIndex > fullText.length) {
                isDeleting = true;
                typingTimer = setTimeout(type, 2500);
                return;
            }
        } else {
            currentCharIndex--;
            if (currentCharIndex < 0) {
                isDeleting = false;
                currentCharIndex = 0;
                currentPhraseIndex = (currentPhraseIndex + 1) % phrases.length;
                typingTimer = setTimeout(type, 500);
                return;
            }
        }

        const textLen = phrase.text.length;
        const hlLen = phrase.highlight.length;
        let html = "";

        if (currentCharIndex <= textLen) {
            html = escapeHtml(phrase.text.substring(0, currentCharIndex));
        } else if (currentCharIndex <= textLen + hlLen) {
            html = `${escapeHtml(phrase.text)}<span class="malim-highlight">${escapeHtml(phrase.highlight.substring(0, currentCharIndex - textLen))}</span>`;
        } else {
            html = `${escapeHtml(phrase.text)}<span class="malim-highlight">${escapeHtml(phrase.highlight)}</span>${escapeHtml(phrase.after.substring(0, currentCharIndex - textLen - hlLen))}`;
        }

        if (anchor) anchor.innerHTML = html;

        const speed = isDeleting ? 35 : 110;
        typingTimer = setTimeout(type, speed);
    }

    onMount(() => {
        anchor = document.getElementById("hero-typed-anchor") as HTMLElement;
        if (anchor) type();
    });

    onDestroy(() => {
        if (typingTimer !== undefined) clearTimeout(typingTimer);
    });
</script>

<main class="flex h-screen w-screen overflow-hidden bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 pt-[env(safe-area-inset-top)]">
    <div class="hidden md:block h-full w-80 shrink-0 z-20 border-r border-zinc-200 dark:border-zinc-800">
        <Sidebar />
    </div>

    {#if $isSidebarOpen}
        <div
            role="button"
            tabindex="0"
            class="absolute inset-0 z-40 bg-black/50 outline-none md:hidden"
            on:click={() => isSidebarOpen.set(false)}
            on:keydown={(e) => {
                if (e.key === "Escape" || e.key === "Enter")
                    isSidebarOpen.set(false);
            }}
            transition:fade
        ></div>

        <div
            class="absolute inset-y-0 left-0 z-50 w-3/4 md:hidden shadow-2xl bg-white dark:bg-zinc-950"
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

    <div class="flex-1 h-full relative z-0 bg-white dark:bg-zinc-950 overflow-hidden">
        <div class="absolute inset-0 z-0 {$currentView !== 'home' ? 'pointer-events-none' : ''}">
            <div class="absolute top-0 left-0 w-full h-14 px-4 flex items-center z-10 md:hidden">
                <button
                    class="p-2 -ml-2 rounded-full hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-600 dark:text-zinc-300 transition-colors"
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

            <div class="flex flex-col items-center justify-center h-full select-none pt-14 pb-24 px-4">
                
                <div class="orb-wrapper mb-10">
                    <svg class="magic-orb" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
                            <feGaussianBlur stdDeviation="6" result="blur" />
                            <feComposite in="SourceGraphic" in2="blur" operator="over" />
                        </filter>
                        
                        <circle cx="50" cy="50" r="16" fill="url(#grad-core)" filter="url(#glow)" class="orb-core" />
                        
                        <g class="orb-ring-1-container">
                            <circle cx="50" cy="50" r="28" stroke="url(#grad-ring1)" stroke-width="2" stroke-linecap="round" stroke-dasharray="50 120" class="orb-ring-1" />
                        </g>
                        <g class="orb-ring-2-container">
                            <circle cx="50" cy="50" r="38" stroke="url(#grad-ring2)" stroke-width="1.5" stroke-linecap="round" stroke-dasharray="80 160" class="orb-ring-2" />
                        </g>
                        
                        <circle cx="20" cy="30" r="1.5" fill="#a78bfa" class="sparkle s-1"/>
                        <circle cx="80" cy="70" r="2" fill="#818cf8" class="sparkle s-2"/>
                        <circle cx="75" cy="25" r="1" fill="#f472b6" class="sparkle s-3"/>

                        <defs>
                            <linearGradient id="grad-core" x1="0%" y1="0%" x2="100%" y2="100%">
                                <stop offset="0%" stop-color="#818cf8" />
                                <stop offset="100%" stop-color="#c084fc" />
                            </linearGradient>
                            <linearGradient id="grad-ring1" x1="100%" y1="0%" x2="0%" y2="100%">
                                <stop offset="0%" stop-color="#60a5fa" />
                                <stop offset="100%" stop-color="#a78bfa" />
                            </linearGradient>
                            <linearGradient id="grad-ring2" x1="0%" y1="100%" x2="100%" y2="0%">
                                <stop offset="0%" stop-color="#f472b6" />
                                <stop offset="100%" stop-color="#818cf8" />
                            </linearGradient>
                        </defs>
                    </svg>
                </div>

                <div class="hero-title text-3xl md:text-5xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100 text-center leading-tight mb-6 min-h-[3.5rem] md:min-h-[4.5rem] flex items-center justify-center">
                    <span id="hero-typed-anchor" class="hero-typed"></span>
                </div>

                <p class="md:hidden text-xs text-zinc-400 dark:text-zinc-500 text-center max-w-[260px]">
                    Tap the menu to pick a text or open chat.
                </p>
                <p class="hidden md:block text-sm text-zinc-400 dark:text-zinc-500 text-center max-w-[400px]">
                    Pick a text to read, chat with AI, and discover what you know.
                </p>
            </div>
        </div>

        <div class="absolute inset-0 transition-transform duration-300 ease-out z-10 bg-white dark:bg-zinc-950 {$currentView === 'editor' ? 'translate-x-0' : 'translate-x-full pointer-events-none shadow-none'}">
            <Editor />
        </div>
        <div class="absolute inset-0 transition-transform duration-300 ease-out z-10 bg-white dark:bg-zinc-950 {$currentView === 'reader' ? 'translate-x-0' : 'translate-x-full pointer-events-none shadow-none'}">
            <Reader />
        </div>
        <div class="absolute inset-0 transition-transform duration-300 ease-out z-10 bg-white dark:bg-zinc-950 {$currentView === 'discover' ? 'translate-x-0' : 'translate-x-full pointer-events-none shadow-none'}">
            <Discover />
        </div>
        <div class="absolute inset-0 transition-transform duration-300 ease-out z-10 bg-white dark:bg-zinc-950 {$currentView === 'chat' ? 'translate-x-0' : 'translate-x-full pointer-events-none shadow-none'}">
            <Chat />
        </div>
    </div>
</main>

<style>
    :global(.malim-highlight) {
        color: #6366f1 !important;
        font-weight: 800;
    }

    :global(.dark) :global(.malim-highlight) {
        color: #818cf8 !important;
    }

    .hero-title {
        letter-spacing: -0.025em;
        animation: heroTitleReveal 0.8s cubic-bezier(0.25, 0.46, 0.45, 0.94) both;
    }

    @keyframes heroTitleReveal {
        from { opacity: 0; transform: translateY(12px); filter: blur(4px); }
        to { opacity: 1; transform: translateY(0); filter: blur(0); }
    }

    .hero-typed {
        display: inline-block; 
        white-space: pre-wrap; 
        vertical-align: middle;
    }

    .hero-typed::after {
        content: "";
        display: inline-block;
        vertical-align: middle;
        width: 3px;
        height: 1.1em;
        background: #6366f1;
        border-radius: 999px;
        margin-left: 4px;
        margin-bottom: 2px;
        animation: heroCursor 0.6s infinite alternate;
    }

    :global(.dark) .hero-typed::after {
        background: #818cf8;
    }

    @keyframes heroCursor {
        from { opacity: 0.8; transform: scaleY(1); }
        to { opacity: 0.1; transform: scaleY(0.6); }
    }

    .orb-wrapper {
        width: 130px;
        height: 130px;
        animation: orbFloat 4s ease-in-out infinite;
    }

    .magic-orb {
        width: 100%;
        height: 100%;
        overflow: visible;
    }

    .orb-core {
        animation: pulseCore 2.5s ease-in-out infinite alternate;
        transform-origin: 50px 50px;
    }

    .orb-ring-1-container {
        transform-origin: 50px 50px;
        animation: spinRight 8s linear infinite;
    }

    .orb-ring-2-container {
        transform-origin: 50px 50px;
        animation: spinLeft 12s linear infinite;
    }

    .sparkle {
        animation: twinkle 2s ease-in-out infinite alternate;
    }
    .s-1 { animation-delay: 0s; }
    .s-2 { animation-delay: 0.7s; }
    .s-3 { animation-delay: 1.3s; }

    @keyframes orbFloat {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-12px); }
    }

    @keyframes pulseCore {
        0% { transform: scale(0.9); opacity: 0.8; }
        100% { transform: scale(1.1); opacity: 1; }
    }

    @keyframes spinRight {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
    }

    @keyframes spinLeft {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(-360deg); }
    }

    @keyframes twinkle {
        0% { opacity: 0.1; transform: scale(0.5); }
        100% { opacity: 1; transform: scale(1.3); }
    }
</style>