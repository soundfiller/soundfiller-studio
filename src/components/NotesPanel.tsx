import type { Section } from '../types';

interface NotesPanelProps {
  section: Section | null;
  bpm: number;
  onUpdateNote: (sectionId: string, noteIndex: number, text: string) => void;
}

function formatDurationFromBar(startBar: number, bars: number, bpm: number): string {
  const startSeconds = ((startBar - 1) * 4 * 60) / bpm;
  const endSeconds = ((startBar - 1 + bars) * 4 * 60) / bpm;
  const fmt = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = Math.round(s % 60);
    return `${m}:${String(sec).padStart(2, '0')}`;
  };
  return `${fmt(startSeconds)}–${fmt(endSeconds)}`;
}

export default function NotesPanel({ section, bpm, onUpdateNote }: NotesPanelProps) {
  if (!section) {
    return (
      <div className="flex items-center justify-center h-full text-white/20 text-sm font-mono px-4 text-center">
        Select a section to view notes
      </div>
    );
  }

  const barEnd = section.start_bar + section.bars - 1;

  return (
    <div className="flex flex-col gap-4">
      {/* Section header */}
      <div>
        <div className="text-base font-medium text-white">{section.name}</div>
        <div className="text-xs font-mono text-white/40 mt-1">
          Bars {section.start_bar}–{barEnd}, {section.bars} bars
        </div>
        <div className="text-xs font-mono text-white/40">
          {formatDurationFromBar(section.start_bar, section.bars, bpm)} at {bpm} BPM
        </div>
      </div>

      {/* Divider */}
      <div style={{ borderTop: '1px solid rgba(255,255,255,0.08)' }} />

      {/* Notes */}
      <div className="flex flex-col gap-3">
        <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider">
          Production Notes
        </div>
        {section.notes.length === 0 && (
          <div className="text-white/20 text-xs italic">No notes for this section.</div>
        )}
        {section.notes.map((note, idx) => (
          <div key={idx} className="group">
            <div className="flex gap-2">
              <span className="text-white/20 font-mono text-xs mt-[2px] shrink-0">
                {String(idx + 1).padStart(2, '0')}
              </span>
              <textarea
                value={note}
                onChange={(e) => onUpdateNote(section.id, idx, e.target.value)}
                className="no-radius flex-1 bg-transparent text-white/75 text-xs leading-relaxed resize-none outline-none border-0 font-sans"
                rows={2}
                style={{ minHeight: '2.5em' }}
              />
            </div>
          </div>
        ))}
      </div>

      {/* References */}
      {section.references.length > 0 && (
        <>
          <div style={{ borderTop: '1px solid rgba(255,255,255,0.08)' }} />
          <div>
            <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-2">
              References
            </div>
            <div className="flex flex-col gap-2">
              {section.references.map((ref, idx) => {
                const fmt = (s: number) => {
                  const m = Math.floor(s / 60);
                  const sec = Math.round(s % 60);
                  return `${m}:${String(sec).padStart(2, '0')}`;
                };
                return (
                  <div
                    key={idx}
                    className="px-2 py-1.5 text-xs"
                    style={{ border: '1px solid rgba(255,255,255,0.08)' }}
                  >
                    <div className="text-white/80 font-medium">{ref.artist} — {ref.title}</div>
                    <div className="text-white/40 font-mono text-[10px] mt-0.5">
                      {fmt(ref.start_seconds)}–{fmt(ref.end_seconds)}
                    </div>
                    {ref.note && (
                      <div className="text-white/50 text-[10px] mt-1 italic">
                        {ref.note}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
