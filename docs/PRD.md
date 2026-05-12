# PRD — Soundfiller Studio

**Product name:** Soundfiller Studio
**Tagline:** *Everything living has a rhythm.*
**Publisher:** Soundfiller
**Owner:** Måns Petterson
**Status:** Draft v2.0
**Date:** 11 May 2026
**Target stack:** Tauri 2.x (Rust backend + Web frontend)
**Target platforms:** macOS (primary, Apple Silicon + Intel), Windows 11, Linux (best-effort)

---

## 1. Summary

Soundfiller Studio is a desktop arrangement planner and analyser for electronic music producers working in Logic Pro. It combines two halves into a single workflow:

1. **Plan** — pick a genre/sub-style template, see a bar-by-bar element activity grid plus per-section production notes, and export markers to Logic Pro.
2. **Analyse** — drop in a reference track (local audio file or YouTube URL), the app detects BPM, key, beat grid, and segments the arrangement, then maps it back into the same grid view so the producer can A/B their work in progress against an actual track.

Both halves share the same data model: a track is a sequence of named bar-aligned sections, each with element-layer activity (0–3 intensity) and freeform notes. Templates ship with the app; analysed tracks become user-saved references.

Starting genres: **Techno**, **Progressive House**, and roadmapped **Deep House**. Reference artists drawn from the Soundfiller inspiration list: Egbert, Deadmau5, Eric Prydz, Dubfire, Oliver Huntemann, Stephan Bodzin, HI-LO, Adam Beyer, Alex Stein, Wehbba — plus Charlotte de Witte, Lilly Palmer, Pfirter, FJAAK, Thomas Schumacher, Enrico Sangiuliano, Cirez D.

---

## 2. Problem and goal

### Problem
Producing club-ready techno or progressive house is 80% arrangement and 20% sound design. Most producers get stuck in a loop — an 8-bar idea that never becomes a track — because they lack a clear blueprint for how the bar count, element layering, and tension arc actually unfold across 6–10 minutes. Studying reference tracks by ear works but is slow, and DAW-side tools like Logic's arrangement markers are blank canvases with no guidance.

### Goal
A single tool that (a) gives the producer a credible starting blueprint matched to the style and reference artists they're targeting, and (b) lets them ingest any reference track to see its arrangement structure laid out in the same grammar — so they can copy proven structures bar-for-bar into Logic Pro.

### Non-goals
- Not a DAW. No audio editing, no MIDI generation, no plugins.
- Not a DJ tool. No mixing, no cue points beyond arrangement markers.
- Not a sample pack or preset library.
- Not a cloud collaboration platform (v1 is single-user, local-first).

---

## 3. Target user

- **Primary:** Soundfiller himself, plus hobbyist-to-intermediate Logic Pro producers making 4/4 dance music at 118–140 BPM who have ideas but struggle to finish tracks.
- **Secondary:** Working DJs who want to deconstruct sets and reference tracks to learn what makes them work.
- **Tertiary:** Music educators teaching electronic music production who want a visual aid.

Skill assumption: knows Logic Pro basics, can identify a kick from a hi-hat, understands what a breakdown is. Not assumed to read music or know DSP.

---

## 4. Success metrics

- **Time-to-marker-export** for a new project < 90 seconds from app launch.
- **Analysis runtime** for a 7-minute MP3 < 30 seconds on M1 Pro.
- **BPM accuracy** within ±1 BPM on ≥90% of 4/4 dance music in the 118–140 BPM range (benchmarked against a curated set of 75 tracks across the reference-artist catalogue).
- **Section-boundary accuracy** within ±2 bars on ≥80% of analysed tracks for the four primary boundaries (intro→build, build→drop, drop→break, break→drop2).
- **Retention proxy:** ≥3 tracks analysed and ≥1 marker file exported in the first week of use.

---

## 5. Feature scope

### 5.1 v1.0 (this PRD)

| ID | Feature | Priority |
|----|---------|----------|
| F1 | Template library — 13 templates across Techno + Progressive House | Must |
| F2 | Bar-grid view with element-activity heatmap | Must |
| F3 | Per-section production notes panel | Must |
| F4 | BPM picker with live duration recalculation | Must |
| F5 | Reference-track field per section (artist, title, timestamps) | Must |
| F6 | Local file drag-and-drop ingestion (MP3, WAV, FLAC, AIFF, M4A) | Must |
| F7 | YouTube URL ingestion (with legal-use acknowledgement gate) | Must |
| F8 | Automatic BPM + key detection on ingested audio | Must |
| F9 | Automatic beat-grid + downbeat detection | Must |
| F10 | Automatic section segmentation (intro/build/drop/break/outro) | Must |
| F11 | Analysed track rendered into the same bar-grid view as templates | Must |
| F12 | Swing/groove field per template (16th swing %, ghost-note density) | Should |
| F13 | Export to Logic Pro marker text file | Must |
| F14 | Save/load user projects (`.argrid` JSON file format) | Must |
| F15 | User-defined templates (clone-and-modify any built-in template) | Should |
| F16 | Side-by-side comparison view (template vs analysed track) | Should |
| F17 | Soundfiller branded splash, About screen, and marker-file header | Must |

