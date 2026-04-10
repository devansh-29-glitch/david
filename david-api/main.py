"""
David v1 — Railway Backend
===========================
URL: https://david-api-production.up.railway.app

DEPENDENCIES: Only pre-built binary wheels. No C compiler needed.
- psycopg2-binary: PostgreSQL (binary, no gcc)
- redis: Redis client (pure Python)
- PyJWT: JWT tokens (pure Python)
- passlib[bcrypt]: Password hashing (binary wheels available)
- httpx: HTTP client for Gemini API
- fastapi + uvicorn: Web framework
"""

import os
import base64
import json
import threading
from datetime import datetime, timedelta
from typing import Optional

import psycopg2
import psycopg2.pool
import redis
import httpx
from fastapi import FastAPI, HTTPException, Depends, Header
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import bcrypt
from dotenv import load_dotenv
import jwt

load_dotenv()

app = FastAPI(title="David AI Backend", version="1.0.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Config ─────────────────────────────────────────────────────────────────────

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "")
GEMINI_BASE = "https://generativelanguage.googleapis.com/v1beta"
GEMINI_VISION_MODEL = "gemini-2.5-flash"
GEMINI_TTS_MODEL = "gemini-2.5-flash-preview-tts"

DATABASE_URL = os.getenv("DATABASE_URL", "")
REDIS_URL = os.getenv("REDIS_URL", "")
JWT_SECRET = os.getenv("JWT_SECRET", "fallback_secret_change_this")
JWT_ALGORITHM = "HS256"
JWT_EXPIRE_HOURS = 24 * 30  # 30 days

TIER_LIMITS = {
    "free":         {"unprompted": 25,     "chat": 10,     "rewrite": 5,    "voice_output": False},
    "starter":      {"unprompted": 50,     "chat": 25,     "rewrite": 15,   "voice_output": True},
    "professional": {"unprompted": 125,    "chat": 60,     "rewrite": 35,   "voice_output": True},
    "unlimited":    {"unprompted": 999999, "chat": 999999, "rewrite": 999999, "voice_output": True},
}

DAVID_SYSTEM = """You are David — an ambient AI presence that lives on the user's screen.

PERSONALITY:
- Witty, slightly sarcastic, always genuinely helpful. Like Jarvis talking to Stark.
- Direct. Say what you see. No "I notice that" or "It appears".
- Short by default. Long only when user needs real detail.
- Call the user "sir" unless they share their name.
- Never say "As an AI". You are David.

WHAT YOU SEE:
You receive screenshots of the user's screen in real time.
Always reference what you actually see. "I can see your Gmail inbox" not "Based on what you shared".

WHAT YOU DO:
- Guide users step by step through anything visible on screen
- Rewrite text in different tones when asked
- Speak unprompted when user is clearly stuck or confused
- Give instructions. You tell. User does.

UNPROMPTED RULES:
- Write-delete loop on same text = ask if they need help
- Same page 2+ minutes without progress = offer guidance
- Obvious mistake visible = point it out
- User clearly in flow = stay completely silent
- Never interrupt audio or video

RESPONSE LENGTH:
- Unprompted: max 2 sentences. Short. Punchy.
- Chat: as long as needed, no padding
- Rewrites: return ONLY the rewritten text. Nothing else.

GOOD examples:
"Four minutes on that DNS page. Want me to walk you through it?"
"You've deleted that paragraph three times. Want me to take a shot at it?"
"Line 47. Missing semicolon. Classic."

BAD examples (never do this):
"I notice you seem to be experiencing difficulty..."
"As an AI assistant, I am here to help..."
"""

# ── Database — synchronous psycopg2 with connection pool ─────────────────────
# Using psycopg2-binary (pre-built, no gcc needed) wrapped in a thread pool
# for non-blocking behavior in FastAPI async endpoints.

_db_pool: Optional[psycopg2.pool.ThreadedConnectionPool] = None
_pool_lock = threading.Lock()

def get_db_pool() -> psycopg2.pool.ThreadedConnectionPool:
    global _db_pool
    if _db_pool is None:
        with _pool_lock:
            if _db_pool is None:
                _db_pool = psycopg2.pool.ThreadedConnectionPool(
                    minconn=2,
                    maxconn=10,
                    dsn=DATABASE_URL,
                )
    return _db_pool


