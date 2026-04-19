import { writable } from 'svelte/store';
import type { Article, Draft, Settings } from './types';
import { invoke } from '@tauri-apps/api/core'
import { get } from 'svelte/store'
export const currentView = writable<'home' | 'editor' | 'reader' | 'discover' | 'chat'>('home');

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
    language: 'RU'
});



// function generateId() {
//   return Date.now().toString(36) + Math.random().toString(36).substring(2);
// }

const defaultSettings: Settings = {
  aiConfigList: [],
  
  defaultAiConfigId: '',
  mainAiConfigId: '',
  shadowAiConfigId: '',
  embedAiConfigId: '',
  grammarAiConfigId: '',

  concurrency: 1,
    criticalValue: 80,
  showGrammarNotes: true,
  autoSpeak: false,
  preCacheAudio: true,
  ttsConcurrency: 1,
  ttsApi: "edge-tts",
  qwenApiKey: '',
  qwenVoice: '',
  sileroUrl: '',
  ruaccentEnabled: false,
  ruaccentUrl: '',
  syncEnabled: false,
  syncServerUrl: '',
  userId: '',
  memoryModelEnabled: true,
    maxTotalTokens: 4000,
    maxRagTokens: 1000,
    maxRagAppendTokens: 1000,
    maxUserTokens: 500,
  userAvatarUrl: 'https://api.dicebear.com/7.x/avataaars/svg?seed=User' ,
  aiAvatarUrl: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Malim' ,
  proactiveEvent: null,
  aiNickname: 'Malim',
};

export const settings = writable<Settings>({ ...defaultSettings });

export const parsingQueue = writable<string[]>([]); // article id
export const isProcessingQueue = writable(false);

const IPC_INITIAL_DELAY_MS = 1000;
const IPC_RETRY_DELAY_MS = 200;
const IPC_MAX_RETRIES = 60;

function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

async function load() {
    const raw = await invoke<string>('load_data');
    if (!raw || raw.trim().length === 0) {
        throw new Error('load_data returned empty payload');
    }

    const data = JSON.parse(raw);

    if (data.articles) {
        const cleanArticles = data.articles.map((item: Article) => {
            if (item.status === "parsing") {
                return { ...item, status: "error" as const };
            }
            return item;
        });
        articles.set(cleanArticles);
    }
    
    if (data.draft) editorDraft.set(data.draft);
    if (data.settings) settings.set({ ...defaultSettings, ...data.settings });
}

async function loadWithIpcRetry() {
    let lastError: unknown = null;

    for (let attempt = 1; attempt <= IPC_MAX_RETRIES; attempt++) {
        try {
            await load();
            return;
        } catch (e) {
            lastError = e;
            if (attempt < IPC_MAX_RETRIES) {
                if (attempt === 1 || attempt % 10 === 0) {
                    console.warn(`Waiting for Tauri IPC bridge... retry ${attempt}/${IPC_MAX_RETRIES}`);
                }
                await sleep(IPC_RETRY_DELAY_MS);
            }
        }
    }

    console.error('Failed to load app state after IPC retries:', lastError);
}

let saveTimeout: ReturnType<typeof setTimeout> | undefined = undefined;
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
    await sleep(IPC_INITIAL_DELAY_MS);
    await loadWithIpcRetry();
    articles.subscribe(save);
    editorDraft.subscribe(save);
    settings.subscribe(save);
})();