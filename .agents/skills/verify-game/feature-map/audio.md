# Audio

## Game boundary: CLI

Run `launch` and `doctor`, then drive the built CLI without a sound device:

```sh
printf '%s\n' \
  '{"audio":{"sample_rate":44100,"frames":3}}' \
  '{"dt":{"secs":0,"nanos":0},"controller":{}}' \
  '{"audio":{"sample_rate":192000,"frames":16384}}' \
  '{"audio":{"sample_rate":1,"frames":0}}' \
  | target/verify-game/debug/cli
```

This is a manual probe, not part of the baseline `drive`. Expect initialization followed by responses in request order, with each audio response containing the requested rate and number of stereo sample pairs. Evaluate sample values against the game behavior being developed rather than retaining assertions about a temporary sound generator or silence stub.

## Playback: Linux

Use a real ALSA device or an explicitly configured, clocked virtual sink such as PulseAudio. An ALSA `null` PCM consumes data without real playback timing and cannot establish underrun behavior. Capture the sink monitor when checking samples submitted by the platform; CLI output alone cannot prove encoding, submission, or playback.

For an authorized temporary tone probe, use distinct left/right amplitudes, compare captured samples and frequency, and test a process pause longer than the queue target with `SIGSTOP`/`SIGCONT`. Require sound after resumption, not merely process survival. Also exercise an unavailable device and require a diagnostic plus nonzero exit. Remove the probe and verify silence again before delivery. Report virtual-sink checks separately from physical-device listening or latency measurements.
