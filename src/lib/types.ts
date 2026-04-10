export interface Block {
    text: string;
    pos: string; // word class
    definition: string;
    chinese_root?: string;
    grammar_note?: string;
    audio_path?: string | null;
    // Russian-specific fields:
    lemma?: string | null;
    gram_case?: number | null;
    gram_gender?: "m" | "f" | "n" | null;
    gram_number?: "sg" | "pl" | null;
    tense?: string | null;
    aspect?: "pf" | "impf" | null;
}

export interface Sentence {
    id: string;
    original: string;
    blocks: Block[];
    translation: string;
    audio_path?: string | null;
}

export interface Article {
    id: string;
    title: string;
    preview: string;
    status: 'parsing' | 'done' | 'error';
    parsingProgress: number;
    sentences: Sentence[];
    draftContent?: string;
    language: string;
    readProgress: number;
    completedCheckpointsList: number[];
    stared: boolean;
}

export interface Draft {
    title: string;
    content: string;
    language: string;
}

export interface AiConfig {
  id: string;
  name: string;
  apiKey: string;
  apiUrl: string;
  modelName: string;
}

export interface ProactiveEvent {
  time: string;
  message: string;
}


export interface Settings {
  aiConfigList: AiConfig[];
  defaultAiConfigId: string; // Default (Article Parsing)
  mainAiConfigId: string;    // Main Chat AI
  shadowAiConfigId: string;  // Shadow AI (Memory)
  embedAiConfigId: string;   // Embedding Model (RAG)
  grammarAiConfigId: string; // Grammar Correction AI

  concurrency: number;
  showGrammarNotes: boolean;
  autoSpeak: boolean;
  preCacheAudio: boolean;
  ttsConcurrency: number;
  ttsApi: "edge-tts" | "qwen3-tts" | "silero-tts";
  qwenApiKey: string;
  qwenVoice: string;
  sileroUrl: string;
  ruaccentEnabled: boolean;
  ruaccentUrl: string;
  syncEnabled: boolean;
  syncServerUrl: string;
  userId: string; 
  memoryModelEnabled: boolean;
  userAvatarUrl: string;
  aiAvatarUrl: string;
  proactiveEvent: ProactiveEvent | null;
  aiNickname: string;
}