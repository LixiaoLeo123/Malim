import threading
from pathlib import Path
import re

import torch
import lameenc
import numpy as np
import soundfile as sf
from fastapi import FastAPI, HTTPException, Response
from pydantic import BaseModel

DEEPFILTER_EXE = Path(__file__).parent / "deep-filter.exe"
if not DEEPFILTER_EXE.exists():
    print("WARNING: deep-filter.exe not found, denoising will be skipped.")

TEMP_DIR = Path(__file__).parent / "temp_wav"
TEMP_DIR.mkdir(exist_ok=True)

# https://github.com/snakers4/silero-models
# This file can be downloaded from https://models.silero.ai/models/tts/ru/v5_2_ru.pt
MODEL_FILE = Path(__file__).parent / "v5_2_ru.pt"
SAMPLE_RATE = 48000
DEVICE = torch.device("cpu")
TORCH_THREADS = 4

TEMP_INPUT_WAV = TEMP_DIR / "input.wav"
TEMP_OUTPUT_WAV = TEMP_DIR / "output.wav"
DF_LOCK = threading.Lock()


print("Loading Silero model...")
if not MODEL_FILE.exists():
    raise SystemExit(
        f"Model file does not exist: {MODEL_FILE}\n"
        "Please download v5_2_ru.pt and rename it to v5_2_ru.pt in this directory."
    )

torch.set_num_threads(TORCH_THREADS)
model = torch.package.PackageImporter(MODEL_FILE).load_pickle("tts_models", "model")
model.to(DEVICE)
print("Silero model loaded.")


def sanitize_russian_text(text: str) -> str:
    translit_map = {
        "email": "имейл",
        "gmail": "джимейл",
        "facebook": "фейсбук",
        "twitter": "твиттер",
        "instagram": "инстаграм",
        "whatsapp": "ватсап",
        "telegram": "телеграм",
        "youtube": "ютуб",
        "google": "гугл",
        "yandex": "яндекс",
        "apple": "эппл",
        "microsoft": "майкрософт",
        "windows": "виндовс",
        "android": "андроид",
        "iphone": "айфон",
        "ipad": "айпад",
        "mac": "мак",
        "pc": "пк",
        "pdf": "пдф",
        "doc": "док",
        "docx": "докс",
        "xls": "иксэль",
        "xlsx": "иксэль",
        "ppt": "поверпоинт",
        "mp3": "мптри",
        "mp4": "мпчетыре",
        "url": "урл",
        "www": "ввв",
        "http": "ашттп",
        "https": "ашттпэс",
        "com": "ком",
        "ru": "ру",
        "org": "орг",
        "net": "нет",
    }
    
    lower_text = text.lower()
    
    for eng, rus in translit_map.items():
        if eng in lower_text:
            start = lower_text.find(eng)
            end = start + len(eng)
            original_sub = text[start:end]
            
            if original_sub.isupper():
                replacement = rus.upper()
            elif original_sub[0].isupper():
                replacement = rus.capitalize()
            else:
                replacement = rus
            
            text = text[:start] + replacement + text[end:]
            lower_text = text.lower()

    # Only Russian letters are allowed for this model
    text = re.sub(r'[a-zA-Z]', '', text)
    
    text = re.sub(r'\s+', ' ', text).strip()
    
    return text


app = FastAPI(title="Silero TTS Server (ru) - Text Sanitizer + HQ MP3")

class TTSRequest(BaseModel):
    text: str
    speaker: str = "xenia"
    sample_rate: int = SAMPLE_RATE
    put_accent: bool = True
    put_yo: bool = True

def encode_to_mp3_hq(audio_np: np.ndarray, sample_rate: int) -> bytes:
    max_val = np.max(np.abs(audio_np))
    if max_val > 1.0:
        audio_np = audio_np / max_val
    audio_int16 = (audio_np * 32767).astype('int16')
    encoder = lameenc.Encoder()
    encoder.set_bit_rate(320)
    encoder.set_in_sample_rate(sample_rate)
    encoder.set_channels(1)
    encoder.set_quality(0)
    mp3_bytes = encoder.encode(audio_int16.tobytes())
    mp3_bytes += encoder.flush()
    return bytes(mp3_bytes)

# DeepFilter doesn't work well, so temporarily disable it(
def denoise_with_deepfilter(audio_np: np.ndarray, sample_rate: int) -> np.ndarray:
    return audio_np

@app.post("/tts")
def tts(req: TTSRequest):
    if not req.text:
        raise HTTPException(status_code=400, detail="text must not be empty")

    try:
        clean_text = sanitize_russian_text(req.text)
        
        if not clean_text:
            raise HTTPException(status_code=400, detail="Text contains no processable Russian characters after cleaning.")

        print(f"Original: {req.text}")
        print(f"Sanitized: {clean_text}")

        with torch.no_grad():
            audio_tensor = model.apply_tts(
                text=clean_text,
                speaker=req.speaker,
                sample_rate=req.sample_rate,
                put_accent=req.put_accent,
                put_yo=req.put_yo,
                put_stress_homo=True,
                put_yo_homo=True,
            )

        audio_np = audio_tensor.cpu().numpy()

        clean_np = audio_np

        mp3_data = encode_to_mp3_hq(clean_np, req.sample_rate)

    except Exception as e:
        import traceback
        traceback.print_exc()
        raise HTTPException(status_code=500, detail=f"TTS failed: {e}")

    return Response(
        content=mp3_data,
        media_type="audio/mpeg",
        headers={
            "Content-Disposition": 'attachment; filename="silero_output_hq.mp3"'
        }
    )

@app.get("/health")
def health():
    return {"status": "ok"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)
