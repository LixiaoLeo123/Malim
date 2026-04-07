<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { settings } from "../lib/stores";
    import {
        ChevronDown,
        Trash2,
        Copy,
        Plus,
        ChevronsUpDown,
    } from "lucide-svelte";
    import { slide } from "svelte/transition";

    export let open = false;
    let showTtsApiSelector = false;

    type AiConfig = {
        id: string;
        name: string;
        apiKey: string;
        apiUrl: string;
        modelName: string;
    };

    type AiRole = "default" | "main" | "shadow" | "embed" | "grammar";

    let tempAiConfigList: AiConfig[] = [];

    let expandedConfigId: string | null = null;

    let selectedIds: Record<AiRole, string> = {
        default: "",
        main: "",
        shadow: "",
        embed: "",
        grammar: "",
    };

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

    let openRoleSelector: Partial<Record<AiRole, boolean>> = {};

    $: if (open) {
        tempAiConfigList = $settings.aiConfigList
            ? JSON.parse(JSON.stringify($settings.aiConfigList))
            : [];

        selectedIds = {
            default: $settings.defaultAiConfigId ?? "",
            main: $settings.mainAiConfigId ?? "",
            shadow: $settings.shadowAiConfigId ?? "",
            embed: $settings.embedAiConfigId ?? "",
            grammar: $settings.grammarAiConfigId ?? "",
        };

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

        expandedConfigId = null;
        openRoleSelector = {};
    }

    function generateId() {
        return (
            Date.now().toString(36) + Math.random().toString(36).substring(2)
        );
    }

    function handleAddConfig() {
        const newConfig: AiConfig = {
            id: generateId(),
            name: "New Config",
            apiKey: "",
            apiUrl: "",
            modelName: "",
        };
        tempAiConfigList = [...tempAiConfigList, newConfig];
        expandedConfigId = newConfig.id;
    }

    function handleDeleteConfig(id: string) {
        tempAiConfigList = tempAiConfigList.filter((c) => c.id !== id);
        if (expandedConfigId === id) expandedConfigId = null;

        for (const role in selectedIds) {
            if (selectedIds[role as AiRole] === id) {
                selectedIds[role as AiRole] = "";
            }
        }
    }

    function handleDuplicateConfig(id: string) {
        const original = tempAiConfigList.find((c) => c.id === id);
        if (original) {
            const copy: AiConfig = {
                ...JSON.parse(JSON.stringify(original)),
                id: generateId(),
                name: `${original.name} (Copy)`,
            };
            tempAiConfigList = [...tempAiConfigList, copy];
        }
    }

    function toggleExpand(id: string) {
        expandedConfigId = expandedConfigId === id ? null : id;
    }

    function handleSave() {
        settings.update((currentSettings) => ({
            ...currentSettings,
            aiConfigList: tempAiConfigList,

            defaultAiConfigId: selectedIds.default,
            mainAiConfigId: selectedIds.main,
            shadowAiConfigId: selectedIds.shadow,
            embedAiConfigId: selectedIds.embed,
            grammarAiConfigId: selectedIds.grammar,

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
        }));
        open = false;
    }

    const roles: { key: AiRole; label: string; description: string }[] = [
        {
            key: "default",
            label: "Default (Article Parsing)",
            description: "Used for parsing articles",
        },
        {
            key: "main",
            label: "Main Chat AI",
            description: "Core conversation logic",
        },
        { key: "shadow", label: "Shadow AI", description: "Memory processing" },
        {
            key: "embed",
            label: "Embedding Model",
            description: "RAG and semantic search",
        },
        {
            key: "grammar",
            label: "Grammar Correction",
            description: "Language correction tasks",
        },
    ];
</script>

