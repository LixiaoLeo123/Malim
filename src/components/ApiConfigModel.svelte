<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { settings } from "../lib/stores";

    import { ChevronDown } from "lucide-svelte";
    import { slide } from "svelte/transition";
    export let open = false;

    let showTtsApiSelector = false;

    let tempKey = "";
    let tempUrl = "";
    let tempModel = "";
    let tempConcurrency = 1;
    let tempAutoSpeak = true;
    let tempPreCacheAudio = true;
    let tempTtsConcurrency = 1;

    type TtsApi = "edge-tts" | "qwen3-tts" | "silero-tts";
    let tempTtsApi: TtsApi = "edge-tts";

    let tempSileroUrl = "http://127.0.0.1:8001";
    let tempQwenApiKey = "";
    let tempQwenVoice = "";

    let tempRuaccentEnabled = false;
    let tempRuaccentUrl = "http://127.0.0.1:8002";

    let tempSyncEnabled = false;
    let tempSyncServerUrl = "";
    let tempUserId = "";
    let tempMemoryModelEnabled = true;

    $: if (open) {
        tempKey = $settings.apiKey;
        tempUrl = $settings.apiUrl;
        tempModel = $settings.modelName;
        tempConcurrency = $settings.concurrency;
        tempAutoSpeak = $settings.autoSpeak;
        tempPreCacheAudio = $settings.preCacheAudio;
        tempTtsConcurrency = $settings.ttsConcurrency;
        tempTtsApi = $settings.ttsApi ?? "edge-tts";
        tempSileroUrl = $settings.sileroUrl ?? "";
        tempQwenApiKey = $settings.qwenApiKey;
        tempQwenVoice = $settings.qwenVoice;
        tempRuaccentEnabled = $settings.ruaccentEnabled ?? false;
        tempRuaccentUrl = $settings.ruaccentUrl ?? "";
        tempSyncEnabled = $settings.syncEnabled ?? false;
        tempSyncServerUrl = $settings.syncServerUrl ?? "";
        tempUserId = $settings.userId ?? "";
        tempMemoryModelEnabled = $settings.memoryModelEnabled ?? true;
    }

    function handleSave() {
        settings.set({
            apiKey: tempKey.trim(),
            apiUrl: tempUrl.trim(),
            modelName: tempModel.trim(),
            concurrency: tempConcurrency,
            autoSpeak: tempAutoSpeak,
            preCacheAudio: tempPreCacheAudio,
            ttsConcurrency: tempTtsConcurrency,
            ttsApi: tempTtsApi,
            sileroUrl: tempSileroUrl.trim(),
            qwenApiKey: tempQwenApiKey.trim(),
            qwenVoice: tempQwenVoice.trim(),
            ruaccentEnabled: tempRuaccentEnabled,
            ruaccentUrl: tempRuaccentUrl.trim(),
            syncEnabled: tempSyncEnabled,
            syncServerUrl: tempSyncServerUrl.trim(),
            userId: tempUserId.trim(),
            memoryModelEnabled: tempMemoryModelEnabled,
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
        <div class="overflow-y-auto px-6 pb-6 space-y-4" style="height: 80vh;">
            <h2
                class="pt-5 text-lg font-semibold text-zinc-800 dark:text-zinc-100"
            >
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
                        placeholder="https://api.example.com"
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
            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />
            <div class="flex items-center justify-between mt-3 mb-2">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Enable Precise RU Accentuation</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempRuaccentEnabled
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
                    aria-label="Enable RUAccent"
                    aria-pressed={tempRuaccentEnabled}
                    on:click={() =>
                        (tempRuaccentEnabled = !tempRuaccentEnabled)}
                >
                    <span
                        class="inline-block h-5 w-5 transform rounded-full bg-white dark:bg-zinc-900 transition-transform {tempRuaccentEnabled
                            ? 'translate-x-5'
                            : 'translate-x-1'}"
                    ></span>
                </button>
            </div>

            {#if tempRuaccentEnabled}
                <div class="mb-4">
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            RUAccent Server URL
                        </span>
                        <input
                            type="text"
                            bind:value={tempRuaccentUrl}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="http://127.0.0.1:8002/"
                        />
                    </label>
                </div>
            {/if}

            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />

            <div class="flex items-center justify-between mt-3 mb-2">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Enable Memory Model</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempMemoryModelEnabled
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
                    aria-label="Enable Memory Model"
                    aria-pressed={tempMemoryModelEnabled}
                    on:click={() => (tempMemoryModelEnabled = !tempMemoryModelEnabled)}
                >
                    <span
                        class="inline-block h-5 w-5 transform rounded-full bg-white dark:bg-zinc-900 transition-transform {tempMemoryModelEnabled
                            ? 'translate-x-5'
                            : 'translate-x-1'}"
                    ></span>
                </button>
            </div>

            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />

            <div class="flex items-center justify-between mt-3 mb-2">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Enable Remote Sync Server</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempSyncEnabled
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
                    aria-label="Enable Remote Sync Server"
                    aria-pressed={tempSyncEnabled}
                    on:click={() => (tempSyncEnabled = !tempSyncEnabled)}
                >
                    <span
                        class="inline-block h-5 w-5 transform rounded-full bg-white dark:bg-zinc-900 transition-transform {tempSyncEnabled
                            ? 'translate-x-5'
                            : 'translate-x-1'}"
                    ></span>
                </button>
            </div>

            {#if tempSyncEnabled}
                <div class="mb-4">
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Sync Server URL
                        </span>
                        <input
                            type="text"
                            bind:value={tempSyncServerUrl}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="http://your-sync-server:3000"
                        />
                        <span class="block text-xs text-zinc-400 mt-1">
                            Address of your Malim sync server (for memory sync).
                        </span>
                    </label>
                </div>
                <div class="mb-4">
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Sync Server User ID
                        </span>
                        <input
                            type="text"
                            bind:value={tempUserId}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="Your User ID"
                        />
                        <span class="block text-xs text-zinc-400 mt-1">
                            User ID for your Malim sync server.
                        </span>
                    </label>
                </div>
            {/if}

            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />
            <!-- Auto Speak -->
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

            <div class="flex items-center justify-between mt-3">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Pre-cache audio during parsing</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempPreCacheAudio
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
                    aria-label="Pre-cache audio during parsing"
                    aria-pressed={tempPreCacheAudio}
                    on:click={() => (tempPreCacheAudio = !tempPreCacheAudio)}
                >
                    <span
                        class="inline-block h-5 w-5 transform rounded-full bg-white dark:bg-zinc-900 transition-transform {tempPreCacheAudio
                            ? 'translate-x-5'
                            : 'translate-x-1'}"
                    ></span>
                </button>
            </div>
            {#if tempPreCacheAudio}
                <!-- TTS Concurrency (enabled only when pre-cache is on) -->
                <label class="block">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        TTS Concurrency
                    </span>
                    <input
                        type="number"
                        bind:value={tempTtsConcurrency}
                        min={1}
                        max={10}
                        step={1}
                        disabled={!tempPreCacheAudio}
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500 disabled:opacity-50 disabled:cursor-not-allowed"
                        style="appearance: textfield; -moz-appearance: textfield;"
                    />
                    <span class="block text-xs text-zinc-400 mt-1">
                        Limits how many TTS requests run in parallel while
                        pre-caching.
                    </span>
                </label>

                <label class="block relative z-20">
                    <span
                        class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                    >
                        TTS API
                    </span>

                    <button
                        type="button"
                        on:click={() =>
                            (showTtsApiSelector = !showTtsApiSelector)}
                        class="w-full flex items-center justify-between px-3 py-2 bg-zinc-100 rounded-lg text-sm font-medium hover:bg-zinc-200 transition text-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
                    >
                        <span>
                            {#if tempTtsApi === "edge-tts"}
                                Edge TTS
                            {:else if tempTtsApi === "qwen3-tts"}
                                Qwen3 TTS
                            {:else if tempTtsApi === "silero-tts"}
                                Silero TTS
                            {/if}
                        </span>
                        <ChevronDown
                            size={16}
                            class="transition-transform duration-200 {showTtsApiSelector
                                ? 'rotate-180'
                                : ''}"
                        />
                    </button>

                    {#if showTtsApiSelector}
                        <div
                            transition:slide={{ duration: 200 }}
                            class="absolute top-full left-0 mt-2 w-full bg-white border border-zinc-200 rounded-xl shadow-xl overflow-hidden z-50 dark:bg-zinc-900 dark:border-zinc-700"
                        >
                            <button
                                type="button"
                                class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                on:click={() => {
                                    tempTtsApi = "edge-tts";
                                    showTtsApiSelector = false;
                                }}
                            >
                                <span>Edge TTS</span>
                            </button>
                            <button
                                type="button"
                                class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                on:click={() => {
                                    tempTtsApi = "qwen3-tts";
                                    showTtsApiSelector = false;
                                }}
                            >
                                <span>Qwen3 TTS</span>
                            </button>
                            <button
                                type="button"
                                class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                on:click={() => {
                                    tempTtsApi = "silero-tts";
                                    showTtsApiSelector = false;
                                }}
                            >
                                <span>Silero TTS</span>
                            </button>
                        </div>
                    {/if}
                </label>

                {#if tempTtsApi === "qwen3-tts"}
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Qwen TTS API Key
                        </span>
                        <input
                            type="password"
                            bind:value={tempQwenApiKey}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="Paste your Qwen TTS API key"
                        />
                    </label>

                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Qwen TTS Voice Instruction
                        </span>
                        <input
                            type="text"
                            bind:value={tempQwenVoice}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="Better use Chinese"
                        />
                    </label>
                {:else if tempTtsApi === "silero-tts"}
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Silero Server URL
                        </span>
                        <input
                            type="text"
                            bind:value={tempSileroUrl}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="http://127.0.0.1:8001"
                        />
                        <span class="block text-xs text-zinc-400 mt-1">
                            Enter the address of your Silero TTS server.
                        </span>
                    </label>
                {/if}
            {/if}
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
