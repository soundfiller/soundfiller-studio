export type Style = 'techno' | 'prog_house' | 'deep_house';
export type DocType = 'template' | 'project' | 'analysis';
export type Intensity = 0 | 1 | 2 | 3;

export interface ReferenceCitation {
  artist: string;
  title: string;
  start_seconds: number;
  end_seconds: number;
  note: string;
}

export interface Section {
  id: string;
  name: string;
  color: string;
  start_bar: number;
  bars: number;
  activity: Record<string, Intensity[]>;
  notes: string[];
  references: ReferenceCitation[];
}

export interface AnalysisMetadata {
  source_file?: string;
  source_url?: string;
  source_kind?: 'local' | 'youtube';
  duration_seconds?: number;
  key_camelot?: string;
  key_standard?: string;
  bpm_confidence?: number;
  downbeat_offset_seconds?: number;
  analysed_at?: string;
  analyser_version?: string;
}

export interface SectionData {
  name: string;
  start_bar: number;
  bars: number;
  color: string;
  confidence: number;
}

export interface AnalysisResult {
  bpm: number;
  bpm_confidence: number;
  key_camelot: string;
  key_standard: string;
  key_confidence: number;
  downbeat_offset_seconds: number;
  beat_positions_seconds: number[];
  duration_seconds: number;
  sections: SectionData[];
}

export interface ArrangementDoc {
  schema_version: string;
  id: string;
  type: DocType;
  title: string;
  style: Style;
  reference_artists: string[];
  bpm: number;
  bpm_range: [number, number];
  swing_percent: number;
  ghost_note_density: number;
  rows: string[];
  total_bars: number;
  sections: Section[];
  analysis_metadata: AnalysisMetadata | null;
}
