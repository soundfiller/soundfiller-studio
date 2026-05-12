import { create } from 'zustand';
import type { ArrangementDoc, Intensity, Section } from '../types';
import { classicPrydz } from '../templates';

interface ProjectStore {
  doc: ArrangementDoc;
  selectedSectionId: string | null;

  setBpm: (bpm: number) => void;
  cycleCellIntensity: (sectionId: string, rowName: string, cellIndex: number) => void;
  selectSection: (sectionId: string | null) => void;
  updateNote: (sectionId: string, noteIndex: number, text: string) => void;

  selectedSection: Section | null;
  totalDurationSeconds: number;
}

function durationForDoc(doc: ArrangementDoc): number {
  return (doc.total_bars * 4 * 60) / doc.bpm;
}

function nextIntensity(current: Intensity): Intensity {
  const cycle: Intensity[] = [0, 1, 2, 3];
  const idx = cycle.indexOf(current);
  return cycle[(idx + 1) % cycle.length] as Intensity;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  doc: { ...classicPrydz, sections: classicPrydz.sections.map((s) => ({ ...s, activity: { ...s.activity }, notes: [...s.notes], references: [...s.references] })) },
  selectedSectionId: classicPrydz.sections[0]?.id ?? null,

  setBpm: (bpm: number) =>
    set((state) => ({
      doc: { ...state.doc, bpm },
    })),

  cycleCellIntensity: (sectionId: string, rowName: string, cellIndex: number) =>
    set((state) => {
      const sections = state.doc.sections.map((sec) => {
        if (sec.id !== sectionId) return sec;
        const activity = { ...sec.activity };
        const row = [...(activity[rowName] ?? [])];
        if (cellIndex >= 0 && cellIndex < row.length) {
          row[cellIndex] = nextIntensity(row[cellIndex] as Intensity);
        }
        activity[rowName] = row;
        return { ...sec, activity };
      });
      return { doc: { ...state.doc, sections } };
    }),

  selectSection: (sectionId: string | null) =>
    set({ selectedSectionId: sectionId }),

  updateNote: (sectionId: string, noteIndex: number, text: string) =>
    set((state) => {
      const sections = state.doc.sections.map((sec) => {
        if (sec.id !== sectionId) return sec;
        const notes = [...sec.notes];
        if (noteIndex >= 0 && noteIndex < notes.length) {
          notes[noteIndex] = text;
        }
        return { ...sec, notes };
      });
      return { doc: { ...state.doc, sections } };
    }),

  get selectedSection(): Section | null {
    const { doc, selectedSectionId } = get();
    if (!selectedSectionId) return null;
    return doc.sections.find((s) => s.id === selectedSectionId) ?? null;
  },

  get totalDurationSeconds(): number {
    return durationForDoc(get().doc);
  },
}));
