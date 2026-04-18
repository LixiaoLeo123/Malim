<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { invoke } from "@tauri-apps/api/core";
    import { X, Copy, Check } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import { notifications } from "$lib/notificationStore";

    export let open = false;

    let newWordRatio = 50;
    let topic = "";
    let articleLength = 200;
    let generatedPrompt = "";
    let isGenerating = false;
    let copySuccess = false;

    const P_LOW_AVG = 0.175;
    const P_MID_AVG = 0.5;
    const P_HIGH_AVG = 0.825;

    $: totalNewWords = articleLength;

    $: targetAvgP = 1 - newWordRatio / 100;

    $: wordCounts = calculateWordDistribution(targetAvgP, totalNewWords);

    function calculateWordDistribution(targetP: number, total: number) {
        if (total === 0) return { low: 0, mid: 0, high: 0 };

        const pMin = P_LOW_AVG;
        const pMax = P_HIGH_AVG;
        const difficulty = Math.max(
            0,
            Math.min(1, (targetP - pMin) / (pMax - pMin)),
        );

        const lowWeight = Math.pow(1 - difficulty, 2);
        const highWeight = Math.pow(difficulty, 2);
        const midWeight = 4 * difficulty * (1 - difficulty);

        const sum = lowWeight + midWeight + highWeight;

        const low = Math.round((total * lowWeight) / sum);
        const high = Math.round((total * highWeight) / sum);
        const mid = total - low - high;

        return { low, mid, high };
    }

    async function generatePrompt() {
        isGenerating = true;
        generatedPrompt = "";
        invoke<string[]>("run_global_calibration");
        try {
            const {
                low: lowCount,
                mid: middleCount,
                high: highCount,
            } = wordCounts;
            const WORD_RATIO = 5;
            const [lowWords, middleWords, highWords] = await Promise.all([
                lowCount > 0
                    ? invoke<string[]>("get_words_in_p_range", {
                          pMin: 0.0,
                          pMax: 0.35,
                          limit: lowCount * WORD_RATIO,
                      })
                    : Promise.resolve([]),

                middleCount > 0
                    ? invoke<string[]>("get_words_in_p_range", {
                          pMin: 0.35,
                          pMax: 0.65,
                          limit: middleCount * WORD_RATIO,
                      })
                    : Promise.resolve([]),

                highCount > 0
                    ? invoke<string[]>("get_words_in_p_range", {
                          pMin: 0.65,
                          pMax: 1.0,
                          limit: highCount * WORD_RATIO,
                      })
                    : Promise.resolve([]),
            ]);

            let topicPrompt = ".";
            if (topic) {
                topicPrompt = " about: " + topic;
            }

            generatedPrompt = `Write a ${articleLength}-word article in Russian${topicPrompt}

Use the following vocabulary words in the article:

**Challenging words (use ${lowCount}):**
${lowWords.length > 0 ? lowWords.join(", ") : "None available"}

**Words for review (use ${middleCount}):**
${middleWords.length > 0 ? middleWords.join(", ") : "None available"}

**Well-known words (use ${highCount}):**
${highWords.length > 0 ? highWords.join(", ") : "None available"}

Requirements:
1. The article should be natural, engaging, and coherent.
2. Focus especially on "Words for review" - these are important for the learner.
3. Try to use as many words as possible from the lists above.
4. You may introduce other vocabulary words as needed to make the article natural and coherent.`;
        } catch (e) {
            console.error("Failed to generate prompt:", e);
            notifications.error(
                `Failed to generate prompt: ${e instanceof Error ? e.message : String(e)}`,
            );
        } finally {
            isGenerating = false;
        }
    }

    async function copyPrompt() {
        try {
            await navigator.clipboard.writeText(generatedPrompt);
            copySuccess = true;
            setTimeout(() => {
                copySuccess = false;
            }, 2000);
        } catch (e) {
            console.error("Failed to copy:", e);
            notifications.error(
                `Failed to copy to clipboard: ${e instanceof Error ? e.message : String(e)}`,
            );
        }
    }
</script>

