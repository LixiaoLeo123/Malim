<script lang="ts">
    import {
        popView, editorDraft, articles,
        activeArticleId, settings,
        parsingQueue,
    } from "../lib/stores";
    import { ArrowLeft, Check, ChevronDown, X } from "lucide-svelte";
    import { slide } from "svelte/transition";
    import { v4 as uuidv4 } from "uuid";
    import Flag from "./Flag.svelte";
    import type { Article, ImageAttachment, ImageParticle } from "../lib/types";
    import { processQueue } from "../lib/parser";
    import { onDestroy } from "svelte";
    import { notifications } from '../lib/notificationStore';
    import { invoke } from '@tauri-apps/api/core';

    let showLangSelector = false;
    $: wordCount = !$editorDraft.content?.trim() ? 0 : $editorDraft.content.trim().split(/\s+/).length;
    $: canSubmit = Boolean($editorDraft.content?.trim() || ($editorDraft.images || []).length);

    function goBack() {
        popView();
    }
    onDestroy(() => {
        // if (unlisten) unlisten();
    });

    function handlePaste(e: ClipboardEvent) {
        const items = e.clipboardData?.items;
        if (!items) return;

        const imageItems = Array.from(items).filter(item => item.type.startsWith('image/'));
        if (imageItems.length > 0) {
            e.preventDefault();
            for (const item of imageItems) {
                const file = item.getAsFile();
                if (!file) continue;
                const reader = new FileReader();
                reader.onload = (ev) => {
                    const dataUrl = ev.target?.result as string;
                    const newImage: ImageAttachment = {
                        id: uuidv4(),
                        dataUrl,
                        mimeType: file.type,
                        fileName: file.name || `pasted_image_${Date.now()}.png`,
                        extractedText: '',
                        ocrStatus: 'pending',
                    };
                    editorDraft.update(d => ({
                        ...d,
                        images: [...(d.images || []), newImage],
                    }));
                };
                reader.readAsDataURL(file);
            }
            return;
        }

        const htmlItem = Array.from(items).find(item => item.type === 'text/html');
        if (htmlItem) {
            htmlItem.getAsString((html) => {
                const imgMatch = html.match(/<img[^>]+src=["']([^"']+)["']/i);
                if (imgMatch) {
                    e.preventDefault();
                    const src = imgMatch[1];
                    if (src.startsWith('data:')) {
                        const mimeMatch = src.match(/^data:([^;]+)/);
                        const mimeType = mimeMatch ? mimeMatch[1] : 'image/png';
                        const newImage: ImageAttachment = {
                            id: uuidv4(),
                            dataUrl: src,
                            mimeType,
                            fileName: `pasted_image_${Date.now()}.png`,
                            extractedText: '',
                            ocrStatus: 'pending',
                        };
                        editorDraft.update(d => ({
                            ...d,
                            images: [...(d.images || []), newImage],
                        }));
                    } else if (src.startsWith('http')) {
                        invoke<string>('fetch_image_as_base64', { url: src })
                            .then((base64) => {
                                const extMatch = src.match(/\.(png|jpg|jpeg|gif|webp|bmp)(\?|$)/i);
                                const ext = extMatch ? extMatch[1].toLowerCase() : 'png';
                                const mimeMap: Record<string, string> = {
                                    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg',
                                    gif: 'image/gif', webp: 'image/webp', bmp: 'image/bmp'
                                };
                                const mimeType = mimeMap[ext] || 'image/png';
                                const dataUrl = `data:${mimeType};base64,${base64}`;
                                const newImage: ImageAttachment = {
                                    id: uuidv4(),
                                    dataUrl,
                                    mimeType,
                                    fileName: `pasted_image_${Date.now()}.${ext}`,
                                    extractedText: '',
                                    ocrStatus: 'pending',
                                };
                                editorDraft.update(d => ({
                                    ...d,
                                    images: [...(d.images || []), newImage],
                                }));
                            })
                            ;
                    }
                }
            });
        }
    }

    function removeImage(id: string) {
        editorDraft.update(d => ({
            ...d,
            images: d.images ? d.images.filter(img => img.id !== id) : [],
        }));
    }

    async function handleConfirm() {
        const contentSnapshot = $editorDraft.content;
        const languageSnapshot = $editorDraft.language;
        const imagesSnapshot = $editorDraft.images || [];


        if (!contentSnapshot.trim() && imagesSnapshot.length === 0) return;

        if (!$settings.defaultAiConfigId) {
            notifications.warning("Default AI configuration not found.");
            return;
        }

        const isEditMode = !!$activeArticleId;
        const id = isEditMode && $activeArticleId ? $activeArticleId : uuidv4();

        const existingArticle = isEditMode ? $articles.find(a => a.id === id) : null;

        let fullContent = contentSnapshot;
        const imageParticles: ImageParticle[] = [];
        imagesSnapshot.forEach((img, idx) => {
            const placeholder = `[image:${img.id}]`;
            fullContent += '\n' + placeholder;
            imageParticles.push({
                attachmentId: img.id,
                dataUrl: img.dataUrl,
                extractedText: '',
                index: idx,
                fileName: img.fileName,
            });
        });

        const firstSentenceEnd = fullContent.search(/[.。!?]\n?/);
        const newArticle: Article = {
            id: id,
            title:
                firstSentenceEnd !== -1
                    ? fullContent.slice(0, firstSentenceEnd + 1) +
                      (fullContent.length > firstSentenceEnd + 1
                          ? "..."
                          : "")
                    : fullContent.slice(0, 20) +
                      (fullContent.length > 20 ? "..." : ""),
            preview:
                firstSentenceEnd !== -1
                    ? fullContent.slice(firstSentenceEnd + 1).slice(0, 50)
                    : fullContent.slice(0, 50),
            status: "parsing",
            parsingProgress: 0,
            sentences: existingArticle?.sentences || [],
            imageParticles: imageParticles,
            draftContent: fullContent,
            language: languageSnapshot,
            readProgress: existingArticle?.readProgress || 0,
            completedCheckpointsList: existingArticle?.completedCheckpointsList || [],
            stared: existingArticle?.stared || false,
            tags: existingArticle?.tags || [],
        };

        if (isEditMode) {
            articles.update((items) =>
                items.map((i) => (i.id === id ? newArticle : i)),
            );
        } else {
            articles.update((items) => [newArticle, ...items]);
        }

        editorDraft.set({ title: "", content: "", language: "RU", images: [] });
        popView();
        parsingQueue.update((q) => [...q, id]);
        processQueue();
        notifications.success(isEditMode ? "Article updated!" : "Article created!");
    }
