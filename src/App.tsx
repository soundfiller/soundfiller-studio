import { useState } from 'react';

export default function App() {
  const [tab, setTab] = useState<'plan' | 'analyse' | 'library' | 'compare'>('plan');

  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: 'var(--color-studio-bg)' }}>
      {/* App shell — M0 placeholder. Tab bar + content area. */}
      <nav className="flex items-center px-6 py-3 gap-6 border-b border-white/10">
        <span className="font-mono text-sm tracking-wider text-white/60 uppercase">Soundfiller Studio</span>
        {(['plan', 'analyse', 'library', 'compare'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`text-sm tracking-wide transition-colors no-radius ${
              tab === t ? 'text-white' : 'text-white/40 hover:text-white/70'
            }`}
          >
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </nav>
      <main className="flex-1 p-6">
        <p className="text-white/35 text-sm font-mono">
          M0 scaffold — {tab} tab. Awaiting template library and bar-grid component.
        </p>
      </main>
    </div>
  );
}
