<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { tweened } from "svelte/motion";
    import { invoke } from "@tauri-apps/api/core";
    import { X } from "lucide-svelte";
    import { onDestroy } from "svelte";

    export let open = false;

    const vocabCount = tweened(0, {
        duration: 2000,
        easing: (t) => t,
    });

    let dailyReading = 0;
    let alpha = 0;
    let chartData: number[] = [];
    let chartLoading = true;
    let pollingInterval: ReturnType<typeof setInterval> | null = null;

    let hasInitializedVocab = false;

    $: chartPath = generateChartPath(chartData);

    async function fetchDailyReading() {
        try {
            const count = await invoke<number>("get_daily_reading");
            dailyReading = count;
        } catch (e) {
            console.error("Failed to fetch daily reading:", e);
        }
    }

    async function fetchAlpha() {
        try {
            const val = await invoke<number>("get_alpha");
            alpha = val;
        } catch (e) {
            console.error("Failed to fetch alpha:", e);
        }
    }

    async function updateVocab(allowAnimation: boolean = true) {
        try {
            const val = await invoke<number>("get_vocabulary_expectation");
            const roundedVal = Math.round(val * 1000) / 1000;
            if (allowAnimation) {
                vocabCount.set(roundedVal);
            } else {
                vocabCount.set(roundedVal, { duration: 0 });
            }
        } catch (e) {
            console.error("Failed to fetch vocab:", e);
        }
    }

    async function fetchHistoryData() {
        chartLoading = true;
        try {
            const promises = [];
            const today = new Date();
            for (let i = 0; i < 30; i++) {
                const date = new Date(today);
                date.setDate(date.getDate() - i);
                const dateStr = date.toISOString().split("T")[0];
                promises.push(
                    invoke<number>("get_reading_by_date", {
                        dateStr,
                    }),
                );
            }
            const results = await Promise.all(promises);

            results.reverse();
            chartData = results;
        } catch (e) {
            console.error("Failed to fetch history:", e);
            chartData = [];
        } finally {
            chartLoading = false;
        }
    }

    function generateChartPath(data: number[]) {
        if (!data || data.length === 0) return "";
        const width = 300;
        const height = 100;
        const maxVal = Math.max(...data, 1);

        const points = data.map((val, i) => {
            const x = (i / (data.length - 1)) * width;
            const y = height - (val / maxVal) * height;
            return `${x},${y}`;
        });

        return `M ${points.join(" L ")}`;
    }

    $: if (open) {
        if (!hasInitializedVocab) {
            hasInitializedVocab = true;

            fetchDailyReading();
            fetchAlpha();
            fetchHistoryData();
            updateVocab(false);
            pollingInterval = setInterval(() => {
                updateVocab(true);
            }, 2000);
        }
    } else {
        if (pollingInterval) {
            clearInterval(pollingInterval);
            pollingInterval = null;
        }
        hasInitializedVocab = false;
    }

    onDestroy(() => {
        if (pollingInterval) clearInterval(pollingInterval);
    });
</script>

