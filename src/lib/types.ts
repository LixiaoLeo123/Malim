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
}

export interface Draft {
    title: string;
    content: string;
    language: string;
}

export interface Settings {
    apiKey: string;
    apiUrl: string;
    modelName: string;
    concurrency: number;
    autoSpeak: boolean;
    preCacheAudio: boolean;
    ttsConcurrency: number;
}