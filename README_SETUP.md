# David v1 — Complete Setup Guide

This is your single source of truth.
Read it fully before doing anything.
Every known error has a fix documented here.

---

# TABLE OF CONTENTS

1. What changed from the previous version
2. Railway backend deployment (Step A)
3. Local development setup (Step B)
4. Building the Mac DMG via GitHub Actions (Step C)
5. Testing the DMG on a Mac (Step D)
6. Setting up the website (Step E)
7. Troubleshooting — every error you might hit
8. How David works (architecture)

---

# 1. WHAT CHANGED FROM PREVIOUS VERSION

The backend was completely rewritten.

**Problem:** The previous version used `asyncpg` which requires a C compiler (`gcc`) to build on Railway. Railway's Python container does not have gcc by default.

**Fix:** Replaced `asyncpg` with `psycopg2-binary` which is a pre-compiled binary package. It installs instantly with zero C compilation. The `requirements.txt` now contains ONLY packages that have pre-built binary wheels — meaning Railway will never fail on a missing C compiler again.

No changes to the Tauri app, orb UI, auth screen, or GitHub Actions workflow.

---

# 2. RAILWAY BACKEND DEPLOYMENT (Step A)

This is the first thing to do. Everything else depends on it.

## A1. Push the code to GitHub

Open Command Prompt. Run these commands:

```
cd C:\Users\hp\Desktop\david-v1-final\david-v1-final
git add david-api/main.py david-api/requirements.txt david-api/runtime.txt
git commit -m "fix: use psycopg2-binary, no C compiler needed"
git push
```

Wait 10 seconds. Go to https://railway.app

## A2. Watch the deployment

Click your Railway project → click `david-api` service → click `Deployments` tab.

You will see a new deployment triggered by your push.

Watch the build logs. The install step should now show:
```
Collecting psycopg2-binary
  Downloading psycopg2_binary-2.9.9...
  Installing collected packages: psycopg2-binary
Successfully installed psycopg2-binary
```

NO errors about gcc, cc, or failed to build wheels.

The deployment takes 2-3 minutes total. When complete, you will see:
```
✓ David backend started. Tables ready.
Uvicorn running on http://0.0.0.0:PORT
```

## A3. Test the backend is alive

Open a browser. Go to:
```
https://david-api-production.up.railway.app/health
```

You must see JSON like this:
```json
{
  "status": "ok",
  "service": "david-api",
  "gemini_configured": true,
  "database_connected": true,
  "timestamp": "2026-04-11T..."
}
```

If `gemini_configured` is `false`:
→ Your GEMINI_API_KEY in Railway Variables is wrong. Go to Railway → david-api → Variables → check it starts with `AIza`.

If `database_connected` is `false`:
→ Your DATABASE_URL variable is wrong. Go to Railway → david-api → Variables → set `DATABASE_URL` to exactly `${{ Postgres.DATABASE_URL }}` (with the dollar sign and double braces).

If the page returns a 502 error:
→ The app crashed on startup. Go to Railway → david-api → Deployments → click the deployment → View logs → look for the red error line.

## A4. Test auth works

In your browser address bar, go to:
```
https://david-api-production.up.railway.app/docs
```

This is the automatic API documentation. You can test endpoints here.
Click POST /auth/register → Try it out → fill in name/email/password → Execute.
You should get a 200 response with a token.

If this works, the backend is fully operational.

---

# 3. LOCAL DEVELOPMENT SETUP (Step B)

This lets you test David's UI on your Windows machine before building the Mac DMG.

## B1. Install required tools (one time only)

Do all of these in order. Do not skip any.

**Rust**
1. Go to https://rustup.rs
2. Download `rustup-init.exe`
3. Run it. When it asks about installation type, press Enter (default)
4. When done, CLOSE and REOPEN your terminal
5. Type `rustc --version` — must show a version number

**Bun**
1. Open PowerShell as Administrator (right-click → Run as administrator)
2. Run: `powershell -c "irm bun.sh/install.ps1 | iex"`
3. When done, close and reopen terminal
4. Type `bun --version` — must show a version number

**Visual Studio Build Tools**
This is required for Rust to compile on Windows. Without it, nothing will compile.
1. Go to https://visualstudio.microsoft.com/visual-cpp-build-tools/
2. Click "Download Build Tools"
3. Run the installer
4. In the installer, find "Desktop development with C++" and check the box
5. Click Install (downloads ~3-4GB, takes 15-20 minutes)
6. After install, RESTART your computer

