import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AnalysisResult } from '../types';

export interface AnalysisStatusEvent {
  id: string;
  status: string;
}

export interface AnalysisCompleteEvent {
  id: string;
  result: AnalysisResult;
}

export async function ingestAudioFile(path: string): Promise<string> {
  return invoke('ingest_audio_file', { path });
}

export async function getAnalysisStatus(id: string): Promise<string> {
  return invoke('get_analysis_status', { id });
}

export async function getAnalysisResult(id: string): Promise<AnalysisResult> {
  return invoke('get_analysis_result', { id });
}

export async function listAnalysedTracks(): Promise<[string, string, AnalysisResult][]> {
  return invoke('list_analysed_tracks');
}

export async function deleteAnalysis(id: string): Promise<void> {
  return invoke('delete_analysis', { id });
}

export function onAnalysisStatusUpdate(
  callback: (event: AnalysisStatusEvent) => void,
): Promise<() => void> {
  return listen<AnalysisStatusEvent>('analysis-status-update', (e) => callback(e.payload));
}

export function onAnalysisComplete(
  callback: (event: AnalysisCompleteEvent) => void,
): Promise<() => void> {
  return listen<AnalysisCompleteEvent>('analysis-complete', (e) => callback(e.payload));
}

export function onAnalysisError(
  callback: (event: { id: string; error: string }) => void,
): Promise<() => void> {
  return listen<{ id: string; error: string }>('analysis-error', (e) => callback(e.payload));
}
