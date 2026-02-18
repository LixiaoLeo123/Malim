export interface Block {
    text: string;
    pos: string; // word class
    definition: string;
    chinese_root?: string; // optional
    grammar_note?: string; // optional
}

export interface Sentence {
    id: string;
    original: string;
    blocks: Block[];
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
}