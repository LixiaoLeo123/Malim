import { writable } from 'svelte/store';
import type { Article, Draft, Settings } from './types';
import { invoke } from '@tauri-apps/api/core'
import { get } from 'svelte/store'
export const currentView = writable<'home' | 'editor' | 'reader'>('home');

export const isSidebarOpen = writable<boolean>(false);

export const articles = writable<Article[]>([]);
// export const articles = writable<Article[]>([
//     {
//         id: '1',
//         title: 'Sample Korean Text',
//         preview: 'This is a draft preview...',
//         status: 'done',
//         parsingProgress: 100,
//         sentences: [], 
//         draftContent: '',
//         language: 'KR'
//     }
// ]);

export const activeArticleId = writable<string | null>(null);

export const editorDraft = writable<Draft>({
    title: '',
    content: '',
    language: 'KR'
});

export const settings = writable<Settings>({
    apiKey: '',
    apiUrl: '',
    modelName: '',
    concurrency: 1,
    autoSpeak: false,
    preCacheAudio: true,
    ttsConcurrency: 1
})

export const parsingQueue = writable<string[]>([]); // article id
export const isProcessingQueue = writable(false);

async function load() {
    const raw = await invoke<string>('load_data');
    if (!raw) return;

    const data = JSON.parse(raw);

    if (data.articles) articles.set(data.articles);
    if (data.draft) editorDraft.set(data.draft);
    if (data.settings) settings.set(data.settings);
}

let saveTimeout: ReturnType<typeof setTimeout>;
async function save() {
    clearTimeout(saveTimeout);

    saveTimeout = setTimeout(async () => {
        const snapshot = {
            articles: get(articles),
            draft: get(editorDraft),
            settings: get(settings)
        };

        await invoke('save_data', {
            data: JSON.stringify(snapshot)
        });
    }, 500);
}

(async () => {
    await load();
    articles.subscribe(save);
    editorDraft.subscribe(save);
    settings.subscribe(save);
})();