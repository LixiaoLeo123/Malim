import re
from typing import List
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from ruaccent import RUAccent

app = FastAPI(title="Russian Accentizer API")

accentizer = RUAccent()
accentizer.load(omograph_model_size='turbo3.1', use_dictionary=True)

class TextRequest(BaseModel):
    text: str

class TextResponse(BaseModel):
    accented_text: str


CYRILLIC_LETTER_RE = re.compile(r'[\u0400-\u04FF]')
PUNCTUATION_SPLIT_RE = re.compile(r'([.,;:!?\-—…«»""()\[\]])')

def convert_plus_to_acute(text: str) -> str:
   result = []
   i = 0
   while i < len(text):
       if text[i] == '+' and i + 1 < len(text):
           result.append(text[i + 1] + '\u0301')
           i += 2
       else:
           result.append(text[i])
           i += 1
   return ''.join(result)

def is_cyrillic_char(ch: str) -> bool:
    return bool(CYRILLIC_LETTER_RE.match(ch))

def split_text_by_cyrillic(text: str) -> List[tuple]:
    if not text:
        return []

    fragments = []
    current_frag = []
    current_is_cyrillic = is_cyrillic_char(text[0])

    for ch in text:
        ch_is_cyrillic = is_cyrillic_char(ch)
        if ch_is_cyrillic == current_is_cyrillic:
            current_frag.append(ch)
        else:
            fragments.append((''.join(current_frag), current_is_cyrillic))
            current_frag = [ch]
            current_is_cyrillic = ch_is_cyrillic

    if current_frag:
        fragments.append((''.join(current_frag), current_is_cyrillic))

    return fragments


def process_cyrillic_with_fallback(text: str) -> str:
    if not text.strip():
        return text
    
    try:
        return accentizer.process_all(text)
    except Exception:
        pass

    parts = PUNCTUATION_SPLIT_RE.split(text)
    res_parts = []
    
    for part in parts:
        if not part:
            continue
        try:
            res_parts.append(accentizer.process_all(part))
        except Exception:
            words = part.split(' ')
            safe_words = []
            for word in words:
                try:
                    safe_words.append(accentizer.process_all(word))
                except Exception:
                    safe_words.append(word)
            res_parts.append(' '.join(safe_words))

    return ''.join(res_parts)


def accentize_text_safe(text: str, accentizer) -> str:
    if not text:
        return ""
    fragments = split_text_by_cyrillic(text)
    result_parts = []

    for frag, is_cyrillic in fragments:
        if is_cyrillic:
            accented = process_cyrillic_with_fallback(frag)
            result_parts.append(accented)
        else:
            result_parts.append(frag)

    return convert_plus_to_acute(''.join(result_parts))


@app.post("/accentize", response_model=TextResponse)
async def accentize_text(req: TextRequest):
    try:
        result = accentize_text_safe(req.text, accentizer)
        return TextResponse(accented_text=result)
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8002)
