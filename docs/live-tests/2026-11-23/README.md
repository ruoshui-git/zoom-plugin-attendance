# Live test 2026-11-23 — hands, feedback, camera, talking flag

Run fully unattended by an agent against the owner's PMI (1234509876), owner's Zoom Workplace as
**host**, driven by `zoom-companion lab`; four web-client guests driven by
`tools/guest-bot/scenario.mjs`. Zoom Workplace 7.x, Plugin SDK 7.2.0.3001. Findings are summarised in
[`docs/ROADMAP.md`](../../ROADMAP.md) ("Live test results"); this file explains the raw data.

## Files

| File | What it is |
|---|---|
| `host-lab.jsonl` | Everything the host SDK saw: events (`kind:"event"`), lab commands/results, `note` markers, and a full participant list every 1 s (`kind:"poll_diff"`, `payload.users[].flags`). Timestamps are UTC. |
| `lab-cmds.txt` | The lab commands, in order (fed to lab's stdin through `tail -f`). |
| `guest-ctl.txt` | The guest commands, in order (`<Guest|*> <action> [arg]`). |
| `guest-truth.jsonl` | Ground truth: every guest action with `before`/`done` wall-clock times (same machine clock as the lab log). |
| `guests-stdout.txt`, `guests-stdout-2.txt` | Guest runner output for run 1 and run 2 (guests were restarted once after a tooling fix). |
| `cohost-panel-clear-all-feedback.png` | Co-host web view: Participants > More > "Clear All Feedback" (the web client's lower-all). |
| `speech-verified-*.png` | Proof the fake microphone speech reached the meeting: Guest_B's view switched to Serve-G2 as active speaker with audio bars. |

User flag bits (`crates/zoom-bridge/src/ffi.rs`): 1 host, 2 video on, 4 myself, 8 spotlighted,
16 attendee, 32 raised hand, 64 audio muted, 128 talking, 256 in waiting room. Example: 98 = 64+32+2
(muted, hand raised, video on).

## Timeline (UTC, note markers in `host-lab.jsonl`)

| Time | Step |
|---|---|
| 12:53:43 | `start 1234509876` (StartMeeting ok), then `wr off`, `poll 1` |
| 12:54–12:56 | Run 1: Guest_A, Serve-G1, Serve-G2, Guest_B join |
| 12:58:02–12:58:23 | 3 hands raised (React > Raise Hand); Guest_A made co-host |
| 12:59:29 | Run 1 guests leave — 3 of them **with hands still raised** |
| 13:00:53–13:01:44 | Run 2: guests rejoin (new user ids) |
| 13:03:43 | `H1`: 3 hands raised; Guest_A co-host |
| 13:03:59 | `H2`: co-host clicks "Clear All Feedback" |
| 13:04:17 | `H3`: Serve-G1 raises, then leaves with hand up |
| 13:04:47–13:05:05 | `F1`: Guest_B feedback Yes, thumbs-up reaction, "I'm away", video off, video on |
| 13:05:34–13:09:09 | `T1`–`T6`: Serve-G2 unmuted/muted on a schedule while a speech loop plays |
| 13:10:11 | `wr on`, `end` (EndMeeting ok) |

## Reproduce

```bash
# host (owner's Zoom Workplace signed in; one SDK process only)
: > cmds.txt; tail -n +1 -f cmds.txt | target/debug/zoom-companion.exe lab --log host-lab.jsonl &
echo "start 1234509876" >> cmds.txt; echo "wr off" >> cmds.txt; echo "poll 1" >> cmds.txt
# guests (tools/guest-bot; `npm i` once). Last guest gets the fake-mic speech file.
node scenario.mjs 1234509876 <pwd> ctl.txt truth.jsonl Guest_A Guest_B Serve-G1 "Serve-G2@<abs path>\media\speech-pattern.wav"
echo "Guest_A click ^React$" >> ctl.txt; echo "Guest_A click ^Raise Hand$" >> ctl.txt
```

The speech file is not committed (6 MB). Regenerate it with Windows SAPI (PowerShell):

```powershell
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$fmt = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(48000, 'Sixteen', 'Mono')
$s.SetOutputToWaveFile("tools\guest-bot\media\speech-pattern.wav", $fmt)
$p = New-Object System.Speech.Synthesis.PromptBuilder
1..4 | % { $p.AppendText("This is test speaker number $_ speaking continuously for a few seconds so the meeting can detect voice activity clearly."); $p.AppendBreak([TimeSpan]::FromSeconds(10)) }
$s.Speak($p); $s.Dispose()   # ~6.7 s speech / 10 s silence, 67 s, Chrome loops it
```

Gotchas found on the way:

- The agent's shell runs **elevated**; `zoom-companion auth login` from it cannot open Edge (no
  window, no error). Launch it de-elevated: `Start-Process explorer.exe <a .cmd that runs auth login>`.
- In the web client, Alt+Y does **not** raise the hand; use React > Raise Hand.
- Lab reads stdin commands ~0.5 s late (its loop sleeps); use SDK event timestamps, not note
  timestamps, for latency.