### 5.2 v1.1 roadmap (planned, not in scope)
- Deep House templates (2 starting templates defined in §6.1.3)
- Stem separation (Demucs) for richer per-row activity in analysed tracks
- Tech house templates

### 5.3 Out of scope for v1.0
- More genres beyond v1.1 roadmap (melodic house, trance, drum & bass, hard techno >140 BPM).
- MIDI export of drum patterns or chord progressions.
- VST/AU plugin version.
- Cloud sync, sharing, public template gallery.
- Mobile companion app.
- Live DAW integration (OSC/MIDI clock sync).

---

## 6. Detailed feature spec

### 6.1 F1 — Template library

13 templates ship with v1.0 across two styles. Deep house (2 more templates) follows in v1.1.

#### 6.1.1 Techno (128–138 BPM) — 8 templates

1. **Driving peak-time** — *Adam Beyer, Wehbba, Pfirter, Dubfire*
   Classic Drumcode/SCI+TEC peak-time arrangement. 128 bars, 32-bar intro, 16-bar breakdown.

2. **Dark hypnotic** — *Charlotte de Witte, Lilly Palmer*
   144 bars, 32-bar long-form breakdown, rigid no-swing 4/4, ice-cannon-ready drops.

3. **Acid-driven** — *HI-LO, Egbert*
   303-line as protagonist. Filter automation continuous throughout. 128 bars.

4. **Industrial / hard** — *FJAAK, Thomas Schumacher*
   Distorted kick, screaming leads, cold mechanical breakdown. 135+ BPM.

5. **Melodic techno (cinematic)** — *Stephan Bodzin*
   Bass-line-as-melody. 156 bars, 48-bar cinematic breakdown, slower tempo (123–126 BPM). Bodzin's signature is the long-form modular synth journey — this template reflects that.

6. **Melodic techno (driving)** — *Enrico Sangiuliano, Wehbba (melodic side)*
   144 bars, 32-bar emotional break, mid-tempo (127–130 BPM). Tighter than the Bodzin template, more dancefloor-focused.

7. **Dark driving** — *Oliver Huntemann, Alex Stein*
   The technical, groovy middle ground between peak-time and dark hypnotic. 128 bars, 16-bar break, more swing on the hi-hats than the pure peak-time template.

8. **Peak-time melodic crossover** — *Dubfire, Alex Stein, Sangiuliano collabs*
   Peak-time energy with melodic motifs. Bridges template 1 and template 6. 136 bars.

#### 6.1.2 Progressive house (122–128 BPM) — 5 templates

9. **Classic Prydz** — *Eric Prydz (Pryda alias)*
   The Opus/Pjanoo blueprint. 176 bars, 32-bar emotional breakdown. The reference template for melodic prog.

10. **Deadmau5 minimal** — *Deadmau5*
    Strobe/Ghosts style. 192 bars, 48-bar intro, melody-first, no cliché white-noise builds.

11. **Dark progressive** — *Cirez D, late-night Pryda*
    Tech-y, chunky, darker tonality. 160 bars.

12. **Tech-prog crossover** — *Hernan Cattaneo school, Bedrock-style*
    The clubbier side of progressive house. More chunky tech-house influence than classic Prydz, less euphoric than Anjuna. 168 bars.

13. **Melodic journey** — *Anjuna/Lane 8 leaning*
    Warm, euphoric, vocal-heavy. 176 bars.

#### 6.1.3 Deep house (118–124 BPM) — v1.1 roadmap, 2 templates

14. **Deep house classic** — *Larry Heard / Kerri Chandler lineage*
    Jazzy chords, vocal-led, long-form (8+ minutes). 120 BPM. To be specified during v1.1 spec work.

15. **Melodic deep house** — *Warm, dawn-of-the-festival sound*
    122 BPM, atmospheric, pads-and-pluck-heavy. To be specified during v1.1 spec work.

Each template stores: total bars, default BPM, BPM range, ordered sections (name, bar count, element-activity matrix, production notes, reference-track citations).

### 6.2 F2 — Bar grid

X-axis: bars, grouped in 8-bar blocks (one cell per block). Y-axis: element layers fixed per style.

