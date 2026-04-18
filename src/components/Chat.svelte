<script lang="ts">
	import { onDestroy, onMount, tick } from "svelte";
	import { fly, fade } from "svelte/transition";
	import { invoke } from "@tauri-apps/api/core";
	import { currentView, settings } from "../lib/stores";
	import { notifications } from "$lib/notificationStore";
	import { playAudio, stopAudio } from "../lib/audio";
	import type { Block, Sentence } from "../lib/types";
	import WordPopover from "./WordPopover.svelte";

	interface GrammarCorrection {
		original: string | null;
		corrected: string | null;
		type: "unchanged" | "modified" | "deleted" | "inserted";
	}

	interface MainAiResponse {
		reply: string;
		proactive_time: string | null;
		proactive_message: string | null;
		user_log_id: number;
		ai_log_id: number;
	}

	interface ChatMessage {
		id: number;
		text: string;
		isMine: boolean;
		timestamp: number;
		status: "sending" | "success" | "error";
		grammarCorrections?: GrammarCorrection[] | null;
		grammarStatus?: "checking" | "success" | "error";
		dbLogId?: number;
		parsedSentences?: Sentence[] | null;
		parseStatus?: "parsing" | "done" | "error" | null;
		detectedLang?: string;
		quoteText?: string | null;
	}

	let messages: ChatMessage[] = [];
	let inputText = "";
	let isTyping = false;
	let isDrawerOpen = false;
	let backgroundUrl = "";
	let translationResult = "";
	let isTranslating = false;
	let pressTimer: number;
	let chatContainer: HTMLElement;
	let isLoadingHistory = false;
	let hasMoreHistory = true;
	let historyLoadedOnce = false;
	let firstLoad = true;
	let containerEl: HTMLElement | null = null;
	let textareaEl: HTMLTextAreaElement | null = null;
	let proactiveTimer: number | null = null;
	let scheduledProactiveKey: string | null = null;

	let activeParsedBlock: Block | null = null;
	let activeParsedBlockEl: HTMLElement | null = null;
	let activeParsedSentence: Sentence | null = null;

	let unconsumedParsedMessages = new Set<number>();
	let fullyConsumedMessageIds = new Set<number>();
	let sessionLemmasByMsgId = new Map<number, Set<string>>();
	let lastClickedBlock: Block | null = null;

	let popoverPos = {
		top: 0,
		left: 0,
		align: "bottom" as "top" | "bottom",
		arrowLeft: 0,
	};

	let contextMenu = {
		visible: false,
		x: 0,
		y: 0,
		msgId: null as number | null,
		position: "top" as "top" | "bottom",
	};

	function parseQuote(raw: string): { quote: string | null; text: string } {
		const m = raw.match(/^\[quote\]([\s\S]*?)\[\/quote\]\n([\s\S]*)$/);
		return m
			? { quote: m[1].trim(), text: m[2] }
			: { quote: null, text: raw };
	}

	let quotingMsg: ChatMessage | null = null;
	function startQuote(msgId: number) {
		closeContextMenu();
		quotingMsg = messages.find((m) => m.id === msgId) || null;
	}
	function cancelQuote() {
		quotingMsg = null;
	}

	function showContextMenu(
		clientX: number,
		msgId: number,
		target: HTMLElement,
	) {
		const rect = target.getBoundingClientRect();
		const containerRect = containerEl
			? containerEl.getBoundingClientRect()
			: { left: 0, top: 0 };
		let pos: "top" | "bottom" = "top";
		let targetY = rect.top - 12;
		if (targetY < 80) {
			pos = "bottom";
			targetY = rect.bottom + 12;
		}
		contextMenu = {
			visible: true,
			x: clientX - containerRect.left,
			y: targetY - containerRect.top,
			msgId,
			position: pos,
		};
	}

	function detectLanguage(text: string): string {
		let total = 0;
		let cyrillic = 0,
			hangul = 0,
			kana = 0,
			cjk = 0;
		for (const char of text) {
			const code = char.codePointAt(0)!;
			if (code >= 0x0400 && code <= 0x04ff) {
				cyrillic++;
				total++;
			} else if (
				(code >= 0xac00 && code <= 0xd7af) ||
				(code >= 0x1100 && code <= 0x11ff)
			) {
				hangul++;
				total++;
			} else if (
				(code >= 0x3040 && code <= 0x309f) ||
				(code >= 0x30a0 && code <= 0x30ff)
			) {
				kana++;
				total++;
			} else if (code >= 0x4e00 && code <= 0x9fff) {
				cjk++;
				total++;
			}
		}
		if (total === 0) return "EN";
		if (cyrillic / total > 0.3) return "RU";
		if (hangul / total > 0.3) return "KR";
		if (kana / total > 0.1 || cjk / total > 0.3) return "JP";
		return "EN";
	}

	function getBlockPosClass(block: Block, lang?: string): string {
		if (
			lang === "RU" &&
			(block.pos === "noun" || block.pos === "pronoun")
		) {
			if (block.gram_gender === "m") return "ru-gender-m";
			if (block.gram_gender === "f") return "ru-gender-f";
			if (block.gram_gender === "n") return "ru-gender-n";
			return "pos-noun";
		}
		const map: Record<string, string> = {
			noun: "pos-noun",
			pronoun: "pos-pronoun",
			verb: "pos-verb",
			adjective: "pos-adjective",
			adverb: "pos-adverb",
			preposition: "pos-func",
			conjunction: "pos-func",
			particle: "pos-func",
			ending: "pos-func",
			punctuation: "pos-punct",
			unknown: "pos-unknown",
		};
		return map[block.pos] || "pos-unknown";
	}

	async function parseMessage(msgId: number) {
		closeContextMenu();
		const mIdx = messages.findIndex((m) => m.id === msgId);
		if (mIdx === -1) return;
		const msg = messages[mIdx];
		const lang = detectLanguage(msg.text);
		msg.detectedLang = lang;
		msg.parseStatus = "parsing";
		// msg.parsedSentences = null;
		messages = [...messages];

		const id = $settings.defaultAiConfigId;
		const config = id
			? $settings.aiConfigList.find((c) => c.id === id)
			: undefined;
		if (!config) {
			notifications.error("Please configure your API first.");
			msg.parseStatus = "error";
			messages = [...messages];
			return;
		}

		try {
			const result: Sentence[] = await invoke("parse_text", {
				id: String(msg.dbLogId || msg.id),
				text: msg.text,
				language: lang,
				apiKey: config.apiKey,
				apiUrl: config.apiUrl,
				modelName: config.modelName,
				concurrency: $settings.concurrency,
				ttsConcurrency: $settings.ttsConcurrency,
				preCacheAudio: $settings.preCacheAudio,
				ttsApi: $settings.ttsApi,
				qwenApiKey: $settings.qwenApiKey,
				qwenVoice: $settings.qwenVoice,
				sileroTtsUrl: $settings.sileroUrl,
				ruaccentEnabled: $settings.ruaccentEnabled,
				ruaccentUrl: $settings.ruaccentUrl,
				oldSentences: msg.parsedSentences || null,
				showGrammarNotes: $settings.showGrammarNotes,
			});

			msg.parsedSentences = result;
			msg.parseStatus = "done";
			
			unconsumedParsedMessages.add(msg.id);

			if (msg.dbLogId) {
				invoke("update_chat_parsed", {
					logId: msg.dbLogId,
					parsedContent: JSON.stringify(result),
				}).catch((e) => {
					console.error("Failed to save parsed:", e);
					notifications.error("Failed to save parsed content:" + (e instanceof Error ? e.message : String(e)));
				});
			}
			messages = [...messages];
		} catch (e) {
			console.error("Parse failed:", e);
			msg.parseStatus = "error";
			messages = [...messages];
			notifications.error("Failed to parse message:" + (e instanceof Error ? e.message : String(e)));
		}
	}

	function handleParsedBlockClick(
		event: MouseEvent,
		block: Block,
		sentence: Sentence,
		msgId: number,
	) {
		event.stopPropagation();
		if (block.pos === "unknown" || block.pos === "punctuation" || block.pos === "error") return;

		// Record the click to backend, like Reader
		if (block.lemma) {
			let msgLemmas = sessionLemmasByMsgId.get(msgId) || new Set();
			if (!msgLemmas.has(block.lemma)) {
				msgLemmas.add(block.lemma);
				sessionLemmasByMsgId.set(msgId, msgLemmas);
				if ($settings.memoryModelEnabled && block.lemma !== lastClickedBlock?.lemma) {
					invoke("record_word_click", {
						lemma: block.lemma,
						clicked: true,
					});
				}
				lastClickedBlock = block;
			}
		}

		if (activeParsedBlock === block) {
			closeParsePopover(true);
			return;
		}
		activeParsedBlock = block;
		activeParsedBlockEl = event.currentTarget as HTMLElement;
		activeParsedSentence = sentence;
		calcPopoverPos();
		if ($settings.autoSpeak && block.audio_path) {
			playAudio(block.audio_path);
		}
	}

	function handleSentenceClick(sentence: Sentence) {
		if (sentence.audio_path) playAudio(sentence.audio_path);
	}

	function calcPopoverPos() {
		if (!activeParsedBlockEl) return;
		const rect = activeParsedBlockEl.getBoundingClientRect();
		const viewportW =
			document.documentElement.clientWidth || window.innerWidth;
		const viewportH =
			document.documentElement.clientHeight || window.innerHeight;
		const containerRect = containerEl
			? containerEl.getBoundingClientRect()
			: { left: 0, top: 0, right: viewportW };
		const popoverWidth = 260;
		const arrowSize = 8;

		const blockCenter = rect.left + rect.width / 2;

		let popoverLeft = blockCenter - popoverWidth / 2;
		if (
			popoverLeft + popoverWidth >
			(containerRect.right || viewportW) - 20
		)
			popoverLeft =
				(containerRect.right || viewportW) - popoverWidth - 20;

		if (popoverLeft < containerRect.left + 10)
			popoverLeft = containerRect.left + 10;

		let arrowLeft = blockCenter - popoverLeft - arrowSize;
		arrowLeft = Math.max(8, Math.min(popoverWidth - 24, arrowLeft));

		const spaceBelow = viewportH - rect.bottom;
		const top = spaceBelow < 250;

		popoverPos = {
			left: popoverLeft - containerRect.left,
			top: (top ? rect.top - 10 : rect.bottom + 10) - containerRect.top,
			align: top ? "top" : "bottom",
			arrowLeft: arrowLeft,
		};
	}

	function closeParsePopover(stop: boolean = true) {
		activeParsedBlock = null;
		activeParsedBlockEl = null;
		activeParsedSentence = null;
		if (stop) stopAudio();
	}

	function formatTime(ts: number): string {
		if (!Number.isFinite(ts)) return "--:--";
		const date = new Date(ts);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();
		const timeStr = date.toLocaleTimeString("en-US", {
			hour: "2-digit",
			minute: "2-digit",
			hour12: false,
		});
		if (isToday) return timeStr;
		const dateStr = `${date.getMonth() + 1}/${date.getDate()}/${date
			.getFullYear()
			.toString()
			.slice(-2)}`;
		return `${dateStr} ${timeStr}`;
	}

	async function scrollToBottom() {
		await tick();
		if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
	}

	async function apiSaveGrammar(logId: number, corrections: any[]) {
		await invoke("save_grammar_corrections", { logId, corrections });
	}

	async function apiTranslate(text: string): Promise<string> {
		return await invoke("translate", { text });
	}

	async function maybeAccentizeRuText(text: string): Promise<string> {
		if (!$settings.ruaccentEnabled) return text;
		const ruaccentUrl = $settings.ruaccentUrl?.trim();
		if (!ruaccentUrl) return text;
		if (!/[\u0400-\u04FFЁё]/.test(text)) return text;

		try {
			return await invoke("accentize_text", {
				text,
				ruaccentUrl,
			});
		} catch (e) {
			console.warn("Accentize translation failed:", e);
			return text;
		}
	}

	function getConfigById(id: string | undefined) {
		if (!id) return undefined;
		return $settings.aiConfigList.find((c) => c.id === id);
	}

	function safeJsonParse<T>(raw: string | null | undefined): T | null {
		if (!raw) return null;
		try {
			return JSON.parse(raw) as T;
		} catch (e) {
			console.warn("Invalid JSON in chat history field:", e);
			return null;
		}
	}

	function safeTimestampToMillis(raw: string): number {
		const parsed = Date.parse(raw);
		return Number.isFinite(parsed) ? parsed : Date.now();
	}

	async function apiCheckGrammar(text: string): Promise<GrammarCorrection[]> {
		const config = getConfigById($settings.grammarAiConfigId);
		if (!config) {
			notifications.warning("Please configure your Grammar AI first.");
			throw new Error("Grammar AI configuration not found.");
		}
		return await invoke("check_grammar", {
			args: {
				apiKey: config.apiKey,
				baseUrl: config.apiUrl,
				modelName: config.modelName,
				text,
			},
		});
	}

	async function apiSendMessage(text: string): Promise<MainAiResponse> {
		const mainConfig = getConfigById($settings.mainAiConfigId);
		const shadowConfig = getConfigById($settings.shadowAiConfigId);
		const embedConfig = getConfigById($settings.embedAiConfigId);
		if (!mainConfig) {
			notifications.warning("Please configure your Main AI first.");
			throw new Error("Main AI configuration not found.");
		}
		if (!shadowConfig) {
			notifications.warning("Please configure your Shadow AI first.");
			throw new Error("Shadow AI configuration not found.");
		}
		if (!embedConfig) {
			notifications.warning("Please configure your Embed AI first.");
			throw new Error("Embed AI configuration not found.");
		}
		return await invoke("send_message", {
			userInput: text,
			mainApiKey: mainConfig.apiKey,
			mainApiUrl: mainConfig.apiUrl,
			mainModelName: mainConfig.modelName,
			shadowApiKey: shadowConfig?.apiKey ?? "",
			shadowApiUrl: shadowConfig?.apiUrl ?? "",
			shadowModelName: shadowConfig?.modelName ?? "",
			embedApiKey: embedConfig?.apiKey ?? "",
			embedApiUrl: embedConfig?.apiUrl ?? "",
			embedModelName: embedConfig?.modelName ?? "",
			maxTotalTokens: Math.max(500, Number($settings.maxTotalTokens || 4000)),
			maxRagTokens: Math.max(100, Number($settings.maxRagTokens || 1000)),
			maxRagAppendTokens: Math.max(
				100,
				Number($settings.maxRagAppendTokens || 1000),
			),
			maxUserTokens: Math.max(50, Number($settings.maxUserTokens || 500)),
		});
	}

	async function loadHistory() {
		if (isLoadingHistory || !hasMoreHistory) return;
		isLoadingHistory = true;
		const beforeId =
			messages.length > 0
				? (messages[0].dbLogId ?? messages[0].id)
				: null;
		try {
			const args =
				beforeId === null ? { limit: 20 } : { beforeId, limit: 20 };
			const res = await invoke<{
				messages: Array<{
					id: number;
					role: string;
					content: string;
					timestamp: string;
					grammar_corrections: string | null;
					parsed_content: string | null;
				}>;
				has_more: boolean;
			}>("get_chat_logs", args);
			historyLoadedOnce = true;
			hasMoreHistory = res.has_more;
			if (res.messages && res.messages.length > 0) {
				const oldScrollHeight = chatContainer
					? chatContainer.scrollHeight
					: 0;
				const formattedLogs = res.messages.map((msg) => {
					const parsedQuote = parseQuote(msg.content);
					const grammarCorrections =
						safeJsonParse<GrammarCorrection[]>(msg.grammar_corrections);
					const parsedSentences =
						safeJsonParse<Sentence[]>(msg.parsed_content);

					return {
						id: msg.id,
						dbLogId: msg.id,
						text: parsedQuote.text,
						isMine: msg.role === "user",
						timestamp: safeTimestampToMillis(msg.timestamp),
						status: "success" as const,
						grammarCorrections: grammarCorrections ?? undefined,
						grammarStatus: grammarCorrections
							? ("success" as const)
							: undefined,
						parsedSentences,
						parseStatus: parsedSentences ? ("done" as const) : null,
						detectedLang: parsedSentences
							? detectLanguage(msg.content)
							: undefined,
						quoteText: parsedQuote.quote,
					};
				});
				messages = [...formattedLogs, ...messages];
				await tick();
				if (chatContainer) {
					if (beforeId === null) {
						scrollToBottom();
					} else {
						chatContainer.style.scrollBehavior = "auto";
						chatContainer.scrollTop =
							chatContainer.scrollHeight - oldScrollHeight;
						chatContainer.style.scrollBehavior = "";
					}
				}
			}
		} catch (e) {
			console.error("Failed to load history:", e);
			notifications.error("Failed to load chat history:" + (e instanceof Error ? e.message : String(e)));
		} finally {
			isLoadingHistory = false;
		}
	}

	async function onChatScroll() {
		if (chatContainer.scrollTop === 0) {
			await loadHistory();
		}
	}

	function handleBack() {
		$currentView = "home";
	}

	async function handleInput(e: Event) {
		const target = e.target as HTMLTextAreaElement;
		target.style.height = "40px";
		target.style.height = Math.min(target.scrollHeight, 150) + "px";
		const match = inputText.match(/([a-zA-Z\s,']+)\?$/);
		if (match) {
			isTranslating = true;
			try {
				const translated = await apiTranslate(match[1].trim());
				translationResult = await maybeAccentizeRuText(translated);
			} catch (e) {
				translationResult = "Translation Error";
			} finally {
				isTranslating = false;
			}
		} else {
			translationResult = "";
		}
	}

	async function sendMessage(
		text = inputText,
		resendId: number | null = null,
	) {
		if (!text.trim() && !resendId) return;

		// Consume all unclicked parsed words in current session
		unconsumedParsedMessages.forEach((pMsgId) => {
			const pMsg = messages.find((m) => m.id === pMsgId);
            if (!pMsg || fullyConsumedMessageIds.has(pMsg.id)) return;

			if (pMsg.parsedSentences) {
				const msgLemmas = sessionLemmasByMsgId.get(pMsgId) || new Set();
				
				pMsg.parsedSentences.forEach((s) => {
					s.blocks.forEach((b) => {
						if (b.pos !== "unknown" && b.pos !== "punctuation" && b.pos !== "error" && b.lemma) {
							if (!msgLemmas.has(b.lemma) && $settings.memoryModelEnabled) {
								msgLemmas.add(b.lemma);
								invoke("record_word_click", {
									lemma: b.lemma,
									clicked: false,
								}).catch((e) =>
									notifications.error(
										`Failed to save vocabulary progress: ${e instanceof Error ? e.message : String(e)}`,
									),
								);
							}
						}
					});
				});
				sessionLemmasByMsgId.set(pMsgId, msgLemmas);
                fullyConsumedMessageIds.add(pMsgId);
			} else if (!pMsg.parsedSentences && pMsg.detectedLang === "RU" && $settings.memoryModelEnabled) {
				// Record unparsed Russian messages directly
				invoke("record_unparsed_text_words", {
					text: pMsg.text
				}).catch((e) =>
					notifications.error(
						`Failed to save vocabulary progress: ${e instanceof Error ? e.message : String(e)}`,
					),
				);
                fullyConsumedMessageIds.add(pMsgId);
			}
		});
		unconsumedParsedMessages.clear();

		let sectionWordCount = 0;
		for (let i = messages.length - 1; i >= 0; i--) {
			const m = messages[i];
			if (m.isMine) break;

			if (m.parsedSentences) {
				m.parsedSentences.forEach((s: any) => {
					if (s.original) {
						const trimmed = s.original.trim();
						if (trimmed.length > 0) {
							sectionWordCount += trimmed.split(/\s+/).length;
						}
					}
				});
			} else if (m.text) {
				const trimmed = m.text.trim();
				if (trimmed.length > 0) {
					sectionWordCount += trimmed.split(/\s+/).length;
				}
			}
		}
		if (sectionWordCount > 0) {
			invoke("update_daily_reading", { count: sectionWordCount }).catch((e) =>
				notifications.error(
					`Failed to update reading progress: ${e instanceof Error ? e.message : String(e)}`,
				),
			);
		}

		let payload: string;
		if (resendId) {
			const originalMsg = messages.find((m) => m.id === resendId);
			if (originalMsg?.quoteText) {
				payload = `[quote]${originalMsg.quoteText.replace(/\[\/?quote\]/g, "")}[/quote]\n${text.trim()}`;
			} else {
				payload = text.trim();
			}
		} else if (quotingMsg) {
			payload = `[quote]${quotingMsg.text.replace(/\[\/?quote\]/g, "")}[/quote]\n${text.trim()}`;
		} else {
			payload = text.trim();
		}

		let msgId = resendId || Date.now();
		if (!resendId) {
			messages = [
				...messages,
				{
					id: msgId,
					text: text,
					isMine: true,
					timestamp: Date.now(),
					status: "sending",
					grammarStatus: "checking",
					quoteText: quotingMsg ? quotingMsg.text : null,
				},
			];
			inputText = "";
			if (textareaEl) textareaEl.style.height = "40px";
			translationResult = "";
			quotingMsg = null;
			scrollToBottom();
		} else {
			let mIdx = messages.findIndex((m) => m.id === msgId);
			if (mIdx !== -1) messages[mIdx].status = "sending";
			messages = [...messages];
		}

		isTyping = true;
		apiSendMessage(payload)
			.then((aiRes) => {
				isTyping = false;
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx === -1) return;
				messages[mIdx].dbLogId = aiRes.user_log_id;
				messages[mIdx].status = "success";
				
				const newAiMsgId = Date.now();
				messages = [
					...messages,
					{
						id: newAiMsgId,
						text: aiRes.reply,
						isMine: false,
						timestamp: newAiMsgId,
						status: "success",
						dbLogId: aiRes.ai_log_id,
						detectedLang: detectLanguage(aiRes.reply)
					},
				];
				unconsumedParsedMessages.add(newAiMsgId);
				
				scrollToBottom();
				if (aiRes.proactive_time && aiRes.proactive_message) {
					settings.update((s) => {
						s.proactiveEvent = {
							time: aiRes.proactive_time!,
							message: aiRes.proactive_message!,
						};
						return s;
					});
				}
				if (
					messages[mIdx].grammarCorrections &&
					messages[mIdx].grammarStatus === "success"
				) {
					apiSaveGrammar(
						aiRes.user_log_id,
						messages[mIdx].grammarCorrections,
					).catch((e) => console.error(e));
				}
				messages = [...messages];
			})
			.catch((err) => {
				isTyping = false;
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx !== -1) messages[mIdx].status = "error";
				messages = [...messages];
				notifications.error("Failed to send message:" + (err instanceof Error ? err.message : String(err)));
			});

		apiCheckGrammar(text)
			.then((grammarRes) => {
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx === -1) return;
				messages[mIdx].grammarCorrections = grammarRes;
				messages[mIdx].grammarStatus = "success";
				if (messages[mIdx].dbLogId) {
					apiSaveGrammar(messages[mIdx].dbLogId!, grammarRes).catch(
						(e) =>
							notifications.error(
								`Failed to save grammar corrections: ${e instanceof Error ? e.message : String(e)}`,
							),
					);
				}
				messages = [...messages];
			})
			.catch((err) => {
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx !== -1) messages[mIdx].grammarStatus = "error";
				messages = [...messages];
				notifications.error(
					`Failed to check grammar: ${err instanceof Error ? err.message : String(err)}`,
				);
			});
	}

	async function regenerateGrammar(msgId: number) {
		closeContextMenu();
		const mIdx = messages.findIndex((m) => m.id === msgId);
		if (mIdx === -1) return;
		messages[mIdx].grammarStatus = "checking";
		messages = [...messages];
		try {
			const result = await apiCheckGrammar(messages[mIdx].text);
			messages[mIdx].grammarCorrections = result;
			messages[mIdx].grammarStatus = "success";
			if (messages[mIdx].dbLogId)
				await apiSaveGrammar(messages[mIdx].dbLogId!, result);
		} catch (e) {
			messages[mIdx].grammarStatus = "error";
			notifications.error(
				`Failed to check grammar: ${e instanceof Error ? e.message : String(e)}`,
			);
		}
		messages = [...messages];
	}

	function clearProactiveTimer() {
		if (proactiveTimer) {
			clearTimeout(proactiveTimer);
			proactiveTimer = null;
		}
	}

	async function fireProactiveEvent(time: string, message: string) {
		messages = [
			...messages,
			{
				id: Date.now(),
				text: message,
				isMine: false,
				timestamp: new Date(time).getTime(),
				status: "success",
			},
		];
		scrollToBottom();

		try {
			await invoke("trigger_proactive", {
				message,
				scheduledTime: time,
			});
		} catch (e) {
			console.error("Failed to sync proactive msg:", e);
		}

		settings.update((s) => {
			s.proactiveEvent = null;
			return s;
		});
	}

	function syncProactiveSchedule() {
		const event = $settings.proactiveEvent;
		if (!event) {
			scheduledProactiveKey = null;
			clearProactiveTimer();
			return;
		}

		const key = `${event.time}__${event.message}`;
		if (scheduledProactiveKey === key) return;

		scheduledProactiveKey = key;
		clearProactiveTimer();

		const delay = new Date(event.time).getTime() - Date.now();
		if (delay <= 0) {
			void fireProactiveEvent(event.time, event.message);
			return;
		}

		proactiveTimer = window.setTimeout(() => {
			proactiveTimer = null;
			void fireProactiveEvent(event.time, event.message);
		}, delay);
	}

	onMount(async () => {
		await loadHistory();
		if (!historyLoadedOnce) {
			setTimeout(() => {
				if (!historyLoadedOnce && messages.length === 0) {
					loadHistory();
				}
			}, 1200);
		}
	});

	onDestroy(() => {
		clearProactiveTimer();
	});

	function onPointerDown(e: PointerEvent | TouchEvent, msgId: number) {
		const target = e.currentTarget as HTMLElement;
		pressTimer = window.setTimeout(() => {
			const rect = target.getBoundingClientRect();
			showContextMenu(rect.left + rect.width / 2, msgId, target);
		}, 500);
	}

	function onPointerUp() {
		clearTimeout(pressTimer);
	}

	function closeContextMenu() {
		contextMenu.visible = false;
	}

	function copyText() {
		const msg = messages.find((m) => m.id === contextMenu.msgId);
		if (msg) navigator.clipboard.writeText(msg.text);
		closeContextMenu();
	}

	$: if ($currentView === "chat" && chatContainer && firstLoad) {
		tick().then(() => {
			requestAnimationFrame(() => {
				if (chatContainer) {
					firstLoad = false;
					chatContainer.style.scrollBehavior = "auto";
					chatContainer.scrollTop = chatContainer.scrollHeight;
					chatContainer.style.scrollBehavior = "";
				}
			});
		});
	}

	$: if (
		$currentView === "chat" &&
		messages.length === 0 &&
		!isLoadingHistory &&
		hasMoreHistory
	) {
		loadHistory();
	}

	$: if ($currentView === "chat") {
		syncProactiveSchedule();
	}
