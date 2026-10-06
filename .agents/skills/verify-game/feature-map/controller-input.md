# Controller input

## Sub-features

- `W`, `A`, `S`, and `D` map to movement directions.
- Arrow keys map to action directions.
- `Q` and `E` map to shoulder actions.
- Space maps to start.

## How to get to it (user POV)

While the `gamz` window is open, press the mapped keys. The current boilerplate game consumes the input but intentionally has no visible response.

## Driving it with verify-game

Run `drive`. Its action transcript lists every mapped key and its before/after captures prove the window remained renderable throughout. When gameplay gives an input a visible or persistent result, add the exact key sequence, expected screen state, and any side-effect check here before verification.

## Gotchas

Input is sampled once per frame. Test press/release behavior through the real window, not by constructing `FrameInput` inside a test. Escape has separate exit behavior and is covered by its own feature entry.