- **Techno rows:** Kick · Sub/Bass · Stab · Lead/Acid · Pads/Atmo · Vox · Perc/Hats · FX/Risers
- **Prog house rows:** Kick · Sub/Bass · Pluck/Arp · Lead · Pads/Strings · Vox · Perc/Hats · FX/Risers
- **Deep house rows (v1.1):** Kick · Sub/Bass · Chord/Stab · Lead · Pads/Strings · Vox · Perc/Hats · FX/Risers

Each cell has an intensity value 0–3 (off / filtered / mid / full) rendered as a 4-stop colour ramp. Cells are editable on click (cycles 0→1→2→3→0). Hover shows tooltip: row name, section, bar number, intensity label.

Section headers sit above the grid as coloured blocks spanning their bar count. Drag the right edge of a section header to resize (in 8-bar increments). Right-click a section for: rename, change colour, duplicate, delete, insert before/after.

### 6.3 F3 — Production notes panel

Right of (or below) the grid. One card per section with: section name, bar range (e.g. "Bars 49–80, 32 bars"), time range (e.g. "1:34–2:35 at 126 BPM"), bullet list of production notes. Notes are markdown-editable. Cards reorder when sections reorder.

### 6.4 F4 — BPM picker

Numeric input + ± buttons + style-appropriate dropdown of common BPMs (128/130/132/135/138 for techno; 122/124/126/128 for prog house; 120/122/124 for deep house). Changing BPM live-recalculates total duration and per-section time ranges. Does not change bar count.

### 6.5 F5 — Reference-track citations per section

Each section can hold 0–N reference-track citations of the form `{artist, title, timestamp_start, timestamp_end, note}`, e.g. *"Bodzin — Powers of Ten @ 2:30–3:34, this is the bass-line-as-melody moment I want."* These render under the section's notes panel as small clickable rows. Clicking a citation that points to a track already in the user's analysed-tracks library opens that track in the comparison view (F16). Citations are stored with the project.

### 6.6 F6 — Local file drag-and-drop ingestion

Drop zone on the **Analyse** tab. Accepts MP3, WAV, FLAC, AIFF, M4A. Multi-file drop queues files for sequential analysis. File is **copied** into the app's data directory (`~/Library/Application Support/com.soundfiller.studio/audio/` on macOS) so the analysis is reproducible even if the source is moved.

After analysis completes, the track appears in the **Library** sidebar.

### 6.7 F7 — YouTube URL ingestion

Paste field accepting `youtube.com/watch?v=…`, `youtu.be/…`, `youtube.com/shorts/…`. Behind this is a **non-commercial personal-use acknowledgement** the user must accept once per install:

> *I confirm I am downloading audio solely for personal reference and analysis (fair-use educational purposes), and that I will not redistribute, publish, or use the downloaded audio commercially. Downloading copyrighted material may violate YouTube's Terms of Service in some jurisdictions; I take sole responsibility for my use of this feature.*

Implementation uses `yt-dlp` as a sidecar binary bundled with the app (one binary per platform). Audio extracted at best available quality, transcoded to 16-bit 44.1 kHz mono WAV for analysis (and kept as compressed MP3 for playback). The fact that yt-dlp is being used and what it does is disclosed in the app's About screen.