**Python 3.11**
1. Go to https://python.org/downloads
2. Download Python 3.11.x (not 3.12 or 3.13)
3. Run installer
4. IMPORTANT: Check "Add Python to PATH" before clicking Install
5. After: type `python --version` — must show 3.11.x

## B2. Install JS dependencies

```
cd C:\Users\hp\Desktop\david-v1-final\david-v1-final\david-app
bun install
```

This takes 30-60 seconds. You should see packages being downloaded.

## B3. Run the backend locally (optional but faster for testing)

Open a new terminal window. Run:

```
cd C:\Users\hp\Desktop\david-v1-final\david-v1-final\david-api
pip install -r requirements.txt
```

Create a `.env` file in the `david-api` folder:
```
GEMINI_API_KEY=your_actual_gemini_key_here
DATABASE_URL=your_railway_postgres_url
REDIS_URL=your_railway_redis_url
JWT_SECRET=david_neuramind_secret_xk9p2025_hearty_bravery_prod
```

You can get your DATABASE_URL and REDIS_URL from Railway:
- Railway → your project → click Postgres → Connect tab → Public Network → copy Connection URL
- Railway → your project → click Redis → Connect tab → Public Network → copy Connection URL

Then run:
```
python main.py
```

You should see:
```
✓ David backend started. Tables ready.
INFO: Uvicorn running on http://127.0.0.1:8000
```

## B4. Change backend URL for local testing

Open `david-app\src-tauri\src\lib.rs` in any text editor.
Find this line:
```rust
pub const BACKEND_URL: &str = "https://david-api-production.up.railway.app";
```
Change it to:
```rust
pub const BACKEND_URL: &str = "http://127.0.0.1:8000";
```
Save the file.

IMPORTANT: Change this back to the Railway URL before pushing to GitHub.
If you forget, the DMG will point to localhost and nothing will work for users.

## B5. Run the Tauri dev app

```
cd C:\Users\hp\Desktop\david-v1-final\david-v1-final\david-app
bun run tauri:dev
```

FIRST TIME: This takes 10-15 minutes. Rust is compiling ~200 packages.
You will see output like:
```
   Compiling david v1.0.0
   ...
   Finished release [optimized] target(s) in 8m 23s
```

After compiling, the David auth window will appear on your screen.
You can register an account and test the orb.

SUBSEQUENT TIMES: ~30-60 seconds to start.

**What works on Windows dev mode:**
- Auth screen (login, register)
- Orb UI (opens, expands, shows panels)
- Typing to David
- Text rewriting
- Activity tracking

**What doesn't work on Windows dev mode:**
- Screen capture (different API on Windows vs Mac — full experience on Mac only)
- Fish Speech TTS (Mac-focused binary)
- Microphone wake word (requires the full bundled models)

---

# 4. BUILDING THE MAC DMG (Step C)

You do NOT need a Mac. GitHub Actions provides a free Mac machine.

## C1. Make sure backend URL is back to Railway

Before pushing, check `david-app/src-tauri/src/lib.rs`:
```rust
pub const BACKEND_URL: &str = "https://david-api-production.up.railway.app";
```
It MUST be the Railway URL, not localhost.

## C2. Push your code

```
cd C:\Users\hp\Desktop\david-v1-final\david-v1-final
git add .
git commit -m "ready for DMG build"
git push
```

## C3. Trigger the GitHub Actions build

Option A — Test build (no release):
1. Go to https://github.com/devansh-29-glitch/david
2. Click the "Actions" tab
3. Click "Build David DMG" in the left sidebar
4. Click the "Run workflow" button on the right
5. Click the green "Run workflow" button

Option B — Public release build:
```
git tag v1.0.0
git push origin v1.0.0
```
This builds AND creates a downloadable GitHub Release.

## C4. Monitor the build

Click on the running workflow to watch progress.
The build has these steps (in order):
1. Checkout — 10 seconds
2. Install Rust — 30 seconds
3. Install Bun — 15 seconds
4. Install JS dependencies — 60 seconds
5. Install Python + PyInstaller — 2 minutes
6. Build Python backend binary — 3-4 minutes
7. Download Whisper model (466MB) — 5-8 minutes
8. Download Silero VAD model — 1 minute
9. Download Fish Speech model (400MB) — 5-8 minutes
10. Create icons — 10 seconds
11. Build Tauri (Rust compilation) — 8-12 minutes
12. Create DMG — 1 minute

