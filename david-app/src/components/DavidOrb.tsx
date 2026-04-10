import { useState, useEffect, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

// ── Types ─────────────────────────────────────────────────────────────────────

type OrbMode = 'idle' | 'thinking' | 'speaking' | 'listening' | 'rewrite-input' | 'rewrite-result' | 'text-notify' | 'limit'

type DavidSpeaksEvent = {
  message: string
  mode: 'voice' | 'text'
  audio_b64: string | null
}

// ── David Orb ─────────────────────────────────────────────────────────────────

export default function DavidOrb() {
  const [mode, setMode] = useState<OrbMode>('idle')
  const [expanded, setExpanded] = useState(false)
  const [message, setMessage] = useState('')
  const [rewriteText, setRewriteText] = useState('')
  const [rewriteInstruction, setRewriteInstruction] = useState('')
  const [rewriteResult, setRewriteResult] = useState('')
  const [chatInput, setChatInput] = useState('')
  const [copied, setCopied] = useState(false)
  const [audioPlaying, setAudioPlaying] = useState(false)
  const [error, setError] = useState('')
  const audioRef = useRef<HTMLAudioElement>(null)
  const chatInputRef = useRef<HTMLInputElement>(null)

  // ── Event listeners ────────────────────────────────────────────────────────

  useEffect(() => {
    const unlisteners: Array<() => void> = []

    listen<DavidSpeaksEvent>('david-speaks', async (event) => {
      const { message: msg, mode: m, audio_b64 } = event.payload
      setMessage(msg)
      setExpanded(true)
      setError('')

      if (m === 'voice' && audio_b64) {
        setMode('speaking')
        await playAudio(audio_b64)
        setMode('idle')
      } else {
        setMode('text-notify')
        setTimeout(() => {
          if (mode === 'text-notify') {
            setMode('idle')
            setExpanded(false)
          }
        }, 9000)
      }
    }).then(u => unlisteners.push(u))

    listen<boolean>('audio-state-changed', (e) => {
      setAudioPlaying(e.payload)
    }).then(u => unlisteners.push(u))

    listen<void>('toggle-orb', () => {
      setExpanded(p => {
        if (!p) setTimeout(() => chatInputRef.current?.focus(), 150)
        return !p
      })
    }).then(u => unlisteners.push(u))

    return () => unlisteners.forEach(u => u())
  }, [mode])

  const playAudio = useCallback(async (b64: string) => {
    if (!audioRef.current) return
    audioRef.current.src = `data:audio/mp3;base64,${b64}`
    try {
      await audioRef.current.play()
      await new Promise<void>(resolve => {
        if (audioRef.current) audioRef.current.onended = () => resolve()
      })
    } catch {}
  }, [])

  // ── Handlers ───────────────────────────────────────────────────────────────

  const handleChat = useCallback(async () => {
    if (!chatInput.trim()) return
    const msg = chatInput.trim()
    setChatInput('')
    setMode('thinking')
    setError('')

    try {
      const resp = await invoke<{ message: string; audio_b64: string | null; mode: string }>('send_chat', { message: msg })
      setMessage(resp.message)

      if (resp.mode === 'voice' && resp.audio_b64) {
        setMode('speaking')
        await playAudio(resp.audio_b64)
        setMode('idle')
      } else {
        setMode('text-notify')
      }
    } catch (e: any) {
      const msg = e?.toString() || 'Something went wrong.'
      if (msg.includes('limit')) {
        setMode('limit')
        setError(msg)
      } else {
        setMode('idle')
        setError(msg)
      }
    }
  }, [chatInput, playAudio])

  const handleRewrite = useCallback(async () => {
    if (!rewriteText.trim() || !rewriteInstruction.trim()) return
    setMode('thinking')
    setError('')

    try {
      const result = await invoke<string>('request_rewrite', {
        text: rewriteText,
        instruction: rewriteInstruction,
      })
      setRewriteResult(result)
      setMode('rewrite-result')
    } catch (e: any) {
      setMode('idle')
      setError(e?.toString() || 'Rewrite failed.')
    }
  }, [rewriteText, rewriteInstruction])

  const handleCopy = useCallback(async () => {
    await writeText(rewriteResult)
    setCopied(true)
    setTimeout(() => {
      setCopied(false)
      setMode('idle')
      setExpanded(false)
      setRewriteText('')
      setRewriteInstruction('')
      setRewriteResult('')
    }, 1500)
  }, [rewriteResult])

  const orbSize = expanded ? 89.6 : 56

  const orbColor = {
    idle: 'radial-gradient(circle at 38% 32%, #a5b4fc, #6366f1 55%, #4338ca)',
    thinking: 'radial-gradient(circle at 38% 32%, #c4b5fd, #8b5cf6 55%, #6d28d9)',
    speaking: 'radial-gradient(circle at 38% 32%, #86efac, #22c55e 55%, #15803d)',
    listening: 'radial-gradient(circle at 38% 32%, #fca5a5, #ef4444 55%, #991b1b)',
    'rewrite-input': 'radial-gradient(circle at 38% 32%, #fde68a, #f59e0b 55%, #b45309)',
    'rewrite-result': 'radial-gradient(circle at 38% 32%, #fde68a, #f59e0b 55%, #b45309)',
    'text-notify': 'radial-gradient(circle at 38% 32%, #7dd3fc, #3b82f6 55%, #1d4ed8)',
    limit: 'radial-gradient(circle at 38% 32%, #fca5a5, #ef4444 55%, #991b1b)',
  }[mode]

  const pulsing = mode === 'thinking' || mode === 'speaking' || mode === 'listening'

  return (
    <div style={{
      position: 'fixed', top: 0, left: 0,
      width: '100vw', height: '100vh',
      display: 'flex', alignItems: 'flex-start', justifyContent: 'flex-start',
      pointerEvents: 'none',
      fontFamily: '"SF Pro Display", system-ui, -apple-system, sans-serif',
    }}>
      <audio ref={audioRef} style={{ display: 'none' }} />

      {/* ── Extended panel ────────────────────────────────────────────────── */}
      {expanded && (
        <div style={{
          position: 'absolute',
          top: 0, left: orbSize + 10,
          width: 340,
          background: 'rgba(10, 10, 20, 0.96)',
          backdropFilter: 'blur(24px)',
          border: '1px solid rgba(99,102,241,0.25)',
          borderRadius: 18,
          padding: '14px 16px',
          color: 'white',
          pointerEvents: 'all',
          boxShadow: '0 12px 48px rgba(0,0,0,0.7), 0 0 0 1px rgba(99,102,241,0.15)',
          animation: 'slideIn 0.18s cubic-bezier(0.34,1.56,0.64,1)',
        }}>

          {/* Header */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
            <span style={{
              fontSize: 12, fontWeight: 800, letterSpacing: 2,
              background: 'linear-gradient(135deg, #818cf8, #c084fc)',
              WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
            }}>DAVID</span>
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              {audioPlaying && <span style={{ fontSize: 10, color: '#6366f1' }}>🎵 text mode</span>}
              <button onClick={() => { setExpanded(false); setMode('idle'); setError('') }}
                style={{ background: 'none', border: 'none', color: '#4b5563', cursor: 'pointer', fontSize: 18, lineHeight: 1 }}>×</button>
            </div>
          </div>

          {/* Thinking */}
          {mode === 'thinking' && (
            <div style={{ padding: '10px 0', textAlign: 'center' }}>
              <div style={{ fontSize: 11, color: '#6366f1', letterSpacing: 2, marginBottom: 10 }}>THINKING</div>
              <div style={{ display: 'flex', justifyContent: 'center', gap: 5 }}>
                {[0,1,2].map(i => (
                  <div key={i} style={{
                    width: 7, height: 7, borderRadius: '50%', background: '#6366f1',
                    animation: `bounce 1.1s ease-in-out ${i * 0.18}s infinite`,
                  }} />
                ))}
              </div>
            </div>
          )}

          {/* Message */}
          {(mode === 'speaking' || mode === 'text-notify') && message && (
            <div style={{
              background: 'rgba(99,102,241,0.08)',
              border: '1px solid rgba(99,102,241,0.2)',
              borderRadius: 10, padding: '10px 12px',
              fontSize: 13, lineHeight: 1.65, color: '#e0e0f0',
              marginBottom: 10, whiteSpace: 'pre-wrap',
            }}>
              {message}
            </div>
          )}

          {/* Error */}
          {error && mode !== 'thinking' && (
            <div style={{
              background: 'rgba(239,68,68,0.1)', border: '1px solid rgba(239,68,68,0.3)',
              borderRadius: 8, padding: '8px 10px', fontSize: 12, color: '#fca5a5',
              marginBottom: 8,
            }}>{error}</div>
          )}

          {/* Limit reached */}
          {mode === 'limit' && (
            <div style={{ textAlign: 'center', padding: '8px 0' }}>
              <div style={{ fontSize: 13, color: '#fca5a5', marginBottom: 6 }}>Daily limit reached</div>
              <div style={{ fontSize: 11, color: '#6b7280' }}>Resets at midnight. Upgrade for more.</div>
            </div>
          )}

          {/* Rewrite input */}
          {mode === 'rewrite-input' && (
            <div>
              <div style={{ fontSize: 11, color: '#6b7280', marginBottom: 6 }}>REWRITE</div>
              <textarea
                value={rewriteText}
                onChange={e => setRewriteText(e.target.value)}
                placeholder="Paste the text you want to rewrite..."
                autoFocus
                style={{
                  width: '100%', height: 80,
                  background: 'rgba(255,255,255,0.04)',
                  border: '1px solid rgba(255,255,255,0.08)',
                  borderRadius: 8, color: 'white',
                  padding: '8px 10px', fontSize: 12,
                  resize: 'none', outline: 'none',
                  fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: 6,
                }}
              />
              <input
                type="text"
                value={rewriteInstruction}
                onChange={e => setRewriteInstruction(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') handleRewrite() }}
                placeholder="How? e.g. more formal, shorter, email tone..."
                style={{
                  width: '100%', background: 'rgba(255,255,255,0.04)',
                  border: '1px solid rgba(255,255,255,0.08)',
                  borderRadius: 8, color: 'white',
                  padding: '7px 10px', fontSize: 12, outline: 'none',
                  fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: 8,
                }}
              />
              <div style={{ display: 'flex', gap: 6 }}>
                <button onClick={handleRewrite}
                  disabled={!rewriteText.trim() || !rewriteInstruction.trim()}
                  style={{
                    flex: 1, background: 'rgba(99,102,241,0.2)',
                    border: '1px solid rgba(99,102,241,0.4)',
                    borderRadius: 8, color: '#a5b4fc',
                    padding: '8px 0', fontSize: 12, fontWeight: 600, cursor: 'pointer',
                  }}>
                  Rewrite →
                </button>
                <button onClick={() => setMode('idle')}
                  style={{
                    background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.08)',
                    borderRadius: 8, color: '#6b7280', padding: '8px 12px', fontSize: 12, cursor: 'pointer',
                  }}>✕</button>
              </div>
            </div>
          )}

          {/* Rewrite result — tongue out */}
          {mode === 'rewrite-result' && rewriteResult && (
            <div>
              <div style={{ fontSize: 11, color: '#6b7280', marginBottom: 6 }}>REWRITTEN</div>
              <div style={{
                background: 'rgba(245,158,11,0.08)',
                border: '1px solid rgba(245,158,11,0.25)',
                borderRadius: 10, padding: '10px 12px',
                fontSize: 13, lineHeight: 1.65, color: '#fef3c7',
                maxHeight: 180, overflowY: 'auto',
                whiteSpace: 'pre-wrap', marginBottom: 10,
              }}>
                {rewriteResult}
              </div>
              <button onClick={handleCopy} style={{
                width: '100%',
                background: copied ? 'rgba(34,197,94,0.15)' : 'rgba(245,158,11,0.12)',
                border: `1px solid ${copied ? 'rgba(34,197,94,0.4)' : 'rgba(245,158,11,0.35)'}`,
                borderRadius: 8, color: copied ? '#86efac' : '#fde68a',
                padding: '9px 0', fontSize: 13, fontWeight: 700, cursor: 'pointer',
                transition: 'all 0.2s',
              }}>
                {copied ? '✓ Copied' : '📋 Copy'}
              </button>
            </div>
          )}

          {/* Idle — chat + rewrite buttons */}
          {(mode === 'idle' || mode === 'text-notify' || mode === 'speaking') && (
            <div style={{ marginTop: message ? 10 : 0 }}>
              {/* Chat input */}
              <input
                ref={chatInputRef}
                type="text"
                value={chatInput}
                onChange={e => setChatInput(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') handleChat() }}
                placeholder="Ask David anything..."
                style={{
                  width: '100%', background: 'rgba(255,255,255,0.05)',
                  border: '1px solid rgba(99,102,241,0.25)',
                  borderRadius: 9, color: 'white',
                  padding: '9px 12px', fontSize: 13, outline: 'none',
                  fontFamily: 'inherit', boxSizing: 'border-box', marginBottom: 8,
                }}
              />

              {/* Rewrite button */}
              <button
                onClick={() => setMode('rewrite-input')}
                style={{
                  width: '100%', background: 'rgba(255,255,255,0.03)',
                  border: '1px solid rgba(255,255,255,0.07)',
                  borderRadius: 8, color: '#9ca3af',
                  padding: '7px 0', fontSize: 12, cursor: 'pointer',
                  display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
                }}>
                ✍️ <span>Rewrite text</span>
              </button>
            </div>
          )}
        </div>
      )}

      {/* ── The Orb ────────────────────────────────────────────────────────── */}
      <div
        onClick={() => {
          setExpanded(p => {
            if (!p) setTimeout(() => chatInputRef.current?.focus(), 150)
            return !p
          })
        }}
        data-tauri-drag-region
        style={{
          width: orbSize, height: orbSize,
          borderRadius: '50%',
          background: orbColor,
          cursor: 'pointer',
          pointerEvents: 'all',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          boxShadow: expanded
            ? '0 0 0 3px rgba(99,102,241,0.5), 0 10px 36px rgba(99,102,241,0.45)'
            : '0 4px 20px rgba(99,102,241,0.35), 0 0 0 1px rgba(99,102,241,0.2)',
          transition: 'width 0.22s cubic-bezier(0.34,1.56,0.64,1), height 0.22s cubic-bezier(0.34,1.56,0.64,1), background 0.3s',
          animation: pulsing ? 'pulse 1.6s ease-in-out infinite' : 'none',
          userSelect: 'none', flexShrink: 0,
        }}
      >
        <div style={{
          width: '55%', height: '55%', borderRadius: '50%',
          background: 'rgba(255,255,255,0.18)',
        }} />
      </div>

      <style>{`
        @keyframes pulse {
          0%,100% { box-shadow: 0 4px 20px rgba(99,102,241,0.35); }
          50% { box-shadow: 0 4px 36px rgba(99,102,241,0.65), 0 0 0 5px rgba(99,102,241,0.2); }
        }
        @keyframes bounce {
          0%,100% { transform: translateY(0); }
          50% { transform: translateY(-5px); }
        }
        @keyframes slideIn {
          from { opacity: 0; transform: translateX(-10px); }
          to { opacity: 1; transform: translateX(0); }
        }
        * { box-sizing: border-box; }
        textarea:focus, input:focus { border-color: rgba(99,102,241,0.5) !important; }
        ::-webkit-scrollbar { width: 4px; }
        ::-webkit-scrollbar-thumb { background: rgba(99,102,241,0.3); border-radius: 2px; }
      `}</style>
    </div>
  )
}
