import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from ruaccent import RUAccent

app = FastAPI(title="Russian Accentizer API")


accentizer = RUAccent()
accentizer.load(omograph_model_size='turbo', dict_load_type='full')

class TextRequest(BaseModel):
    text: str

class TextResponse(BaseModel):
    accented_text: str

@app.post("/accentize", response_model=TextResponse)
async def accentize_text(req: TextRequest):
    try:
        result = accentizer.process_all(req.text)
        return TextResponse(accented_text=result)
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=8002)