Total: 25-40 minutes.

## C5. Download the DMG

When the build shows a green checkmark:
1. Click on the completed workflow run
2. Scroll to the bottom of the page
3. Under "Artifacts" section, click "David-macOS-DMG"
4. A zip file downloads
5. Extract the zip — inside is `David.dmg`

---

# 5. TESTING THE DMG ON A MAC (Step D)

## D1. Install on a Mac

1. Double-click `David.dmg` to open it
2. A window appears showing the David icon and an Applications folder
3. Drag the David icon onto the Applications folder
4. Eject the DMG (drag to trash or press Cmd+E)

## D2. First launch (Gatekeeper bypass)

IMPORTANT: Do NOT double-click to open David.
macOS will say "cannot be opened because it is from an unidentified developer".

Do this instead:
1. Open Finder → Applications
2. Find David
3. RIGHT-CLICK on David → click "Open"
4. A dialog appears: "macOS cannot verify the developer. Are you sure you want to open it?"
5. Click "Open"

This only needs to be done once. After this, David opens normally.

If right-click → Open still doesn't work:
- Open Terminal
- Run: `xattr -cr /Applications/David.app`
- Then try right-click → Open again

## D3. Grant permissions

When David launches:
- macOS will ask for Screen Recording permission
  → Click "Open System Preferences"
  → In Privacy & Security → Screen Recording → find David → toggle ON
  → Quit and relaunch David

- macOS will ask for Microphone permission
  → Click "OK" to grant it

Both permissions are required. David cannot function without screen recording.

## D4. Create account and use David

1. The sign-in window appears
2. Click "Create account"
3. Enter name, email, password (minimum 8 characters)
4. Click "Create account"
5. The window closes
6. The David orb appears on your screen (right side, small purple circle)
7. Press Cmd+Shift+D to toggle the orb panel
8. Say "David" to activate voice mode
9. Type in the orb text box to ask David anything

---

# 6. SETTING UP THE WEBSITE (Step E)

## E1. Update the website file

Open `website/index.html` in any text editor.
Find these two placeholders and replace them:
- `devansh-29-glitch` → your actual GitHub username
- `hello@neuramindlabs.com` → your actual email

## E2. Host on GitHub Pages (free)

1. Go to your GitHub repo → Settings → Pages
2. Under "Source" select "Deploy from a branch"
3. Branch: main, folder: /website
4. Click Save

Your website will be live at:
`https://devansh-29-glitch.github.io/david`

Wait 2-3 minutes after saving for it to go live.

## E3. Point your domain to it

In your domain provider (Cloudflare, Namecheap, etc.):
Add a CNAME record:
- Name: `david`
- Value: `devansh-29-glitch.github.io`

This makes `david.neuramindlabs.com` point to your GitHub Pages site.
DNS propagation takes 5-30 minutes.

---

# 7. TROUBLESHOOTING

## Railway Errors

**"Failed to build asyncpg" or "command cc failed"**
This was the original error. It is fixed in this version.
If you see it again, make sure you pushed the NEW `requirements.txt` from this zip.
Check: `david-api/requirements.txt` should NOT contain `asyncpg`. It should have `psycopg2-binary`.

**"Deployment failed during build process"**
1. Click "View logs" on the failed deployment
2. Scroll to the red error line
3. Common causes:
   - Missing environment variable → check Railway Variables tab
   - Syntax error in main.py → check you didn't accidentally edit main.py
   - Memory limit → Railway free tier has 512MB RAM limit. The app should stay well under this.

**"gemini_configured: false"**
Your `GEMINI_API_KEY` is missing or wrong.
Go to Railway → david-api service → Variables tab.
The key must start with `AIza` and be about 40 characters long.
Get it from: https://aistudio.google.com/app/apikey

**"database_connected: false"**
Your `DATABASE_URL` is wrong.
In Railway, the correct way to set it is:
Variable name: `DATABASE_URL`
Variable value: `${{ Postgres.DATABASE_URL }}`
(literally those characters, with the dollar sign and double curly braces)
Railway resolves this automatically to the internal Postgres connection string.

**502 Bad Gateway**
The app is crashed or starting up. Wait 30 seconds and try again.
If it persists, check deployment logs.