**Alternative path:** if the user declines the acknowledgement, the YouTube field is disabled and a tooltip points to the local-file drop zone instead. A future v1.x feature may add system-audio recording as a third option (record what's already playing through the speakers).

### 6.8 F8 — BPM + key detection

Powered by `stratum-dsp` (pure Rust, no FFI). Outputs:
- BPM rounded to 1 decimal place, with confidence score 0–1.
- Camelot key notation (e.g. 8A, 4B) and standard notation (e.g. A minor, D♭ major).
- Per-segment BPM in case of tempo drift (rare in modern electronic music but possible for hand-played tracks).

If BPM confidence < 0.7, surface a warning chip on the analysed track and allow the user to manually override BPM (with a tap-tempo helper).

### 6.9 F9 — Beat grid + downbeat detection

Detected beats are aligned to the audio and a downbeat (bar 1) inferred. The user can drag the first downbeat to correct it if auto-detection is off — common failure mode for tracks with long ambient intros. All subsequent bar/section detection is relative to this anchor.

### 6.10 F10 — Section segmentation

Two-stage approach:

1. **Energy/feature segmentation.** Compute frame-level features (RMS, spectral centroid, spectral flatness, sub-band energy for kick/bass/mid/high, kick-drum presence binary) at 1 Hz. Detect change-points using a sliding-window cost function (squared loss on the feature vector; minimum segment length = 16 bars). This produces a candidate set of boundaries.

2. **Section labelling.** Each candidate segment is classified into one of {intro, build, drop, break, outro} using simple feature rules first (kick presence + energy + position in track) with optional ML refinement in v1.x:
    - Kick absent + high pad/atmo energy → break
    - Kick present + low energy + position < 25% of track → intro
    - Kick present + rising energy + duration ≤ 32 bars → build
    - Kick present + max energy → drop
    - Kick present + falling energy + position > 75% → outro

Outputs a sequence of `{label, start_bar, end_bar, confidence}` that maps directly into the same bar-grid data model as a template.

### 6.11 F11 — Analysed track in bar-grid view

The analysed track renders in the same grid as templates. Element activity is inferred from sub-band energy:
- Kick row: kick-presence binary (drum-kit detector on the 60–100 Hz band with onset gating)
- Sub/Bass row: 30–150 Hz energy (excluding the kick transients)
- Pads/Atmo row: 150 Hz – 2 kHz sustained energy with low spectral flatness
- Lead/Vox/Stab rows: combined into a "Mid/Lead" row for v1.0 (separating these requires stem separation, see §5.2)
- Perc/Hats row: 4–12 kHz transient energy
- FX/Risers row: spectral flatness rising over ≥4 bars + final-bar peak

Each row's intensity is binned to the 0–3 scale. The analysed grid is **read-only** by default; toggle "Edit copy" to convert it into a user-editable project.

### 6.12 F12 — Swing/groove field

Per template (and per analysed track), a `swing_percent` field (0–25%, default 0 for most techno, but elevated for the dark-driving and Bodzin templates which use micro-timing) and a `ghost_note_density` field (0–3) appear under the BPM picker. These are display-only in v1.0 — they remind the user what micro-timing feel the style calls for. Future versions may export these to Logic Pro groove templates.

For analysed tracks, swing% is inferred from the median offset of off-beat hi-hats from the perfect 16th grid.

### 6.13 F13 — Export to Logic Pro

Two export formats:

1. **Marker text file (`.txt`)** — Logic Pro 11 accepts marker lists pasted into the Marker List editor. Format:

    ```
    # Generated by Soundfiller Studio — soundfiller.com
    # Project: <project name>
    # BPM: 132  |  Total bars: 144
    # Everything living has a rhythm.

    Bar    Position    Length    Name
    1      1 1 1 1     32 0 0 0  Intro
    33     33 1 1 1    16 0 0 0  Build 1
    49     49 1 1 1    32 0 0 0  Drop 1
    ...
    ```

    The user copies this and pastes it via Logic's Marker List "Paste" command. Markers can then be converted to arrangement markers via `Marker > Convert to Arrangement Markers`.

2. **Logic Pro template file (`.logicx`)** — out of scope for v1.0 (would require reverse-engineering Logic's bundle format). Parked for v2.

### 6.14 F14 — Project save/load

`.argrid` is a JSON file with the schema in §7. Save via ⌘S, load via drag-drop or File menu. Recent projects in the start screen.

### 6.15 F15 — User templates

Any project can be saved as a template via File → Save as Template. User templates live in `~/Library/Application Support/com.soundfiller.studio/templates/user/` and appear in the template picker under a "My templates" group.

### 6.16 F16 — Side-by-side comparison view

Two grids stacked vertically (template above, analysed track below) sharing the same X-axis bar scale. Useful for visually checking "is my drop the same length as Bodzin's?". Time scrub bar at the bottom plays the analysed track from a chosen bar (sync requires audio playback engine, see §8.5).

### 6.17 F17 — Soundfiller branding

- **Splash screen** on app launch: Soundfiller robot mark + "Soundfiller Studio" wordmark + tagline "Everything living has a rhythm." (2-second display, dismissable).
- **About screen** (Menu → About): robot mark, version number, "A Soundfiller production — soundfiller.com" link, attribution to yt-dlp, ffmpeg, stratum-dsp, symphonia. License texts accessible from this screen.
- **Window title bar:** "Soundfiller Studio — <project name>"
- **App icon (macOS .icns, Windows .ico, Linux .png):** Soundfiller robot mark on near-black background (`#0A0A0A`) with white mark.
- **Exported marker file:** header comment block as shown in §6.13 above.
- **Empty-state copy** throughout the app uses Soundfiller voice — direct, no marketing fluff. Example empty Library: "No tracks analysed yet. Drop one in to start."

---

## 7. Data model

Single canonical schema for templates, user projects, and analysed tracks.

```json
{
  "schema_version": "1.0",
  "id": "uuid-v4",
  "type": "template | project | analysis",
  "title": "Melodic techno (cinematic)",
  "style": "techno | prog_house | deep_house",
  "reference_artists": ["Stephan Bodzin"],
  "bpm": 124,
  "bpm_range": [123, 126],
  "swing_percent": 0,
  "ghost_note_density": 2,
  "rows": ["Kick", "Sub/Bass", "Stab", "Lead/Acid", "Pads/Atmo", "Vox", "Perc/Hats", "FX/Risers"],
  "total_bars": 156,
  "sections": [
    {
      "id": "uuid-v4",
      "name": "Intro",
      "color": "gray",
      "start_bar": 1,
      "bars": 32,
      "activity": {
        "Kick": [3, 3, 3, 3],
        "Sub/Bass": [0, 2, 2, 3],
        "...": "..."
      },
      "notes": [
        "Cinematic pads from bar 1 — minor key, slow attack, long reverb tail.",
        "Bar 17: bass-line begins. Bodzin signature: bass IS the melody, not the kick partner."
      ],
      "references": [
        {
          "artist": "Stephan Bodzin",
          "title": "Powers of Ten",
          "start_seconds": 0,
          "end_seconds": 96,
          "note": "Reference intro layering"
        }
      ]
    }
  ],
  "analysis_metadata": {
    "source_file": "audio/abc123.wav",
    "source_url": "https://youtu.be/xyz",
    "source_kind": "local | youtube",
    "duration_seconds": 412.5,
    "key_camelot": "8A",
    "key_standard": "A minor",
    "bpm_confidence": 0.94,
    "downbeat_offset_seconds": 1.234,
    "analysed_at": "2026-05-11T14:32:00Z",
    "analyser_version": "stratum-dsp 1.0.0"
  }
}
```

`analysis_metadata` is null for templates and user-authored projects.

---

## 8. Architecture

### 8.1 Stack

- **Shell:** Tauri 2.x (Rust)
- **Frontend:** React 18 + TypeScript + Vite, styled with Tailwind. Frontend stays simple — Tauri manages the window, IPC is the contract.
- **State:** Zustand (frontend) + Rust-side state via `tauri::State` for the active project and analysis queue.
- **Persistence:** SQLite via `sqlx` for the library and project index; raw `.argrid` JSON files for portable project storage; raw audio files on disk.
- **IPC:** Tauri commands (request/response) for short ops; Tauri events (broadcast) for analysis progress.

### 8.2 Why Tauri

- Single Rust process owns the audio analysis and file I/O — no Electron memory bloat.
- Bundles small (~10–20 MB shipped), important when shipping yt-dlp + ffmpeg sidecars.
- Cross-platform with one codebase.
- Native file-drag support via `tauri::DragDropEvent`.

### 8.3 Rust crates

| Concern | Crate | Notes |
|---|---|---|
| Audio decoding | `symphonia` | Pure Rust, handles MP3/AAC/FLAC/WAV |
| BPM + key + beat-grid | `stratum-dsp` | Production-grade, DJ-focused, ~88% BPM accuracy on real DJ tracks per author benchmarks |
| Resampling | `rubato` | If `stratum-dsp` doesn't already handle the target sample rate |
| FFT | `rustfft` | For custom feature extraction (sub-band energy, spectral flatness) beyond what `stratum-dsp` provides |
| Audio playback (F16) | `rodio` or `cpal` | Stereo playback with seek; `rodio` is simpler |
| Database | `sqlx` (SQLite) | Async, compile-time-checked queries |
| YouTube ingestion | `yt-dlp` sidecar binary | Invoked via `tauri::api::process::Command` |
| Audio transcoding | `ffmpeg` sidecar binary | For YouTube audio → WAV; also normalizes loudness |

### 8.4 Process layout

```
┌─────────────────────────────────────────────────────────────┐
│ Tauri main process (Rust)                                   │
│ ┌──────────────────────┐   ┌──────────────────────────────┐ │
│ │ Tauri runtime        │   │ Analysis worker pool         │ │
│ │ - IPC commands       │   │ (tokio tasks, 1-N concurrent)│ │
│ │ - File-drop handler  │◀─▶│ - Decode (symphonia)         │ │
│ │ - SQLite (sqlx)      │   │ - Analyse (stratum-dsp +     │ │
│ │ - State<Project>     │   │   custom feature extraction) │ │
│ └──────────┬───────────┘   │ - Emit progress events       │ │
│            │               └──────────────┬───────────────┘ │
│            │                              │                 │
│            ▼                              ▼                 │
│ ┌──────────────────────┐   ┌──────────────────────────────┐ │
│ │ Sidecar: yt-dlp      │   │ Sidecar: ffmpeg              │ │
│ └──────────────────────┘   └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                    │ IPC (commands + events)
                    ▼
┌─────────────────────────────────────────────────────────────┐
│ WebView (React + TS)                                        │
│ - Tabs: Plan, Analyse, Library, Compare                     │
│ - Bar-grid component (canvas-rendered for perf at >256 bars)│
│ - Notes editor                                              │
│ - Drop zone + YouTube URL field                             │
│ - Analysis progress UI (per-track)                          │
└─────────────────────────────────────────────────────────────┘
```

### 8.5 Audio playback engine (for F16)

Decoded audio held in memory for the active comparison track (≤ 100 MB for a 10-minute 44.1 kHz stereo WAV — acceptable). `rodio` for playback with seek; bar-position cursor synced to a `tauri::Event` stream emitting current playback position at 30 Hz.

### 8.6 YouTube ingestion flow

1. User pastes URL into Analyse tab field.
2. Frontend calls Tauri command `ingest_youtube(url, ack_token)`. The ack token is set when the user accepts the legal acknowledgement and stored in app config.
3. Rust validates URL pattern (regex against `youtube.com` / `youtu.be` / `youtube.com/shorts/`).
4. Rust invokes `yt-dlp -x --audio-format wav --audio-quality 0 -o {dataDir}/audio/{uuid}.%(ext)s {url}` via the sidecar.
5. Progress events emitted to frontend (yt-dlp's stdout is parsed for `[download] X%`).
6. Once download completes, the WAV path is queued for analysis (F8–F11).
7. The track shows up in Library with title and artist extracted from yt-dlp's metadata (`%(uploader)s — %(title)s`).

### 8.7 Bundling sidecars

Per Tauri's `bundle.externalBin` configuration, yt-dlp and ffmpeg are bundled per-platform. yt-dlp is ~10 MB (Python frozen binary), ffmpeg is ~70 MB — total app size ~100 MB which is acceptable. Both are GPL/MIT licensed and redistributable. Licenses surfaced in the About screen.

### 8.8 Update strategy

`yt-dlp` requires frequent updates as YouTube changes its API. Two options:

- **Bundled (simple):** ship a version, update via app auto-update (Tauri Updater plugin). Risk: app falls behind, YouTube breaks for users.
- **Self-updating (preferred):** on app launch, check if bundled yt-dlp is older than 30 days; if so, prompt the user to update it (download fresh binary from yt-dlp's GitHub releases). User-cancellable, transparent.

Go with self-updating. Tauri Updater handles the app itself; yt-dlp updates independently.

---

## 9. Visual design

### 9.1 Brand foundation

Soundfiller's visual identity is starkly monochrome: pure black, pure white, hard geometric edges (per the robot mark and the soundfiller.com site). Soundfiller Studio carries this directly into the app UI.

### 9.2 Palette

- **Background:** `#0A0A0A` (near-black, not pure black — eases eye strain in long sessions)
- **Surfaces:** pure black panels (`#000000`) with 1px white-at-10%-opacity borders
- **Text primary:** `#FFFFFF`
- **Text secondary:** `#FFFFFF` at 60% opacity
- **Text tertiary:** `#FFFFFF` at 35% opacity
- **Accent (active state, single colour only):** `#9FE870` acid green — picks up the 303-acid heritage and gives a hint of warmth against the monochrome shell. Used only for: active section in grid, focused input, success states, playback cursor.
- **Warning:** `#FFB020` amber — for low-confidence analysis chips.
- **Error:** `#FF4444` red — destructive actions only.

Element activity heatmap uses a 4-stop white ramp on black:
- Off: `#FFFFFF` at 5% (barely visible cell outline)
- Low: `#FFFFFF` at 25%
- Mid: `#FFFFFF` at 55%
- Full: `#FFFFFF` at 100%

This keeps the grid monochrome-aesthetic and lets the acid-green accent only mark *interactive* state (selected section, hover, focus).

### 9.3 Typography

- **UI / body:** Inter (or Apple system font on macOS — SF Pro). Weights 400 and 500 only.
- **Bar grid / monospaced data (BPM, bar numbers, key labels):** JetBrains Mono or system monospace.
- **Wordmark / splash:** Soundfiller logo lockup, no custom typeface needed beyond what the logo provides.

### 9.4 Geometric rules

- **No rounded corners on grid cells, section blocks, or the bar grid.** Matches the logo's hard-pixel aesthetic.
- **Rounded corners allowed on:** modal dialogs, dropdown menus, buttons (4px radius only — minimal).
- **No gradients. No drop shadows. No glow effects.** Flat fills only.
- **Border weight:** always 1px. Never 0.5px (logo is built on 1px-equivalent unit grid).

### 9.5 Iconography

- Tabler icons (outline weight) for UI controls. Matches the logo's outlined style.
- App icon, splash, and About screen use the Soundfiller robot mark exclusively.

---

## 10. UX flow

### 10.1 First launch
1. Splash: robot mark + "Soundfiller Studio" + "Everything living has a rhythm." (2 seconds).
2. Welcome screen: "Plan a new track" / "Analyse a reference track" / "Open recent".
3. If "Analyse" chosen with a YouTube URL, show legal-acknowledgement modal first (one-time).
4. Onboarding tooltip on the bar grid: "Click a cell to change element intensity. Drag a section's right edge to resize."

### 10.2 Plan-first flow
1. Pick style (Techno / Prog House) → pick template → grid renders.
2. Adjust BPM, edit notes, edit cells, add references.
3. File → Export → Logic Pro markers → save .txt → open Logic Pro, paste in Marker List.
4. Save project (⌘S) → `.argrid` file.

### 10.3 Analyse-first flow
1. Drag MP3 onto the Analyse tab, or paste YouTube URL.
2. Progress bar shows: Downloading (if YT) → Decoding → Detecting tempo → Detecting key → Segmenting → Done.
3. Track opens in grid view with read-only analysis.
4. Optionally: File → Save analysis as project (becomes editable).
5. Use as reference in another project's section citations.

### 10.4 Compare flow
1. Open a project.
2. In the Library sidebar, drag an analysed track onto the project's title bar.
3. Compare view opens: project grid above, analysed grid below, playback transport at bottom.
4. Click any bar on the analysed grid to seek playback there.

---

## 11. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| YouTube breaks yt-dlp ingestion | High (recurring) | High | Self-update of yt-dlp on launch; clear error messages pointing to local-file alternative |
| Section segmentation accuracy below acceptable | Medium | High | User-editable boundaries (drag section edges in analysis view); ship with calibration test set of 75 reference tracks |
| BPM detection fails on certain tracks (half/double tempo confusion) | Medium | Medium | Confidence threshold + tap-tempo manual override; pin BPM to detected ±2x options |
| Legal exposure from YouTube ingestion (DMCA, ToS) | Low for personal use, higher if marketed wrong | High | Explicit acknowledgement gate; positioning the app as a "study tool" not a "YouTube downloader"; no batch-download UI; no playlist support in v1 |
| Apple Notarization friction (sidecar binaries) | Medium | Medium | Sign and notarize ffmpeg + yt-dlp as part of the bundle; this is well-trodden ground in the Tauri community |
| User confusion about "marker file → paste into Logic" workflow | Medium | Low | First-export modal with a 30-second screen recording showing the paste step |
| Performance degradation on long tracks (>15 min DJ sets) | Low | Medium | Cap analysis at 20 minutes for v1; gracefully truncate longer files with a warning |
| `stratum-dsp` API breaks before v1 ships | Low | Medium | Author flags API as stable for BPM/key/beat-grid; pin minor version; have `aubio` (FFI) as a backup option |
| Soundfiller artist brand and consulting brand (Hugiin) get conflated | Low | Low | Soundfiller Studio ships exclusively under soundfiller.com domain; no Hugiin co-branding; About screen explicitly attributes to Soundfiller |

---

## 12. Legal and privacy notes

- App is local-first. No telemetry in v1.0. No accounts. No cloud.
- Downloaded YouTube audio stored only in the user's local app data directory, never uploaded anywhere.
- User-acknowledgement gate makes the user's responsibility for fair-use compliance explicit.
- App description in store listings (when shipped) will frame the YouTube feature as "personal reference and analysis" and will not advertise it as a YouTube downloader.
- yt-dlp and ffmpeg attributions in About screen with links to their license texts.
- Soundfiller Studio is shipped as a Soundfiller production. Distribution lives at `soundfiller.com/studio`. Use Soundfiller's existing Squarespace site as the marketing surface — no separate domain needed.

---

## 13. Open questions

1. **Stem separation in v1.1?** Demucs or Spleeter could give per-row activity for analysed tracks (separating lead vs vox vs stab). But model sizes are 100–500 MB and inference is slow on CPU. Decide after v1.0 ship based on user feedback.
2. **Logic Pro Drummer integration?** Logic auto-generates drummer parts per arrangement marker. Worth a v2 experiment — could the exported markers also seed a drummer track?
3. **Reference set for calibration.** Måns/Soundfiller to provide 75 reference tracks across the 13 templates for analyser benchmarking. Suggest: 5 tracks per techno template × 8 + 5 per prog template × 5 = 65, plus 10 edge cases (DJ tools, hybrid genres, unusual tempos).
4. **Pricing model if shipped beyond personal use.** Options: free download from soundfiller.com, "name your price" via Gumroad, fixed price (suggest €29–€49 one-time), or open-source. Recommend free download for v1.0 to build artist brand and user base; introduce pricing for v2.0 if there's demand. Out of scope for the build PRD.
5. **App vs. artist brand integration.** Should the splash screen play a 2-second Soundfiller audio sting (subtle, low-volume)? Could be a signature touch. Out of scope for v1.0 but worth considering for v1.1.

---

## 14. Milestones

| Milestone | Scope | Estimated effort (solo dev) |
|---|---|---|
| M0 — Spike | Tauri skeleton + grid view rendering one hard-coded template | 1 week |
| M1 — Template library | All 13 templates rendered, BPM picker, notes editor, save/load | 2.5 weeks |
| M2 — Local file analysis | Drag-drop, symphonia decode, stratum-dsp BPM/key/beats, basic grid output | 3 weeks |
| M3 — Section segmentation | Custom feature extraction, change-point detection, section labelling, accuracy tuning against reference set | 3 weeks |
| M4 — YouTube ingestion | yt-dlp + ffmpeg sidecars, legal gate, progress UI | 1.5 weeks |
| M5 — Comparison view + Logic export | F13, F16, polish | 1.5 weeks |
| M6 — Branding pass | Soundfiller splash, About screen, app icons, marker-file headers, full visual design pass | 1 week |
| M7 — Packaging | macOS notarization, Windows code signing, Linux AppImage | 1 week |
| **Total** | | **~14.5 weeks** |

Assumes part-time effort from Olof on the Rust-heavy DSP work (M2/M3) and Måns on the design, templates, and reference-track curation. If Olof is at full SmartCraft capacity, M3 grows by 2–3 weeks.

---

## 15. Appendix A — Element-row taxonomy

The fixed Y-axis rows per style are deliberately small and stable. They are not literal track-mapping (no producer has exactly 8 buses) — they are functional categories of arrangement.

**Techno**
- *Kick* — the 4/4 pulse.
- *Sub/Bass* — anything below ~150 Hz that's not the kick. In Bodzin-style templates, this row is the **melodic protagonist** — the bass IS the lead.
- *Stab* — short chord/note hits, the signature mid-frequency hook.
- *Lead/Acid* — 303-style lines, leads, melodic synths.
- *Pads/Atmo* — sustained background, drones, FX beds.
- *Vox* — chopped vocals, one-shots, full vocal phrases.
- *Perc/Hats* — hi-hats, claps, snares, secondary percussion.
- *FX/Risers* — transitions, sweeps, impacts, downlifters.

**Progressive house**
- *Kick* — 4/4 pulse, usually side-chained.
- *Sub/Bass* — rolling 16th sub, side-chained tightly.
- *Pluck/Arp* — the signature Prydz-style arpeggiated synth.
- *Lead* — the breakdown melody / main hook synth.
- *Pads/Strings* — emotional/atmospheric layers.
- *Vox* — chopped or full vocal lines.
- *Perc/Hats* — hi-hats, shakers, claps.
- *FX/Risers* — transitions and impact moments.

**Deep house (v1.1)**
- *Kick* — 4/4 pulse, often softer/rounder than techno.
- *Sub/Bass* — warm, groovy bass-line.
- *Chord/Stab* — the genre-defining jazzy 7th/9th chord stabs.
- *Lead* — melodic synth or sax-style lead.
- *Pads/Strings* — warm Rhodes-style pads.
- *Vox* — vocal-led genre; this row is often the protagonist.
- *Perc/Hats* — shakers, congas, hand percussion.
- *FX/Risers* — minimal in this genre; mostly textural.

---

## 16. Appendix B — Reference-artist matrix

Mapping which template each Soundfiller-inspiration artist primarily informs. Single source of truth for the in-app "Inspired by..." labels.

| Artist | Primary template | Secondary templates |
|---|---|---|
| Adam Beyer | Driving peak-time | Peak-time melodic crossover |
| Charlotte de Witte | Dark hypnotic | — |
| Lilly Palmer | Dark hypnotic | Dark driving |
| Wehbba | Driving peak-time | Melodic techno (driving), Dark driving |
| Pfirter | Driving peak-time | — |
| FJAAK | Industrial / hard | — |
| Thomas Schumacher | Industrial / hard | Driving peak-time |
| HI-LO | Acid-driven | Driving peak-time |
| Egbert | Acid-driven | — |
| Stephan Bodzin | Melodic techno (cinematic) | — |
| Enrico Sangiuliano | Melodic techno (driving) | Peak-time melodic crossover |
| Oliver Huntemann | Dark driving | Dark hypnotic |
| Alex Stein | Dark driving | Peak-time melodic crossover |
| Dubfire | Peak-time melodic crossover | Driving peak-time |
| Eric Prydz (Pryda) | Classic Prydz | Dark progressive |
| Eric Prydz (Cirez D) | Dark progressive | — |
| Deadmau5 | Deadmau5 minimal | — |
| Hernan Cattaneo | Tech-prog crossover | Classic Prydz |

---

**End of PRD v2.0.**
