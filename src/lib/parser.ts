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
import { notifications } from './notificationStore';
import type { Article, Sentence } from './types';


export async function processQueue() {
    if (get(isProcessingQueue)) return;
    const queue = get(parsingQueue);
    if (queue.length === 0) return;
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

        const ocrConfig = getConfigById(currentSettings.ocrAiConfigId);
        if (!ocrConfig) {
            notifications.error("Please configure your OCR API first.");
            throw new Error("OCR AI configuration not found");
        }

        const imageInputs: { id: string; dataUrl: string; fileName: string }[] = (currentArticle.imageParticles || [])
            .map(ip => ({ id: ip.attachmentId, dataUrl: ip.dataUrl || '', fileName: ip.fileName || '' }))
            .filter(ip => !!ip.dataUrl);

        const result = await invoke<Sentence[]>("parse_text", {
            id: currentId,
            text: currentArticle.draftContent,
            language: currentArticle.language,
            apiKey: defaultConfig.apiKey,
            apiUrl: defaultConfig.apiUrl,
            modelName: defaultConfig.modelName,
            concurrency: currentSettings.concurrency,
            criticalValue: currentSettings.criticalValue,
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
            images: imageInputs,
            ocrApiKey: ocrConfig.apiKey,
            ocrApiUrl: ocrConfig.apiUrl,
            ocrModelName: ocrConfig.modelName,
        });

        articles.update((items) =>
            items.map((i) => {
                if (i.id === currentId) {
                    return { ...i, status: "done" as const, sentences: result, parsingProgress: 100 };
                }
                return i;
            })
        );
        notifications.success("Parsing completed!");
    } catch (e) {
        console.error("Analysis Failed:", e);
        articles.update((items) =>
            items.map((i) =>
                i.id === currentId
                    ? { ...i, status: "error" as const, parsingProgress: 0 }
                    : i
            )
        );
        notifications.error(`Parsing failed: ${e instanceof Error ? e.message : e}`);
    } finally {
        unlisten();
        parsingQueue.update((q) => q.slice(1));
        isProcessingQueue.set(false);
        processQueue();
    }
}