{#if open}
    <div
        class="fixed inset-0 z-40 bg-black/30 backdrop-blur-sm dark:bg-black/50"
        on:click={() => (open = false)}
        transition:fade={{ duration: 200 }}
        role="presentation"
    ></div>

    <div
        class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[90vw] max-w-lg bg-white rounded-2xl shadow-2xl overflow-hidden dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700"
        transition:fly={{ y: 20, duration: 200 }}
        role="dialog"
        aria-modal="true"
    >
        <div
            class="flex justify-between items-center px-5 py-4 border-b border-zinc-100 dark:border-zinc-800"
        >
            <h2 class="text-lg font-bold text-zinc-900 dark:text-zinc-50">
                Article Prompt Generator
            </h2>
            <button
                on:click={() => (open = false)}
                class="p-1 rounded-full text-zinc-400 hover:text-zinc-600 hover:bg-zinc-100 dark:hover:text-zinc-200 dark:hover:bg-zinc-800 transition-colors"
            >
                <X size={20} />
            </button>
        </div>

        <div class="px-5 py-4 space-y-4 max-h-[60vh] overflow-y-auto">
            <div>
                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1.5 dark:text-zinc-400"
                    >
                        Article Topic
                    </span>
                    <input
                        type="text"
                        bind:value={topic}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2.5 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white"
                        placeholder="e.g., A day in Moscow, Learning to cook..."
                    />
                </label>
            </div>

            <div>
                <div class="flex justify-between items-center mb-2">
                    <span
                        class="text-xs font-medium text-zinc-500 dark:text-zinc-400"
                    >
                        New Word Ratio (Difficulty)
                    </span>
                    <span
                        class="text-sm font-bold text-zinc-700 dark:text-zinc-300"
                    >
                        {newWordRatio}% ({Math.round(newWordRatio * totalNewWords / 100)} words)
                    </span>
                </div>
                <input
                    type="range"
                    min="1"
                    max="100"
                    bind:value={newWordRatio}
                    class="w-full h-2 bg-zinc-200 rounded-lg appearance-none cursor-pointer dark:bg-zinc-700 accent-blue-500"
                />
                <div
                    class="flex justify-between text-[10px] text-zinc-400 mt-1"
                >
                    <span>1% (Simple)</span>
                    <span>100% (Challenging)</span>
                </div>
            </div>

            <div>
                <div class="flex justify-between items-center mb-2">
                    <span
                        class="text-xs font-medium text-zinc-500 dark:text-zinc-400"
                    >
                        Article Length
                    </span>
                    <span
                        class="text-sm font-bold text-zinc-700 dark:text-zinc-300"
                    >
                        {articleLength} words
                    </span>
                </div>
                <input
                    type="range"
                    min="100"
                    max="500"
                    step="50"
                    bind:value={articleLength}
                    class="w-full h-2 bg-zinc-200 rounded-lg appearance-none cursor-pointer dark:bg-zinc-700 accent-emerald-500"
                />
                <div
                    class="flex justify-between text-[10px] text-zinc-400 mt-1"
                >
                    <span>100 (Short)</span>
                    <span>500 (Long)</span>
                </div>
            </div>

            <div class="grid grid-cols-3 gap-2">
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-lg p-2.5 text-center border border-zinc-100 dark:border-zinc-700"
                >
                    <div class="text-[10px] text-zinc-500 dark:text-zinc-400">
                        Challenging
                    </div>
                    <div
                        class="text-sm font-bold text-purple-600 dark:text-purple-400"
                    >
                        ~{wordCounts.low}
                    </div>
                </div>
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-lg p-2.5 text-center border border-zinc-100 dark:border-zinc-700 ring-2 ring-blue-100 dark:ring-blue-900/50"
                >
                    <div class="text-[10px] text-zinc-500 dark:text-zinc-400">
                        Review Words
                    </div>
                    <div
                        class="text-sm font-bold text-blue-600 dark:text-blue-400"
                    >
                        ~{wordCounts.mid}
                    </div>
                </div>
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-lg p-2.5 text-center border border-zinc-100 dark:border-zinc-700"
                >
                    <div class="text-[10px] text-zinc-500 dark:text-zinc-400">
                        Known Words
                    </div>
                    <div
                        class="text-sm font-bold text-emerald-600 dark:text-emerald-400"
                    >
                        ~{wordCounts.high}
                    </div>
                </div>
            </div>

            <button
                on:click={generatePrompt}
                disabled={isGenerating}
                class="w-full bg-zinc-900 hover:bg-zinc-800 disabled:bg-zinc-400 text-white py-2.5 rounded-xl font-medium transition-colors dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200 dark:disabled:bg-zinc-600"
            >
                {isGenerating ? "Generating..." : "Generate Prompt"}
            </button>

            {#if generatedPrompt}
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-xl p-4 border border-zinc-100 dark:border-zinc-700"
                    transition:slide={{ duration: 200 }}
                >
                    <div class="flex justify-between items-center mb-2">
                        <span
                            class="text-xs font-medium text-zinc-500 dark:text-zinc-400"
                        >
                            Generated Prompt
                        </span>
                        <button
                            on:click={copyPrompt}
                            class="flex items-center gap-1 px-2 py-1 text-xs font-medium bg-white dark:bg-zinc-700 rounded-lg border border-zinc-200 dark:border-zinc-600 hover:bg-zinc-50 dark:hover:bg-zinc-600 transition-colors"
                        >
                            {#if copySuccess}
                                <Check size={14} class="text-emerald-500" />
                                <span class="text-emerald-500">Copied!</span>
                            {:else}
                                <Copy size={14} />
                                <span>Copy</span>
                            {/if}
                        </button>
                    </div>
                    <pre
                        class="text-xs text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap font-mono leading-relaxed max-h-60 overflow-y-auto">{generatedPrompt}</pre>
                </div>
            {/if}
        </div>

        <div
            class="bg-zinc-50 dark:bg-zinc-800/30 px-5 py-3 border-t border-zinc-100 dark:border-zinc-800"
        >
            <p class="text-[10px] text-zinc-400 text-center">
                Distribution calculated to match target retention rate
            </p>
        </div>
    </div>
{/if}

<style>
    input[type="range"]::-webkit-slider-thumb {
        -webkit-appearance: none;
        appearance: none;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: currentColor;
        cursor: pointer;
    }

    input[type="range"]::-moz-range-thumb {
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: currentColor;
        cursor: pointer;
        border: none;
    }
</style>
