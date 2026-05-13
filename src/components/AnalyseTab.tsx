import { useState, useEffect, useCallback, useRef } from 'react';
import type { AnalysisResult, ArrangementDoc, Section } from '../types';
import BarGrid from './BarGrid';
import NotesPanel from './NotesPanel';
import {
  ingestAudioFile,
  ingestYoutube,
  getAnalysisStatus,
  getAnalysisResult,
  listAnalysedTracks,
  deleteAnalysis,
  onAnalysisStatusUpdate,
  onAnalysisComplete,
  onAnalysisError,
  onYoutubeDownloadProgress,
} from '../lib/tauri';

interface TrackEntry {
  id: string;
  filename: string;
  status: 'queued' | 'downloading' | 'analysing' | 'done' | 'error';
  error?: string;
  result?: AnalysisResult;
}

export default function AnalyseTab() {
  const [tracks, setTracks] = useState<TrackEntry[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const [openTrackId, setOpenTrackId] = useState<string | null>(null);
  const [selectedSectionId, setSelectedSectionId] = useState<string | null>(null);
  const [youtubeUrl, setYoutubeUrl] = useState('');
  const [legalAckAccepted, setLegalAckAccepted] = useState(
    () => localStorage.getItem('soundfiller-youtube-ack') === 'accepted_v1'
  );
  const [showLegalModal, setShowLegalModal] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [youtubeLoading, setYoutubeLoading] = useState(false);
  const cleanupRef = useRef<() => void>(() => {});

  // Listen for analysis events
  useEffect(() => {
    const unsubs: (() => void)[] = [];

    onAnalysisStatusUpdate((event) => {
      setTracks((prev) =>
        prev.map((t) =>
          t.id === event.id ? { ...t, status: event.status as TrackEntry['status'] } : t,
        ),
      );
    }).then((u) => unsubs.push(u));

    onAnalysisComplete((event) => {
      setTracks((prev) =>
        prev.map((t) =>
          t.id === event.id ? { ...t, status: 'done', result: event.result } : t,
        ),
      );
    }).then((u) => unsubs.push(u));

    onAnalysisError((event) => {
      setTracks((prev) =>
        prev.map((t) =>
          t.id === event.id ? { ...t, status: 'error', error: event.error } : t,
        ),
      );
    }).then((u) => unsubs.push(u));

    onYoutubeDownloadProgress((event) => {
      setDownloadProgress((prev) => ({ ...prev, [event.id]: event.progress }));
    }).then((u) => unsubs.push(u));

    cleanupRef.current = () => unsubs.forEach((u) => u());
    return cleanupRef.current;
  }, []);

  // Load existing tracks on mount
  useEffect(() => {
    listAnalysedTracks().then((results) => {
      setTracks(
        results.map(([id, filename, result]) => ({
          id,
          filename,
          status: 'done' as const,
          result,
        })),
      );
    }).catch(() => { /* not in Tauri context */ });
  }, []);

  const handleDrop = useCallback(
    async (files: FileList | null) => {
      if (!files || files.length === 0) return;
      setDragOver(false);

      for (const file of Array.from(files)) {
        const filename = file.name;
        const entry: TrackEntry = {
          id: `pending-${Date.now()}-${Math.random().toString(36).slice(2)}`,
          filename,
          status: 'queued',
        };
        setTracks((prev) => [...prev, entry]);

        try {
          // Use Tauri file dialog path — for drag-and-drop we get the file path from webview
          // In Tauri 2.x, drag-drop gives us paths via the drag-drop event
          const path = (file as any).path;
          if (path) {
            const id = await ingestAudioFile(path);
            setTracks((prev) =>
              prev.map((t) => (t.id === entry.id ? { ...t, id } : t)),
            );
          } else {
            console.warn('Drag-drop file path not available, try using Browse');
          }
        } catch (err) {
          setTracks((prev) =>
            prev.map((t) =>
              t.id === entry.id
                ? { ...t, status: 'error', error: String(err) }
                : t,
            ),
          );
        }
      }
    },
    [],
  );

  const handleBrowse = useCallback(async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Audio',
            extensions: ['mp3', 'wav', 'flac', 'aiff', 'aif', 'm4a', 'ogg', 'wma'],
          },
        ],
      });
      if (!selected) return;

      const path = typeof selected === 'string' ? selected : selected.path;
      const filename = path.split('/').pop() || path;

      const entry: TrackEntry = {
        id: `pending-${Date.now()}`,
        filename,
        status: 'queued',
      };
      setTracks((prev) => [...prev, entry]);

      const id = await ingestAudioFile(path);
      setTracks((prev) =>
        prev.map((t) => (t.id === entry.id ? { ...t, id } : t)),
      );
    } catch (err) {
      console.error('Browse error:', err);
    }
  }, []);

  const handleYoutubeSubmit = useCallback(async () => {
    const url = youtubeUrl.trim();
    if (!url) return;

    if (!legalAckAccepted) {
      setShowLegalModal(true);
      return;
    }

    setYoutubeLoading(true);
    const pendingId = `pending-yt-${Date.now()}`;
    try {
      const entry: TrackEntry = {
        id: pendingId,
        filename: url,
        status: 'queued',
      };
      setTracks((prev) => [...prev, entry]);

      const id = await ingestYoutube(url, 'accepted_v1');
      setTracks((prev) =>
        prev.map((t) => (t.id === pendingId ? { ...t, id } : t)),
      );
      setYoutubeUrl('');
    } catch (err) {
      console.error('YouTube ingest error:', err);
      setTracks((prev) =>
        prev.map((t) =>
          t.id === pendingId
            ? { ...t, status: 'error', error: String(err) }
            : t,
        ),
      );
    } finally {
      setYoutubeLoading(false);
    }
  }, [youtubeUrl, legalAckAccepted]);

  const handleAcceptLegal = useCallback(() => {
    localStorage.setItem('soundfiller-youtube-ack', 'accepted_v1');
    setLegalAckAccepted(true);
    setShowLegalModal(false);
  }, []);

  const handleDelete = useCallback(
    (id: string) => {
      deleteAnalysis(id).catch(() => {});
      setTracks((prev) => prev.filter((t) => t.id !== id));
      if (openTrackId === id) setOpenTrackId(null);
    },
    [openTrackId],
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setDragOver(false);
  }, []);

  // Find the open track
  const openTrack = tracks.find((t) => t.id === openTrackId);
  const openResult = openTrack?.result;

  // Convert analysis result to ArrangementDoc-compatible shape for BarGrid
  const makeDoc = useCallback((result: AnalysisResult, filename: string) => {
    const totalBars = result.sections.length > 0
      ? Math.max(
          result.sections[result.sections.length - 1].start_bar +
            result.sections[result.sections.length - 1].bars -
            1,
          16,
        )
      : 16;

    const isTechno = result.bpm >= 128;
    const rows = isTechno
      ? ['Kick', 'Sub/Bass', 'Perc/Hats', 'Mid/Lead', 'Pads/Strings', 'Vox', 'FX/Risers']
      : ['Kick', 'Sub/Bass', 'Pluck/Arp', 'Mid/Lead', 'Pads/Strings', 'Vox', 'Perc/Hats', 'FX/Risers'];

    return {
      schema_version: '0.1.0',
      id: `analysis-${filename}`,
      type: 'analysis' as const,
      title: `Analysis: ${filename}`,
      style: isTechno ? 'techno' as const : 'prog_house' as const,
      reference_artists: [] as string[],
      bpm: Math.round(result.bpm),
      bpm_range: [Math.round(result.bpm - 2), Math.round(result.bpm + 2)] as [number, number],
      swing_percent: 0,
      ghost_note_density: 0,
      rows,
      total_bars: totalBars,
      sections: result.sections.map((s) => {
        const bc = Math.ceil(s.bars / 8);
        const activity: Record<string, number[]> = {};
        for (const row of rows) {
          activity[row] = Array(bc).fill(2);
        }
        return {
          id: `sec-${s.name}-${s.start_bar}`,
          name: s.name,
          color: s.color,
          start_bar: s.start_bar,
          bars: s.bars,
          confidence: s.confidence,
          activity,
          notes: [] as string[],
          references: [] as { artist: string; title: string; start_seconds: number; end_seconds: number; note: string }[],
        };
      }),
      analysis_metadata: {
        source_file: filename,
        source_kind: 'local' as const,
        duration_seconds: result.duration_seconds,
        key_camelot: result.key_camelot,
        key_standard: result.key_standard,
        bpm_confidence: result.bpm_confidence,
        downbeat_offset_seconds: result.downbeat_offset_seconds,
        analysed_at: new Date().toISOString(),
        analyser_version: '0.1.0',
      },
    };
  }, []);

  const formatDuration = (sec: number): string => {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  const analysisDoc = useRef<ArrangementDoc | null>(null);
  if (openTrack && openResult) {
    analysisDoc.current = makeDoc(openResult, openTrack.filename);
    const doc = analysisDoc.current;
    const selectedSection = selectedSectionId
      ? doc.sections.find((s) => s.id === selectedSectionId) ?? null
      : null;

    return (
      <div className="flex flex-1 flex-col" style={{ minHeight: 0 }}>
        <div className="flex items-center gap-3 px-5 py-2 border-b border-white/10">
          <button
            onClick={() => { setOpenTrackId(null); setSelectedSectionId(null); }}
            className="text-xs font-mono text-white/50 hover:text-white transition-colors no-radius"
          >
            ← Back to list
          </button>
          <span className="text-xs font-mono text-white/60">
            Analyze tab
          </span>
          <span className="text-[10px] font-mono text-white/30 ml-auto">
            Click a section header to view details →
          </span>
        </div>
        <div className="flex-1 flex" style={{ minHeight: 0 }}>
          {/* Center — bar grid */}
          <div className="flex-1 p-5 overflow-auto" style={{ minWidth: 0 }}>
            <div className="mb-2 text-xs font-mono text-white/50 flex items-center gap-3 flex-wrap">
              <span className="bg-white/10 px-2 py-0.5">🔍 Analysed — {openTrack.filename}</span>
              <span>{Math.round(result.bpm)} BPM</span>
              <span>{result.key_camelot} ({result.key_standard})</span>
              <span>Conf: {(result.bpm_confidence * 100).toFixed(0)}%</span>
              <span className="text-white/40 font-mono text-[10px]">
                {result.sections.length} sections
              </span>
              <span className="text-white/40 font-mono text-[10px]">
                avg confidence {Math.round(
                  result.sections.reduce((sum, s) => sum + (s.confidence ?? 0), 0) /
                    Math.max(result.sections.length, 1) * 100
                )}%
              </span>
            </div>
            <BarGrid
              doc={doc as any}
              selectedSectionId={selectedSectionId}
              onSelectSection={setSelectedSectionId}
              onCycleCell={() => {}}
              showConfidence={true}
            />
          </div>

          {/* Right panel — section notes */}
          <aside
            className="w-72 shrink-0 p-5 overflow-y-auto"
            style={{ borderLeft: '1px solid rgba(255,255,255,0.08)' }}
          >
            <NotesPanel
              section={selectedSection}
              bpm={Math.round(result.bpm)}
              onUpdateNote={() => {}}
            />
          </aside>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1" style={{ minHeight: 0 }}>
      <div className="flex-1 p-5 overflow-y-auto">
        {/* Drop zone + browse */}
        <div
          className="flex flex-col items-center justify-center"
          style={{
            border: dragOver
              ? '2px solid var(--color-studio-accent, #50FA7B)'
              : '1px solid rgba(255,255,255,0.1)',
            padding: '48px 24px',
            marginBottom: '24px',
            transition: 'border-color 0.15s, border-width 0.15s',
            backgroundColor: dragOver ? 'rgba(80, 250, 123, 0.05)' : 'transparent',
          }}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={(e) => {
            e.preventDefault();
            handleDrop(e.dataTransfer.files);
          }}
        >
          <div className="text-3xl mb-3 opacity-50">🎵</div>
          <div className="text-sm font-mono text-white/60 mb-3">
            Drop an audio file here
          </div>
          <button
            onClick={handleBrowse}
            className="text-xs font-mono no-radius transition-colors cursor-pointer"
            style={{
              backgroundColor: 'transparent',
              border: '1px solid rgba(255,255,255,0.2)',
              color: 'rgba(255,255,255,0.7)',
              padding: '8px 16px',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = 'rgba(255,255,255,0.05)';
              e.currentTarget.style.borderColor = 'var(--color-studio-accent, #50FA7B)';
              e.currentTarget.style.color = 'var(--color-studio-accent, #50FA7B)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = 'transparent';
              e.currentTarget.style.borderColor = 'rgba(255,255,255,0.2)';
              e.currentTarget.style.color = 'rgba(255,255,255,0.7)';
            }}
          >
            Browse files...
          </button>
          <div className="text-[10px] font-mono text-white/30 mt-3">
            MP3, WAV, FLAC, AIFF, M4A
          </div>
        </div>

        {/* YouTube URL input */}
        <div style={{ marginBottom: '16px' }}>
          <div className="flex items-end gap-2">
            <div className="flex-1">
              <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-1.5">
                Or paste YouTube URL
              </div>
              <input
                type="text"
                value={youtubeUrl}
                onChange={(e) => setYoutubeUrl(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleYoutubeSubmit()}
                placeholder="https://www.youtube.com/watch?v=…"
                className="w-full text-xs font-mono text-white/70 bg-white/[0.03] border border-white/10 px-3 py-2 outline-none no-radius
                  focus:border-[var(--color-studio-accent,#50FA7B)] transition-colors
                  placeholder:text-white/20"
              />
            </div>
            <button
              onClick={handleYoutubeSubmit}
              disabled={youtubeLoading || !youtubeUrl.trim()}
              className="text-xs font-mono no-radius transition-colors px-4 py-2 shrink-0 disabled:opacity-30"
              style={{
                backgroundColor: 'transparent',
                border: '1px solid rgba(255,255,255,0.15)',
                color: 'rgba(255,255,255,0.6)',
              }}
            >
              {youtubeLoading ? '…' : 'Analyse'}
            </button>
          </div>
          {!legalAckAccepted && (
            <button
              onClick={() => setShowLegalModal(true)}
              className="text-[10px] font-mono text-white/30 hover:text-white/50 transition-colors mt-1.5 no-radius"
            >
              ⚠️ Legal acknowledgement required for YouTube features
            </button>
          )}
        </div>

        {/* Track list */}
        {tracks.length > 0 && (
          <div>
            <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-2">
              Analysis Queue ({tracks.length})
            </div>
            <div className="flex flex-col gap-[1px]">
              {tracks.map((track) => {
                const isClickable = track.status === 'done' && track.result;
                return (
                  <div
                    key={track.id}
                    onClick={() => isClickable && setOpenTrackId(track.id)}
                    className={`
                      group flex items-center gap-3 px-4 py-2.5 text-xs font-mono
                      transition-all duration-150
                      ${isClickable ? 'cursor-pointer hover:bg-white/[0.06]' : 'cursor-default'}
                    `}
                    style={{
                      backgroundColor: 'rgba(255,255,255,0.015)',
                    }}
                  >
                    {/* Status indicator */}
                    <div
                      className="w-2 h-2 rounded-full shrink-0"
                      style={{
                        backgroundColor:
                          track.status === 'queued' ? 'rgba(255,255,255,0.25)' :
                          track.status === 'downloading' ? '#B8860B' :
                          track.status === 'analysing' ? '#50FA7B' :
                          track.status === 'done' ? '#50FA7B' :
                          '#FF5555',
                        animation: (track.status === 'analysing' || track.status === 'downloading') ? 'pulse 1s infinite' : 'none',
                      }}
                    />

                    {/* Filename */}
                    <span className="flex-1 min-w-0 truncate text-white/70 group-hover:text-white/90 transition-colors">
                      {track.filename}
                    </span>

                    {/* Status badge / metadata */}
                    <span className="shrink-0 flex items-center gap-3 text-[10px]">
                      {track.status === 'queued' && (
                        <span className="text-white/25">Queued</span>
                      )}
                      {track.status === 'downloading' && (
                        <span className="text-[#B8860B]">
                          Downloading{downloadProgress[track.id] !== undefined ? ` ${Math.round(downloadProgress[track.id])}%` : '…'}
                        </span>
                      )}
                      {track.status === 'analysing' && (
                        <span className="text-[#50FA7B]">Analysing…</span>
                      )}
                      {track.status === 'done' && track.result && (
                        <>
                          <span className="text-white/35 tabular-nums">{Math.round(track.result.bpm)} BPM</span>
                          <span className="text-white/35">{track.result.key_camelot}</span>
                          <span className="text-white/25 tabular-nums">{formatDuration(track.result.duration_seconds)}</span>
                        </>
                      )}
                      {track.status === 'error' && (
                        <span className="text-red-400/70 truncate max-w-[160px]">
                          {track.error || 'Error'}
                        </span>
                      )}
                    </span>

                    {/* Delete button — visible on hover */}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDelete(track.id);
                      }}
                      className="
                        shrink-0 text-white/15 hover:text-red-400
                        opacity-0 group-hover:opacity-100
                        transition-all duration-150 no-radius ml-1 px-1
                      "
                      style={{ fontSize: '11px' }}
                      title="Remove"
                    >
                      ✕
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {tracks.length === 0 && (
          <div className="text-center py-12">
            <div className="text-xs font-mono text-white/30">
              No tracks analysed yet. Drop an audio file to begin.
            </div>
          </div>
        )}

        {/* Legal acknowledgement modal */}
        {showLegalModal && (
          <div
            className="fixed inset-0 z-50 flex items-center justify-center"
            style={{ backgroundColor: 'rgba(0,0,0,0.7)' }}
            onClick={() => setShowLegalModal(false)}
          >
            <div
              className="max-w-md mx-4 p-6"
              style={{
                backgroundColor: '#1a1a1a',
                border: '1px solid rgba(255,255,255,0.12)',
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h2 className="text-sm font-mono text-white mb-4">
                ⚖️ Legal Acknowledgement
              </h2>
              <div className="text-xs font-mono text-white/60 leading-relaxed space-y-3 mb-5">
                <p>
                  I confirm I am downloading audio <strong className="text-white/80">solely for personal reference and analysis</strong>
                  (fair-use educational purposes), and that I will not redistribute, publish, or
                  use the downloaded audio commercially.
                </p>
                <p className="text-white/40">
                  Downloading copyrighted material may violate YouTube's Terms of Service in
                  some jurisdictions; I take sole responsibility for my use of this feature.
                </p>
              </div>
              <div className="flex gap-3 justify-end">
                <button
                  onClick={() => setShowLegalModal(false)}
                  className="text-xs font-mono text-white/40 hover:text-white/70 transition-colors no-radius px-3 py-1.5"
                >
                  Cancel
                </button>
                <button
                  onClick={handleAcceptLegal}
                  className="text-xs font-mono no-radius px-4 py-1.5 transition-colors"
                  style={{
                    backgroundColor: 'var(--color-studio-accent, #50FA7B)',
                    color: '#000',
                    fontWeight: 600,
                  }}
                >
                  I Accept
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
