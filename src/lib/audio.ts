import { type } from "@tauri-apps/plugin-os";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  play as pluginPlay,
  stop as pluginStop,
} from "tauri-plugin-media-toolkit-api";

let osType: string | null = null;

function getOsType() {
  if (osType === null) {
    try {
      osType = type();
    } catch {
      osType = "unknown";
    }
  }

  return osType;
}

let currentPlayerId: string | null = null;
let audioPlayer: HTMLAudioElement | null = null;

export function stopAudio() {
  if (currentPlayerId) {
    pluginStop().catch(() => {});
    currentPlayerId = null;
    return;
  }

  if (audioPlayer) {
    audioPlayer.pause();
    audioPlayer.currentTime = 0;
    audioPlayer = null;
  }
}

export async function playAudio(localPath?: string | null) {
  stopAudio();
  if (!localPath) return;

  const osType = getOsType();

  if (osType === "linux") {
    await pluginPlay({
      filePath: localPath,
      volume: 1.0,
    });
    currentPlayerId = "default";
  } else {
    const audioUrl = convertFileSrc(localPath);
    
    audioPlayer = new Audio(audioUrl);
    audioPlayer.volume = 1.0;
    
    await audioPlayer.play().catch(err => console.error("Error:", err));
  }
}