## GitHub Actions Errors

**"Whisper model download failed" or timeout**
The Hugging Face download timed out (HF servers are sometimes slow).
Solution: Go to Actions → find the failed run → click "Re-run all jobs" (top right).
The retry usually succeeds.

**"Fish Speech model download failed"**
Same as above — HF timeout. Re-run the workflow.
David still works without Fish Speech (voice falls back to Gemini TTS).
The app will still launch and function — just voice output quality differs.

**Rust compilation error**
Click the failed step to see the exact error.
Most common: a dependency version conflict.
Post the exact error message — it will have a specific fix.

**"No DMG found" at the end**
The Tauri build itself failed silently.
Look at the "Build Tauri app" step logs for the real error.

## Windows Dev Mode Errors

**"bun: command not found"**
Bun was installed but the terminal doesn't see it.
Close all terminals and open a new one.
If still not found, run: `$env:PATH += ";$env:USERPROFILE\.bun\bin"` in PowerShell.

**Rust compilation error: "linker not found" or "link.exe not found"**
Visual Studio Build Tools are not installed or not yet detected.
Restart your computer after installing VS Build Tools.
Then run: `rustup toolchain install stable`

**"tauri: command not found"**
Run: `bun install` inside the david-app folder first.
The tauri CLI is installed as a local dev dependency.

**App compiles but windows don't appear**
Check if Windows Defender or antivirus is blocking it.
Add an exception for the project folder.

---

# 8. HOW DAVID WORKS (Architecture)

## The two parts

**Part 1: The Railway Backend** (`david-api/`)
- Runs on YOUR server at david-api-production.up.railway.app
- Holds your Gemini API key — users never see it
- Handles: user accounts, usage limits, all AI calls
- Uses PostgreSQL to store users and conversation history
- Uses Redis for session management

**Part 2: The Tauri App** (`david-app/`)
- Runs locally on the user's Mac
- Shows the floating orb on screen
- Captures screenshots
- Detects audio, keyboard activity
- Sends screenshots to the Railway backend for AI processing

## The data flow

```
User's Mac (David app)
  ↓ Takes screenshot every 1.6/3.4/7.9 seconds
  ↓ Sends to Railway via HTTPS
Railway Backend
  ↓ First: cheap confusion check (~50 tokens, no screenshot)
  ↓ If confused: full Gemini 2.5 Flash vision call with screenshot
  ↓ If response needed: optionally call Gemini TTS for audio
  ↓ Returns: {should_speak, message, audio_b64, mode}
User's Mac (David app)
  ↓ Shows text notification OR plays audio via Fish Speech
  ↓ User sees/hears David's response
```

## Screenshot frequency

- Rigorous activity (fast typing, rapid clicking): **1.6 seconds**
- Slow activity (reading, writing, slow scroll): **3.4 seconds**
- Idle or video playing: **7.9 seconds**

This is determined locally by the Tauri app watching keyboard/mouse events.
David never sends more screenshots than necessary.

## Free tier limits (per day, resets at midnight UTC)

- 25 unprompted AI interactions
- 10 chat messages
- 5 text rewrites
- No voice output (text only)

## Cost estimate for you (the developer)

Gemini 2.5 Flash: ~$0.30 per million input tokens
A screenshot at JPEG quality 55 ≈ 600-800 tokens
1000 active users × 25 interactions/day × 1000 tokens each = 25 million tokens/day
= $7.50/day in Gemini costs for 1000 active free-tier users

---

# FINAL CHECKLIST

Do this before telling anyone about David:

[ ] https://david-api-production.up.railway.app/health returns gemini_configured: true AND database_connected: true
[ ] You can register a new account via the David app auth window
[ ] You can log in with the registered account
[ ] The orb appears after login
[ ] Cmd+Shift+D toggles the orb panel
[ ] Typing in the orb chat box gets a response from David
[ ] Text rewrite works (paste text → describe tone → get result)
[ ] After 2+ minutes idle on same screen → David speaks or shows text notification
[ ] GitHub Release has David.dmg that downloads successfully

When all boxes are checked, post to:
- Reddit: r/MacApps, r/artificial, r/productivity
- Twitter/X: tag @sama, @karpathy, @levelsio
- Product Hunt
- Hacker News: "Show HN: David — an ambient AI that watches your screen and helps unprompted"