def db_execute(query: str, params=None, fetch: str = None):
    """
    Execute a database query synchronously.
    fetch: None (no result), 'one' (fetchone), 'all' (fetchall)
    """
    pool = get_db_pool()
    conn = pool.getconn()
    try:
        with conn.cursor() as cur:
            cur.execute(query, params)
            conn.commit()
            if fetch == 'one':
                row = cur.fetchone()
                if row and cur.description:
                    cols = [d[0] for d in cur.description]
                    return dict(zip(cols, row))
                return None
            elif fetch == 'all':
                rows = cur.fetchall()
                if rows and cur.description:
                    cols = [d[0] for d in cur.description]
                    return [dict(zip(cols, row)) for row in rows]
                return []
            return None
    except Exception as e:
        conn.rollback()
        raise e
    finally:
        pool.putconn(conn)


# ── Redis ──────────────────────────────────────────────────────────────────────

_redis_client: Optional[redis.Redis] = None

def get_redis() -> redis.Redis:
    global _redis_client
    if _redis_client is None:
        _redis_client = redis.from_url(REDIS_URL, decode_responses=True)
    return _redis_client


# ── Startup: create tables ────────────────────────────────────────────────────

@app.on_event("startup")
def startup():
    db_execute("""
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            tier VARCHAR(50) DEFAULT 'free',
            created_at TIMESTAMP DEFAULT NOW(),
            last_active TIMESTAMP DEFAULT NOW()
        )
    """)
    db_execute("""
        CREATE TABLE IF NOT EXISTS usage_logs (
            id SERIAL PRIMARY KEY,
            user_id INTEGER REFERENCES users(id),
            action_type VARCHAR(50) NOT NULL,
            date DATE DEFAULT CURRENT_DATE,
            count INTEGER DEFAULT 0,
            UNIQUE(user_id, action_type, date)
        )
    """)
    db_execute("""
        CREATE TABLE IF NOT EXISTS sessions (
            id SERIAL PRIMARY KEY,
            user_id INTEGER REFERENCES users(id),
            session_id VARCHAR(255) UNIQUE NOT NULL,
            conversation_history TEXT DEFAULT '[]',
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        )
    """)
    print("✓ David backend started. Tables ready.")


# ── Auth helpers ───────────────────────────────────────────────────────────────

import bcrypt


def hash_password(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()

def verify_password(plain: str, hashed: str) -> bool:
    return bcrypt.checkpw(plain.encode(), hashed.encode())


def create_token(user_id: int, email: str) -> str:
    payload = {
        "user_id": user_id,
        "email": email,
        "exp": datetime.utcnow() + timedelta(hours=JWT_EXPIRE_HOURS),
    }
    return jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)


def get_current_user(authorization: str = Header(...)):
    try:
        token = authorization.replace("Bearer ", "")
        payload = jwt.decode(token, JWT_SECRET, algorithms=[JWT_ALGORITHM])
        return payload
    except jwt.ExpiredSignatureError:
        raise HTTPException(status_code=401, detail="Token expired. Please log in again.")
    except jwt.InvalidTokenError:
        raise HTTPException(status_code=401, detail="Invalid token. Please log in again.")


# ── Usage tracking ─────────────────────────────────────────────────────────────

def check_and_increment_usage(user_id: int, action_type: str, tier: str) -> bool:
    limit = TIER_LIMITS.get(tier, TIER_LIMITS["free"]).get(action_type, 0)
    if limit == 999999:
        return True

    row = db_execute("""
        INSERT INTO usage_logs (user_id, action_type, date, count)
        VALUES (%s, %s, CURRENT_DATE, 0)
        ON CONFLICT (user_id, action_type, date)
        DO UPDATE SET count = usage_logs.count
        RETURNING count
    """, (user_id, action_type), fetch='one')

    current = row["count"] if row else 0
    if current >= limit:
        return False

    db_execute("""
        UPDATE usage_logs SET count = count + 1
        WHERE user_id = %s AND action_type = %s AND date = CURRENT_DATE
    """, (user_id, action_type))
    return True


def get_usage_today(user_id: int, tier: str) -> dict:
    rows = db_execute(
        "SELECT action_type, count FROM usage_logs WHERE user_id = %s AND date = CURRENT_DATE",
        (user_id,), fetch='all'
    ) or []
    limits = TIER_LIMITS.get(tier, TIER_LIMITS["free"])
    usage = {row["action_type"]: row["count"] for row in rows}
    return {
        "unprompted": {"used": usage.get("unprompted", 0), "limit": limits["unprompted"]},
        "chat":       {"used": usage.get("chat", 0),       "limit": limits["chat"]},
        "rewrite":    {"used": usage.get("rewrite", 0),    "limit": limits["rewrite"]},
        "voice_output": limits["voice_output"],
    }


