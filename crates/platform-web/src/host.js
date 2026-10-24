const canvas = document.querySelector("canvas");
const overlay = document.querySelector("#overlay");
const status = document.querySelector("#status");
const play = document.querySelector("#play");
const sources = new Set();
const COMPLETE = 0;
const END_OF_FILE = 1;
const CANCELLED = 2;
const NOT_FOUND = 3;
const PERMISSION_DENIED = 4;
const TOO_LARGE = 5;
const IO_ERROR = 6;
const EMPTY_BYTES = new Uint8Array();
const lifecycle = [];
const queue = [];
let slots = [];
let max_io_bytes = 0;
let max_file_bytes = 0;
let draining = false;
let database;
let phase = "loading";
let audio;
let next_audio_time = 0;

function lifecycle_event(event) {
    if (lifecycle.length === slots.length) {
        fail("Browser lifecycle event queue is full.");
        return;
    }
    lifecycle.push(event);
}

function pause() {
    if (phase !== "running") return;
    phase = "paused";
    lifecycle_event(1);
    if (phase === "error") return;
    for (const source of sources) source.stop();
    sources.clear();
    next_audio_time = 0;
    if (audio) audio.suspend().catch(error => fail(String(error)));
    status.textContent = "Paused";
    play.textContent = "Resume";
    overlay.hidden = false;
}

