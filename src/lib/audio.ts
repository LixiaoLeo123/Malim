import {
  play as pluginPlay,
  stop as pluginStop,
} from "tauri-plugin-media-toolkit-api";

let currentPlayerId: string | null = null;

export function stopAudio() {
  if (currentPlayerId) {
    pluginStop().catch(() => {});
    currentPlayerId = null;
  }
}

export async function playAudio(localPath?: string | null) {
  stopAudio();
  if (!localPath) return;

  await pluginPlay({
    filePath: localPath,
    volume: 1.0,
  });

  currentPlayerId = "default";
}