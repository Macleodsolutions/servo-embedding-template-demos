import { useState, useEffect } from 'react'
import reactLogo from './assets/react.svg'
import viteLogo from './assets/vite.svg'
import heroImg from './assets/hero.png'
import './App.css'

declare global {
  interface Window {
    gameEngine: {
      spawnEnemy: (id: string, x: number, y: number) => boolean
      addEventListener: (event: string, cb: (e: any) => void) => void
    }
  }
}

function BridgeDemo() {
  const [log, setLog] = useState<string[]>([])

  useEffect(() => {
    window.gameEngine.addEventListener('enemydied', (e: any) => {
      setLog(prev => [`← enemydied: id=${e.enemyId} x=${e.x.toFixed(1)} y=${e.y.toFixed(1)}`, ...prev])
    })
  }, [])

  const spawn = () => {
    const id = 'goblin_' + Date.now()
    window.gameEngine.spawnEnemy(id, Math.random() * 100, Math.random() * 100)
    setLog(prev => [`→ spawnEnemy(${id})`, ...prev])
  }

  return (
    <div style={{ margin: '20px 0', fontFamily: 'monospace', fontSize: 13 }}>
      <button onClick={spawn}>Spawn Enemy (JS → Rust → JS)</button>
      <div style={{ marginTop: 8, maxHeight: 120, overflow: 'auto' }}>
        {log.map((l, i) => <div key={i}>{l}</div>)}
      </div>
    </div>
  )
}

function App() {
  const [count, setCount] = useState(0)

  return (
    <>
      <section id="center">
        <div className="hero">
          <img src={heroImg} className="base" width="170" height="179" alt="" />
          <img src={reactLogo} className="framework" alt="React logo" />
          <img src={viteLogo} className="vite" alt="Vite logo" />
        </div>
        <div>
          <h1>Get started</h1>
          <p>
            Edit <code>src/App.tsx</code> and save to test <code>HMR</code>
          </p>
        </div>
        <button
          className="counter"
          onClick={() => setCount((count) => count + 1)}
        >
          Count is {count}
        </button>
        <BridgeDemo />
      </section>

      <div className="ticks"></div>

      <section id="next-steps">
        <div id="docs">
          <svg className="icon" role="presentation" aria-hidden="true">
            <use href="/icons.svg#documentation-icon"></use>
          </svg>
          <h2>Documentation</h2>
          <p>Your questions, answered</p>
          <ul>
            <li>
              <a href="https://vite.dev/" target="_blank">
                <img className="logo" src={viteLogo} alt="" />
                Explore Vite
              </a>
            </li>
            <li>
              <a href="https://react.dev/" target="_blank">
                <img className="button-icon" src={reactLogo} alt="" />
                Learn more
              </a>
            </li>
          </ul>
        </div>
        <div id="social">
          <svg className="icon" role="presentation" aria-hidden="true">
            <use href="/icons.svg#social-icon"></use>
          </svg>
          <h2>Connect with us</h2>
          <p>Join the Vite community</p>
          <ul>
            <li>
              <a href="https://github.com/vitejs/vite" target="_blank">
                <svg className="button-icon" role="presentation" aria-hidden="true">
                  <use href="/icons.svg#github-icon"></use>
                </svg>
                GitHub
              </a>
            </li>
            <li>
              <a href="https://chat.vite.dev/" target="_blank">
                <svg className="button-icon" role="presentation" aria-hidden="true">
                  <use href="/icons.svg#discord-icon"></use>
                </svg>
                Discord
              </a>
            </li>
          </ul>
        </div>
      </section>

      <div className="ticks"></div>
      <section id="spacer"></section>
    </>
  )
}

export default App