{#if open}
    <div
        class="fixed inset-0 z-40 bg-black/30 backdrop-blur-sm dark:bg-black/50"
        on:click={() => (open = false)}
        transition:fade={{ duration: 200 }}
        role="presentation"
    ></div>

    <div
        class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[90vw] max-w-md bg-white rounded-2xl shadow-2xl overflow-hidden dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700"
        transition:fly={{ y: 20, duration: 200 }}
        role="dialog"
        aria-modal="true"
    >
        <div
            class="flex justify-between items-center px-5 py-4 border-b border-zinc-100 dark:border-zinc-800"
        >
            <h2 class="text-lg font-bold text-zinc-900 dark:text-zinc-50">
                Statistics
            </h2>
            <button
                on:click={() => (open = false)}
                class="p-1 rounded-full text-zinc-400 hover:text-zinc-600 hover:bg-zinc-100 dark:hover:text-zinc-200 dark:hover:bg-zinc-800 transition-colors"
            >
                <X size={20} />
            </button>
        </div>

        <div class="px-5 py-4 space-y-5">
            <div
                class="bg-zinc-50 dark:bg-zinc-800/50 rounded-xl p-4 text-center border border-zinc-100 dark:border-zinc-700"
            >
                <div
                    class="text-xs font-medium text-zinc-500 dark:text-zinc-400 uppercase tracking-wider mb-1"
                >
                    Estimated Vocabulary
                </div>
                <div
                    class="text-4xl font-bold text-zinc-900 dark:text-white tabular-nums"
                >
                    {$vocabCount.toFixed(3)}
                </div>
            </div>

            <div class="grid grid-cols-2 gap-3">
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-xl p-3 border border-zinc-100 dark:border-zinc-700"
                >
                    <div class="text-xs text-zinc-500 dark:text-zinc-400 mb-1">
                        Words Read Today
                    </div>
                    <div
                        class="text-lg font-bold text-emerald-600 dark:text-emerald-400"
                    >
                        {dailyReading}
                    </div>
                </div>
                <div
                    class="bg-zinc-50 dark:bg-zinc-800/50 rounded-xl p-3 border border-zinc-100 dark:border-zinc-700"
                >
                    <div class="text-xs text-zinc-500 dark:text-zinc-400 mb-1">
                        Learning Rate
                    </div>
                    <div
                        class="text-lg font-bold text-blue-600 dark:text-blue-400"
                    >
                        {alpha.toFixed(3)}
                    </div>
                </div>
            </div>

            <!-- chart -->
            <div>
                <div
                    class="text-sm font-medium text-zinc-600 dark:text-zinc-300 mb-2 px-1"
                >
                    Reading History (Last 30 Days)
                </div>
                <div
                    class="w-full h-24 bg-zinc-50 dark:bg-zinc-800/50 rounded-lg relative overflow-hidden border border-zinc-100 dark:border-zinc-700"
                >
                    {#if chartLoading}
                        <div
                            class="absolute inset-0 flex items-center justify-center text-xs text-zinc-400"
                        >
                            Loading...
                        </div>
                    {:else if chartData.length > 0}
                        <svg
                            viewBox="0 0 300 100"
                            class="w-full h-full"
                            preserveAspectRatio="none"
                        >
                            <defs>
                                <linearGradient
                                    id="chartGradient"
                                    x1="0%"
                                    y1="0%"
                                    x2="0%"
                                    y2="100%"
                                >
                                    <stop
                                        offset="0%"
                                        style="stop-color:#10b981;stop-opacity:0.3"
                                    />
                                    <stop
                                        offset="100%"
                                        style="stop-color:#10b981;stop-opacity:0"
                                    />
                                </linearGradient>
                            </defs>
                            <path
                                d="{chartPath} L 300 100 L 0 100 Z"
                                fill="url(#chartGradient)"
                            />
                            <path
                                d={chartPath}
                                fill="none"
                                stroke="#10b981"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            />
                        </svg>
                    {:else}
                        <div
                            class="absolute inset-0 flex items-center justify-center text-xs text-zinc-400"
                        >
                            No data available
                        </div>
                    {/if}
                </div>
            </div>
        </div>

        <!-- Info -->
        <div
            class="bg-zinc-50 dark:bg-zinc-800/30 px-5 py-3 flex items-center justify-between border-t border-zinc-100 dark:border-zinc-800"
        >
            <div class="flex items-center gap-2">
                <span
                    class="text-2xl font-bold text-zinc-700 dark:text-zinc-200"
                    >말</span
                >
                <div class="flex flex-col">
                    <span
                        class="text-xs font-bold text-zinc-700 dark:text-zinc-200 leading-tight"
                        >Malim</span
                    >
                    <span class="text-[10px] text-zinc-400">v0.4.0</span>
                </div>
            </div>
            <span class="text-[10px] text-zinc-400">by Drantiss</span>
        </div>
    </div>
{/if}