export function fail(message) {
    if (phase === "exited" || phase === "error") return;
    phase = "error";
    shutdown();
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

export function failed() {
    return phase === "error";
}

export function next_frame() {
    return new Promise(resolve => requestAnimationFrame(resolve));
}

export function sample_rate() {
    return audio.sampleRate;
}

export function needs_audio() {
    return audio.state === "running" && sources.size < 3 && next_audio_time - audio.currentTime < 0.06;
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
    const start = Math.max(next_audio_time, audio.currentTime + 0.005);
    source.start(start);
    next_audio_time = start + buffer.duration;
}

export function configure_services(requests, io_bytes, file_bytes) {
    slots = new Array(requests).fill(null);
    max_io_bytes = io_bytes;
    max_file_bytes = file_bytes;
}

function submit(slot, request) {
    if (phase === "exited" || phase === "error" || slot >= slots.length || slots[slot]) {
        throw new Error("Browser service unavailable");
    }
    request.state = "queued";
    request.controller = new AbortController();
    request.payload = EMPTY_BYTES;
    request.count = 0;
    slots[slot] = request;
    queue.push(request);
    if (!draining) {
        draining = true;
        queueMicrotask(drain);
    }
}

export function request_read(slot, path, offset, length) {
    if (length > max_io_bytes || offset + length > max_file_bytes) {
        throw new RangeError("Read exceeds browser file bounds");
    }
    submit(slot, { kind: "read", path, offset, length });
}

export function request_write(slot, path, offset, bytes) {
    if (bytes.length > max_io_bytes || (offset ?? 0) + bytes.length > max_file_bytes) {
        throw new RangeError("Write exceeds browser file bounds");
    }
    submit(slot, { kind: "write", path, offset, bytes: bytes.slice() });
}

export function request_window(slot, kind, first, second, title) {
    submit(slot, { kind: "window", change: kind, first, second, title });
}

export function request_cancel(slot) {
    const request = slots[slot];
    if (!request || request.state === "complete") return false;
    request.controller.abort();
    if (request.state === "queued") {
        queue.splice(queue.indexOf(request), 1);
        request.bytes = undefined;
        request.status = CANCELLED;
        request.state = "complete";
        return true;
    }
    if (request.transaction) {
        try { request.transaction.abort(); } catch { return false; }
    }
    return true;
}

function file_status(error) {
    if (typeof error === "number") return error;
    if (error?.name === "AbortError") return CANCELLED;
    if (error?.name === "NotAllowedError" || error?.name === "SecurityError") return PERMISSION_DENIED;
    if (error?.name === "QuotaExceededError") return TOO_LARGE;
    return IO_ERROR;
}

async function drain() {
    while (queue.length) {
        const request = queue.shift();
        request.state = "running";
        try {
            request.controller.signal.throwIfAborted();
            if (request.kind === "read") await read_file(request);
            else if (request.kind === "write") await write_file(request);
            else await change_window(request);
        } catch (error) {
            request.status = file_status(error);
            request.payload = EMPTY_BYTES;
        }
        request.bytes = undefined;
        request.transaction = undefined;
        request.state = "complete";
    }
    draining = false;
}

function open_database() {
    if (database) return Promise.resolve(database);
    return new Promise((resolve, reject) => {
        const opening = indexedDB.open("gamz-files", 1);
        let blocked = false;
        opening.onupgradeneeded = () => opening.result.createObjectStore("files");
        opening.onerror = () => reject(opening.error);
        opening.onblocked = () => {
            blocked = true;
            reject(new Error("Browser file storage is blocked"));
        };
        opening.onsuccess = () => {
            if (blocked || phase === "exited" || phase === "error") {
                opening.result.close();
                reject(new Error("Browser file storage is closed"));
                return;
            }
            database = opening.result;
            database.onversionchange = () => { database.close(); database = undefined; };
            resolve(database);
        };
    });
}

async function stored_file(request) {
    const storage = await open_database();
    request.controller.signal.throwIfAborted();
    return new Promise((resolve, reject) => {
        const transaction = storage.transaction("files", "readonly");
        request.transaction = transaction;
        const read = transaction.objectStore("files").get(request.path);
        transaction.oncomplete = () => { request.transaction = undefined; resolve(read.result); };
        transaction.onabort = () => reject(transaction.error ?? new DOMException("Cancelled", "AbortError"));
    });
}

async function fetch_file(request) {
    const relative = request.path.split("/").map(encodeURIComponent).join("/");
    const response = await fetch(new URL(relative, document.baseURI), { signal: request.controller.signal });
    if (!response.ok) {
        if (response.status === 404) throw NOT_FOUND;
        if (response.status === 401 || response.status === 403) throw PERMISSION_DENIED;
        throw IO_ERROR;
    }
    return response;
}

async function read_file(request) {
    const blob = await stored_file(request);
    request.controller.signal.throwIfAborted();
    if (blob !== undefined) {
        if (!(blob instanceof Blob)) throw IO_ERROR;
        if (blob.size > max_file_bytes) throw TOO_LARGE;
        request.payload = new Uint8Array(await blob.slice(request.offset, request.offset + request.length).arrayBuffer());
        request.controller.signal.throwIfAborted();
        request.status = request.payload.length < request.length ? END_OF_FILE : COMPLETE;
        return;
    }
    const response = await fetch_file(request);
    const reader = response.body?.getReader();
    if (!reader) throw IO_ERROR;
    const bytes = new Uint8Array(request.length);
    let size = 0;
    let count = 0;
    try {
        for (;;) {
            const { value, done } = await reader.read();
            if (done) break;
            const end = size + value.length;
            if (end > max_file_bytes) throw TOO_LARGE;
            const start = Math.max(size, request.offset);
            const stop = Math.min(end, request.offset + request.length);
            if (stop > start) {
                bytes.set(value.subarray(start - size, stop - size), start - request.offset);
                count += stop - start;
            }
            size = end;
        }
    } finally {
        await reader.cancel();
    }
    request.payload = bytes.subarray(0, count);
    request.status = count < request.length ? END_OF_FILE : COMPLETE;
}

async function write_base(request) {
    if (request.offset === undefined || await stored_file(request) !== undefined) return undefined;
    let response;
    try {
        response = await fetch_file(request);
    } catch (error) {
        if (error === NOT_FOUND) return undefined;
        throw error;
    }
    const reader = response.body?.getReader();
    if (!reader) throw IO_ERROR;
    const chunks = [];
    let size = 0;
    try {
        for (;;) {
            const { value, done } = await reader.read();
            if (done) break;
            size += value.length;
            if (size > max_file_bytes) throw TOO_LARGE;
            chunks.push(value);
        }
    } finally {
        await reader.cancel();
    }
    return new Blob(chunks);
}

async function write_file(request) {
    const fallback = await write_base(request);
    const storage = await open_database();
    request.controller.signal.throwIfAborted();
    await new Promise((resolve, reject) => {
        const transaction = storage.transaction("files", "readwrite");
        request.transaction = transaction;
        const files = transaction.objectStore("files");
        let failure;
        transaction.oncomplete = () => {
            request.transaction = undefined;
            request.count = request.bytes.length;
            request.status = COMPLETE;
            resolve();
        };
        transaction.onabort = () => reject(failure ?? transaction.error ?? new DOMException("Cancelled", "AbortError"));
        if (request.offset === undefined) {
            try {
                files.put(new Blob([request.bytes]), request.path);
            } catch (error) {
                failure = error;
                transaction.abort();
            }
            return;
        }
        const read = files.get(request.path);
        read.onsuccess = () => {
            try {
                const previous = read.result ?? fallback ?? new Blob();
                if (!(previous instanceof Blob)) throw IO_ERROR;
                if (previous.size > max_file_bytes) throw TOO_LARGE;
                if (request.bytes.length === 0) {
                    files.put(previous, request.path);
                    return;
                }
                const parts = [previous.slice(0, request.offset)];
                if (request.offset > previous.size) parts.push(new Uint8Array(request.offset - previous.size));
                parts.push(request.bytes, previous.slice(request.offset + request.bytes.length));
                files.put(new Blob(parts), request.path);
            } catch (error) {
                failure = error;
                transaction.abort();
            }
        };
    });
}

async function change_window(request) {
    switch (request.change) {
        case 0: {
            canvas.style.width = `${request.first}px`;
            canvas.style.height = `${request.second}px`;
            const bounds = canvas.getBoundingClientRect();
            if (Math.round(bounds.width) !== request.first || Math.round(bounds.height) !== request.second) throw PERMISSION_DENIED;
            break;
        }
        case 1:
            if (request.first) {
                if (document.fullscreenElement !== canvas) await canvas.requestFullscreen();
            } else if (document.fullscreenElement) await document.exitFullscreen();
            if ((document.fullscreenElement === canvas) !== Boolean(request.first)) throw PERMISSION_DENIED;
            break;
        case 2:
            canvas.style.cursor = request.first ? "none" : "default";
            break;
        case 3:
            document.title = request.title;
            break;
        default: throw PERMISSION_DENIED;
    }
    request.status = COMPLETE;
}

export function poll_slot() {
    return slots.findIndex(request => request?.state === "complete");
}

export function completion_status(slot) { return slots[slot].status; }
export function completion_count(slot) { return slots[slot].count; }
export function completion_payload(slot) { return slots[slot].payload; }
export function release_completion(slot) { slots[slot] = null; }
export function poll_lifecycle() { return lifecycle.shift() ?? 0; }
export function has_events() { return lifecycle.length !== 0 || poll_slot() !== -1; }

export function window_property(property) {
    switch (property) {
        case 0: return Math.round(canvas.getBoundingClientRect().width);
        case 1: return Math.round(canvas.getBoundingClientRect().height);
        case 2: return canvas.width;
        case 3: return canvas.height;
        case 4: return Number(document.hasFocus() && document.activeElement === canvas);
        case 5: return Number(document.fullscreenElement === canvas);
        case 6: return Number(canvas.style.cursor === "none");
        default: throw new Error("Unknown window property");
    }
}

function shutdown() {
    for (const source of sources) { source.stop(); source.disconnect(); }
    sources.clear();
    if (audio) void audio.close().catch(() => {});
    for (let slot = 0; slot < slots.length; slot += 1) request_cancel(slot);
    queue.length = 0;
    slots.fill(null);
    lifecycle.length = 0;
    database?.close();
    database = undefined;
    if (document.fullscreenElement === canvas) void document.exitFullscreen().catch(() => {});
}

export function exited() {
    phase = "exited";
    shutdown();
    status.textContent = "Exited";
    play.hidden = true;
    overlay.hidden = false;
}

export async function prepare() {
    if (!navigator.gpu) throw new Error("This game requires WebGPU. Use a supported desktop browser over HTTPS.");
    play.addEventListener("click", async () => {
        if (phase !== "ready" && phase !== "paused") return;
        play.disabled = true;
        try {
            audio ??= new AudioContext({ latencyHint: "interactive" });
            await audio.resume();
            if (phase !== "ready" && phase !== "paused") return;
            canvas.focus();
            phase = "running";
            lifecycle_event(2);
            if (phase === "error") return;
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
    window.addEventListener("pagehide", () => { pause(); });
}
