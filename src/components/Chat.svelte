<script lang="ts">
	import { onMount, tick } from "svelte";
	import { fly, fade } from "svelte/transition";
	import { invoke } from "@tauri-apps/api/core";
	import { settings } from "../lib/stores";
	import { notifications } from "$lib/notificationStore";

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

	let contextMenu = {
		visible: false,
		x: 0,
		y: 0,
		msgId: null as number | null,
		position: "top" as "top" | "bottom",
	};

	function formatTime(ts: number): string {
		const date = new Date(ts);
		const now = new Date();
		const isToday = date.toDateString() === now.toDateString();
		const timeStr = date.toLocaleTimeString("en-US", {
			hour: "2-digit",
			minute: "2-digit",
			hour12: false,
		});
		if (isToday) return timeStr;
		const dateStr = `${date.getMonth() + 1}/${date.getDate()}/${date.getFullYear().toString().slice(-2)}`;
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

	function getConfigById(id: string | undefined) {
		if (!id) return undefined;
		return $settings.aiConfigList.find((c) => c.id === id);
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
		});
	}

	async function loadHistory() {
		if (isLoadingHistory || !hasMoreHistory) return;
		isLoadingHistory = true;

		// use the id of the topmost message to load earlier messages
		const beforeId = messages.length > 0 ? messages[0].id : null;

		try {
			const res = await invoke<{
				messages: Array<{
					id: number;
					role: string;
					content: string;
					timestamp: string;
					grammar_corrections: string | null;
				}>;
				has_more: boolean;
			}>("get_chat_logs", { beforeId, limit: 20 });

			hasMoreHistory = res.has_more;

			if (res.messages && res.messages.length > 0) {
				const oldScrollHeight = chatContainer
					? chatContainer.scrollHeight
					: 0;
				const formattedLogs = res.messages.map((msg) => ({
					id: msg.id,
					text: msg.content,
					isMine: msg.role === "user",
					timestamp: new Date(msg.timestamp).getTime(),
					status: "success" as const,
					grammarCorrections: msg.grammar_corrections
						? JSON.parse(msg.grammar_corrections)
						: undefined,
					grammarStatus: msg.grammar_corrections
						? ("success" as const)
						: undefined,
				}));

				messages = [...formattedLogs, ...messages];

				await tick();
				if (chatContainer) {
					if (beforeId === null) {
						chatContainer.scrollTop = chatContainer.scrollHeight;
					} else {
						 chatContainer.scrollTop =
						 	chatContainer.scrollHeight - oldScrollHeight;
					}
				}
			}
		} catch (e) {
			console.error("Failed to load history:", e);
		} finally {
			isLoadingHistory = false;
		}
	}

	async function onChatScroll() {
		if (chatContainer.scrollTop === 0) {
			await loadHistory();
		}
	}

	function handleBack() {}

	async function handleInput() {
		const match = inputText.match(/([a-zA-Z\s,']+)\?$/);
		if (match) {
			isTranslating = true;
			try {
				translationResult = await apiTranslate(match[1].trim());
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
		let msgId = resendId || Date.now();

		if (!resendId) {
			messages = [
				...messages,
				{
					id: msgId,
					text: text.trim(),
					isMine: true,
					timestamp: Date.now(),
					status: "sending",
					grammarStatus: "checking",
				},
			];
			inputText = "";
			translationResult = "";
			scrollToBottom();
		} else {
			let mIdx = messages.findIndex((m) => m.id === msgId);
			if (mIdx !== -1) messages[mIdx].status = "sending";
			messages = [...messages];
		}

		isTyping = true;

		apiSendMessage(text)
			.then((aiRes) => {
				isTyping = false;
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx === -1) return;

				messages[mIdx].dbLogId = aiRes.user_log_id;
				messages[mIdx].status = "success";
				messages = [
					...messages,
					{
						id: Date.now(),
						text: aiRes.reply,
						isMine: false,
						timestamp: Date.now(),
						status: "success",
					},
				];
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
			});

		apiCheckGrammar(text)
			.then((grammarRes) => {
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx === -1) return;

				messages[mIdx].grammarCorrections = grammarRes;
				messages[mIdx].grammarStatus = "success";

				if (messages[mIdx].dbLogId) {
					apiSaveGrammar(messages[mIdx].dbLogId!, grammarRes).catch(
						(e) => console.error(e),
					);
				}
				messages = [...messages];
			})
			.catch((err) => {
				let mIdx = messages.findIndex((m) => m.id === msgId);
				if (mIdx !== -1) messages[mIdx].grammarStatus = "error";
				messages = [...messages];
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
		}
		messages = [...messages];
	}

	onMount(async () => {
		if ($settings.proactiveEvent) {
			const { time, message } = $settings.proactiveEvent;
			const delay = new Date(time).getTime() - Date.now();

			const fireProactive = async () => {
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
						message: message,
						scheduledTime: time,
					});
				} catch (e) {
					console.error("Failed to sync proactive msg:", e);
				}

				settings.update((s) => {
					s.proactiveEvent = null;
					return s;
				});
			};

			if (delay <= 0) fireProactive();
			else setTimeout(fireProactive, delay);
		}

		await loadHistory();
	});

	function onPointerDown(e: PointerEvent | TouchEvent, msgId: number) {
		const target = e.currentTarget as HTMLElement;

		pressTimer = window.setTimeout(() => {
			const rect = target.getBoundingClientRect();
			let pos: "top" | "bottom" = "top";
			let targetY = rect.top - 12;

			if (targetY < 80) {
				pos = "bottom";
				targetY = rect.bottom + 12;
			}

			contextMenu = {
				visible: true,
				x: rect.left + rect.width / 2,
				y: targetY,
				msgId,
				position: pos,
			};
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
</script>

<svelte:window
	on:pointerdown={(e) => {
		if (
			contextMenu.visible &&
			!(e.target as Element).closest(".context-menu")
		)
			closeContextMenu();
	}}
/>

<div
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
		<div class="title">{isTyping ? "Typing..." : "malim"}</div>
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
						<div
							class="bubble"
							on:pointerdown={(e) => onPointerDown(e, msg.id)}
							on:pointerup={onPointerUp}
							on:pointermove={onPointerUp}
							on:pointercancel={onPointerUp}
						>
							{msg.text}
						</div>

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
										>
									{:else if correction.type === "deleted"}
										<span class="gc-block deleted"
											>{correction.original}</span
										>
									{:else if correction.type === "modified"}
										<span
											class="gc-block modified"
											title={correction.original || ""}
											>{correction.corrected}</span
										>
									{:else if correction.type === "inserted"}
										<span class="gc-block inserted"
											>{correction.corrected}</span
										>
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
		<div class="input-wrapper">
			<input
				type="text"
				bind:value={inputText}
				on:input={handleInput}
				on:keydown={(e) => {
					if (e.key === "Enter") sendMessage();
				}}
			/>
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
					<rect x="9" y="9" width="13" height="13" rx="2" ry="2"
					></rect>
					<path
						d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
					></path>
				</svg>
				<span>Copy</span>
			</button>
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
						></path>
						<polygon points="18 2 22 6 12 16 8 16 8 12 18 2"
						></polygon>
					</svg>
					<span>Grammar</span>
				</button>
			{/if}
		</div>
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
		height: 100vh;
		height: 100dvh;
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
		max-width: 68%;
		gap: 4px;
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
		border-radius: 8px 8px 8px 8px;
	}

	.theirs .bubble {
		background-color: var(--bubble-theirs);
		color: var(--bubble-theirs-text);
		border-radius: 8px 8px 8px 8px;
	}


	.bubble::before {
		content: "";
		position: absolute;
		top: 14px;
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

	.grammar-check {
		font-size: 12px;
		padding: 6px 10px;
		background-color: rgba(0, 0, 0, 0.15);
		border-radius: 6px;
		color: var(--text-primary);
		align-self: flex-end;
		display: flex;
		flex-wrap: wrap;
		gap: 3px;
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
		background-color: var(--bottombar-bg);
		border-top: 0.5px solid var(--border-color);
		padding: 6px 16px;
		padding-bottom: calc(10px + env(safe-area-inset-bottom, 0px));
		position: relative;
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

	input[type="text"] {
		flex: 1;
		background-color: var(--input-bg);
		border: none;
		color: var(--text-primary);
		padding: 0 12px;
		height: 36px;
		border-radius: 4px;
		font-size: 16px;
		outline: none;
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

	.context-menu {
		position: fixed;
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
</style>
