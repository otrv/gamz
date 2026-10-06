# Exit behavior

## Sub-features

- Escape requests a clean game exit.
- Closing the window requests a clean game exit.

## How to get to it (user POV)

With the game window open, press Escape or use the window close control.

## Driving it with verify-game

After completing any desired visual evidence, run `drive`; it sends Escape last. Confirm `tmux has-session -t "gamz-verify-$UID"` fails and the copied game log contains no `gamz:` error. Relaunch before testing a different feature.

## Gotchas

`drive` includes Escape last, so it intentionally exits the game. Capture evidence for other input first. The helper's cleanup remains necessary to terminate Xvfb and remove scratch state.
