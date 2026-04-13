<script lang="ts">
    import { fade } from "svelte/transition";
    import type { Block } from "../lib/types";

    export let block: Block;
    export let position: {
        left: number;
        top: number;
        align: "top" | "bottom";
        arrowLeft: number;
    };
    export let language: string | undefined = undefined;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
    class="fixed z-50 w-[260px] bg-zinc-900/95 backdrop-blur text-white rounded-xl shadow-2xl p-3 transition-all"
    style="left: {position.left}px; top: {position.top}px; transform: translateY({position.align ===
    'top'
        ? '-100%'
        : '0'});"
    transition:fade={{ duration: 150 }}
    on:click|stopPropagation
>
    <div class="flex flex-col space-y-1.5 relative">
        {#if block.chinese_root}
            <div
                class="inline-block self-start bg-yellow-500/20 text-yellow-300 text-[10px] px-1 py-0.5 rounded border border-yellow-500/50 mb-0.5"
            >
                [{block.chinese_root}]
            </div>
        {/if}
        <div class="text-base font-bold text-white pr-5 leading-snug">
            {block.definition}
        </div>
        {#if language === "RU" && block.gram_number === "pl"}
            <div
                class="absolute top-1 right-1 w-4 h-4 rounded-full bg-purple-600 text-white text-[9px] flex items-center justify-center font-bold"
            >
                P
            </div>
        {/if}
        {#if language === "RU" && block.pos === "verb"}
            <div class="flex gap-1.5 mt-1.5 text-[10px]">
                {#if block.tense}
                    <div
                        class="inline-block self-start bg-blue-500/15 text-blue-300 px-1 py-0.5 rounded border border-blue-500/40"
                    >
                        {block.tense}
                    </div>
                {/if}
                {#if block.aspect}
                    <div
                        class={`inline-block self-start px-1 py-0.5 rounded border ${
                            block.aspect === "pf"
                                ? "bg-orange-500/15 text-orange-300 border-orange-500/40"
                                : "bg-cyan-500/15 text-cyan-300 border-cyan-500/40"
                        }`}
                    >
                        {block.aspect === "pf" ? "PF" : "IPF"}
                    </div>
                {/if}
            </div>
        {/if}
        {#if language === "RU" && block.lemma}
            <div class="text-[12px] text-zinc-300 mt-1">
                Lemma:
                <span class="font-semibold">{block.lemma}</span>
            </div>
        {/if}
        {#if block.grammar_note}
            <div class="text-zinc-400 text-[12px] border-t border-zinc-700/80 pt-1.5 mt-1 leading-snug">
                {block.grammar_note}
            </div>
        {/if}
    </div>
    <div
        class="absolute w-3 h-3 bg-zinc-900/95 rotate-45"
        style="left: {position.arrowLeft}px; {position.align === 'top'
            ? 'bottom: -4px;'
            : 'top: -4px;'}"
    ></div>
</div>
