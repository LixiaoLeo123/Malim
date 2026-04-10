<script lang="ts">
    import { fly } from "svelte/transition";
    import { flip } from "svelte/animate";
    import { quintOut } from "svelte/easing";
    import { notifications } from "../lib/notificationStore";
    import { X } from "lucide-svelte";

    function getTypeStyles(type: string) {
        switch (type) {
            case 'error': 
                return 'bg-red-600/95 border-red-500/50 text-white';
            case 'success': 
                return 'bg-green-600/95 border-green-500/50 text-white';
            case 'warning': 
                return 'bg-yellow-500/95 border-yellow-400/50 text-black';
            default: 
                return 'bg-zinc-900/95 border-zinc-700/50 text-white dark:bg-white/95 dark:text-zinc-900 dark:border-zinc-200/50';
        }
    }
</script>

<div class="fixed top-4 pt-1 left-1/2 -translate-x-1/2 z-[100] space-y-3 w-[90vw] max-w-sm flex flex-col items-center pointer-events-none">
    {#each $notifications as n (n.id)}
        <div 
            animate:flip={{ duration: 400, easing: quintOut }}
            class="p-3 px-4 min-w-[70%] max-w-full rounded-2xl shadow-[0_8px_30px_rgb(0,0,0,0.12)] text-sm flex items-center justify-between border pointer-events-auto backdrop-blur-sm {getTypeStyles(n.type)}"
            transition:fly={{ y: -15, duration: 500, easing: quintOut }}
            role="alert"
        >
            <span class="font-medium leading-relaxed">{n.message}</span>
            <button 
                on:click={() => notifications.dismiss(n.id)} 
                class="ml-5 p-1 -mr-1 shrink-0 rounded-full hover:bg-black/10 dark:hover:bg-white/10 transition-colors"
                aria-label="Close"
            >
                <X size={16} />
            </button>
        </div>
    {/each}
</div>