# ── Gemini helpers ─────────────────────────────────────────────────────────────

async def call_gemini_vision(
    screenshot_b64: str,
    prompt: str,
    history: list,
    max_tokens: int = 512,
) -> str:
    contents = []
    for turn in history[-8:]:
        contents.append({"role": turn["role"], "parts": [{"text": turn["content"]}]})

    contents.append({
        "role": "user",
        "parts": [
            {"inline_data": {"mime_type": "image/jpeg", "data": screenshot_b64}},
            {"text": prompt},
        ]
    })

    payload = {
        "system_instruction": {"parts": [{"text": DAVID_SYSTEM}]},
        "contents": contents,
        "generationConfig": {
            "temperature": 0.85,
            "maxOutputTokens": max_tokens,
            "topP": 0.95,
        },
    }

    url = f"{GEMINI_BASE}/models/{GEMINI_VISION_MODEL}:generateContent?key={GEMINI_API_KEY}"
    async with httpx.AsyncClient(timeout=20.0) as client:
        resp = await client.post(url, json=payload)
        resp.raise_for_status()
        data = resp.json()
        return data["candidates"][0]["content"]["parts"][0]["text"].strip()


async def call_gemini_confusion_check(
    dwell_seconds: float,
    write_delete_count: int,
    activity_level: str,
    current_app: str,
    time_since_last_unprompted: float,
) -> dict:
    """Cheap text-only Gemini call. No screenshot. Runs every 4.3 seconds."""
    if time_since_last_unprompted < 45:
        return {"should_speak": False, "confidence": 0.0, "reason": "cooldown"}

    prompt = f"""Behavioral signals:
- Seconds on same screen: {dwell_seconds:.0f}
- Write-delete cycles: {write_delete_count}
- Activity: {activity_level}
- App: {current_app}
- Seconds since last David message: {time_since_last_unprompted:.0f}

Should David speak unprompted? Yes only if user is clearly confused or stuck.
Reply ONLY with JSON: {{"should_speak": true/false, "confidence": 0.0-1.0, "reason": "short phrase"}}"""

    payload = {
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {"temperature": 0.2, "maxOutputTokens": 60},
    }

    url = f"{GEMINI_BASE}/models/{GEMINI_VISION_MODEL}:generateContent?key={GEMINI_API_KEY}"
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            resp = await client.post(url, json=payload)
            resp.raise_for_status()
            data = resp.json()
            raw = data["candidates"][0]["content"]["parts"][0]["text"].strip()
            raw = raw.replace("```json", "").replace("```", "").strip()
            return json.loads(raw)
    except Exception:
        return {"should_speak": False, "confidence": 0.0, "reason": "check_failed"}


