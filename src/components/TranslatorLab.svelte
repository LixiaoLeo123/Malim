<script lang="ts">
	import { onDestroy, onMount, tick } from "svelte";
	import { fade, slide } from "svelte/transition";
	import { listen } from "@tauri-apps/api/event";
	import { invoke } from "@tauri-apps/api/core";
	import { v4 as uuidv4 } from "uuid";
	import {
		ArrowLeft, ArrowLeftRight, Check, ChevronDown, Languages,
		LoaderCircle, Search, Sparkles, Trash2, X
	} from "lucide-svelte";
	import {
		pushView, popView, settings, translatorLabTransfer,
		translatorSessions, dictionarySearchQuery,
		translatorSourceLang, translatorTargetLang, dictionaryLanguage
	} from "../lib/stores";
	import { notifications } from "$lib/notificationStore";
	import type { Block, Sentence, TranslatorSession, DictionaryLanguage } from "../lib/types";
	import { TRANSLATOR_LANGUAGES, languageName } from "../lib/types";
	import WordPopover from "./WordPopover.svelte";
	import { playAudio, stopAudio } from "../lib/audio";

	let sourceText = "";
	let targetText = "";
	let isTranslating = false;
	let parseResultsEl: HTMLElement | null = null;
	let containerEl: HTMLElement | null = null;
	let progressUnlisten: (() => void) | null = null;

	let activeBlock: Block | null = null;
	let activeBlockEl: HTMLElement | null = null;
	let activeSentence: Sentence | null = null;
	let activeJobLanguage: string = "RU";
	let lastClickedBlock: Block | null = null;
	let popoverPos = { top: 0, left: 0, align: "bottom" as "top" | "bottom", arrowLeft: 0 };

	// Custom dropdown state
	let sourceDropdownOpen = false;
	let targetDropdownOpen = false;
	let sourceSearchQuery = "";
	let targetSearchQuery = "";
	let sourceDropdownEl: HTMLElement | null = null;
	let targetDropdownEl: HTMLElement | null = null;

	const CHECKPOINT_SIZE = 5;

	const colorMap: Record<string, string> = {
		noun: "bg-blue-50 text-blue-700 active:bg-blue-100 dark:bg-blue-950/40 dark:text-blue-200 dark:active:bg-blue-900/55",
		pronoun: "bg-indigo-50 text-indigo-700 active:bg-indigo-100 dark:bg-indigo-950/40 dark:text-indigo-200 dark:active:bg-indigo-900/55",
		verb: "bg-red-50 text-red-700 active:bg-red-100 dark:bg-red-950/40 dark:text-red-200 dark:active:bg-red-900/55",
		adjective: "bg-amber-50 text-amber-700 active:bg-amber-100 dark:bg-amber-950/40 dark:text-amber-200 dark:active:bg-amber-900/55",
		adverb: "bg-emerald-50 text-emerald-700 active:bg-emerald-100 dark:bg-emerald-950/40 dark:text-emerald-200 dark:active:bg-emerald-900/55",
		preposition: "bg-gray-100 text-gray-600 active:bg-gray-200 dark:bg-gray-800/60 dark:text-gray-200 dark:active:bg-gray-700/70",
		conjunction: "bg-gray-100 text-gray-600 active:bg-gray-200 dark:bg-gray-800/60 dark:text-gray-200 dark:active:bg-gray-700/70",
		particle: "bg-zinc-100 text-zinc-600 active:bg-zinc-200 dark:bg-zinc-800/60 dark:text-zinc-200 dark:active:bg-zinc-700/70",
		article: "bg-amber-50 text-amber-700 active:bg-amber-100 dark:bg-amber-950/40 dark:text-amber-200 dark:active:bg-amber-900/55",
		interjection: "bg-pink-50 text-pink-700 active:bg-pink-100 dark:bg-pink-950/40 dark:text-pink-200 dark:active:bg-pink-900/55",
		ending: "bg-gray-100 text-gray-600 active:bg-gray-200 dark:bg-gray-800/60 dark:text-gray-200 dark:active:bg-gray-700/70",
		punctuation: "bg-transparent text-zinc-400 cursor-default dark:text-zinc-500",
		unknown: "bg-slate-50 text-slate-500 active:bg-slate-100 dark:bg-slate-800/50 dark:text-slate-200 dark:active:bg-slate-700/65",
	};

	$: sourceLangLabel = $translatorSourceLang === "AUTO" ? "Auto Detect" : languageName($translatorSourceLang);
	$: targetLangLabel = languageName($translatorTargetLang);

	// Filtered language lists for custom dropdowns
	$: filteredSourceLangs = [
		{ code: "AUTO", name: "Auto Detect" },
		...TRANSLATOR_LANGUAGES
	].filter(l => !sourceSearchQuery || l.name.toLowerCase().includes(sourceSearchQuery.toLowerCase()) || l.code.toLowerCase().includes(sourceSearchQuery.toLowerCase()));

	$: filteredTargetLangs = TRANSLATOR_LANGUAGES.filter(l =>
		!targetSearchQuery || l.name.toLowerCase().includes(targetSearchQuery.toLowerCase()) || l.code.toLowerCase().includes(targetSearchQuery.toLowerCase())
	);

	$: parseLang = (() => {
		const tgt = $translatorTargetLang;
		if (tgt === "RU" || tgt === "KR" || tgt === "ES") return tgt;
		return "RU";
	})();

	function getBlockPosClass(block: Block, lang?: string): string {
		if (lang === "RU" && (block.pos === "noun" || block.pos === "pronoun")) {
			if (block.gram_gender === "m") return "bg-violet-50 text-violet-700 active:bg-violet-100 dark:bg-violet-950/40 dark:text-violet-200 dark:active:bg-violet-900/55";
			if (block.gram_gender === "f") return "bg-cyan-50 text-cyan-700 active:bg-cyan-100 dark:bg-cyan-950/40 dark:text-cyan-200 dark:active:bg-cyan-900/55";
			if (block.gram_gender === "n") return "bg-blue-50 text-blue-700 active:bg-blue-100 dark:bg-blue-950/40 dark:text-blue-200 dark:active:bg-blue-900/55";
			return colorMap.noun;
		}
		if (lang === "ES" && (block.pos === "noun" || block.pos === "adjective" || block.pos === "article")) {
			if (block.gram_gender === "m") return "bg-violet-50 text-violet-700 active:bg-violet-100 dark:bg-violet-950/40 dark:text-violet-200 dark:active:bg-violet-900/55";
			if (block.gram_gender === "f") return "bg-cyan-50 text-cyan-700 active:bg-cyan-100 dark:bg-cyan-950/40 dark:text-cyan-200 dark:active:bg-cyan-900/55";
			return colorMap[block.pos] || colorMap.unknown;
		}
		return colorMap[block.pos] || colorMap.unknown;
	}

	function getConfigById(id: string | undefined) {
		if (!id) return undefined;
		return $settings.aiConfigList.find((c) => c.id === id);
	}

	function safeTitle(text: string) {
		const clean = text.replace(/\s+/g, " ").trim();
		if (!clean) return "Untitled analysis";
		return clean.length > 72 ? `${clean.slice(0, 72)}...` : clean;
	}

	// Custom dropdown handlers
	function openSourceDropdown() {
		sourceSearchQuery = "";
		sourceDropdownOpen = true;
	}
	function closeSourceDropdown() {
		sourceDropdownOpen = false;
	}
	function selectSourceLang(code: string) {
		translatorSourceLang.set(code);
		sourceDropdownOpen = false;
		sourceSearchQuery = "";
	}

	function openTargetDropdown() {
		targetSearchQuery = "";
		targetDropdownOpen = true;
	}
	function closeTargetDropdown() {
		targetDropdownOpen = false;
	}
	function selectTargetLang(code: string) {
		translatorTargetLang.set(code);
		targetDropdownOpen = false;
		targetSearchQuery = "";
	}

	// Close dropdowns and popover when clicking outside
	function handleClickOutside(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (sourceDropdownOpen && sourceDropdownEl && !sourceDropdownEl.contains(target)) {
			sourceDropdownOpen = false;
		}
		if (targetDropdownOpen && targetDropdownEl && !targetDropdownEl.contains(target)) {
			targetDropdownOpen = false;
		}
		// Close word popover when clicking outside interactive blocks and popover
		if (activeBlock && !target.closest(".analysis-block") && !target.closest(".reader-popover")) {
			closePopover(true);
		}
	}

	// Swap source and target languages
	function swapLanguages() {
		const src = $translatorSourceLang;
		const tgt = $translatorTargetLang;
		if (src === "AUTO") return;
		translatorSourceLang.set(tgt);
		translatorTargetLang.set(src);
	}

	async function translateText() {
		const source = sourceText.trim();
		if (!source) return;
		isTranslating = true;
		try {
			let translated = await invoke<string>("translate_llm", {
				text: source,
				sourceLang: $translatorSourceLang,
				targetLang: $translatorTargetLang,
			});
			// Auto-add stress marks for Russian when enabled
			if ($translatorTargetLang === "RU" && $settings.ruaccentEnabled && $settings.ruaccentUrl) {
				try {
					translated = await invoke<string>("accentize_text", {
						text: translated,
						ruaccentUrl: $settings.ruaccentUrl,
					});
				} catch (_) {
					// Silently fall back to unaccented text
				}
			}
			targetText = translated;
			notifications.success(`Translation: ${sourceLangLabel} → ${targetLangLabel}`);
		} catch (e) {
			notifications.error(`Translation failed: ${e instanceof Error ? e.message : String(e)}`);
		} finally {
			isTranslating = false;
		}
	}

	async function parseSourceText(source: string, shouldUpdateInput: boolean, jobId?: string) {
		if (!source) return;
		if (shouldUpdateInput) { targetText = source; }
		const defaultConfig = getConfigById($settings.defaultAiConfigId);
		if (!defaultConfig) { notifications.warning("Please configure your API first."); return; }

		const currentJobId = jobId || uuidv4();
		const job: TranslatorSession = {
			id: currentJobId, sourceText: source, status: "parsing", progress: 2,
			expanded: false, sentences: null, createdAt: Date.now(),
			language: parseLang,
		};
		translatorSessions.update((sessions) => {
			const withoutCurrent = sessions.filter((item) => item.id !== currentJobId);
			return [job, ...withoutCurrent].slice(0, 256);
		});
		await tick();
		if (parseResultsEl) parseResultsEl.scrollTop = 0;

		try {
			const result: Sentence[] = await invoke("parse_text", {
				id: currentJobId, text: source, language: parseLang,
				apiKey: defaultConfig.apiKey, apiUrl: defaultConfig.apiUrl,
				modelName: defaultConfig.modelName, concurrency: $settings.concurrency,
				criticalValue: $settings.criticalValue, ttsConcurrency: $settings.ttsConcurrency,
				preCacheAudio: $settings.preCacheAudio, ttsApi: $settings.ttsApi,
				qwenApiKey: $settings.qwenApiKey, qwenVoice: $settings.qwenVoice,
				sileroTtsUrl: $settings.sileroUrl, ruaccentEnabled: $settings.ruaccentEnabled,
				ruaccentUrl: $settings.ruaccentUrl, oldSentences: null,
				showGrammarNotes: $settings.showGrammarNotes,
			});
			translatorSessions.update((sessions) =>
				sessions.map((item) => item.id === currentJobId
					? { ...item, status: "done", progress: 100, sentences: result, expanded: false } : item));
			notifications.success(`Analysis completed: ${safeTitle(source)}`);
		} catch (e) {
			translatorSessions.update((sessions) =>
				sessions.map((item) => item.id === currentJobId ? { ...item, status: "error" } : item));
			notifications.error(`Parsing failed: ${e instanceof Error ? e.message : String(e)}`);
		}
	}

	async function parseTargetText() {
		const text = targetText.trim();
		if (!text) return;
		targetText = "";
		await parseSourceText(text, true);
	}

	function handleJobCardClick(job: TranslatorSession) {
		if (job.status === "done") toggleJobExpanded(job.id);
	}
	function retryJob(job: TranslatorSession) {
		translatorSessions.update((sessions) =>
			sessions.map((item) => item.id === job.id ? { ...item, status: "parsing", progress: 2, expanded: false } : item));
		void parseSourceText(job.sourceText, false, job.id);
	}
	let deleteConfirmId: string | null = null;
	function toggleJobExpanded(jobId: string) {
		translatorSessions.update((sessions) =>
			sessions.map((job) => job.id === jobId ? { ...job, expanded: !job.expanded } : job));
	}
	async function deleteJob(event: MouseEvent, id: string) {
		event.stopPropagation();
		await invoke("delete_article_audio", { id });
		translatorSessions.update((sessions) => sessions.filter((job) => job.id !== id));
		deleteConfirmId = null;
		notifications.success("Analysis session deleted.");
	}

	function renderSections(sentences: Sentence[] | null) {
		if (!sentences || sentences.length === 0) return [];
		const sections: Sentence[][] = [];
		for (let i = 0; i < sentences.length; i += CHECKPOINT_SIZE) {
			sections.push(sentences.slice(i, i + CHECKPOINT_SIZE));
		}
		return sections;
	}

	function calculatePosition() {
		if (!activeBlockEl) return;
		const rect = activeBlockEl.getBoundingClientRect();
		const viewportW = document.documentElement.clientWidth || window.innerWidth;
		const viewportH = document.documentElement.clientHeight || window.innerHeight;
		const containerRect = containerEl ? containerEl.getBoundingClientRect() : { left: 0, top: 0, right: viewportW };
		const popoverWidth = 260, arrowSize = 8;
		const blockCenter = rect.left + rect.width / 2;
		let popoverLeft = blockCenter - popoverWidth / 2;
		if (popoverLeft + popoverWidth > (containerRect.right || viewportW) - 20) { popoverLeft = (containerRect.right || viewportW) - popoverWidth - 20; }
		if (popoverLeft < containerRect.left + 10) { popoverLeft = containerRect.left + 10; }
		let arrowLeft = blockCenter - popoverLeft - arrowSize;
		arrowLeft = Math.max(8, Math.min(popoverWidth - 24, arrowLeft));
		const spaceBelow = viewportH - rect.bottom;
		const showOnTop = spaceBelow < 250;
		popoverPos = {
			left: popoverLeft - containerRect.left,
			top: (showOnTop ? rect.top - 10 : rect.bottom + 10) - containerRect.top,
			align: showOnTop ? "top" : "bottom", arrowLeft,
		};
	}

	function closePopover(stop: boolean = true) {
		activeBlock = null; activeBlockEl = null; activeSentence = null;
		if (stop) stopAudio();
	}

	let pressTimer: number;
	function searchInDictionary(text: string) {
		clearTimeout(pressTimer);
		if (parseLang === "KR") dictionaryLanguage.set("KR");
		else if (parseLang === "ES") dictionaryLanguage.set("ES");
		else dictionaryLanguage.set("RU");
		dictionarySearchQuery.set(text.trim());
		pushView("dictionary");
	}
	function onPointerDown(e: PointerEvent | TouchEvent, text: string) {
		pressTimer = window.setTimeout(() => { searchInDictionary(text); }, 500);
	}
	function onPointerUp() { if (pressTimer) clearTimeout(pressTimer); }

	function handleBlockClick(event: MouseEvent, block: Block, sentence: Sentence, language: string) {
		event.stopPropagation();
		if (block.pos === "unknown" || block.pos === "punctuation" || block.pos === "error") return;
		if (activeBlock === block) { closePopover(true); return; }
		if ($settings.memoryModelEnabled && block.lemma && block.lemma !== lastClickedBlock?.lemma) {
			invoke("record_word_click", { lemma: block.lemma, clicked: true });
			lastClickedBlock = block;
		}
		activeJobLanguage = language;
			activeBlock = block; activeBlockEl = event.currentTarget as HTMLElement; activeSentence = sentence;
		calculatePosition();
		if ($settings.autoSpeak && block.audio_path) playAudio(block.audio_path);
	}

	function handleSentenceClick(sentence: Sentence) {
		if (sentence.audio_path) playAudio(sentence.audio_path);
	}

	function handleBack() { popView(); }

	$: if ($translatorLabTransfer) {
		const transfer = $translatorLabTransfer;
		translatorLabTransfer.set(null);
		if (transfer.mode === "parse") { void parseSourceText(transfer.text, false); }
		else { targetText = transfer.text; }
	}

	onMount(() => {
		void (async () => {
			progressUnlisten = await listen<{ id: string; percent: number }>("parsing-progress", (event) => {
				const payload = event.payload;
				translatorSessions.update((sessions) =>
					sessions.map((job) => job.id === payload.id && job.status === "parsing"
						? { ...job, progress: Math.max(job.progress, payload.percent) } : job));
			});
		})();
	});
	onDestroy(() => { progressUnlisten?.(); stopAudio(); });
