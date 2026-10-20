const canvas = document.querySelector("canvas");
const overlay = document.querySelector("#overlay");
const status = document.querySelector("#status");
const play = document.querySelector("#play");
const files = new Map();
const sources = new Set();
let phase = "loading";
let audio;
let nextAudioTime = 0;

function pause() {
    if (phase !== "running") return;
    phase = "paused";
    for (const source of sources) source.stop();
    sources.clear();
    nextAudioTime = 0;
    if (audio) audio.suspend().catch(error => fail(String(error)));
    status.textContent = "Paused";
    play.textContent = "Resume";
    overlay.hidden = false;
}

export function fail(message) {
    pause();
    phase = "error";
    status.textContent = message;
    play.hidden = true;
    overlay.hidden = false;
    console.error(message);
}

export function ready() {
    phase = "ready";
    status.textContent = "Ready to play";
    play.disabled = false;
}

export function active() {
    return phase === "running" && !document.hidden && document.activeElement === canvas;
}

export function next_frame() {
    return new Promise(resolve => requestAnimationFrame(resolve));
}

export function sample_rate() {
    return audio.sampleRate;
}

export function needs_audio() {
    return audio.state === "running" && sources.size < 3 && nextAudioTime - audio.currentTime < 0.06;
}

export function play_audio(left, right) {
    const buffer = audio.createBuffer(2, left.length, audio.sampleRate);
    buffer.copyToChannel(left, 0);
    buffer.copyToChannel(right, 1);
    const source = audio.createBufferSource();
    source.buffer = buffer;
    source.connect(audio.destination);
    source.onended = () => { sources.delete(source); source.disconnect(); };
    sources.add(source);
    const start = Math.max(nextAudioTime, audio.currentTime + 0.005);
    source.start(start);
    nextAudioTime = start + buffer.duration;
}

export function asset(path) {
    return files.get(path);
}

async function readBounded(url, maximum) {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
    const reader = response.body.getReader();
    const chunks = [];
    let size = 0;
    try {
        for (;;) {
            const { value, done } = await reader.read();
            if (done) break;
            size += value.length;
            if (size > maximum) throw new Error(`${url}: size limit exceeded`);
            chunks.push(value);
        }
    } finally {
        await reader.cancel();
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
    return bytes;
}

export async function prepare() {
    if (!navigator.gpu) throw new Error("This game requires WebGPU. Use a supported desktop browser over HTTPS.");
    const manifest = JSON.parse(new TextDecoder().decode(await readBounded(new URL("assets.json", document.baseURI), 65536)));
    if (!Array.isArray(manifest) || manifest.length > 256) throw new Error("assets.json must list at most 256 relative file paths");
    let remaining = 64 * 1024 * 1024;
    for (const path of manifest) {
        if (typeof path !== "string" || !path || path.length > 1024 || /[\\:\0?#%]/u.test(path) || path.split("/").some(part => !part || part === "." || part === "..") || files.has(path)) {
            throw new Error("Invalid or duplicate asset path in assets.json");
        }
        const bytes = await readBounded(new URL(path, document.baseURI), Math.min(16 * 1024 * 1024 + 8, remaining));
        remaining -= bytes.length;
        files.set(path, bytes);
    }
    play.addEventListener("click", async () => {
        if (phase !== "ready" && phase !== "paused") return;
        play.disabled = true;
        try {
            audio ??= new AudioContext({ latencyHint: "interactive" });
            await audio.resume();
            canvas.focus();
            phase = "running";
            overlay.hidden = true;
        } catch (error) {
            fail(String(error));
        } finally {
            play.disabled = false;
        }
    });
    canvas.addEventListener("blur", pause);
    window.addEventListener("blur", pause);
    document.addEventListener("visibilitychange", () => { if (document.hidden) pause(); });
}
