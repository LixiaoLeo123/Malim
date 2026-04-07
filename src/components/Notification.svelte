<script lang="ts">
    import { fly, fade, scale } from "svelte/transition";
    import { quintOut } from "svelte/easing";
    import { notifications } from "../lib/notificationStore";
    import { X } from "lucide-svelte";

    function getTypeStyles(type: string) {
        switch (type) {
            case 'error': 
                return 'bg-red-600 border-red-500 text-white';
            case 'success': 
                return 'bg-green-600 border-green-500 text-white';
            case 'warning': 
                return 'bg-yellow-500 border-yellow-400 text-black';
            default: 
                return 'bg-zinc-900 border-zinc-700 text-white';
        }
    }
</script>

{#if $notifications.length > 0}
    <div class="fixed top-4 left-1/2 -translate-x-1/2 z-[100] space-y-2 w-[90vw] max-w-sm pointer-events-none" transition:fade={{ duration: 100 }}>
        {#each $notifications as n (n.id)}
            <div 
                class="p-3 rounded-xl shadow-2xl text-sm flex items-center justify-between border pointer-events-auto {getTypeStyles(n.type)}"
                transition:fly={{ y: -20, duration: 300, easing: quintOut }}
                role="alert"
            >
                <span class="font-medium">{n.message}</span>
                <button 
                    on:click={() => notifications.dismiss(n.id)} 
                    class="ml-4 p-1 rounded-full hover:bg-black/10 dark:hover:bg-white/10 transition-colors"
                    aria-label="Close"
                >
                    <X size={16} />
                </button>
            </div>
        {/each}
    </div>
{/if}
