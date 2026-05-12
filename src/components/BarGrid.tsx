import { useState, useCallback } from 'react';
import type { ArrangementDoc, Intensity } from '../types';
import SectionHeaders from './SectionHeaders';

const INTENSITY_COLORS: Record<Intensity, string> = {
  0: 'rgba(255,255,255,0.05)',
  1: 'rgba(255,255,255,0.25)',
  2: 'rgba(255,255,255,0.55)',
  3: 'rgba(255,255,255,1.0)',
};

const INTENSITY_LABELS: Record<Intensity, string> = {
  0: 'Off',
  1: 'Low',
  2: 'Mid',
  3: 'Full',
};

const BLOCK_SIZE = 64; // px per 8-bar block
const ROW_HEIGHT = 28; // px per row

function getSectionForBlock(sections: ArrangementDoc['sections'], blockIndex: number): { name: string; startBar: number } | null {
  const barStart = blockIndex * 8 + 1;
  for (const section of sections) {
    const secStart = section.start_bar;
    const secEnd = section.start_bar + section.bars - 1;
    if (barStart >= secStart && barStart <= secEnd) {
      return { name: section.name, startBar: secStart };
    }
  }
  return null;
}

interface TooltipData {
  x: number;
  y: number;
  rowName: string;
  sectionName: string;
  barStart: number;
  barEnd: number;
  intensity: Intensity;
}

interface BarGridProps {
  doc: ArrangementDoc;
  selectedSectionId: string | null;
  onSelectSection: (id: string) => void;
  onCycleCell: (sectionId: string, rowName: string, cellIndex: number) => void;
  showConfidence?: boolean;
}

export default function BarGrid({ doc, selectedSectionId, onSelectSection, onCycleCell, showConfidence }: BarGridProps) {
  const blockCount = Math.ceil(doc.total_bars / 8);
  const [tooltip, setTooltip] = useState<TooltipData | null>(null);

  const handleMouseEnter = useCallback(
    (e: React.MouseEvent, rowName: string, blockIndex: number) => {
      const section = getSectionForBlock(doc.sections, blockIndex);
      if (!section) return;
      const barStart = blockIndex * 8 + 1;
      const barEnd = Math.min(barStart + 7, doc.total_bars);

      // Find the intensity for this row+block in the relevant section
      const foundSection = doc.sections.find((s) => s.id === doc.sections.find((s) => s.start_bar <= barStart && s.start_bar + s.bars > blockIndex * 8)?.id);
      let intensity: Intensity = 0;
      if (foundSection) {
        const rowActivity = foundSection.activity[rowName];
        if (rowActivity) {
          const localBlockIndex = Math.floor((blockIndex * 8 + 1 - foundSection.start_bar) / 8);
          if (localBlockIndex >= 0 && localBlockIndex < rowActivity.length) {
            intensity = rowActivity[localBlockIndex] as Intensity;
          }
        }
      }

      setTooltip({
        x: e.clientX,
        y: e.clientY,
        rowName,
        sectionName: section.name,
        barStart,
        barEnd,
        intensity,
      });
    },
    [doc],
  );

  const handleMouseLeave = useCallback(() => {
    setTooltip(null);
  }, []);

  const handleCellClick = useCallback(
    (blockIndex: number, rowName: string) => {
      // Find which section this block belongs to
      const barStart = blockIndex * 8 + 1;
      const section = doc.sections.find(
        (s) => barStart >= s.start_bar && barStart < s.start_bar + s.bars,
      );
      if (!section) return;

      const localBlockIndex = Math.floor((barStart - section.start_bar) / 8);
      onCycleCell(section.id, rowName, localBlockIndex);
    },
    [doc.sections, onCycleCell],
  );

  const getCellIntensity = (rowName: string, blockIndex: number): Intensity => {
    const barStart = blockIndex * 8 + 1;
    const section = doc.sections.find(
      (s) => barStart >= s.start_bar && barStart < s.start_bar + s.bars,
    );
    if (!section) return 0;
    const rowActivity = section.activity[rowName];
    if (!rowActivity) return 0;
    const localBlockIndex = Math.floor((barStart - section.start_bar) / 8);
    if (localBlockIndex < 0 || localBlockIndex >= rowActivity.length) return 0;
    return rowActivity[localBlockIndex] as Intensity;
  };

  return (
    <div className="relative overflow-x-auto" style={{ paddingBottom: '4px' }}>
      {/* Section headers row */}
      <SectionHeaders
        sections={doc.sections}
        totalBars={doc.total_bars}
        selectedSectionId={selectedSectionId}
        onSelectSection={onSelectSection}
        showConfidence={showConfidence}
      />

      {/* Bar number labels */}
      <div
        className="no-radius"
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${blockCount}, ${BLOCK_SIZE}px)`,
          gap: '1px',
          marginBottom: '1px',
        }}
      >
        {Array.from({ length: blockCount }, (_, i) => (
          <div
            key={i}
            className="no-radius flex items-center justify-center text-[10px] font-mono text-white/35 h-4"
          >
            {i * 8 + 1}
          </div>
        ))}
      </div>

      {/* Grid rows */}
      {doc.rows.map((rowName) => (
        <div
          key={rowName}
          className="no-radius"
          style={{
            display: 'grid',
            gridTemplateColumns: `repeat(${blockCount}, ${BLOCK_SIZE}px)`,
            gap: '1px',
            height: `${ROW_HEIGHT}px`,
            marginBottom: '1px',
          }}
        >
          {Array.from({ length: blockCount }, (_, blockIndex) => {
            const intensity = getCellIntensity(rowName, blockIndex);
            return (
              <div
                key={blockIndex}
                className="no-radius cursor-pointer transition-colors duration-75"
                style={{
                  backgroundColor: INTENSITY_COLORS[intensity],
                  border: '1px solid rgba(255,255,255,0.04)',
                  height: `${ROW_HEIGHT}px`,
                }}
                onClick={() => handleCellClick(blockIndex, rowName)}
                onMouseEnter={(e) => handleMouseEnter(e, rowName, blockIndex)}
                onMouseLeave={handleMouseLeave}
              />
            );
          })}
        </div>
      ))}

      {/* Row labels side */}
      <div
        className="absolute left-0 top-0 flex flex-col pointer-events-none"
        style={{ marginTop: '36px' }} // offset for section headers + bar numbers
      >
        {doc.rows.map((rowName) => (
          <div
            key={rowName}
            className="no-radius flex items-center justify-end pr-2 text-[11px] font-mono text-white/40"
            style={{ height: `${ROW_HEIGHT + 1}px` }}
          >
            {rowName}
          </div>
        ))}
      </div>

      {/* Tooltip */}
      {tooltip && (
        <div
          className="no-radius fixed z-50 pointer-events-none px-2 py-1 text-xs font-mono"
          style={{
            left: `${tooltip.x + 12}px`,
            top: `${tooltip.y + 12}px`,
            backgroundColor: '#000',
            border: '1px solid rgba(255,255,255,0.2)',
            color: 'rgba(255,255,255,0.9)',
          }}
        >
          <div className="text-white/60">{tooltip.rowName}</div>
          <div>
            <span className="text-white/80">{tooltip.sectionName}</span>
          </div>
          <div>
            Bars {tooltip.barStart}–{tooltip.barEnd}
          </div>
          <div>
            <span
              className="inline-block w-2 h-2 mr-1 align-middle"
              style={{ backgroundColor: INTENSITY_COLORS[tooltip.intensity] }}
            />
            {INTENSITY_LABELS[tooltip.intensity]}
          </div>
        </div>
      )}
    </div>
  );
}