</script>

<div class="flex flex-col h-full bg-[#fffefe] relative dark:bg-[#17141d]">
    <div
        class="grid grid-cols-[auto_1fr_auto] items-center gap-3 border-b border-violet-100/80 bg-white/75 p-4 backdrop-blur-xl dark:border-white/10 dark:bg-[#17141d]/85"
    >
        <button
            type="button"
            aria-label="Back"
            on:click={goBack}
            class="rounded-2xl border border-violet-100 bg-white/80 p-2.5 text-zinc-600 shadow-sm transition duration-100 ease-out hover:bg-white active:scale-95 dark:border-white/10 dark:bg-white/5 dark:text-zinc-300 dark:hover:bg-white/10"
        >
            <ArrowLeft size={24} />
        </button>

        <div class="min-w-0 text-center">
            <h1 class="truncate text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                {$activeArticleId ? "Edit reading" : "New reading"}
            </h1>
        </div>

        <div class="relative z-50">
            <button
                type="button"
                aria-haspopup="menu"
                aria-expanded={showLangSelector}
                on:click={() => (showLangSelector = !showLangSelector)}
                class="flex items-center space-x-2 px-2 py-1.5 bg-zinc-100 rounded-lg text-sm font-medium hover:bg-zinc-200 transition text-zinc-700 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
            >
                <Flag code={$editorDraft.language} size={20} />
                <span
                    >{$editorDraft.language === "KR"
                        ? "Korean"
                        : $editorDraft.language === "ES"
                        ? "Spanish"
                        : "Russian"}</span
                >
                <ChevronDown
                    size={14}
                    class="transition-transform {showLangSelector
                        ? 'rotate-180'
                        : ''}"
                />
            </button>

            {#if showLangSelector}
                <div
                    role="menu"
                    aria-label="Reading language"
                    transition:slide
                    class="absolute top-full right-0 mt-2 w-40 bg-white border border-zinc-200 rounded-xl shadow-xl overflow-hidden dark:bg-zinc-900 dark:border-zinc-700"
                >
                    <button
                        type="button"
                        role="menuitem"
                        class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                        on:click={() => {
                            $editorDraft.language = "KR";
                            showLangSelector = false;
                        }}
                    >
                        <Flag code="KR" size={18} />
                        <span>Korean</span>
                    </button>
                    <button
                        type="button"
                        role="menuitem"
                        class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                        on:click={() => {
                            $editorDraft.language = "RU";
                            showLangSelector = false;
                        }}
                    >
                        <Flag code="RU" size={18} />
                        <span>Russian</span>
                    </button>
                    <button
                        type="button"
                        role="menuitem"
                        class="w-full text-left px-4 py-2 hover:bg-zinc-50 flex items-center space-x-3 text-sm text-zinc-700 dark:hover:bg-zinc-800 dark:text-zinc-200"
                        on:click={() => {
                            $editorDraft.language = "ES";
                            showLangSelector = false;
                        }}
                    >
                        <Flag code="ES" size={18} />
                        <span>Spanish</span>
                    </button>
                </div>
            {/if}
        </div>
    </div>

    <div class="flex-1 flex flex-col overflow-hidden">
        <textarea
            aria-label="Reading text"
            class="flex-1 w-full p-6 text-lg resize-none outline-none text-zinc-800 placeholder:text-zinc-300 leading-relaxed dark:text-zinc-100 dark:placeholder:text-zinc-600 dark:bg-transparent"
            placeholder="Paste your text here... (you can also paste images)"
            bind:value={$editorDraft.content}
            on:paste={handlePaste}
        ></textarea>
    </div>

    {#if ($editorDraft.images || []).length > 0}
        <div class="border-t border-zinc-100 dark:border-zinc-800">
            <div class="px-6 py-3 space-y-3">
                <div class="text-xs font-medium text-zinc-400 uppercase tracking-wider">Attached Images</div>
                <div class="flex flex-wrap gap-3">
                    {#each ($editorDraft.images || []) as img (img.id)}
                        <div class="relative group">
                            <img
                                src={img.dataUrl}
                                alt={img.fileName}
                                class="w-28 h-28 object-cover rounded-xl border border-zinc-200 dark:border-zinc-700 shadow-sm"
                            />
                            <button
                                type="button"
                                aria-label={`Remove ${img.fileName}`}
                                on:click={() => removeImage(img.id)}
                                class="absolute -top-2 -right-2 w-6 h-6 bg-red-500 text-white rounded-full flex items-center justify-center opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100 transition-opacity shadow-md hover:bg-red-600"
                            >
                                <X size={14} />
                            </button>
                            <div class="absolute bottom-1 left-1 right-1 bg-black/50 text-white text-[10px] px-1.5 py-0.5 rounded text-center truncate opacity-0 group-hover:opacity-100 transition-opacity">
                                {img.fileName}
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}

    <div
        class="flex items-center justify-between gap-4 border-t border-violet-100/80 bg-white/80 p-4 backdrop-blur-xl dark:border-white/10 dark:bg-[#17141d]/85"
    >
        <div class="flex min-w-0 items-center gap-2 text-xs text-zinc-400">
            <span>{wordCount} {wordCount === 1 ? "word" : "words"}</span>
            {#if ($editorDraft.images || []).length > 0}
                <span aria-hidden="true">·</span>
                <span>{($editorDraft.images || []).length} {($editorDraft.images || []).length === 1 ? "image" : "images"}</span>
            {/if}
        </div>
        <button
            type="button"
            disabled={!canSubmit}
            on:click={handleConfirm}
            class="flex items-center gap-2 rounded-full px-4 py-2.5 text-sm font-semibold transition shadow-lg {canSubmit ? 'bg-violet-600 text-white shadow-violet-500/20 hover:bg-violet-700 hover:shadow-xl active:scale-95 dark:bg-violet-500 dark:text-white dark:hover:bg-violet-400' : 'cursor-not-allowed bg-zinc-200 text-zinc-400 shadow-none dark:bg-zinc-800 dark:text-zinc-600'}"
        >
            <Check size={18} aria-hidden="true" />
            <span>{$activeArticleId ? "Update" : "Process text"}</span>
        </button>
    </div>
</div>
