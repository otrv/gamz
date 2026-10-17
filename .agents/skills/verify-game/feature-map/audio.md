# Audio

## Check generated samples through the CLI

Run `launch` and `doctor`, then drive the built CLI without a sound device:

```sh
printf '%s\n' \
	'{"audio":{"sample_rate":44100,"frames":3}}' \
	'{"dt":{"secs":0,"nanos":0},"controller":{}}' \
	'{"audio":{"sample_rate":192000,"frames":16384}}' \
	'{"audio":{"sample_rate":1,"frames":0}}' \
	| target/verify-game/debug/cli
```

This manual probe is separate from the baseline `drive`. Expect initialization followed by responses in request order. Each audio response contains the requested rate and number of stereo sample pairs. Compare sample values with the selected game's expected behavior. Do not retain assertions about a temporary tone probe.

## Check playback only when Linux is altered

Use a real ALSA device or an explicitly configured, clocked virtual sink such as PulseAudio. Route ALSA `default` to that device or sink. An ALSA `null` PCM consumes data without real playback timing and cannot establish underrun behavior. Capture the sink monitor to check submitted samples. CLI output alone cannot prove encoding, submission, or playback.

For an authorized temporary tone probe, use distinct left and right amplitudes. Compare captured samples and frequency. Pause the process with `SIGSTOP`, then resume it with `SIGCONT` after one second. That pause exceeds the platform's maximum half-second buffer. Require sound after resumption, not just process survival.

Also test an unavailable device and require a diagnostic and nonzero exit status. Remove the tone probe and verify the restored game's output before delivery. Report virtual-sink checks separately from physical-device listening or latency measurements.