</script>

<svelte:window on:click={handleClickOutside} />

<div bind:this={containerEl} class="relative h-full w-full overflow-y-auto overscroll-contain bg-[#f6f7fb] text-zinc-900 dark:bg-[#09090b] dark:text-zinc-100 xl:overflow-hidden">
	<div class="absolute inset-0 pointer-events-none bg-[radial-gradient(circle_at_top_left,rgba(102,8,116,0.12),transparent_36%),radial-gradient(circle_at_bottom_right,rgba(102,8,116,0.1),transparent_30%),linear-gradient(180deg,rgba(255,255,255,0.88),rgba(255,255,255,0.72))] dark:bg-[radial-gradient(circle_at_top_left,rgba(154,46,176,0.12),transparent_34%),radial-gradient(circle_at_bottom_right,rgba(154,46,176,0.1),transparent_28%),linear-gradient(180deg,rgba(9,9,11,0.98),rgba(9,9,11,0.92))]"></div>

	<div class="relative flex min-h-full flex-col xl:h-full">
		<header class="sticky top-0 z-40 border-b border-zinc-100 bg-white/90 backdrop-blur dark:border-zinc-800 dark:bg-zinc-950/90">
			<div class="flex items-center justify-between gap-4 p-4 pb-2 md:px-6 md:py-4">
				<div class="flex min-w-0 items-center gap-2">
					<button on:click={handleBack} class="p-2 -ml-2 hover:bg-zinc-100 active:scale-95 active:bg-zinc-200 rounded-full text-zinc-600 transition duration-100 ease-out dark:text-zinc-300 dark:hover:bg-zinc-800 dark:active:bg-zinc-700" aria-label="Back">
						<ArrowLeft size={24} />
					</button>
					<div class="min-w-0">
						<div class="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-[0.2em] text-zinc-500 dark:text-zinc-400">
							<Languages size={13} />
							<span>Translator Lab</span>
						</div>
						<h1 class="mt-0.5 max-w-[14rem] truncate text-lg font-semibold tracking-tight text-zinc-900 dark:text-white md:max-w-none md:text-xl">translate and analyze</h1>
					</div>
				</div>
			</div>
		</header>

		<div class="flex flex-1 flex-col gap-4 p-3 md:gap-6 md:p-6 xl:grid xl:min-h-0 xl:grid-cols-[minmax(0,1.02fr)_minmax(0,0.98fr)] xl:overflow-hidden">
			<section class="flex flex-col gap-4 xl:grid xl:min-h-0 xl:grid-rows-[minmax(0,1fr)_minmax(0,1fr)]">

				<!-- Source Panel -->
				<div class="flex flex-col overflow-hidden rounded-[20px] border border-zinc-200/70 bg-white shadow-sm dark:border-zinc-800/80 dark:bg-zinc-900 md:rounded-[24px]">
					<div class="flex items-center justify-between border-b border-zinc-100 px-3 py-2.5 dark:border-zinc-800 md:px-5 md:py-4">
						<div class="flex items-center gap-3">
							<div>
								<div class="text-[10px] font-bold uppercase tracking-widest text-zinc-400">Source</div>
								<div class="mt-0.5 text-sm font-semibold text-zinc-900 dark:text-white md:text-base">{sourceLangLabel}</div>
							</div>
							<!-- Custom Source Language Dropdown -->
							<div bind:this={sourceDropdownEl} class="relative">
								<button
									on:click={sourceDropdownOpen ? closeSourceDropdown : openSourceDropdown}
									class="flex items-center gap-1.5 rounded-xl border border-zinc-200 bg-zinc-50 px-3 py-2 text-xs font-medium text-zinc-700 transition hover:border-zinc-300 hover:bg-white active:scale-[0.98] dark:border-zinc-700 dark:bg-zinc-800/80 dark:text-zinc-300 dark:hover:border-zinc-600 dark:hover:bg-zinc-800"
								>
									<span class="max-w-[100px] truncate">
										{$translatorSourceLang === "AUTO" ? "Auto" : languageName($translatorSourceLang)}
									</span>
									<ChevronDown size={12} class="text-zinc-400 transition-transform {sourceDropdownOpen ? 'rotate-180' : ''}" />
								</button>
								{#if sourceDropdownOpen}
									<div class="absolute left-0 top-full z-50 mt-1 w-64 rounded-2xl border border-zinc-200 bg-white shadow-xl shadow-zinc-950/10 dark:border-zinc-700 dark:bg-zinc-900" transition:fade={{ duration: 120 }}>
										<div class="p-2">
											<div class="relative mb-1">
												<Search size={13} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-400" />
												<input
													bind:value={sourceSearchQuery}
													placeholder="Search language..."
													class="w-full rounded-lg border border-zinc-200 bg-zinc-50 py-1.5 pl-8 pr-3 text-[12px] outline-none focus:border-[#660874] focus:ring-2 focus:ring-[#660874]/10 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:focus:border-[#9a2eb0]"
													on:click|stopPropagation
													on:keydown|stopPropagation
												/>
											</div>
											<div class="max-h-52 overflow-y-auto space-y-0.5">
												{#each filteredSourceLangs as lang (lang.code)}
													<button
														on:click={() => selectSourceLang(lang.code)}
														class="flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-[13px] transition hover:bg-[#660874]/5 dark:hover:bg-[#9a2eb0]/10"
													>
														<span class="font-medium {($translatorSourceLang === lang.code) ? 'text-[#660874] dark:text-[#9a2eb0]' : 'text-zinc-700 dark:text-zinc-300'}">{lang.name}</span>
														<span class="text-[10px] text-zinc-400">{lang.code === "AUTO" ? "" : lang.code}</span>
														{#if $translatorSourceLang === lang.code}
															<Check size={14} class="text-[#660874] dark:text-[#9a2eb0]" />
														{/if}
													</button>
												{/each}
											</div>
										</div>
									</div>
								{/if}
							</div>
						</div>
					</div>
					<textarea
						bind:value={sourceText}
						on:keydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); if (!isTranslating && sourceText.trim()) translateText(); } }}
						class="min-h-[120px] flex-1 resize-none bg-transparent px-3 py-3 text-[15px] leading-relaxed outline-none placeholder:text-zinc-300 dark:placeholder:text-zinc-600 md:min-h-[160px] md:px-5 md:py-4 md:text-[16px]"
						placeholder={$translatorSourceLang === "AUTO" ? "Write or paste text in any language..." : `Write or paste ${sourceLangLabel} text here...`}
					></textarea>
					<div class="flex items-center border-t border-zinc-100 p-2 dark:border-zinc-800 md:px-5 md:py-3 box-border">
						<div class="hidden md:flex items-center gap-2">
							<button
								on:click={swapLanguages}
								disabled={$translatorSourceLang === "AUTO"}
								class="inline-flex items-center justify-center gap-1.5 rounded-xl border border-zinc-200 bg-white px-3 py-2 text-[12px] font-semibold text-zinc-600 shadow-sm transition-all active:scale-95 hover:bg-zinc-50 hover:border-zinc-300 disabled:opacity-30 disabled:cursor-not-allowed dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-700 dark:hover:border-zinc-600"
								title="Swap source and target languages"
							>
								<ArrowLeftRight size={15} />
							</button>
						</div>
						<button
							on:click={translateText}
							disabled={isTranslating || !sourceText.trim()}
							class="flex-1 md:flex-none md:ml-auto inline-flex items-center justify-center gap-2 rounded-xl bg-zinc-900 px-4 py-3 text-[13px] font-semibold text-white shadow-sm transition-all active:scale-95 disabled:opacity-50 hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-950 dark:hover:bg-zinc-200 md:px-5 md:py-2.5"
						>
							{#if isTranslating}
								<LoaderCircle size={16} class="animate-spin" />
								<span>Translating...</span>
							{:else}
								<Sparkles size={16} />
								<span>Translate to {targetLangLabel}</span>
							{/if}
						</button>
					</div>
				</div>

				<!-- Target Panel -->
				<div class="flex flex-col overflow-hidden rounded-[20px] border border-zinc-200/70 bg-white shadow-sm dark:border-zinc-800/80 dark:bg-zinc-900 md:rounded-[24px]">
					<div class="flex items-center justify-between border-b border-zinc-100 px-3 py-2.5 dark:border-zinc-800 md:px-5 md:py-4">
						<div class="flex items-center gap-3">
							<div>
								<div class="text-[10px] font-bold uppercase tracking-widest text-zinc-400">Target</div>
								<div class="mt-0.5 text-sm font-semibold text-zinc-900 dark:text-white md:text-base">{targetLangLabel}</div>
							</div>
							<!-- Custom Target Language Dropdown -->
							<div bind:this={targetDropdownEl} class="relative">
								<button
									on:click={targetDropdownOpen ? closeTargetDropdown : openTargetDropdown}
									class="flex items-center gap-1.5 rounded-xl border border-zinc-200 bg-zinc-50 px-3 py-2 text-xs font-medium text-zinc-700 transition hover:border-zinc-300 hover:bg-white active:scale-[0.98] dark:border-zinc-700 dark:bg-zinc-800/80 dark:text-zinc-300 dark:hover:border-zinc-600 dark:hover:bg-zinc-800"
								>
									<span class="max-w-[100px] truncate">{languageName($translatorTargetLang)}</span>
									<ChevronDown size={12} class="text-zinc-400 transition-transform {targetDropdownOpen ? 'rotate-180' : ''}" />
								</button>
								{#if targetDropdownOpen}
									<div class="absolute left-0 top-full z-50 mt-1 w-64 rounded-2xl border border-zinc-200 bg-white shadow-xl shadow-zinc-950/10 dark:border-zinc-700 dark:bg-zinc-900" transition:fade={{ duration: 120 }}>
										<div class="p-2">
											<div class="relative mb-1">
												<Search size={13} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-400" />
												<input
													bind:value={targetSearchQuery}
													placeholder="Search language..."
													class="w-full rounded-lg border border-zinc-200 bg-zinc-50 py-1.5 pl-8 pr-3 text-[12px] outline-none focus:border-[#660874] focus:ring-2 focus:ring-[#660874]/10 dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:focus:border-[#9a2eb0]"
													on:click|stopPropagation
													on:keydown|stopPropagation
												/>
											</div>
											<div class="max-h-52 overflow-y-auto space-y-0.5">
												{#each filteredTargetLangs as lang (lang.code)}
													<button
														on:click={() => selectTargetLang(lang.code)}
														class="flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-[13px] transition hover:bg-[#660874]/5 dark:hover:bg-[#9a2eb0]/10"
													>
														<span class="font-medium {($translatorTargetLang === lang.code) ? 'text-[#660874] dark:text-[#9a2eb0]' : 'text-zinc-700 dark:text-zinc-300'}">{lang.name}</span>
														<span class="text-[10px] text-zinc-400">{lang.code}</span>
														{#if $translatorTargetLang === lang.code}
															<Check size={14} class="text-[#660874] dark:text-[#9a2eb0]" />
														{/if}
													</button>
												{/each}
											</div>
										</div>
									</div>
								{/if}
							</div>
						</div>
						<!-- Mobile swap button -->
						<button
							on:click={swapLanguages}
							disabled={$translatorSourceLang === "AUTO"}
							class="md:hidden inline-flex items-center justify-center rounded-xl border border-zinc-200 bg-white p-2 text-zinc-500 transition active:scale-95 disabled:opacity-30 disabled:cursor-not-allowed dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-400"
							title="Swap languages"
						>
							<ArrowLeftRight size={16} />
						</button>
					</div>
					<textarea
						bind:value={targetText}
						on:keydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); if (targetText.trim()) parseTargetText(); } }}
						class="min-h-[120px] flex-1 resize-none bg-transparent px-3 py-3 text-[15px] leading-relaxed outline-none placeholder:text-zinc-300 dark:placeholder:text-zinc-600 md:min-h-[160px] md:px-5 md:py-4 md:text-[16px]"
						placeholder={`The ${targetLangLabel} translation appears here`}
						on:dblclick={() => { const selection = window.getSelection()?.toString()?.trim(); if (selection) searchInDictionary(selection); }}
					></textarea>
					<div class="flex items-center border-t border-zinc-100 p-2 dark:border-zinc-800 md:justify-end md:px-5 md:py-3 box-border">
						<button
							on:click={parseTargetText}
							disabled={!targetText.trim()}
							class="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-blue-600 px-4 py-3 text-[13px] font-semibold text-white shadow-sm transition-all active:scale-95 disabled:opacity-50 hover:bg-blue-500 md:flex-none md:px-5 md:py-2.5"
						>
							<Languages size={16} />
							<span>Analyze Text</span>
						</button>
					</div>
				</div>
			</section>

			<!-- Analysis Results Panel -->
			<section class="flex flex-col rounded-[20px] border border-zinc-200/70 bg-white shadow-[0_4px_30px_-10px_rgba(15,23,42,0.1)] dark:border-zinc-800/80 dark:bg-zinc-900 md:rounded-[24px] xl:min-h-0 xl:h-full xl:overflow-hidden">
				<div class="flex shrink-0 items-center justify-between border-b border-zinc-100 px-3 py-2.5 dark:border-zinc-800 md:px-5 md:py-4">
					<div>
						<div class="text-[10px] font-bold uppercase tracking-widest text-zinc-400">Analysis queue</div>
						<div class="text-sm font-semibold text-zinc-900 dark:text-white md:text-base">Parsed text sessions</div>
					</div>
					<div class="rounded-full bg-zinc-100 px-3 py-1 text-[11px] font-bold text-zinc-500 dark:bg-zinc-800 dark:text-zinc-300">{$translatorSessions.length}</div>
				</div>

				<div bind:this={parseResultsEl} class="flex-1 space-y-3 p-3 md:space-y-4 md:p-4 xl:h-[calc(100%-73px)] xl:overflow-y-auto">
					{#if $translatorSessions.length === 0}
						<div class="flex min-h-[160px] items-center justify-center text-center">
							<div class="text-sm font-medium text-zinc-400">No analysis yet</div>
						</div>
					{:else}
						{#each $translatorSessions as job (job.id)}
							<div
								role="button" tabindex="0"
								aria-label={job.status === "done" ? "Analysis card" : "Analysis in progress"}
								aria-expanded={job.status === "done" ? job.expanded : undefined}
								class="overflow-hidden rounded-2xl border border-zinc-200/70 bg-white shadow-sm transition-all hover:shadow-md hover:bg-zinc-50 dark:border-zinc-800/80 dark:bg-zinc-900 dark:hover:bg-zinc-800/50 cursor-pointer"
								on:click={() => handleJobCardClick(job)}
								on:keydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleJobCardClick(job); } }}
							>
								<div class="flex w-full items-start justify-between gap-3 px-3 py-2.5 text-left transition {job.status === 'parsing' ? 'opacity-80' : ''}">
									<div class="min-w-0 flex-1">
										<div class="text-[13px] font-medium text-zinc-800 dark:text-zinc-200">{safeTitle(job.sourceText)}</div>
									</div>
									<div class="flex shrink-0 flex-row items-center gap-2 justify-center">
										{#if job.status === "parsing"}
											<span class="flex items-center gap-1 text-[10px] font-bold text-amber-500 dark:text-amber-400">
												<LoaderCircle size={10} class="animate-spin" /> Parsing
											</span>
										{:else if job.status === "error"}
											<div class="flex items-center gap-2 rounded-lg px-2 py-1 transition active:scale-[0.99] hover:bg-red-50 dark:hover:bg-red-500/10 cursor-pointer">
												<span class="text-[10px] font-bold text-red-500">Error</span>
												<button class="text-zinc-400 hover:text-blue-500 transition-colors bg-transparent border-none p-1" on:click|stopPropagation={() => retryJob(job)} title="Retry analysis">
													<LoaderCircle size={13} />
												</button>
												<button class="text-zinc-400 hover:text-red-500 transition-colors bg-transparent border-none p-1" on:click|stopPropagation={(e) => deleteJob(e, job.id)} title="Delete Error Session">
													<Trash2 size={13} />
												</button>
											</div>
										{:else}
											<div class="flex items-center gap-2 rounded-lg px-2 py-1 transition active:scale-[0.99] hover:bg-zinc-50 dark:hover:bg-zinc-800/50 cursor-pointer">
												{#if deleteConfirmId === job.id}
													<button class="text-zinc-400 hover:text-zinc-900 dark:hover:text-white transition-colors bg-transparent border-none p-1" on:click|stopPropagation={() => deleteConfirmId = null}>
														<X size={13} />
													</button>
													<button class="text-red-500 hover:text-red-700 transition-colors bg-transparent border-none p-1" on:click|stopPropagation={(e) => deleteJob(e, job.id)}>
														<div class="text-[11px] font-bold">Del?</div>
													</button>
												{:else}
													<button class="text-zinc-400 hover:text-red-500 transition-colors bg-transparent border-none p-1" on:click|stopPropagation={() => deleteConfirmId = job.id}>
														<Trash2 size={13} />
													</button>
												{/if}
												<ChevronDown size={14} class="text-zinc-400 transition-transform {job.expanded ? 'rotate-180' : ''}" />
											</div>
										{/if}
									</div>
								</div>

								{#if job.status === "parsing"}
									<div class="h-0.5 w-full bg-zinc-100 dark:bg-zinc-800">
										<div class="h-full bg-blue-500 transition-all duration-300" style="width: {Math.max(5, Math.min(100, job.progress))}%"></div>
									</div>
								{/if}

								{#if job.status === "done" && job.expanded && job.sentences}
									<div in:slide={{ duration: 180 }} out:slide={{ duration: 120 }} class="border-t border-zinc-100 bg-zinc-50/50 p-2 dark:border-zinc-800/60 dark:bg-zinc-900/50 md:p-3">
										<div class="space-y-2 md:space-y-3">
											{#each renderSections(job.sentences) as section, sectionIdx}
												<div class="rounded-xl border border-zinc-200/70 bg-white p-2.5 dark:border-zinc-800 dark:bg-zinc-950/80 md:rounded-2xl md:p-3">
													<div class="mb-1.5 text-[9px] font-bold uppercase tracking-wider text-zinc-400 dark:text-zinc-500">Part {sectionIdx + 1}</div>
													<div class="space-y-3 md:space-y-4">
														{#each section as sentence}
															<div class="parsed-sentence rounded-xl px-2.5 py-2 cursor-pointer transition-colors active:bg-zinc-100 hover:bg-zinc-50 dark:active:bg-zinc-800/80 dark:hover:bg-zinc-900/50"
																role="button" tabindex="0" aria-label="Play sentence"
																on:click={(e) => { e.stopPropagation(); handleSentenceClick(sentence); }}
																on:keydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); handleSentenceClick(sentence); } }}
															>
																<div class="flex flex-wrap gap-x-1 gap-y-2 leading-7 text-[15px] font-medium text-zinc-800 dark:text-zinc-100 md:leading-8 md:text-[16px]">
																	{#each sentence.blocks as block}
																		{#if block.pos === "punctuation"}
																			<span class="analysis-block {getBlockPosClass(block, job.language)}">{block.text}</span>
																		{:else}
																			<button class="analysis-block rounded-md px-1 py-0.5 transition-transform duration-75 ease-out active:scale-95 {getBlockPosClass(block, job.language)}"
																				on:click|stopPropagation={(e) => handleBlockClick(e, block, sentence, job.language)}
																				on:contextmenu|preventDefault={(e) => searchInDictionary(block.lemma || block.text)}
																				on:pointerdown={(e) => onPointerDown(e, block.lemma || block.text)}
																				on:pointerup={onPointerUp}
																				on:pointercancel={onPointerUp}
																			>
																				{block.text}
																				{#if (block.pos === "noun" || block.pos === "pronoun") && block.gram_case && job.language === "RU"}
																					<sup class="ml-0.5 text-[10px] font-bold opacity-70">{block.gram_case}</sup>
																				{/if}
																				{#if block.pos === "verb" && block.mood && job.language === "ES"}
																					<sup class="ml-0.5 text-[9px] font-bold opacity-70">{block.mood}</sup>
																				{/if}
																			</button>
																		{/if}
																	{/each}
																</div>
																{#if sentence.translation}
																	<div class="mt-1.5 text-xs leading-5 text-zinc-500 dark:text-zinc-400 md:text-sm md:leading-6">{sentence.translation}</div>
																{/if}
															</div>
														{/each}
													</div>
												</div>
											{/each}
										</div>
									</div>
								{/if}
							</div>
						{/each}
					{/if}
				</div>
			</section>
		</div>
	</div>

	{#if activeBlock}
		<WordPopover block={activeBlock} position={popoverPos} language={activeJobLanguage} />
	{/if}
</div>
