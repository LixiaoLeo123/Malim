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
  // Spanish-specific fields:
  mood?: "ind" | "subj" | "imp" | "cond" | null;
  gram_person?: 1 | 2 | 3 | null;
}

export interface LanguageOption {
  code: string;
  name: string;
}

export const TRANSLATOR_LANGUAGES: LanguageOption[] = [
  { code: "AR", name: "Arabic" },
  { code: "BG", name: "Bulgarian" },
  { code: "ZH", name: "Chinese (Simplified)" },
  { code: "ZH-TR", name: "Chinese (Traditional)" },
  { code: "HR", name: "Croatian" },
  { code: "CS", name: "Czech" },
  { code: "DA", name: "Danish" },
  { code: "NL", name: "Dutch" },
  { code: "EN", name: "English" },
  { code: "ET", name: "Estonian" },
  { code: "FI", name: "Finnish" },
  { code: "FR", name: "French" },
  { code: "DE", name: "German" },
  { code: "EL", name: "Greek" },
  { code: "HE", name: "Hebrew" },
  { code: "HI", name: "Hindi" },
  { code: "HU", name: "Hungarian" },
  { code: "ID", name: "Indonesian" },
  { code: "IT", name: "Italian" },
  { code: "JA", name: "Japanese" },
  { code: "KO", name: "Korean" },
  { code: "LV", name: "Latvian" },
  { code: "LT", name: "Lithuanian" },
  { code: "NO", name: "Norwegian" },
  { code: "FA", name: "Persian" },
  { code: "PL", name: "Polish" },
  { code: "PT", name: "Portuguese" },
  { code: "RO", name: "Romanian" },
  { code: "RU", name: "Russian" },
  { code: "SR", name: "Serbian" },
  { code: "SK", name: "Slovak" },
  { code: "SL", name: "Slovenian" },
  { code: "ES", name: "Spanish" },
  { code: "SV", name: "Swedish" },
  { code: "TH", name: "Thai" },
  { code: "TR", name: "Turkish" },
  { code: "UK", name: "Ukrainian" },
  { code: "VI", name: "Vietnamese" },
];

export type ParsingLanguage = "RU" | "KR" | "ES";
export type DictionaryLanguage = "RU" | "KR" | "ES";

export function languageName(code: string): string {
  const found = TRANSLATOR_LANGUAGES.find((l) => l.code === code);
  return found ? found.name : code;
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
  scrollPosition?: number;
  tags: string[];
}

export interface TranslatorSession {
  id: string;
  sourceText: string;
  status: 'parsing' | 'done' | 'error';
  progress: number;
  expanded: boolean;
  sentences: Sentence[] | null;
  createdAt: number;
  language: string; // "RU" | "KR" | "ES" — the parsing language used
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

export interface DictionaryEntry {
  headword: string;
  lemma: string;
  forms: string[];
  definition_html: string;
  definition_text: string;
  translation: DictionaryTranslationSection | null;
  matched_terms: string[];
}

export interface DictionaryTranslationExample {
  sense: string;
  example: string;
  translation: string;
  info: string;
}

export interface DictionaryTranslationExpression {
  term: string;
  gloss: string;
}

export interface DictionaryTranslationSection {
  intro: string;
  examples: DictionaryTranslationExample[];
  usage_info: string;
  expressions: DictionaryTranslationExpression[];
  notes: string;
}

export interface DictionaryHistoryEntry {
  query: string;
  normalizedQuery: string;
  resultCount: number;
  searchedAt: number;
}

export interface DictionarySearchResponse {
  query: string;
  normalized_query: string;
  results: DictionaryEntry[];
}

export interface Settings {
  aiConfigList: AiConfig[];
  defaultAiConfigId: string; // Default (Article Parsing)
  mainAiConfigId: string; // Main Chat AI
  shadowAiConfigId: string; // Shadow AI (Memory)
  embedAiConfigId: string; // Embedding Model (RAG)
  grammarAiConfigId: string; // Grammar Correction AI
  concurrency: number;
  criticalValue: number;
  showGrammarNotes: boolean;
  autoSpeak: boolean;
  preCacheAudio: boolean;
  preloadDict: boolean;
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
  maxTotalTokens: number;
  maxRagTokens: number;
  maxRagAppendTokens: number;
  maxUserTokens: number;
  userAvatarUrl: string;
  aiAvatarUrl: string;
  proactiveEvent: ProactiveEvent | null;
  aiNickname: string;
}
