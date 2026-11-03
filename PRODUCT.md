# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Desktop app for macOS only (Tauri 2 webview, Next.js 14 static export, Tailwind 3 + shadcn/ui). It lives in the menu bar and opens one main window; a small always-on-top banner window appears when a call starts. Native macOS expectations apply: follows system light/dark appearance, native title bar, standard controls.

## Users

Vito (Italian developer at an IT company) and a few colleagues, all on Mac. They spend much of the day in Microsoft Teams calls (also Zoom and Google Meet), often with clients. They need a reliable record of who said what, to write meeting minutes afterwards with the `minuta-call` skill.

## Product Purpose

Dixit records and transcribes calls locally, without being asked. It notices when a call starts on Teams, Zoom or Meet, starts recording with a cancellable notice, and stops when the call ends. It attributes every line to a real person: the microphone is the user, the call audio is diarized locally, and on Teams the names come from live captions and from Teams' own transcription, which Dixit starts automatically. Success: no call lost, no recording left running after a call, every Teams turn carrying a real name, minutes written without manual fixes.

## Positioning

Fully local (audio and text never leave the Mac) and fully automatic, with real speaker names taken from Teams itself instead of anonymous "Speaker 1". Competitors either need a bot in the call, a cloud service, or a manual start.

## Operating Context

- During a call: the window is often open on a second screen to watch the live transcript scroll; the user must not be disturbed (no focus stealing, nothing visible while screen sharing).
- After a call: the user reviews the transcript, renames speakers, generates an AI summary inside the app (Meetily's summary feature with local or chosen providers), and opens the transcript file.
- Meetings are saved under `~/Documents/Dixit/<YYYY-MM-DD HH.MM> <title>/` (audio, JSON, a Markdown transcript in Teams format readable by `minuta-call`). `~/Documents` is iCloud-synced on the owner's Mac.
- Calls run from 1 minute to 3.5 hours; ~65 meetings recorded in five months.

## Capabilities and Constraints

- Surfaces in use: live transcript during recording, meeting detail (transcript + speakers + AI summary editor), sidebar archive of meetings, settings (general, automation, recordings, transcription, summary), onboarding with macOS permissions (microphone, screen recording, accessibility), the call-start banner, the menu bar menu.
- Speakers are first-class: each line belongs to a named person or a lettered placeholder ("Parlante A"), and names can be corrected after the call.
- UI language: Italian and English (follows the system language).
- Light and dark appearance, following macOS.
- Inherited from Meetily (MIT): summary providers, model management (Whisper/Parakeet), audio import. Must keep working.
- Undecided: none recorded.

## Brand Commitments

- Name: **Dixit** (Latin, "said": who said what). Replaces the provisional name "Trascrivi" and the Meetily brand.
- Visual identity: antracite #2B2B2D and white, petrolio #1F7E92, green #8DC928, the blue→green scale (#2099D8 … #7EC44E); Montserrat Bold for titles, Open Sans for text. Clean, Mac-like, no themed decoration.
- Rejected by the user: the "Epigrafe romana" world (inscriptional capitals, bronze, stone), judged too themed, too dark and cluttered.
- Must not look like Meetily. Dixit has its own mark.

## Evidence on Hand

- Real meetings in `~/Movies/trascrivi-recordings/` (older) and the Meetily archive; no testimonials, metrics or customers exist and none may be invented.
- App icon sources in `frontend/src-tauri/icons/source/` (to be replaced).

## Product Principles

1. Automatic first: the best interaction is the one the user never had to start.
2. Never in the way during a call: quiet, no focus stealing, honest about what is running (Teams native transcription is visible to everyone and must always be announced).
3. Names are the product: who said what must be visible and correctable everywhere.
4. Local and trustworthy: nothing leaves the Mac; state (recording, Teams integration, saving) is always legible.