{#if open}
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

            <div class="space-y-2">
                <div class="flex justify-between items-center">
                    <h3
                        class="text-sm font-medium text-zinc-700 dark:text-zinc-300"
                    >
                        AI Profiles
                    </h3>
                    <button
                        type="button"
                        on:click={handleAddConfig}
                        class="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200"
                    >
                        <Plus size={14} />
                        <span>Add Profile</span>
                    </button>
                </div>

                {#if tempAiConfigList.length === 0}
                    <div
                        class="text-center py-4 text-sm text-zinc-400 dark:text-zinc-500 border border-dashed border-zinc-200 dark:border-zinc-700 rounded-lg"
                    >
                        No profiles added yet.
                    </div>
                {:else}
                    <div class="space-y-2">
                        {#each tempAiConfigList as config (config.id)}
                            <div
                                class="bg-zinc-50 dark:bg-zinc-800 rounded-lg overflow-hidden border border-zinc-200 dark:border-zinc-700"
                            >
                                <div
                                    class="flex items-center justify-between p-2 cursor-pointer hover:bg-zinc-100 dark:hover:bg-zinc-700/50"
                                    on:click={() => toggleExpand(config.id)}
                                >
                                    <div
                                        class="flex items-center gap-2 overflow-hidden"
                                    >
                                        <span
                                            class="font-medium text-sm text-zinc-800 dark:text-zinc-100 truncate"
                                            >{config.name}</span
                                        >
                                        <span
                                            class="text-xs text-zinc-400 truncate"
                                            >{config.modelName ||
                                                "No model set"}</span
                                        >
                                    </div>
                                    <ChevronsUpDown
                                        size={16}
                                        class="text-zinc-400"
                                    />
                                </div>

                                {#if expandedConfigId === config.id}
                                    <div
                                        class="px-3 pb-3 pt-1 space-y-2 border-t border-zinc-200 dark:border-zinc-700"
                                        transition:slide={{ duration: 200 }}
                                    >
                                        <label class="block">
                                            <span
                                                class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                                                >Profile Name</span
                                            >
                                            <input
                                                type="text"
                                                bind:value={config.name}
                                                class="w-full text-sm bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-lg px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:text-white"
                                                placeholder="e.g. DeepSeek Chat"
                                            />
                                        </label>
                                        <label class="block">
                                            <span
                                                class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                                                >API Key</span
                                            >
                                            <input
                                                type="password"
                                                bind:value={config.apiKey}
                                                class="w-full text-sm bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-lg px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:text-white"
                                                placeholder="sk-..."
                                            />
                                        </label>
                                        <label class="block">
                                            <span
                                                class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                                                >Base URL</span
                                            >
                                            <input
                                                type="text"
                                                bind:value={config.apiUrl}
                                                class="w-full text-sm bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-lg px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:text-white"
                                                placeholder="https://api.example.com"
                                            />
                                        </label>
                                        <label class="block">
                                            <span
                                                class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                                                >Model Name</span
                                            >
                                            <input
                                                type="text"
                                                bind:value={config.modelName}
                                                class="w-full text-sm bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-lg px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:text-white"
                                                placeholder="deepseek-chat"
                                            />
                                        </label>

                                        <div
                                            class="flex justify-end gap-2 pt-1"
                                        >
                                            <button
                                                type="button"
                                                on:click={() =>
                                                    handleDuplicateConfig(
                                                        config.id,
                                                    )}
                                                class="p-1 text-zinc-400 hover:text-blue-500"
                                                title="Duplicate"
                                            >
                                                <Copy size={14} />
                                            </button>
                                            <button
                                                type="button"
                                                on:click={() =>
                                                    handleDeleteConfig(
                                                        config.id,
                                                    )}
                                                class="p-1 text-zinc-400 hover:text-red-500"
                                                title="Delete"
                                            >
                                                <Trash2 size={14} />
                                            </button>
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>

            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />

            <div class="space-y-2">
                <h3
                    class="text-sm font-medium text-zinc-700 dark:text-zinc-300"
                >
                    Role Assignment
                </h3>
                {#if tempAiConfigList.length === 0}
                    <p class="text-xs text-zinc-400">
                        Please add at least one AI Profile above.
                    </p>
                {:else}
                    <div class="space-y-2">
                        {#each roles as role}
                            {@const selectedConfig = tempAiConfigList.find(
                                (c) => c.id === selectedIds[role.key],
                            )}
                            <div
                                class="flex items-center justify-between bg-zinc-50 dark:bg-zinc-800 p-2 rounded-lg gap-2"
                            >
                                <div class="flex-1 min-w-0">
                                    <div
                                        class="text-xs font-medium text-zinc-700 dark:text-zinc-200"
                                    >
                                        {role.label}
                                    </div>
                                    <div class="text-xs text-zinc-400 truncate">
                                        {role.description}
                                    </div>
                                </div>

                                <div class="relative w-40 shrink-0">
                                    <button
                                        type="button"
                                        on:click={() => {
                                            openRoleSelector[role.key] =
                                                !openRoleSelector[role.key];
                                        }}
                                        class="w-full flex items-center justify-between px-3 py-1.5 text-xs font-medium bg-zinc-100 dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-lg hover:bg-zinc-200 dark:hover:bg-zinc-800 transition text-zinc-700 dark:text-zinc-200"
                                    >
                                        <span class="truncate">
                                            {selectedConfig
                                                ? selectedConfig.name
                                                : "Select..."}
                                        </span>
                                        <ChevronDown
                                            size={12}
                                            class="transition-transform duration-200 {openRoleSelector[
                                                role.key
                                            ]
                                                ? 'rotate-180'
                                                : ''}"
                                        />
                                    </button>

                                    {#if openRoleSelector[role.key]}
                                        <div
                                            transition:slide={{ duration: 200 }}
                                            class="absolute z-50 mt-1 w-full bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-700 rounded-xl shadow-xl overflow-hidden text-xs"
                                        >
                                            {#each tempAiConfigList as c}
                                                <button
                                                    type="button"
                                                    class="w-full text-left px-3 py-2 hover:bg-zinc-50 dark:hover:bg-zinc-800 text-zinc-700 dark:text-zinc-200 {selectedIds[
                                                        role.key
                                                    ] === c.id
                                                        ? 'bg-zinc-100 dark:bg-zinc-800'
                                                        : ''}"
                                                    on:click={() => {
                                                        selectedIds[role.key] =
                                                            c.id;
                                                        openRoleSelector[
                                                            role.key
                                                        ] = false;
                                                    }}
                                                >
                                                    {c.name}
                                                </button>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}
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
                    on:click={() =>
                        (tempMemoryModelEnabled = !tempMemoryModelEnabled)}
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
                <div class="mb-4 space-y-2">
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
                    </label>
                    <label class="block">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            User ID
                        </span>
                        <input
                            type="text"
                            bind:value={tempUserId}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                            placeholder="Your User ID"
                        />
                    </label>
                </div>
            {/if}

            <hr class="border-zinc-200 dark:border-zinc-700 my-4" />

            <div class="flex items-center justify-between mt-3">
                <span class="text-sm text-zinc-600 dark:text-zinc-300"
                    >Auto Speak</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempAutoSpeak
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
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
                    >Pre-cache audio</span
                >
                <button
                    type="button"
                    class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {tempPreCacheAudio
                        ? 'bg-zinc-900 dark:bg-zinc-100'
                        : 'bg-zinc-300 dark:bg-zinc-700'}"
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
                <label class="block mt-3">
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
                        class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white dark:focus:ring-zinc-500"
                        style="appearance: textfield; -moz-appearance: textfield;"
                    />
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

                <div class="block relative z-20 mt-3">
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
                        <span>{tempTtsApi}</span>
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
                            {#each ["edge-tts", "qwen3-tts", "silero-tts"] as api}
                                <button
                                    type="button"
                                    class="w-full text-left px-4 py-2 hover:bg-zinc-50 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                                    on:click={() => {
                                        tempTtsApi = api as TtsApi;
                                        showTtsApiSelector = false;
                                    }}
                                >
                                    {api}
                                </button>
                            {/each}
                        </div>
                    {/if}
                </div>

                {#if tempTtsApi === "qwen3-tts"}
                    <label class="block mt-3">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Qwen TTS API Key
                        </span>
                        <input
                            type="password"
                            bind:value={tempQwenApiKey}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white"
                            placeholder="Paste your key"
                        />
                    </label>
                    <label class="block mt-3">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Qwen TTS Voice
                        </span>
                        <input
                            type="text"
                            bind:value={tempQwenVoice}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white"
                            placeholder="Voice instruction"
                        />
                    </label>
                {:else if tempTtsApi === "silero-tts"}
                    <label class="block mt-3">
                        <span
                            class="block text-xs font-medium text-zinc-500 mb-1 dark:text-zinc-400"
                        >
                            Silero Server URL
                        </span>
                        <input
                            type="text"
                            bind:value={tempSileroUrl}
                            class="w-full text-sm bg-zinc-50 border border-zinc-200 rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-zinc-500 dark:bg-zinc-800 dark:border-zinc-700 dark:text-white"
                            placeholder="http://127.0.0.1:8001"
                        />
                    </label>
                {/if}
            {/if}
        </div>

        <div
            class="flex justify-end gap-2 px-6 py-3 border-t border-zinc-100 dark:border-zinc-800 bg-white dark:bg-zinc-900"
        >
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
{/if}
