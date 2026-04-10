import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

type AuthMode = 'login' | 'register'

export default function AuthScreen() {
  const [mode, setMode] = useState<AuthMode>('login')
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async () => {
    setError('')
    if (!email.trim() || !password.trim()) {
      setError('Email and password are required.')
      return
    }
    if (mode === 'register' && !name.trim()) {
      setError('Your name is required.')
      return
    }
    if (password.length < 8) {
      setError('Password must be at least 8 characters.')
      return
    }

    setLoading(true)
    try {
      if (mode === 'register') {
        await invoke('register', { name: name.trim(), email: email.trim(), password })
      } else {
        await invoke('login', { email: email.trim(), password })
      }
      // Close auth window — orb takes over
      const win = getCurrentWindow()
      await win.hide()
    } catch (e: any) {
      setError(e?.toString() || 'Something went wrong. Try again.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div style={{
      background: '#08080f',
      height: '100vh',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: '"SF Pro Display", system-ui, -apple-system, sans-serif',
      color: 'white',
      padding: 32,
    }}>

      {/* Logo + title */}
      <div style={{ textAlign: 'center', marginBottom: 28 }}>
        <div style={{
          width: 52, height: 52,
          borderRadius: '50%',
          background: 'radial-gradient(circle at 38% 32%, #a5b4fc, #6366f1 55%, #4338ca)',
          margin: '0 auto 14px',
          boxShadow: '0 4px 24px rgba(99,102,241,0.5)',
        }} />
        <h1 style={{
          fontSize: 22, fontWeight: 900, margin: 0,
          background: 'linear-gradient(135deg, #e0e0ff, #818cf8 40%, #c084fc)',
          WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
          letterSpacing: '-0.5px',
        }}>David</h1>
        <p style={{ color: '#4b5563', fontSize: 12, marginTop: 4 }}>
          An ambient AI that lives with you
        </p>
      </div>

      {/* Form */}
      <div style={{
        width: '100%',
        background: 'rgba(255,255,255,0.03)',
        border: '1px solid rgba(255,255,255,0.07)',
        borderRadius: 16,
        padding: 24,
      }}>
        {/* Tab switcher */}
        <div style={{
          display: 'flex',
          background: 'rgba(255,255,255,0.04)',
          borderRadius: 10,
          padding: 3,
          marginBottom: 20,
        }}>
          {(['login', 'register'] as AuthMode[]).map(m => (
            <button
              key={m}
              onClick={() => { setMode(m); setError('') }}
              style={{
                flex: 1,
                background: mode === m ? 'rgba(99,102,241,0.2)' : 'none',
                border: mode === m ? '1px solid rgba(99,102,241,0.35)' : '1px solid transparent',
                borderRadius: 8,
                color: mode === m ? '#a5b4fc' : '#6b7280',
                padding: '7px 0',
                fontSize: 13,
                fontWeight: mode === m ? 700 : 400,
                cursor: 'pointer',
                transition: 'all 0.15s',
                fontFamily: 'inherit',
              }}
            >
              {m === 'login' ? 'Sign in' : 'Create account'}
            </button>
          ))}
        </div>

        {/* Name field (register only) */}
        {mode === 'register' && (
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 11, color: '#6b7280', display: 'block', marginBottom: 4 }}>
              YOUR NAME
            </label>
            <input
              type="text"
              value={name}
              onChange={e => setName(e.target.value)}
              placeholder="Tony Stark"
              autoFocus
              style={inputStyle}
            />
          </div>
        )}

        {/* Email */}
        <div style={{ marginBottom: 12 }}>
          <label style={{ fontSize: 11, color: '#6b7280', display: 'block', marginBottom: 4 }}>
            EMAIL
          </label>
          <input
            type="email"
            value={email}
            onChange={e => setEmail(e.target.value)}
            placeholder="you@example.com"
            autoFocus={mode === 'login'}
            onKeyDown={e => { if (e.key === 'Enter') handleSubmit() }}
            style={inputStyle}
          />
        </div>

        {/* Password */}
        <div style={{ marginBottom: 20 }}>
          <label style={{ fontSize: 11, color: '#6b7280', display: 'block', marginBottom: 4 }}>
            PASSWORD
          </label>
          <input
            type="password"
            value={password}
            onChange={e => setPassword(e.target.value)}
            placeholder={mode === 'register' ? 'Min 8 characters' : '••••••••'}
            onKeyDown={e => { if (e.key === 'Enter') handleSubmit() }}
            style={inputStyle}
          />
        </div>

        {/* Error */}
        {error && (
          <div style={{
            background: 'rgba(239,68,68,0.1)',
            border: '1px solid rgba(239,68,68,0.3)',
            borderRadius: 8,
            padding: '8px 12px',
            fontSize: 12,
            color: '#fca5a5',
            marginBottom: 14,
          }}>
            {error}
          </div>
        )}

        {/* Submit */}
        <button
          onClick={handleSubmit}
          disabled={loading}
          style={{
            width: '100%',
            background: loading
              ? 'rgba(99,102,241,0.1)'
              : 'linear-gradient(135deg, #6366f1, #8b5cf6)',
            border: 'none',
            borderRadius: 10,
            color: 'white',
            padding: '11px 0',
            fontSize: 14,
            fontWeight: 700,
            cursor: loading ? 'not-allowed' : 'pointer',
            transition: 'opacity 0.15s',
            opacity: loading ? 0.7 : 1,
            fontFamily: 'inherit',
          }}
        >
          {loading
            ? 'One moment...'
            : mode === 'login' ? 'Sign in to David' : 'Create account'}
        </button>
      </div>

      {/* Footer note */}
      <p style={{ fontSize: 11, color: '#1f2937', marginTop: 20, textAlign: 'center', lineHeight: 1.6 }}>
        David uses your account to power AI features.<br />
        No data is sold. Your screen is never recorded to our servers.
      </p>
    </div>
  )
}

const inputStyle: React.CSSProperties = {
  width: '100%',
  background: 'rgba(255,255,255,0.05)',
  border: '1px solid rgba(255,255,255,0.09)',
  borderRadius: 9,
  color: 'white',
  padding: '9px 12px',
  fontSize: 13,
  outline: 'none',
  fontFamily: 'inherit',
  boxSizing: 'border-box',
  transition: 'border-color 0.15s',
}
