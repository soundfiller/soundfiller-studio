interface BpmPickerProps {
  bpm: number;
  totalBars: number;
  onBpmChange: (bpm: number) => void;
}

const COMMON_BPMS = [122, 124, 126, 128];

function formatDuration(totalBars: number, bpm: number): string {
  const totalSeconds = (totalBars * 4 * 60) / bpm;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = Math.round(totalSeconds % 60);
  return `${minutes}:${String(seconds).padStart(2, '0')}`;
}

export default function BpmPicker({ bpm, totalBars, onBpmChange }: BpmPickerProps) {
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value, 10);
    if (!isNaN(val) && val >= 60 && val <= 200) {
      onBpmChange(val);
    }
  };

  const increment = () => onBpmChange(Math.min(bpm + 1, 200));
  const decrement = () => onBpmChange(Math.max(bpm - 1, 60));

  return (
    <div className="flex flex-col gap-4">
      {/* BPM display */}
      <div>
        <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-1">BPM</div>
        <div className="flex items-center gap-0" style={{ border: '1px solid rgba(255,255,255,0.1)' }}>
          <button
            onClick={decrement}
            className="no-radius px-2 py-1 text-white/60 hover:text-white hover:bg-white/10 text-sm font-mono cursor-pointer border-0"
            style={{ minWidth: '28px' }}
          >
            −
          </button>
          <input
            type="number"
            value={bpm}
            onChange={handleChange}
            className="no-radius w-16 text-center bg-transparent text-white font-mono text-lg outline-none border-0"
            style={{ borderLeft: '1px solid rgba(255,255,255,0.1)', borderRight: '1px solid rgba(255,255,255,0.1)' }}
            min={60}
            max={200}
          />
          <button
            onClick={increment}
            className="no-radius px-2 py-1 text-white/60 hover:text-white hover:bg-white/10 text-sm font-mono cursor-pointer border-0"
            style={{ minWidth: '28px' }}
          >
            +
          </button>
        </div>
      </div>

      {/* Common BPM presets */}
      <div className="flex flex-wrap gap-1">
        {COMMON_BPMS.map((preset) => (
          <button
            key={preset}
            onClick={() => onBpmChange(preset)}
            className={`no-radius px-2 py-1 text-xs font-mono transition-colors cursor-pointer border-0 ${
              bpm === preset
                ? 'text-black bg-[var(--color-studio-accent)]'
                : 'text-white/50 hover:text-white/80 bg-white/5'
            }`}
          >
            {preset}
          </button>
        ))}
      </div>

      {/* Duration display */}
      <div>
        <div className="text-[10px] font-mono text-white/35 uppercase tracking-wider mb-1">Duration</div>
        <div className="font-mono text-white/80 text-base">
          {formatDuration(totalBars, bpm)}
        </div>
      </div>
    </div>
  );
}
