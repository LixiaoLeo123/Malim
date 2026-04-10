// src/lib/parser.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { get } from "svelte/store";
import {
    articles,
    parsingQueue,
    isProcessingQueue,
    settings,
} from "./stores";
import { notifications } from '../lib/notificationStore';

export async function processQueue() {
    const isProcessing = get(isProcessingQueue);
    const queue = get(parsingQueue);

    if (isProcessing || queue.length === 0) return;

    isProcessingQueue.set(true);

    const currentId = queue[0];
    const currentArticles = get(articles);
    const currentArticle = currentArticles.find((a) => a.id === currentId);

    if (!currentArticle || !currentArticle.draftContent) {
        parsingQueue.update((q) => q.slice(1));
        isProcessingQueue.set(false);
        processQueue();
        return;
    }

    const currentSettings = get(settings);

    const unlisten = await listen<any>("parsing-progress", (event) => {
        const payload = event.payload;
        if (payload.id === currentId) {
            articles.update((items) =>
                items.map((i) => {
                    if (i.id === currentId) {
                        return { ...i, parsingProgress: payload.percent };
                    }
                    return i;
                })
            );
        }
    });

    try {
        function getConfigById(id: string | undefined) {
            if (!id) return undefined;
            return currentSettings.aiConfigList.find((c) => c.id === id);
        }
        const defaultConfig = getConfigById(currentSettings.defaultAiConfigId);
        if (!defaultConfig) {
            notifications.error("Please configure your API first.");
            throw new Error("Default AI configuration not found");
        }
        const result: any = await invoke("parse_text", {
            id: currentId,
            text: currentArticle.draftContent,
            language: currentArticle.language,
            apiKey: defaultConfig.apiKey,
            apiUrl: defaultConfig.apiUrl,
            modelName: defaultConfig.modelName,
            concurrency: currentSettings.concurrency,
            ttsConcurrency: currentSettings.ttsConcurrency,
            preCacheAudio: currentSettings.preCacheAudio,
            ttsApi: currentSettings.ttsApi,
            qwenApiKey: currentSettings.qwenApiKey,
            qwenVoice: currentSettings.qwenVoice,
            sileroTtsUrl: currentSettings.sileroUrl,
            ruaccentEnabled: currentSettings.ruaccentEnabled,
            ruaccentUrl: currentSettings.ruaccentUrl,
            oldSentences: currentArticle.sentences || null,
            showGrammarNotes: currentSettings.showGrammarNotes,
        });

        articles.update((items) =>
            items.map((i) => {
                if (i.id === currentId) {
                    return { ...i, status: "done" as const, sentences: result, parsingProgress: 100 };
                }
                return i;
            })
        );
    } catch (e) {
        console.error("Analysis Failed:", e);
        articles.update((items) =>
            items.map((i) =>
                i.id === currentId
                    ? { ...i, status: "error" as const, parsingProgress: 0 }
                    : i
            )
        );
    } finally {
        unlisten();
        parsingQueue.update((q) => q.slice(1));
        isProcessingQueue.set(false);
        processQueue();
    }
}
