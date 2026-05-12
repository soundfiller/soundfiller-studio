import { useState } from 'react';
import BarGrid from './components/BarGrid';
import BpmPicker from './components/BpmPicker';
import NotesPanel from './components/NotesPanel';
import { useProjectStore } from './store/useProjectStore';

type Tab = 'plan' | 'analyse' | 'library' | 'compare';

export default function App() {
  const [tab, setTab] = useState<Tab>('plan');

  const doc = useProjectStore((s) => s.doc);
  const selectedSectionId = useProjectStore((s) => s.selectedSectionId);
  const selectedSection = useProjectStore((s) => s.selectedSection);
  const selectSection = useProjectStore((s) => s.selectSection);
  const setBpm = useProjectStore((s) => s.setBpm);
  const cycleCellIntensity = useProjectStore((s) => s.cycleCellIntensity);
  const updateNote = useProjectStore((s) => s.updateNote);

  return (
    <div className="min-h-screen flex flex-col" style={{ backgroundColor: 'var(--color-studio-bg)' }}>
      {/* Tab bar */}
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

      {/* Main content */}
      <main className="flex-1 flex" style={{ minHeight: 0 }}>
        {tab === 'plan' && (
          <div className="flex flex-1" style={{ minHeight: 0 }}>
            {/* Left sidebar — template picker + BPM */}
            <aside
              className="w-56 shrink-0 flex flex-col gap-6 p-5 overflow-y-auto"
              style={{ borderRight: '1px solid rgba(255,255,255,0.08)' }}
            >
              {/* Template picker */}
              <div>
                <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-2">
                  Template
                </div>
                <div
                  className="px-3 py-2 text-sm font-mono"
                  style={{
                    border: '1px solid var(--color-studio-accent)',
                    color: 'var(--color-studio-accent)',
                  }}
                >
                  Classic Prydz
                </div>
                <div className="text-[10px] font-mono text-white/25 mt-1">
                  Progressive House · 176 bars
                </div>
              </div>

              {/* BPM picker */}
              <BpmPicker
                bpm={doc.bpm}
                totalBars={doc.total_bars}
                onBpmChange={setBpm}
              />

              {/* Swing info */}
              <div style={{ borderTop: '1px solid rgba(255,255,255,0.08)', paddingTop: '12px' }}>
                <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-2">
                  Swing / Groove
                </div>
                <div className="flex gap-4 text-xs font-mono text-white/50">
                  <span>Swing: {doc.swing_percent}%</span>
                  <span>Ghosts: {doc.ghost_note_density}/3</span>
                </div>
              </div>

              {/* Reference artists */}
              <div style={{ borderTop: '1px solid rgba(255,255,255,0.08)', paddingTop: '12px' }}>
                <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-2">
                  Inspired By
                </div>
                <div className="text-xs font-mono text-white/50 leading-relaxed">
                  {doc.reference_artists.join(', ')}
                </div>
              </div>
            </aside>

            {/* Center — bar grid */}
            <div className="flex-1 p-5 overflow-auto" style={{ minWidth: 0 }}>
              <BarGrid
                doc={doc}
                selectedSectionId={selectedSectionId}
                onSelectSection={selectSection}
                onCycleCell={cycleCellIntensity}
              />
            </div>

            {/* Right panel — notes */}
            <aside
              className="w-72 shrink-0 p-5 overflow-y-auto"
              style={{ borderLeft: '1px solid rgba(255,255,255,0.08)' }}
            >
              <NotesPanel
                section={selectedSection}
                bpm={doc.bpm}
                onUpdateNote={updateNote}
              />
            </aside>
          </div>
        )}

        {/* Other tabs — placeholder */}
        {tab !== 'plan' && (
          <div className="flex items-center justify-center flex-1">
            <p className="text-white/35 text-sm font-mono">
              {tab.charAt(0).toUpperCase() + tab.slice(1)} tab — coming soon.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}
