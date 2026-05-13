import type { Section } from '../types';

const SECTION_COLORS: Record<string, string> = {
  gray: '#666666',
  amber: '#B8860B',
  red: '#CC3333',
  blue: '#3366CC',
};

interface SectionHeadersProps {
  sections: Section[];
  totalBars: number;
  selectedSectionId: string | null;
  onSelectSection: (id: string) => void;
  showConfidence?: boolean;
}

const BLOCK_SIZE = 64; // px per 8-bar block — matches BarGrid
const LABEL_WIDTH = 110; // px for row label gutter — matches BarGrid

export default function SectionHeaders({
  sections,
  totalBars,
  selectedSectionId,
  onSelectSection,
  showConfidence,
}: SectionHeadersProps) {
  const blockCount = Math.ceil(totalBars / 8);

  const confidenceColor = (conf: number): string => {
    if (conf >= 0.8) return '#50FA7B';
    if (conf >= 0.6) return '#B8860B';
    return '#FF5555';
  };

  return (
    <div
      className="relative"
      style={{
        display: 'grid',
        gridTemplateColumns: `${LABEL_WIDTH}px repeat(${blockCount}, ${BLOCK_SIZE}px)`,
        gap: '1px',
        marginBottom: '1px',
      }}
    >
      {/* Empty cell to fill the label column */}
      <div className="no-radius" />
      {sections.map((section) => {
        const startBlock = Math.floor((section.start_bar - 1) / 8);
        const blockSpan = Math.ceil(section.bars / 8);
        const color = SECTION_COLORS[section.color] ?? '#666666';
        const isSelected = section.id === selectedSectionId;
        const sectionData = (section as unknown) as { confidence?: number };
        const hasConfidence = showConfidence && sectionData.confidence !== undefined;

        return (
          <button
            key={section.id}
            onClick={() => onSelectSection(section.id)}
            title={`${section.name} — Bars ${section.start_bar}–${section.start_bar + section.bars - 1}${hasConfidence ? ` (conf: ${(sectionData.confidence! * 100).toFixed(0)}%)` : ''}`}
            className="no-radius h-8 flex items-center justify-center text-[11px] font-mono tracking-tight transition-colors border-0 cursor-pointer"
            style={{
              gridColumn: `${startBlock + 1} / span ${blockSpan}`,
              backgroundColor: isSelected ? 'var(--color-studio-accent)' : color,
              color: isSelected ? '#000' : 'rgba(255,255,255,0.8)',
              fontWeight: isSelected ? 600 : 400,
              position: 'relative',
            }}
          >
            <span className="leading-none">{section.name}</span>
            {hasConfidence && (
              <span
                className="text-[9px] ml-0.5"
                style={{ color: confidenceColor(sectionData.confidence!) }}
              >
                ·{(sectionData.confidence! * 100).toFixed(0)}%
              </span>
            )}

          </button>
        );
      })}
    </div>
  );
}
