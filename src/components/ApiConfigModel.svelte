<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { settings } from "../lib/stores";

    export let open = false;

    let tempKey = "";
    let tempUrl = "";
    let tempModel = "";
    let tempConcurrency = 1;
    let tempAutoSpeak = true;

    $: if (open) {
        tempKey = $settings.apiKey;
        tempUrl = $settings.apiUrl;
        tempModel = $settings.modelName;
        tempConcurrency = $settings.concurrency;
        tempAutoSpeak = $settings.autoSpeak;
    }

    function handleSave() {
        settings.set({
            apiKey: tempKey.trim(),
            apiUrl: tempUrl.trim(),
            modelName: tempModel.trim(),
            concurrency: tempConcurrency,
            autoSpeak: tempAutoSpeak,
        });
        open = false;
    }
</script>

{#if open}
    <!-- background -->
    <button
        type="button"
        class="fixed inset-0 bg-black/40 z-40 cursor-default"
        transition:fade={{ duration: 200 }}
        on:click={() => (open = false)}
    >
        <span class="sr-only">Close settings dialog</span>
    </button>

    <div
        class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[90vw] max-w-md bg-white rounded-2xl shadow-2xl overflow-hidden dark:bg-zinc-900"
        transition:fly={{ y: 20, duration: 200 }}
    >
        <div class="p-6 space-y-4">
            <h2 class="text-lg font-semibold text-zinc-800 dark:text-zinc-100">
                API Configuration
            </h2>

            <div class="space-y-3">
                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        API Key
                    </span>
                    <input
                        type="password"
                        bind:value={tempKey}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                        placeholder="Paste your API key"
                    />
                </label>

                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        Base URL
                    </span>
                    <input
                        type="text"
                        bind:value={tempUrl}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                        placeholder="https://llmapi.paratera.com/v1"
                    />
                </label>

                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        Model Name
                    </span>
                    <input
                        type="text"
                        bind:value={tempModel}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                        placeholder="GLM-5"
                    />
                </label>

                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        Concurrency
                    </span>
                    <input
                        type="number"
                        bind:value={tempConcurrency}
                        min={1}
                        max={10}
                        step={1}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                        style="appearance: textfield; -moz-appearance: textfield;"
                    />
                    <span class="block text-xs text-zinc-400 mt-1">
                        Recommended 3-5. Higher rates may trigger API limits.
                    </span>
                    <style>
                        input[type="number"]::-webkit-inner-spin-button,
                        input[type="number"]::-webkit-outer-spin-button {
                            -webkit-appearance: none;
                            margin: 0;
                        }
                        input[type="number"]::-moz-inner-spin-button,
                        input[type="number"]::-moz-outer-spin-button {
                            -moz-appearance: none;
                            margin: 0;
                        }
                    </style>
                </label>
            </div>
            <div class="flex items-center justify-between mt-3">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Auto Speak</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempAutoSpeak
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
                    aria-label="Auto Speak"
                    aria-pressed={tempAutoSpeak}
                    on:click={() => (tempAutoSpeak = !tempAutoSpeak)}
                >
                    <span
                        class="inline-block h-5 w-5 transform rounded-full bg-white dark:bg-zinc-900 transition-transform {tempAutoSpeak
                            ? 'translate-x-5'
                            : 'translate-x-1'}"
                    ></span>
                </button>
            </div>

            <div class="flex justify-end pt-2 gap-2">
                <button
                    type="button"
                    class="px-4 py-1.5 text-sm font-medium text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200"
                    on:click={() => (open = false)}
                >
                    Cancel
                </button>
                <button
                    type="button"
                    class="px-4 py-1.5 text-sm font-medium text-white bg-zinc-900 hover:bg-zinc-800 rounded-lg active:scale-95 transition duration-100 ease-out dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
                    on:click={handleSave}
                >
                    Save
                </button>
            </div>
        </div>
    </div>
{/if}