</script>

<svelte:window
	on:pointerdown={(e) => {
		if (
			contextMenu.visible &&
			!(e.target as Element).closest(".context-menu")
		)
			closeContextMenu();
	}}
	on:click={(e) => {
		if (
			activeParsedBlock &&
			!(e.target as Element).closest(".parse-popover") &&
			!(e.target as Element).closest(".word-block")
		) {
			const isSentence = !!(e.target as Element).closest(".parsed-sentence");
			closeParsePopover(!isSentence);
		}
	}}
/>

<div
	bind:this={containerEl}
	class="app-container"
	style="--chat-bg-image: {backgroundUrl ? `url(${backgroundUrl})` : 'none'}"
>
	<header class="top-bar">
		<button class="icon-btn" on:click={handleBack}>
			<svg
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M15 19l-7-7 7-7" />
			</svg>
		</button>
		<div class="title">{isTyping ? "Typing..." : $settings.aiNickname}</div>
		<button class="icon-btn" on:click={() => (isDrawerOpen = true)}>
			<svg viewBox="0 0 24 24" fill="currentColor">
				<circle cx="5" cy="12" r="2.5" />
				<circle cx="12" cy="12" r="2.5" />
				<circle cx="19" cy="12" r="2.5" />
			</svg>
		</button>
	</header>

	<main class="chat-area" bind:this={chatContainer} on:scroll={onChatScroll}>
		{#each messages as msg, i (msg.id)}
			{#if msg.text !== ""}
				<div class="msg-wrapper {msg.isMine ? 'mine' : 'theirs'}">
					{#if i === 0 || msg.timestamp - messages[i - 1].timestamp > 5 * 60 * 1000}
						<div class="timestamp">{formatTime(msg.timestamp)}</div>
					{/if}
					<div class="msg-content">
						{#if !msg.isMine}
							<img
								src={$settings.aiAvatarUrl}
								class="avatar"
								alt="AI"
							/>
						{/if}
						{#if msg.isMine && msg.status === "error"}
							<button
								class="error-icon"
								on:click={() => sendMessage(msg.text, msg.id)}
							>
								<svg viewBox="0 0 24 24" fill="var(--danger)">
									<circle cx="12" cy="12" r="10" />
									<path
										d="M12 8v4m0 4h.01"
										stroke="white"
										stroke-width="2"
										stroke-linecap="round"
									/>
								</svg>
							</button>
						{/if}
						<div class="bubble-group">
							{#if !msg.isMine && msg.parseStatus === "done" && msg.parsedSentences}
								<div
									class="bubble parsed-bubble"
									on:pointerdown={(e) =>
										onPointerDown(e, msg.id)}
									on:pointerup={onPointerUp}
									on:pointermove={onPointerUp}
									on:pointercancel={onPointerUp}
									on:contextmenu={(e) => {
										e.preventDefault();
										showContextMenu(
											e.clientX,
											msg.id,
											e.currentTarget as HTMLElement,
										);
									}}
								>
									{#each msg.parsedSentences as sentence}
										<div
											class="parsed-sentence"
											on:click={() =>
												handleSentenceClick(sentence)}
										>
											<div class="parsed-words">
												{#each sentence.blocks as block}
													{#if block.pos === "punctuation"}
														<span
															class="word-block {getBlockPosClass(
																block,
																msg.detectedLang,
															)}"
															>{block.text}</span
														>
													{:else}
														<button
															class="word-block {getBlockPosClass(
																block,
																msg.detectedLang,
															)}"
															on:click|stopPropagation={(
																e,
															) =>
																handleParsedBlockClick(
																	e,
																	block,
																	sentence,
																	msg.id,
																)}
														>
															{block.text}
															{#if msg.detectedLang === "RU" && (block.pos === "noun" || block.pos === "pronoun") && block.gram_case}
																<sup
																	class="case-sup"
																	>{block.gram_case}</sup
																>
															{/if}
														</button>
													{/if}
												{/each}
											</div>
											{#if sentence.translation}
												<div
													class="sentence-translation"
												>
													{sentence.translation}
												</div>
											{/if}
										</div>
									{/each}
								</div>
							{:else if !msg.isMine && msg.parseStatus === "parsing"}
								<div
									class="bubble"
									on:pointerdown={(e) =>
										onPointerDown(e, msg.id)}
									on:pointerup={onPointerUp}
									on:pointermove={onPointerUp}
									on:pointercancel={onPointerUp}
									on:contextmenu={(e) => {
										e.preventDefault();
										showContextMenu(
											e.clientX,
											msg.id,
											e.currentTarget as HTMLElement,
										);
									}}
								>
									<div class="parse-loading">
										<span class="parse-spinner"></span>
										Parsing...
									</div>
								</div>
							{:else if !msg.isMine && msg.parseStatus === "error"}
								<div
									class="bubble"
									on:pointerdown={(e) =>
										onPointerDown(e, msg.id)}
									on:pointerup={onPointerUp}
									on:pointermove={onPointerUp}
									on:pointercancel={onPointerUp}
									on:contextmenu={(e) => {
										e.preventDefault();
										showContextMenu(
											e.clientX,
											msg.id,
											e.currentTarget as HTMLElement,
										);
									}}
								>
									{msg.text}
									<div class="parse-error-hint">
										Parse failed · Long press to retry
									</div>
								</div>
							{:else}
								<div
									class="bubble"
									on:pointerdown={(e) =>
										onPointerDown(e, msg.id)}
									on:pointerup={onPointerUp}
									on:pointermove={onPointerUp}
									on:pointercancel={onPointerUp}
									on:contextmenu={(e) => {
										e.preventDefault();
										showContextMenu(
											e.clientX,
											msg.id,
											e.currentTarget as HTMLElement,
										);
									}}
								>
									{msg.text}
								</div>
							{/if}
							{#if msg.quoteText}
								<div class="quote-bubble">
									<div class="quote-text">{msg.quoteText}</div>
								</div>
							{/if}

							{#if msg.isMine && msg.grammarStatus === "checking"}
								<div class="grammar-check checking">
									Checking...
								</div>
							{:else if msg.isMine && msg.grammarCorrections}
								<div class="grammar-check result">
									{#each msg.grammarCorrections as correction}
										{#if correction.type === "unchanged"}
											<span class="gc-block"
												>{correction.corrected}</span
											>{" "}
										{:else if correction.type === "deleted"}
											<span class="gc-block deleted"
												>{correction.original}</span
											>{" "}
										{:else if correction.type === "modified"}
											<span
												class="gc-block modified"
												title={correction.original ||
													""}
												>{correction.corrected}</span
											>{" "}
										{:else if correction.type === "inserted"}
											<span class="gc-block inserted"
												>{correction.corrected}</span
											>{" "}
										{/if}
									{/each}
								</div>
							{/if}
						</div>
						{#if msg.isMine}
							<img
								src={$settings.userAvatarUrl}
								class="avatar"
								alt="You"
							/>
						{/if}
					</div>
				</div>
			{/if}
		{/each}
	</main>

	<footer class="bottom-bar">
		{#if translationResult}
			<div
				class="translation-popup"
				transition:fly={{ y: 10, duration: 200 }}
			>
				<span class="lang-tag">RU</span>{translationResult}
			</div>
		{/if}
		{#if quotingMsg}
			<div class="quote-input-bar">
				<div class="quote-input-text">{quotingMsg.text}</div>
				<button class="quote-input-cancel" on:click={cancelQuote}
					>✕</button
				>
			</div>
		{/if}
		<div class="input-wrapper">
			<textarea
				bind:this={textareaEl}
				bind:value={inputText}
				on:input={handleInput}
				on:keydown={(e) => {
					if (e.key === "Enter" && !e.shiftKey) {
						e.preventDefault();
						sendMessage();
					}
				}}
				placeholder="Message"
				rows="1"
			></textarea>
			{#if inputText.trim().length > 0}
				<button
					class="send-btn"
					on:click={() => sendMessage()}
					transition:fly={{ x: 20, duration: 200 }}
				>
					Send
				</button>
			{/if}
		</div>
	</footer>

	{#if contextMenu.visible}
		<div
			class="context-menu pos-{contextMenu.position}"
			style="top: {contextMenu.y}px; left: {contextMenu.x}px;"
			transition:fade={{ duration: 150 }}
		>
			<button class="context-item" on:click={copyText}>
				<svg
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
					<path
						d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
					/>
				</svg>
				<span>Copy</span>
			</button>
			<button
				class="context-item"
				on:click={() => startQuote(contextMenu.msgId!)}
			>
				<svg
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="9 17 4 12 9 7"></polyline>
					<path d="M20 18v-2a4 4 0 0 0-4-4H4"></path>
				</svg>
				<span>Quote</span>
			</button>
			{#if !messages.find((m) => m.id === contextMenu.msgId)?.isMine}
				<div class="divider"></div>
				<button
					class="context-item"
					on:click={() => parseMessage(contextMenu.msgId!)}
				>
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path
							d="M4 7V4a2 2 0 0 1 2-2h8.5L20 7.5V20a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-3"
						/>
						<polyline points="14 2 14 8 20 8" />
						<path d="M9 15l2 2 4-4" />
					</svg>
					<span
						>{messages.find((m) => m.id === contextMenu.msgId)
							?.parseStatus === "done"
							? "Re-parse"
							: "Parse"}</span
					>
				</button>
			{/if}
			{#if messages.find((m) => m.id === contextMenu.msgId)?.isMine}
				<div class="divider"></div>
				<button
					class="context-item"
					on:click={() => regenerateGrammar(contextMenu.msgId!)}
				>
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path
							d="M20 14.66V20a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h5.34"
						/>
						<polygon points="18 2 22 6 12 16 8 16 8 12 18 2" />
					</svg>
					<span>Grammar</span>
				</button>
			{/if}
		</div>
	{/if}

{#if activeParsedBlock}
		<WordPopover
			block={activeParsedBlock}
			position={{
				left: popoverPos.left,
				top: popoverPos.top,
				align: popoverPos.align,
				arrowLeft: popoverPos.arrowLeft,
			}}
			language={activeParsedBlockEl?.closest("[data-lang]")?.getAttribute("data-lang") || "RU"}
		/>
	{/if}

	{#if isDrawerOpen}
		<div
			class="drawer-overlay"
			on:click={() => (isDrawerOpen = false)}
			transition:fade={{ duration: 200 }}
		></div>
		<div class="drawer" transition:fly={{ x: 300, duration: 300 }}>
			<div class="drawer-header">
				<h2>Settings</h2>
				<button on:click={() => (isDrawerOpen = false)}>Close</button>
			</div>
			<div class="setting-item">
				<label>Your Avatar URL</label>
				<input type="text" bind:value={$settings.userAvatarUrl} />
			</div>
			<div class="setting-item">
				<label>AI Avatar URL</label>
				<input type="text" bind:value={$settings.aiAvatarUrl} />
			</div>
			<div class="setting-item">
				<label>Chat Background URL</label>
				<input
					type="text"
					bind:value={backgroundUrl}
					placeholder="Leave empty for default"
				/>
			</div>
			<div class="setting-item">
				<label>AI Nickname</label>
				<input
					type="text"
					bind:value={$settings.aiNickname}
				/>
			</div>
			<div class="setting-item token-setting">
				<label>Chat Token Limits</label>
				<div class="setting-tip">Recommended: 4000 / 1000 / 1000 / 500</div>
				<div class="token-grid">
					<div class="token-row">
						<div class="token-row-head">Max Total Tokens</div>
						<div class="token-row-desc">Total token budget for one chat turn context.</div>
						<input
							type="number"
							class="token-input"
							min="500"
							step="100"
							bind:value={$settings.maxTotalTokens}
							placeholder="4000"
						/>
					</div>
					<div class="token-row">
						<div class="token-row-head">Max RAG Tokens</div>
						<div class="token-row-desc">Max tokens injected from retrieved RAG memory.</div>
						<input
							type="number"
							class="token-input"
							min="100"
							step="50"
							bind:value={$settings.maxRagTokens}
							placeholder="1000"
						/>
					</div>
					<div class="token-row">
						<div class="token-row-head">Max RAG Append Tokens</div>
						<div class="token-row-desc">Max historical RAG tokens used when appending new memory.</div>
						<input
							type="number"
							class="token-input"
							min="100"
							step="50"
							bind:value={$settings.maxRagAppendTokens}
							placeholder="1000"
						/>
					</div>
					<div class="token-row">
						<div class="token-row-head">Max User Tokens</div>
						<div class="token-row-desc">Max tokens allowed for a single user message.</div>
						<input
							type="number"
							class="token-input"
							min="50"
							step="25"
							bind:value={$settings.maxUserTokens}
							placeholder="500"
						/>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	:global(:root) {
		--bg-color: #eceeec;
		--topbar-bg: #f7f7f7;
		--bottombar-bg: #f7f7f7;
		--border-color: #e5e5e5;
		--text-primary: #000000;
		--text-secondary: #999999;
		--bubble-mine: #8aec5f;
		--bubble-mine-text: #000000;
		--bubble-theirs: #efefef;
		--bubble-theirs-text: #000000;
		--input-bg: #ffffff;
		--accent: #05c160;
		--danger: #fa5151;
		--context-bg: #4c4c4c;
		--context-text: #ffffff;
		--drawer-bg: #f2f2f2;
		--popover-bg: #1e1e1e;
		--popover-text: #ffffff;
		--popover-muted: #a1a1aa;
		--popover-border: rgba(255, 255, 255, 0.08);
	}

	@media (prefers-color-scheme: dark) {
		:global(:root) {
			--bg-color: #171a17;
			--topbar-bg: #1e1e1e;
			--bottombar-bg: #1e1e1e;
			--border-color: #000000;
			--text-primary: #ffffff;
			--text-secondary: #888888;
			--bubble-mine: #24b671;
			--bubble-mine-text: #000000;
			--bubble-theirs: #1b1a1b;
			--bubble-theirs-text: #ffffff;
			--input-bg: #2c2c2c;
			--context-bg: #333333;
			--drawer-bg: #111111;
			--popover-bg: #2a2a2a;
			--popover-text: #f0f0f0;
			--popover-muted: #888;
			--popover-border: rgba(255, 255, 255, 0.06);
		}
	}

	:global(body) {
		margin: 0;
		padding: 0;
		background-color: var(--bg-color);
		color: var(--text-primary);
		font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue",
			Helvetica, "Segoe UI", Arial, sans-serif;
		overflow: hidden;
	}

	.app-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		background-image: var(--chat-bg-image);
		background-size: cover;
		background-position: center;
		position: relative;
	}

	.top-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		height: 52px;
		background-color: var(--topbar-bg);
		border-bottom: 0.5px solid var(--border-color);
		z-index: 10;
	}

	.icon-btn {
		background: none;
		border: none;
		color: var(--text-primary);
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		padding: 0;
	}

	.icon-btn svg {
		width: 24px;
		height: 24px;
	}

	.title {
		font-size: 17px;
		font-weight: 600;
	}

	.chat-area {
		flex: 1;
		overflow-y: auto;
		padding: 16px;
		display: flex;
		flex-direction: column;
		gap: 20px;
		scroll-behavior: smooth;
		-webkit-overflow-scrolling: touch;
		overscroll-behavior-y: contain;
	}

	.msg-wrapper {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
	}

	.timestamp {
		font-size: 12px;
		color: var(--text-secondary);
		margin-bottom: 16px;
	}

	.msg-content {
		display: flex;
		width: 100%;
		align-items: flex-start;
		gap: 12px;
		min-width: 0;
		overflow: visible;
	}

	.mine .msg-content {
		justify-content: flex-end;
	}

	.theirs .msg-content {
		justify-content: flex-start;
	}

	.avatar {
		width: 40px;
		height: 40px;
		border-radius: 4px;
		object-fit: cover;
		background-color: #ccc;
		flex-shrink: 0;
	}

	.bubble-group {
		display: flex;
		flex-direction: column;
		max-width: 75%;
		gap: 4px;
		min-width: 0;
		overflow: visible;
	}
	.mine .bubble-group {
		align-items: flex-end;
	}
	.theirs .bubble-group {
		align-items: flex-start;
	}
	.quote-bubble {
		width: 0;
		min-width: 100%;
		border-radius: 8px;
		background-color: rgba(0, 0, 0, 0.05);
		padding: 8px 12px;
		margin-top: 6px;
		box-sizing: border-box;

		font-size: 12px;
		color: var(--text-secondary);
	}
	.quote-text {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: normal;
		overflow-wrap: break-word;
		word-break: break-word;
		line-height: 1.4;
	}

	@media (prefers-color-scheme: dark) {
		.quote-bubble {
			background-color: rgba(255, 255, 255, 0.1);
		}
	}
	.bubble {
		position: relative;
		padding: 10px 14px;
		font-size: 16px;
		line-height: 1.5;
		word-break: break-word;
		user-select: none;
		min-height: 20px;
	}

	.mine .bubble {
		background-color: var(--bubble-mine);
		color: var(--bubble-mine-text);
		border-radius: 8px;
	}

	.theirs .bubble {
		background-color: var(--bubble-theirs);
		color: var(--bubble-theirs-text);
		border-radius: 8px;
	}

	.bubble::before {
		content: "";
		position: absolute;
		top: 20px;
		transform: translateY(-50%);
		border-style: solid;
	}

	.mine .bubble::before {
		right: -5px;
		border-width: 5px 0 5px 6px;
		border-color: transparent transparent transparent var(--bubble-mine);
	}

	.theirs .bubble::before {
		left: -5px;
		border-width: 5px 6px 5px 0;
		border-color: transparent var(--bubble-theirs) transparent transparent;
	}

	/* ---------- Parsed bubble ---------- */
	.parsed-bubble {
		background-color: transparent !important;
		padding: 6px 4px !important;
		line-height: 1.8;
	}

	.parsed-bubble::before {
		display: none !important;
	}

	.parsed-sentence {
		margin-bottom: 6px;
		cursor: pointer;
		border-radius: 8px;
		padding: 4px 6px;
		transition: background-color 0.15s;
	}

	.parsed-sentence:last-child {
		margin-bottom: 0;
	}

	.parsed-sentence:active {
		background-color: rgba(128, 128, 128, 0.1);
	}

	.parsed-words {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0;
	}

	.word-block {
		display: inline-block;
		padding: 1px 4px;
		margin: 0 0.5px;
		border-radius: 4px;
		font-size: inherit;
		line-height: inherit;
		transition:
			transform 0.075s ease-out,
			filter 0.15s;
		border: none;
		background: none;
		cursor: pointer;
		font-family: inherit;
		position: relative;
	}

	.word-block:active {
		transform: scale(0.93);
	}

	.case-sup {
		font-size: 9px;
		margin-left: 1px;
		opacity: 0.6;
		font-weight: 600;
	}

	.sentence-translation {
		font-size: 12px;
		color: var(--text-secondary);
		padding: 2px 6px 0;
		line-height: 1.4;
		font-style: italic;
	}

	/* ---- POS colors (light) ---- */
	.pos-noun {
		background-color: rgba(59, 130, 246, 0.13);
		color: #1d4ed8;
	}
	.pos-pronoun {
		background-color: rgba(99, 102, 241, 0.13);
		color: #4338ca;
	}
	.pos-verb {
		background-color: rgba(239, 68, 68, 0.13);
		color: #b91c1c;
	}
	.pos-adjective {
		background-color: rgba(245, 158, 11, 0.13);
		color: #b45309;
	}
	.pos-adverb {
		background-color: rgba(16, 185, 129, 0.13);
		color: #047857;
	}
	.pos-func {
		background-color: rgba(156, 163, 175, 0.15);
		color: #4b5563;
	}
	.pos-punct {
		background-color: transparent;
		color: var(--text-secondary);
		cursor: default;
		padding: 1px 1px;
	}
	.pos-unknown {
		background-color: rgba(100, 116, 139, 0.1);
		color: #64748b;
	}

	/* Russian gender overrides */
	.ru-gender-m {
		background-color: rgba(139, 92, 246, 0.13);
		color: #6d28d9;
	}
	.ru-gender-f {
		background-color: rgba(6, 182, 212, 0.13);
		color: #0e7490;
	}
	.ru-gender-n {
		background-color: rgba(59, 130, 246, 0.13);
		color: #1d4ed8;
	}

	@media (prefers-color-scheme: dark) {
		.pos-noun {
			background-color: rgba(59, 130, 246, 0.22);
			color: #93c5fd;
		}
		.pos-pronoun {
			background-color: rgba(99, 102, 241, 0.22);
			color: #a5b4fc;
		}
		.pos-verb {
			background-color: rgba(239, 68, 68, 0.22);
			color: #fca5a5;
		}
		.pos-adjective {
			background-color: rgba(245, 158, 11, 0.22);
			color: #fcd34d;
		}
		.pos-adverb {
			background-color: rgba(16, 185, 129, 0.22);
			color: #6ee7b7;
		}
		.pos-func {
			background-color: rgba(156, 163, 175, 0.18);
			color: #d1d5db;
		}
		.pos-unknown {
			background-color: rgba(100, 116, 139, 0.18);
			color: #94a3b8;
		}
		.ru-gender-m {
			background-color: rgba(139, 92, 246, 0.22);
			color: #c4b5fd;
		}
		.ru-gender-f {
			background-color: rgba(6, 182, 212, 0.22);
			color: #67e8f9;
		}
		.ru-gender-n {
			background-color: rgba(59, 130, 246, 0.22);
			color: #93c5fd;
		}
	}

	/* Parse loading */
	.parse-loading {
		display: flex;
		align-items: center;
		gap: 8px;
		opacity: 0.6;
		font-size: 14px;
	}

	.parse-spinner {
		width: 14px;
		height: 14px;
		border: 2px solid transparent;
		border-top-color: currentColor;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.parse-error-hint {
		font-size: 11px;
		color: var(--danger);
		margin-top: 6px;
		opacity: 0.8;
	}

	/* ---------- Grammar check ---------- */
	.grammar-check {
		font-size: 12px;
		padding: 6px 10px;
		background-color: rgba(0, 0, 0, 0.15);
		border-radius: 6px;
		color: var(--text-primary);
		align-self: flex-end;
		/* display: flex;
		flex-wrap: wrap;
		gap: 0.3em; */
	}

	@media (prefers-color-scheme: dark) {
		.grammar-check {
			background-color: rgba(255, 255, 255, 0.1);
		}
	}

	.grammar-check.checking {
		font-style: italic;
		opacity: 0.7;
	}

	.gc-block.deleted {
		text-decoration: line-through;
		color: var(--danger);
	}

	.gc-block.modified {
		border-bottom: 2px dashed #ffd700;
		color: #d4a017;
	}

	@media (prefers-color-scheme: dark) {
		.gc-block.modified {
			color: #ffd700;
		}
	}

	.gc-block.inserted {
		color: var(--accent);
		text-decoration: underline;
	}

	.error-icon {
		background: none;
		border: none;
		width: 24px;
		height: 24px;
		padding: 0;
		align-self: center;
		cursor: pointer;
	}

	.bottom-bar {
		width: 100%;
		box-sizing: border-box;
		background-color: var(--bottombar-bg);
		border-top: 0.5px solid var(--border-color);
		padding: 6px 16px;
		padding-bottom: calc(10px + env(safe-area-inset-bottom, 0px));
		position: relative;
	}

	.quote-input-bar {
		display: flex;
		align-items: center;
		background-color: var(--input-bg);
		border-radius: 4px;
		padding: 8px 10px;
		margin-bottom: 6px;
		border-left: 3px solid var(--accent);
		width: 100%;
		max-width: 100%;
		overflow: hidden;
	}

	.quote-input-text {
		flex: 1;
		min-width: 0;
		width: 0;
		font-size: 13px;
		color: var(--text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.quote-input-cancel {
		background: none;
		border: none;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0 0 0 8px;
		font-size: 14px;
		flex-shrink: 0;
	}

	.translation-popup {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 16px;
		max-width: calc(100% - 32px);
		background-color: var(--context-bg);
		color: var(--context-text);
		padding: 8px 12px;
		border-radius: 6px;
		font-size: 14px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
		display: flex;
		align-items: center;
		gap: 8px;
		z-index: 5;
	}

	.lang-tag {
		font-size: 10px;
		background: rgba(255, 255, 255, 0.2);
		padding: 2px 4px;
		border-radius: 4px;
	}

	.input-wrapper {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.input-wrapper textarea {
		flex: 1;
		background-color: var(--input-bg);
		border: none;
		color: var(--text-primary);
		padding: 8px 12px;
		border-radius: 4px;
		font-size: 16px;
		outline: none;
		resize: none;
		height: 40px;
		min-height: 40px;
		max-height: 150px;
		line-height: 24px;
		overflow-y: auto;
		font-family: inherit;
		white-space: pre-wrap;
		word-wrap: break-word;
		word-break: break-word;
		box-sizing: border-box;
	}

	.send-btn {
		background-color: var(--accent);
		color: white;
		border: none;
		height: 26px;
		padding: 0 7px;
		border-radius: 4px;
		font-size: 15px;
		font-weight: 500;
		cursor: pointer;
	}

	/* ---------- Context menu ---------- */
	.context-menu {
		position: absolute;
		background-color: var(--context-bg);
		border-radius: 8px;
		display: flex;
		flex-direction: row;
		padding: 6px;
		gap: 4px;
		z-index: 100;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
	}

	.context-menu.pos-top {
		transform: translate(-50%, -100%);
	}

	.context-menu.pos-bottom {
		transform: translate(-50%, 0);
	}

	.context-menu::after {
		content: "";
		position: absolute;
		left: 50%;
		transform: translateX(-50%);
		border: 6px solid transparent;
	}

	.context-menu.pos-top::after {
		top: 100%;
		border-top-color: var(--context-bg);
	}

	.context-menu.pos-bottom::after {
		bottom: 100%;
		border-bottom-color: var(--context-bg);
	}

	.context-item {
		background: none;
		border: none;
		color: var(--context-text);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 4px;
		min-width: 60px;
		padding: 8px;
		font-size: 11px;
		cursor: pointer;
		border-radius: 6px;
	}

	.context-item:active {
		background-color: rgba(255, 255, 255, 0.1);
	}

	.context-item svg {
		width: 22px;
		height: 22px;
	}

	.context-menu .divider {
		width: 1px;
		background-color: rgba(255, 255, 255, 0.2);
		margin: 4px 4px;
	}

	/* ---------- Parse popover ---------- */
	.parse-popover {
		position: fixed;
		z-index: 150;
		width: 264px;
		background-color: var(--popover-bg);
		color: var(--popover-text);
		border-radius: 14px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
		overflow: visible;
	}

	.popover-inner {
		padding: 12px 14px;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.popover-def {
		font-size: 16px;
		font-weight: 700;
		line-height: 1.3;
	}

	.popover-note {
		font-size: 13px;
		color: var(--popover-muted);
		line-height: 1.4;
		border-top: 1px solid var(--popover-border);
		padding-top: 6px;
		margin-top: 2px;
	}

	.popover-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin-top: 2px;
	}

	.popover-tag {
		display: inline-block;
		font-size: 10px;
		font-weight: 600;
		padding: 2px 6px;
		border-radius: 4px;
		letter-spacing: 0.02em;
	}

	.root-tag {
		background-color: rgba(234, 179, 8, 0.2);
		color: #facc15;
		border: 1px solid rgba(234, 179, 8, 0.4);
		align-self: flex-start;
	}

	.tense-tag {
		background-color: rgba(59, 130, 246, 0.18);
		color: #60a5fa;
		border: 1px solid rgba(59, 130, 246, 0.35);
	}

	.aspect-tag.pf {
		background-color: rgba(249, 115, 22, 0.18);
		color: #fb923c;
		border: 1px solid rgba(249, 115, 22, 0.35);
	}

	.aspect-tag.impf {
		background-color: rgba(6, 182, 212, 0.18);
		color: #22d3ee;
		border: 1px solid rgba(6, 182, 212, 0.35);
	}

	.plural-tag {
		background-color: rgba(168, 85, 247, 0.2);
		color: #c084fc;
		border: 1px solid rgba(168, 85, 247, 0.35);
	}

	.lemma-tag {
		background-color: rgba(255, 255, 255, 0.08);
		color: var(--popover-muted);
		border: 1px solid var(--popover-border);
		font-weight: 500;
	}

	.popover-arrow {
		position: absolute;
		width: 12px;
		height: 12px;
		background-color: var(--popover-bg);
		transform: rotate(45deg);
		border-radius: 2px;
	}

	/* ---------- Drawer ---------- */
	.drawer-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.6);
		z-index: 20;
	}

	.drawer {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: 80%;
		max-width: 320px;
		background-color: var(--drawer-bg);
		z-index: 30;
		padding: 20px;
		padding-bottom: calc(20px + env(safe-area-inset-bottom, 0px));
		overflow-y: auto;
		-webkit-overflow-scrolling: touch;
		overscroll-behavior: contain;
		box-shadow: -4px 0 15px rgba(0, 0, 0, 0.1);
	}

	.drawer-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 24px;
		padding-bottom: 12px;
	}

	.drawer-header h2 {
		font-size: 18px;
		margin: 0;
	}

	.drawer-header button {
		background: none;
		border: none;
		color: var(--accent);
		font-size: 16px;
	}

	.setting-item {
		margin-bottom: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.setting-item label {
		font-size: 14px;
		color: var(--text-secondary);
	}

	.setting-item input {
		background-color: var(--input-bg);
		border: 1px solid var(--border-color);
		color: var(--text-primary);
		padding: 12px;
		border-radius: 6px;
		outline: none;
	}

	.token-setting {
		padding: 10px;
		border: 1px solid var(--border-color);
		border-radius: 10px;
		background: color-mix(in srgb, var(--input-bg) 86%, transparent);
	}

	.setting-tip {
		font-size: 12px;
		color: var(--text-secondary);
		margin-top: -2px;
	}

	.token-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.token-row {
		border: 1px solid var(--border-color);
		border-radius: 8px;
		padding: 8px;
		background-color: color-mix(in srgb, var(--input-bg) 92%, transparent);
	}

	.token-row-head {
		font-size: 13px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.token-row-desc {
		font-size: 11px;
		color: var(--text-secondary);
		margin-top: 2px;
		margin-bottom: 6px;
		line-height: 1.35;
	}

	.token-row input {
		padding: 10px 10px;
		font-size: 13px;
	}

	.token-input {
		appearance: textfield;
		-moz-appearance: textfield;
	}

	.token-input::-webkit-outer-spin-button,
	.token-input::-webkit-inner-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}
	@media (prefers-color-scheme: dark) {
		.quote-bubble {
			background-color: rgba(255, 255, 255, 0.08);
		}

		.token-setting {
			background: color-mix(in srgb, var(--input-bg) 82%, transparent);
		}
	}
</style>
