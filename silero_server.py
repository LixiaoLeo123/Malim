# silero_server_mp3_hq.py
from pathlib import Path
from io import BytesIO

import torch
import lameenc
from fastapi import FastAPI, HTTPException, Response
from pydantic import BaseModel

MODEL_FILE = Path(__file__).parent / "model.pt"
SAMPLE_RATE = 48000
DEVICE = torch.device("cpu")
TORCH_THREADS = 4



print("Loading Silero model...")
if not MODEL_FILE.is_file():
    raise SystemExit(
        f"Model file does not exist: {MODEL_FILE}\n"
        "Please manually download v5_ru.pt and place it in the same directory, renaming it to model.pt"
    )

torch.set_num_threads(TORCH_THREADS)
model = torch.package.PackageImporter(MODEL_FILE).load_pickle("tts_models", "model")
model.to(DEVICE)
print("Model loaded.")



app = FastAPI(title="Silero TTS Server (ru) - High Quality MP3")

class TTSRequest(BaseModel):
    text: str
    speaker: str = "xenia"
    sample_rate: int = SAMPLE_RATE
    put_accent: bool = True
    put_yo: bool = True

def encode_to_mp3_hq(audio_tensor: torch.Tensor, sample_rate: int) -> bytes:
    audio_np = audio_tensor.cpu().numpy()
    audio_int16 = (audio_np * 32767).astype('int16')

    encoder = lameenc.Encoder()
    encoder.set_bit_rate(320)
    encoder.set_in_sample_rate(sample_rate)
    encoder.set_channels(1)
    encoder.set_quality(0)
    
    mp3_bytearray = encoder.encode(audio_int16.tobytes())
    mp3_bytearray += encoder.flush()
    
    return bytes(mp3_bytearray)

@app.post("/tts")
def tts(req: TTSRequest):
    if not req.text:
        raise HTTPException(status_code=400, detail="text must not be empty")

    try:
        with torch.no_grad():
            audio = model.apply_tts(
                text=req.text,
                speaker=req.speaker,
                sample_rate=req.sample_rate,
                put_accent=req.put_accent,
                put_yo=req.put_yo,
                put_stress_homo=True,
                put_yo_homo=True,
            )
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"TTS failed: {e}")

    mp3_data = encode_to_mp3_hq(audio, req.sample_rate)

    return Response(
        content=mp3_data,
        media_type="audio/mpeg",
        headers={"Content-Disposition": 'attachment; filename="silero_output_hq.mp3"'}
    )

@app.get("/health")
def health():
    return {"status": "ok"}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8001)
