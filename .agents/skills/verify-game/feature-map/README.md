# gamz verification feature map

Use CLI for game behavior. Use Linux window driving only for rendering and platform behavior. Add entries as gameplay, persistence, assets, or audio become observable; record concrete input sequences and expected outputs, not just successful process execution.

| Feature | Entry | Default surface |
| --- | --- | --- |
| Initialization and frame output; optional pixel presentation | [Initialization and rendering](launch-and-render.md) | CLI; Linux for rendering |
| Controller values and controlled time; keyboard integration | [Controller input](controller-input.md) | CLI; Linux for keyboard mapping |
| EOF/errors; Escape and window close | [Exit behavior](exit.md) | CLI; Linux for window lifecycle |