async def call_gemini_tts(text: str) -> Optional[bytes]:
    """Gemini 2.5 Flash TTS. Returns MP3 bytes or None."""
    payload = {
        "contents": [{"parts": [{"text": text}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {
                "voiceConfig": {"prebuiltVoiceConfig": {"voiceName": "Charon"}}
            }
        }
    }
    url = f"{GEMINI_BASE}/models/{GEMINI_TTS_MODEL}:generateContent?key={GEMINI_API_KEY}"
    try:
        async with httpx.AsyncClient(timeout=15.0) as client:
            resp = await client.post(url, json=payload)
            resp.raise_for_status()
            data = resp.json()
            audio_b64 = data["candidates"][0]["content"]["parts"][0]["inlineData"]["data"]
            return base64.b64decode(audio_b64)
    except Exception:
        return None


# ── Request models ─────────────────────────────────────────────────────────────

class RegisterRequest(BaseModel):
    name: str
    email: str
    password: str

class LoginRequest(BaseModel):
    email: str
    password: str

class ScreenshotRequest(BaseModel):
    screenshot_b64: str
    activity_level: str
    audio_playing: bool
    dwell_seconds: float
    write_delete_count: int
    current_app: str
    session_id: str
    time_since_last_unprompted: float

class ChatRequest(BaseModel):
    message: str
    screenshot_b64: Optional[str] = None
    session_id: str
    audio_playing: bool = False

class RewriteRequest(BaseModel):
    text: str
    instruction: str
    screenshot_b64: Optional[str] = None


# ── Auth endpoints ─────────────────────────────────────────────────────────────

@app.post("/auth/register")
async def register(req: RegisterRequest):
    if len(req.password) < 8:
        raise HTTPException(status_code=400, detail="Password must be at least 8 characters.")

    existing = db_execute(
        "SELECT id FROM users WHERE email = %s", (req.email.lower(),), fetch='one'
    )
    if existing:
        raise HTTPException(status_code=400, detail="An account with this email already exists.")

    user = db_execute(
        "INSERT INTO users (name, email, password_hash) VALUES (%s, %s, %s) RETURNING id, tier",
        (req.name, req.email.lower(), hash_password(req.password)),
        fetch='one'
    )

    token = create_token(user["id"], req.email.lower())
    return {
        "token": token,
        "user": {"id": user["id"], "name": req.name, "email": req.email, "tier": user["tier"]},
        "message": f"Welcome to David, {req.name.split()[0]}.",
    }


@app.post("/auth/login")
async def login(req: LoginRequest):
    user = db_execute(
        "SELECT id, name, email, password_hash, tier FROM users WHERE email = %s",
        (req.email.lower(),), fetch='one'
    )
    if not user or not verify_password(req.password, user["password_hash"]):
        raise HTTPException(status_code=401, detail="Incorrect email or password.")

    db_execute("UPDATE users SET last_active = NOW() WHERE id = %s", (user["id"],))
    token = create_token(user["id"], user["email"])
    return {
        "token": token,
        "user": {"id": user["id"], "name": user["name"], "email": user["email"], "tier": user["tier"]},
    }


@app.get("/auth/me")
async def get_me(user=Depends(get_current_user)):
    row = db_execute(
        "SELECT id, name, email, tier FROM users WHERE id = %s",
        (user["user_id"],), fetch='one'
    )
    if not row:
        raise HTTPException(status_code=404, detail="User not found.")
    usage = get_usage_today(user["user_id"], row["tier"])
    return {"user": row, "usage": usage}


# ── David endpoints ────────────────────────────────────────────────────────────

@app.post("/david/screenshot")
async def process_screenshot(req: ScreenshotRequest, user=Depends(get_current_user)):
    row = db_execute(
        "SELECT tier FROM users WHERE id = %s", (user["user_id"],), fetch='one'
    )
    tier = row["tier"] if row else "free"

    # Step 1: cheap confusion check (no screenshot, ~50 tokens)
    confusion = await call_gemini_confusion_check(
        dwell_seconds=req.dwell_seconds,
        write_delete_count=req.write_delete_count,
        activity_level=req.activity_level,
        current_app=req.current_app,
        time_since_last_unprompted=req.time_since_last_unprompted,
    )

    if not confusion.get("should_speak") or confusion.get("confidence", 0) < 0.75:
        return {"should_speak": False, "message": "", "mode": "silent"}

    allowed = check_and_increment_usage(user["user_id"], "unprompted", tier)
    if not allowed:
        return {"should_speak": False, "message": "", "mode": "limit_reached"}

    # Step 2: full vision response
    reason = confusion.get("reason", "")
    if "write" in reason or "delete" in reason:
        prompt = "The user has been writing and deleting the same text repeatedly. Look at what they're writing and offer help briefly and wittily. Max 2 sentences."
    elif "dwell" in reason:
        prompt = f"The user has been on this screen for {req.dwell_seconds:.0f} seconds. If something looks clearly confusing or stuck, offer help. If everything looks normal, respond with exactly: STAY_SILENT"
    else:
        prompt = "Look at this screen. Is there something the user clearly needs help with? If yes, 1-2 sentences. If no, respond with exactly: STAY_SILENT"

    session_row = db_execute(
        "SELECT conversation_history FROM sessions WHERE session_id = %s",
        (req.session_id,), fetch='one'
    )
    history = json.loads(session_row["conversation_history"]) if session_row else []

    message = await call_gemini_vision(
        screenshot_b64=req.screenshot_b64,
        prompt=prompt,
        history=history,
        max_tokens=150,
    )

    if "STAY_SILENT" in message:
        return {"should_speak": False, "message": "", "mode": "silent"}

    voice_enabled = TIER_LIMITS[tier]["voice_output"]
    mode = "text" if req.audio_playing or not voice_enabled else "voice"

    audio_b64 = None
    if mode == "voice":
        audio_bytes = await call_gemini_tts(message)
        if audio_bytes:
            audio_b64 = base64.b64encode(audio_bytes).decode()
        else:
            mode = "text"

    return {"should_speak": True, "message": message, "mode": mode, "audio_b64": audio_b64}


@app.post("/david/chat")
async def chat(req: ChatRequest, user=Depends(get_current_user)):
    row = db_execute("SELECT tier FROM users WHERE id = %s", (user["user_id"],), fetch='one')
    tier = row["tier"] if row else "free"

    allowed = check_and_increment_usage(user["user_id"], "chat", tier)
    if not allowed:
        raise HTTPException(status_code=429, detail="Daily chat limit reached. Upgrade for more.")

    session_row = db_execute(
        "SELECT conversation_history FROM sessions WHERE session_id = %s",
        (req.session_id,), fetch='one'
    )
    history = json.loads(session_row["conversation_history"]) if session_row else []

    if not req.screenshot_b64:
        raise HTTPException(status_code=400, detail="Screenshot required.")

    response = await call_gemini_vision(
        screenshot_b64=req.screenshot_b64,
        prompt=req.message,
        history=history,
        max_tokens=1024,
    )

    history.append({"role": "user", "content": req.message})
    history.append({"role": "model", "content": response})
    if len(history) > 30:
        history = history[-20:]

    db_execute("""
        INSERT INTO sessions (user_id, session_id, conversation_history, updated_at)
        VALUES (%s, %s, %s, NOW())
        ON CONFLICT (session_id)
        DO UPDATE SET conversation_history = EXCLUDED.conversation_history, updated_at = NOW()
    """, (user["user_id"], req.session_id, json.dumps(history)))

    voice_enabled = TIER_LIMITS[tier]["voice_output"]
    mode = "text" if req.audio_playing or not voice_enabled else "voice"

    audio_b64 = None
    if mode == "voice":
        audio_bytes = await call_gemini_tts(response)
        if audio_bytes:
            audio_b64 = base64.b64encode(audio_bytes).decode()
        else:
            mode = "text"

    return {"message": response, "audio_b64": audio_b64, "mode": mode}


@app.post("/david/rewrite")
async def rewrite(req: RewriteRequest, user=Depends(get_current_user)):
    row = db_execute("SELECT tier FROM users WHERE id = %s", (user["user_id"],), fetch='one')
    tier = row["tier"] if row else "free"

    allowed = check_and_increment_usage(user["user_id"], "rewrite", tier)
    if not allowed:
        raise HTTPException(status_code=429, detail="Daily rewrite limit reached. Upgrade for more.")

    prompt = f"""The user selected this text:
---
{req.text}
---
Their instruction: "{req.instruction}"
Rewrite it exactly as instructed. Return ONLY the rewritten text. Nothing else."""

    if req.screenshot_b64:
        result = await call_gemini_vision(
            screenshot_b64=req.screenshot_b64,
            prompt=prompt, history=[], max_tokens=2048,
        )
    else:
        payload = {
            "system_instruction": {"parts": [{"text": DAVID_SYSTEM}]},
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "generationConfig": {"temperature": 0.7, "maxOutputTokens": 2048},
        }
        url = f"{GEMINI_BASE}/models/{GEMINI_VISION_MODEL}:generateContent?key={GEMINI_API_KEY}"
        async with httpx.AsyncClient(timeout=20.0) as client:
            resp = await client.post(url, json=payload)
            data = resp.json()
            result = data["candidates"][0]["content"]["parts"][0]["text"].strip()

    return {"rewritten": result}


@app.post("/david/reset")
async def reset_session(session_id: str, user=Depends(get_current_user)):
    db_execute(
        "UPDATE sessions SET conversation_history = '[]' WHERE session_id = %s AND user_id = %s",
        (session_id, user["user_id"])
    )
    return {"status": "reset"}


@app.get("/health")
async def health():
    db_ok = False
    try:
        db_execute("SELECT 1", fetch='one')
        db_ok = True
    except Exception:
        pass

    return {
        "status": "ok",
        "service": "david-api",
        "gemini_configured": bool(GEMINI_API_KEY),
        "database_connected": db_ok,
        "timestamp": datetime.utcnow().isoformat(),
    }
