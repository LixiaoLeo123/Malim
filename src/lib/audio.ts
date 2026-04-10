// import {
//   play as pluginPlay,
//   stop as pluginStop,
// } from "tauri-plugin-media-toolkit-api";

// let currentPlayerId: string | null = null;

// export function stopAudio() {
//   if (currentPlayerId) {
//     pluginStop().catch(() => {});
//     currentPlayerId = null;
//   }
// }

// export async function playAudio(localPath?: string | null) {
//   stopAudio();
//   if (!localPath) return;

//   await pluginPlay({
//     filePath: localPath,
//     volume: 1.0,
//   });

//   currentPlayerId = "default";
// }


// works on windows
import { convertFileSrc } from "@tauri-apps/api/core";

let audioPlayer: HTMLAudioElement | null = null;

export function stopAudio() {
  if (audioPlayer) {
    audioPlayer.pause();
    audioPlayer.currentTime = 0;
    audioPlayer = null;
  }
}

export async function playAudio(localPath?: string | null) {
  stopAudio();
  if (!localPath) return;

  const audioUrl = convertFileSrc(localPath);
  
  audioPlayer = new Audio(audioUrl);
  audioPlayer.volume = 1.0;
  
  await audioPlayer.play().catch(err => console.error("Error:", err));
}