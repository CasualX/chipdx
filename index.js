import { createWasmAPI } from "./shade.js";

const PREFS_KEY = "chipdx-prefs";
const DEFAULT_PREFS = {
	audioEnabled: true,
	fullscreenEnabled: true,
};

const INPUT_UP = 1 << 0;
const INPUT_LEFT = 1 << 1;
const INPUT_DOWN = 1 << 2;
const INPUT_RIGHT = 1 << 3;
const INPUT_A = 1 << 4;
const INPUT_B = 1 << 5;
const INPUT_START = 1 << 6;
const INPUT_SELECT = 1 << 7;

window.chipGame = function chipGame() {
	return {
		loadingStatus: "Starting...",
		loadingProgress: 0,
		hudText: "",
		levelSetFilePromptVisible: false,
		showDisplayModeCta: false,
		detectedControlScheme: "keyboard",
		selectedControlScheme: "keyboard",
		userSelectedControlScheme: false,
		isLandscapeLayout: false,
		loadingPanelScale: 1,
		loadingPanelFrameWidth: 620,
		loadingPanelFrameHeight: 0,
		hasTouchSupport: false,
		prefs: { ...DEFAULT_PREFS },
		touchEnabled: false,
		touchPressed: {
			up: false,
			left: false,
			down: false,
			right: false,
			a: false,
			b: false,
			start: false,
			select: false,
		},
		padDir: null,
		gameActive: false,
		pageActive: true,
		pressedKeys: new Set(),
		padTouchId: null,
		audioCtx: null,
		audioMasterGain: null,
		audioLoaded: false,
		soundBank: new Map(),
		musicBank: new Map(),
		musicSource: null,
		currentMusicKey: null,
		requestedMusicKey: null,
		wasmExports: null,
		wasmMemory: null,
		wasmGamePtr: 0,
		lastWasmError: null,
		frameHandle: 0,
		initialized: false,
		cleanupRegistered: false,

		init() {
			if (this.initialized) return;
			this.initialized = true;
			this.loadPrefs();
			this.initWatchers();
			this.syncViewportLayout();
			this.hasTouchSupport = this.detectTouchSupport();
			this.syncControlSchemeFromEnvironment();
			let self = this;
			window.addEventListener("error", (event) => self.handleWindowError(event));
			window.addEventListener("unhandledrejection", (event) => self.handleUnhandledRejection(event));
			window.addEventListener("keydown", (event) => self.handleKeyDown(event));
			window.addEventListener("keyup", (event) => self.handleKeyUp(event));
			window.addEventListener("focus", () => self.syncPageActivity());
			window.addEventListener("blur", () => self.syncPageActivity());
			window.addEventListener("gamepadconnected", () => self.handleGamepadChange());
			window.addEventListener("gamepaddisconnected", () => self.handleGamepadChange());
			window.addEventListener("resize", () => self.syncViewportLayout());
			document.addEventListener("fullscreenchange", () => self.updateDisplayModeCta());
			document.addEventListener("visibilitychange", () => self.syncPageActivity());
			if (screen && screen.orientation && screen.orientation.addEventListener) {
				screen.orientation.addEventListener("change", () => self.syncViewportLayout());
			}

			this.updateDisplayModeCta();
			this.syncPageActivity();
			this.load().catch((err) => {
				console.error(err);
				this.setLoadingStatus(String(err && err.message ? err.message : err));
			});
		},

		initWatchers() {
			this.$watch("pageActive", (active, wasActive) => {
				this.syncAudioVisibility();
				if (wasActive && !active) {
					this.clearAllInputs();
				}
			});
			this.$watch("gameActive", () => {
				this.syncTouchControls();
				this.queueLoadingPanelScaleSync();
			});
			this.$watch("selectedControlScheme", () => {
				this.syncTouchControls();
				this.queueLoadingPanelScaleSync();
			});
			this.$watch("isLandscapeLayout", () => {
				this.updateDisplayModeCta();
				this.queueLoadingPanelScaleSync();
			});
			this.$watch("loadingProgress", () => this.queueLoadingPanelScaleSync());
			this.$watch("loadingStatus", () => this.queueLoadingPanelScaleSync());
		},

		get overlayStatusText() {
			return `Loading game: ${this.loadingStatus}`;
		},

		get isReadyToPlay() {
			return this.loadingProgress >= 1 && !!this.wasmGamePtr;
		},

		get showAllControlHints() {
			return this.isLandscapeLayout;
		},

		setLoadingStatus(text, progress) {
			this.loadingStatus = String(text);
			if (typeof progress === "number") {
				this.loadingProgress = Math.max(0, Math.min(1, progress));
			}
		},

		loadPrefs() {
			try {
				let raw = localStorage.getItem(PREFS_KEY);
				this.prefs = raw ? JSON.parse(raw) : { ...DEFAULT_PREFS };
			}
			catch (err) {
				console.warn("Prefs read failed", err);
				this.prefs = { ...DEFAULT_PREFS };
				return;
			}
		},

		savePrefs() {
			try {
				localStorage.setItem(PREFS_KEY, JSON.stringify(this.prefs));
			}
			catch (err) {
				console.warn("Prefs write failed", err);
			}
		},

		detectTouchSupport() {
			if (navigator.maxTouchPoints > 0) return true;
			if (window.matchMedia && window.matchMedia("(pointer: coarse)").matches) return true;
			return "ontouchstart" in window;
		},

		hasConnectedGamepad() {
			const pads = navigator.getGamepads ? navigator.getGamepads() : [];
			for (const gamepad of pads) {
				if (gamepad && gamepad.connected) {
					return true;
				}
			}
			return false;
		},

		getPreferredControlScheme() {
			if (this.hasConnectedGamepad()) return "gamepad";
			if (this.hasTouchSupport) return "touch";
			return "keyboard";
		},

		syncControlSchemeFromEnvironment() {
			this.detectedControlScheme = this.getPreferredControlScheme();
			if (!this.userSelectedControlScheme) {
				this.selectedControlScheme = this.detectedControlScheme;
			}
		},

		selectControlScheme(scheme) {
			this.userSelectedControlScheme = true;
			this.selectedControlScheme = scheme;
		},

		getCustomLevelPayload() {
			const params = new URLSearchParams(window.location.search);
			const compressed = params.get("levelc");
			if (compressed !== null) {
				return { value: compressed, compressed: true };
			}
			const plain = params.get("level");
			if (plain !== null) {
				return { value: plain, compressed: false };
			}
			return null;
		},

		isLandscapeViewport() {
			return window.innerWidth >= window.innerHeight;
		},

		syncViewportLayout() {
			this.isLandscapeLayout = this.isLandscapeViewport();
		},

		queueLoadingPanelScaleSync() {
			if (this.gameActive) return;
			const sync = () => this.syncLoadingPanelScale();
			if (typeof this.$nextTick === "function") {
				this.$nextTick(() => requestAnimationFrame(sync));
				return;
			}
			requestAnimationFrame(sync);
		},

		syncLoadingPanelScale() {
			const panel = this.$refs.loadingPanel;
			if (!panel) return;
			const viewportPadding = 24;
			const availableWidth = Math.max(1, window.innerWidth - viewportPadding);
			const availableHeight = Math.max(1, window.innerHeight - viewportPadding);
			const panelWidth = panel.offsetWidth;
			const panelHeight = panel.offsetHeight;
			if (panelWidth <= 0 || panelHeight <= 0) {
				this.loadingPanelScale = 1;
				this.loadingPanelFrameWidth = 620;
				this.loadingPanelFrameHeight = 0;
				return;
			}
			this.loadingPanelScale = Math.min(1, availableWidth / panelWidth, availableHeight / panelHeight);
			this.loadingPanelFrameWidth = panelWidth * this.loadingPanelScale;
			this.loadingPanelFrameHeight = panelHeight * this.loadingPanelScale;
		},

		isFullscreenActive() {
			return document.fullscreenElement === document.documentElement;
		},

		updateDisplayModeCta() {
			this.showDisplayModeCta = !(this.isFullscreenActive() && this.isLandscapeLayout);
		},

		handleWindowError(event) {
			console.error("window.error", event.error ?? event.message ?? event);
			this.setLoadingStatus(String(event.error?.message ?? event.message ?? event));
		},

		handleUnhandledRejection(event) {
			console.error("unhandledrejection", event.reason);
			this.setLoadingStatus(String(event.reason?.message ?? event.reason ?? event));
		},

		resizeCanvasToDisplaySize() {
			const canvas = this.$refs.canvas;
			const dpr = Math.max(1, Math.min(3, window.devicePixelRatio ?? 1));
			const rect = canvas.getBoundingClientRect();
			const width = Math.max(1, Math.floor(rect.width * dpr));
			const height = Math.max(1, Math.floor(rect.height * dpr));
			if (canvas.width !== width || canvas.height !== height) {
				canvas.width = width;
				canvas.height = height;
			}
			return { width, height, dpr };
		},

		ensureAudioContext() {
			if (this.audioCtx) return this.audioCtx;
			const AudioContextCtor = window.AudioContext ?? window.webkitAudioContext;
			if (!AudioContextCtor) return null;
			this.audioCtx = new AudioContextCtor();
			this.audioMasterGain = this.audioCtx.createGain();
			this.audioMasterGain.connect(this.audioCtx.destination);
			this.syncAudioVisibility();
			return this.audioCtx;
		},

		syncAudioVisibility() {
			if (!this.audioMasterGain) return;
			this.audioMasterGain.gain.value = this.pageActive ? 1.0 : 0.0;
		},

		syncPageActivity() {
			const hasFocus = typeof document.hasFocus === "function" ? document.hasFocus() : true;
			this.pageActive = document.visibilityState === "visible" && hasFocus;
		},

		async enableAudio() {
			if (this.audioLoaded) return;
			const ctx = this.ensureAudioContext();
			if (!ctx) return;
			if (ctx.state === "suspended") {
				try {
					await ctx.resume();
				}
				catch {
					return;
				}
			}
			if (!this.wasmExports) return;
			try {
				this.wasmExports.audioInit();
				this.audioLoaded = true;
			}
			catch (err) {
				console.warn("audio init failed", err);
			}
		},

		handleKeyDown(event) {
			this.pressedKeys.add(event.code);
			if (
				event.code.startsWith("Arrow") ||
				event.code === "Space" ||
				event.code === "Backspace" ||
				event.code === "Enter" ||
				event.code === "ShiftLeft" ||
				event.code === "ShiftRight"
			) {
				event.preventDefault();
			}
		},

		handleKeyUp(event) {
			this.pressedKeys.delete(event.code);
			if (
				event.code.startsWith("Arrow") ||
				event.code === "Space" ||
				event.code === "Backspace" ||
				event.code === "Enter" ||
				event.code === "ShiftLeft" ||
				event.code === "ShiftRight"
			) {
				event.preventDefault();
			}
		},

		getButtonsBitmask() {
			if (!this.pageActive) return 0;
			if (this.levelSetFilePromptVisible) return 0;

			let buttons = 0;
			if (this.pressedKeys.has("ArrowUp") || this.pressedKeys.has("KeyW")) buttons |= INPUT_UP;
			if (this.pressedKeys.has("ArrowLeft") || this.pressedKeys.has("KeyA")) buttons |= INPUT_LEFT;
			if (this.pressedKeys.has("ArrowDown") || this.pressedKeys.has("KeyS")) buttons |= INPUT_DOWN;
			if (this.pressedKeys.has("ArrowRight") || this.pressedKeys.has("KeyD")) buttons |= INPUT_RIGHT;
			if (this.pressedKeys.has("Space")) buttons |= INPUT_A;
			if (this.pressedKeys.has("Backspace")) buttons |= INPUT_B;
			if (this.pressedKeys.has("Enter")) buttons |= INPUT_START;
			if (this.pressedKeys.has("ShiftLeft") || this.pressedKeys.has("ShiftRight")) buttons |= INPUT_SELECT;

			if (this.touchPressed.up) buttons |= INPUT_UP;
			if (this.touchPressed.left) buttons |= INPUT_LEFT;
			if (this.touchPressed.down) buttons |= INPUT_DOWN;
			if (this.touchPressed.right) buttons |= INPUT_RIGHT;
			if (this.touchPressed.a) buttons |= INPUT_A;
			if (this.touchPressed.b) buttons |= INPUT_B;
			if (this.touchPressed.start) buttons |= INPUT_START;
			if (this.touchPressed.select) buttons |= INPUT_SELECT;

			buttons |= this.getGamepadButtonsBitmask();
			return buttons;
		},

		getGamepadButtonsBitmask() {
			const pads = navigator.getGamepads ? navigator.getGamepads() : [];
			let buttons = 0;
			const deadzone = 0.5;

			for (const gamepad of pads) {
				if (!gamepad || !gamepad.connected) continue;
				const gamepadButtons = gamepad.buttons ?? [];
				const axes = gamepad.axes ?? [];

				if (gamepadButtons[12]?.pressed) buttons |= INPUT_UP;
				if (gamepadButtons[13]?.pressed) buttons |= INPUT_DOWN;
				if (gamepadButtons[14]?.pressed) buttons |= INPUT_LEFT;
				if (gamepadButtons[15]?.pressed) buttons |= INPUT_RIGHT;
				if (gamepadButtons[0]?.pressed) buttons |= INPUT_A;
				if (gamepadButtons[1]?.pressed) buttons |= INPUT_B;
				if (gamepadButtons[9]?.pressed) buttons |= INPUT_START;
				if (gamepadButtons[8]?.pressed || gamepadButtons[16]?.pressed) buttons |= INPUT_SELECT;

				const lx = axes[0] ?? 0;
				const ly = axes[1] ?? 0;
				if (lx <= -deadzone) buttons |= INPUT_LEFT;
				if (lx >= deadzone) buttons |= INPUT_RIGHT;
				if (ly <= -deadzone) buttons |= INPUT_UP;
				if (ly >= deadzone) buttons |= INPUT_DOWN;
			}

			return buttons;
		},

		syncTouchControls() {
			const shouldEnable = this.gameActive && this.selectedControlScheme === "touch";
			if (shouldEnable === this.touchEnabled) return;
			this.touchEnabled = shouldEnable;
			if (!shouldEnable) {
				this.clearTouchInputs();
			}
		},

		clearTouchInputs() {
			for (const key of Object.keys(this.touchPressed)) {
				this.touchPressed[key] = false;
			}
			this.padTouchId = null;
			this.padDir = null;
		},

		clearAllInputs() {
			this.pressedKeys.clear();
			this.clearTouchInputs();
		},

		requestFullscreenOnTouch() {
			if (document.fullscreenElement) return;
			const target = document.documentElement;
			if (!target || !target.requestFullscreen) return;
			try {
				const result = target.requestFullscreen();
				if (result && typeof result.catch === "function") {
					result.catch(() => {});
				}
			}
			catch {
				// ignore
			}
		},

		async requestLandscapeLock() {
			const orientation = screen && screen.orientation;
			if (!orientation || !orientation.lock) return;
			try {
				await orientation.lock("landscape");
			}
			catch {
				// ignore
			}
		},

		async requestPreferredDisplayMode() {
			this.requestFullscreenOnTouch();
			await this.requestLandscapeLock();
			this.updateDisplayModeCta();
		},

		requestLevelSetFile() {
			this.levelSetFilePromptVisible = true;
			requestAnimationFrame(() => {
				try {
					this.$refs.levelSetFileInput?.click();
				}
				catch {
					// The visible button remains available.
				}
			});
		},

		allocWasmBytes(bytes) {
			const exports = this.wasmExports;
			const wasmMemory = this.wasmMemory;
			if (!exports || !wasmMemory || !exports.allocBytes || !exports.freeBytes) {
				throw new Error("WASM byte allocation bridge is not ready");
			}
			const capacity = Math.max(1, bytes.length);
			const ptr = exports.allocBytes(capacity);
			if (!ptr) {
				throw new Error("WASM allocBytes() returned null");
			}
			new Uint8Array(wasmMemory.buffer, ptr, bytes.length).set(bytes);
			return {
				ptr,
				len: bytes.length,
				capacity,
			};
		},

		getLevelSetDisplayName(file) {
			const rawName = file && file.name ? file.name : "Local Levelset";
			const baseName = rawName.split(/[\\/]/).pop() || rawName;
			const displayName = baseName.replace(/\.(dat|ccl)$/i, "").trim();
			return displayName || "Local Levelset";
		},

		async loadLocalPlayLevelSet(event) {
			const input = event.target;
			const file = input.files && input.files[0];
			input.value = "";
			this.levelSetFilePromptVisible = false;
			if (!file) return;
			try {
				const dataBytes = new Uint8Array(await file.arrayBuffer());
				const nameBytes = new TextEncoder().encode(this.getLevelSetDisplayName(file));
				const data = this.allocWasmBytes(dataBytes);
				const name = this.allocWasmBytes(nameBytes);
				try {
					this.lastWasmError = null;
					const result = this.wasmExports.loadLocalPlayLevelSet(this.wasmGamePtr, data.ptr, data.len, name.ptr, name.len);
					if (this.lastWasmError instanceof Error) {
						throw this.lastWasmError;
					}
					if (result !== 0) {
						throw new Error("Levelset file could not be loaded");
					}
				}
				finally {
					this.wasmExports.freeBytes(name.ptr, name.capacity);
					this.wasmExports.freeBytes(data.ptr, data.capacity);
				}
			}
			catch (err) {
				console.error("levelset file load failed", err);
				this.setLoadingStatus(String(err && err.message ? err.message : err));
			}
		},

		setPadDirection(dir) {
			if (this.padDir === dir) return;
			if (this.padDir) {
				this.touchPressed[this.padDir] = false;
			}
			this.padDir = dir;
			if (this.padDir) {
				this.touchPressed[this.padDir] = true;
			}
		},

		getPadDirectionFromTouch(touch) {
			const rect = this.$refs.touchPad.getBoundingClientRect();
			const x = touch.clientX - rect.left;
			const y = touch.clientY - rect.top;
			const dx = x - rect.width / 2;
			const dy = y - rect.height / 2;
			const minDim = Math.min(rect.width, rect.height);
			const dist = Math.hypot(dx, dy);
			if (dist < minDim * 0.09) return null;
			if (Math.abs(dx) > Math.abs(dy)) {
				return dx > 0 ? "right" : "left";
			}
			return dy > 0 ? "down" : "up";
		},

		pressTouchButton(id) {
			if (!this.touchEnabled) return;
			this.touchPressed[id] = true;
		},

		releaseTouchButton(id) {
			this.touchPressed[id] = false;
		},

		handleGamepadChange() {
			this.syncControlSchemeFromEnvironment();
		},

		async startGame() {
			if (!this.isReadyToPlay) {
				return;
			}
			this.savePrefs();
			if (this.prefs.fullscreenEnabled) {
				await this.requestPreferredDisplayMode();
			}
			if (this.prefs.audioEnabled) {
				await this.enableAudio();
			}
			this.gameActive = true;
		},

		onPadTouchStart(event) {
			if (!this.touchEnabled || this.padTouchId !== null) return;
			const touch = event.changedTouches[0];
			if (!touch) return;
			this.padTouchId = touch.identifier;
			this.setPadDirection(this.getPadDirectionFromTouch(touch));
		},

		onPadTouchMove(event) {
			if (!this.touchEnabled || this.padTouchId === null) return;
			const touch = Array.from(event.touches).find((item) => item.identifier === this.padTouchId);
			if (!touch) return;
			this.setPadDirection(this.getPadDirectionFromTouch(touch));
		},

		onPadTouchEnd(event) {
			if (this.padTouchId === null) return;
			for (const touch of event.changedTouches) {
				if (touch.identifier === this.padTouchId) {
					this.padTouchId = null;
					this.setPadDirection(null);
					break;
				}
			}
		},

		async load() {
			this.updateDisplayModeCta();
			const customLevelPayload = this.getCustomLevelPayload();
			this.setLoadingStatus("Initializing WebGL...", 0.1);
			const shade = createWasmAPI(this.$refs.canvas, {
				alpha: false,
				desynchronized: true,
				antialias: false,
				premultipliedAlpha: false,
			});

			let wasmMemory = null;
			const decoder = new TextDecoder();
			const encoder = new TextEncoder();

			const readUtf8 = (ptr, len) => {
				if (!wasmMemory || ptr === 0 || len === 0) return "";
				return decoder.decode(new Uint8Array(wasmMemory.buffer, ptr, len));
			};

			const randomBytes = (ptr, len) => {
				if (!wasmMemory) return;
				const out = new Uint8Array(wasmMemory.buffer, ptr, len);
				if (globalThis.crypto && globalThis.crypto.getRandomValues) {
					globalThis.crypto.getRandomValues(out);
					return;
				}
				for (let index = 0; index < out.length; index++) {
					out[index] = (Math.random() * 256) | 0;
				}
			};

			let resultValue = null;

			const playSound = (sound_id) => {
				if (!this.prefs.audioEnabled) return;
				const ctx = this.ensureAudioContext();
				if (!ctx) return;
				const entry = this.soundBank.get(sound_id | 0);
				if (!entry || !entry.buffer || ctx.state !== "running") return;
				const source = ctx.createBufferSource();
				source.buffer = entry.buffer;
				source.connect(entry.gain);
				source.start(0);
			};

			const stopMusic = () => {
				if (!this.musicSource) return;
				try {
					this.musicSource.stop();
				}
				catch {
					// ignore
				}
				this.musicSource.disconnect();
				this.musicSource = null;
				this.currentMusicKey = null;
			};

			const playMusic = (music_id) => {
				if (!this.prefs.audioEnabled) {
					this.requestedMusicKey = null;
					stopMusic();
					return;
				}
				const ctx = this.ensureAudioContext();
				if (!ctx) return;
				const id = music_id | 0;
				if (id < 0) {
					this.requestedMusicKey = null;
					stopMusic();
					return;
				}
				this.requestedMusicKey = id;
				if (this.currentMusicKey === id && this.musicSource) return;
				const entry = this.musicBank.get(id);
				if (!entry || !entry.buffer || ctx.state !== "running") {
					if (this.currentMusicKey !== id) {
						stopMusic();
					}
					return;
				}
				stopMusic();
				const source = ctx.createBufferSource();
				source.buffer = entry.buffer;
				source.loop = true;
				source.connect(entry.gain);
				source.start(0);
				this.musicSource = source;
				this.currentMusicKey = id;
			};

			const registerSound = (sound_id, data_ptr, data_len) => {
				const ctx = this.ensureAudioContext();
				if (!ctx || !wasmMemory) return;
				const bytes = new Uint8Array(wasmMemory.buffer, data_ptr, data_len);
				const copy = new Uint8Array(bytes);
				ctx.decodeAudioData(copy.buffer).then((buffer) => {
					const gain = ctx.createGain();
					gain.gain.value = 1.0;
					gain.connect(this.audioMasterGain ?? ctx.destination);
					this.soundBank.set(sound_id | 0, { buffer, gain });
				}).catch((err) => {
					console.warn("decodeAudioData failed", sound_id, err);
				});
			};

			const registerMusic = (music_id, data_ptr, data_len) => {
				const ctx = this.ensureAudioContext();
				if (!ctx || !wasmMemory) return;
				const bytes = new Uint8Array(wasmMemory.buffer, data_ptr, data_len);
				const copy = new Uint8Array(bytes);
				ctx.decodeAudioData(copy.buffer).then((buffer) => {
					const id = music_id | 0;
					const gain = ctx.createGain();
					gain.gain.value = 0.375;
					gain.connect(this.audioMasterGain ?? ctx.destination);
					this.musicBank.set(id, { buffer, gain });
					if (this.requestedMusicKey === id && (!this.musicSource || this.currentMusicKey !== id)) {
						playMusic(id);
					}
				}).catch((err) => {
					console.warn("decodeAudioData failed", music_id, err);
				});
			};

			const setTitle = (title_ptr, title_len) => {
				document.title = readUtf8(title_ptr, title_len) || "Chip's Challenge DX";
			};

			const quitGame = () => {
				console.log("quitGame requested");
			};

			const resultError = (message_ptr, message_len) => {
				resultValue = new Error(readUtf8(message_ptr, message_len) || "Unknown error");
				this.lastWasmError = resultValue;
			};

			const requestLevelSetFile = () => {
				this.requestLevelSetFile();
			};

			const readFile = (path_ptr, path_len, content_ptr, content_len_ptr) => {
				if (!wasmMemory) return -1;
				const path = readUtf8(path_ptr, path_len);
				if (!path || !content_len_ptr) return -1;
				let content = null;
				try {
					content = localStorage.getItem(path);
				}
				catch (err) {
					console.warn("readFile localStorage failed", err);
					return -1;
				}
				if (content === null) return -1;
				const bytes = encoder.encode(content);
				const lenView = new Uint32Array(wasmMemory.buffer, content_len_ptr, 1);
				if (!content_ptr) {
					lenView[0] = bytes.length;
					return 0;
				}
				const cap = lenView[0] >>> 0;
				const out = new Uint8Array(wasmMemory.buffer, content_ptr, cap);
				const toCopy = Math.min(cap, bytes.length);
				out.set(bytes.subarray(0, toCopy));
				lenView[0] = toCopy;
				return 0;
			};

			const writeFile = (path_ptr, path_len, content_ptr, content_len) => {
				if (!wasmMemory) return -1;
				const path = readUtf8(path_ptr, path_len);
				if (!path || !content_ptr) return -1;
				const bytes = new Uint8Array(wasmMemory.buffer, content_ptr, content_len);
				const content = decoder.decode(bytes);
				try {
					localStorage.setItem(path, content);
					return 0;
				}
				catch (err) {
					console.warn("writeFile localStorage failed", err);
					return -1;
				}
			};

			this.setLoadingStatus("Loading WASM...", 0.25);
			const response = await fetch("./chipwasm.wasm");
			if (!response.ok) {
				throw new Error(`Failed to fetch chipwasm.wasm: ${response.status} ${response.statusText}`);
			}

			const imports = {
				webgl: shade,
				env: {
					randomBytes,
					playSound,
					playMusic,
					registerSound,
					registerMusic,
					setTitle,
					requestLevelSetFile,
					quitGame,
					resultError,
					readFile,
					writeFile,
				},
			};

			this.setLoadingStatus("Compiling WASM...", 0.45);
			const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), imports);
			shade.bindInstance(instance);
			wasmMemory = instance.exports.memory;

			const exports = instance.exports;
			this.wasmExports = exports;
			this.wasmMemory = wasmMemory;

			const allocWasmBytes = (bytes) => {
				return this.allocWasmBytes(bytes);
			};

			this.setLoadingStatus("Starting...", 0.8);
			let gamePtr = 0;
			try {
				if (customLevelPayload !== null) {
					this.setLoadingStatus("Loading custom level...", 0.8);
					resultValue = null;
					const payloadBytes = encoder.encode(customLevelPayload.value);
					const payload = allocWasmBytes(payloadBytes);
					try {
						gamePtr = exports.createCustomPlayLevel(payload.ptr, payload.len, customLevelPayload.compressed);
					}
					finally {
						exports.freeBytes(payload.ptr, payload.capacity);
					}
					if (resultValue instanceof Error) {
						throw resultValue;
					}
				}
				else {
					gamePtr = exports.createPlayInstance();
				}
			}
			catch (err) {
				const message = err && err.message ? err.message : String(err);
				throw new Error(`${customLevelPayload !== null ? "createCustomPlayLevel()" : "createPlayInstance()"} trapped: ${message}`);
			}
			if (!gamePtr) {
				const label = customLevelPayload === null ? "createPlayInstance()" : "createCustomPlayLevel()";
				throw new Error(`${label} returned null (0) instance pointer`);
			}
			this.wasmGamePtr = gamePtr;

			this.setLoadingStatus("Ready to play", 1);

			const stepMs = 1000 / 60;
			let acc = 0;
			let last = performance.now();
			let drawCount = 0;
			let thinkCount = 0;
			let lastHud = performance.now();

			const frame = (now) => {
				if (!this.pageActive) {
					last = now;
					acc = 0;
					this.resizeCanvasToDisplaySize();
					this.frameHandle = requestAnimationFrame(frame);
					return;
				}

				const dt = Math.min(250, now - last);
				last = now;
				acc += dt;

				if (!this.userSelectedControlScheme) {
					this.syncControlSchemeFromEnvironment();
				}

				if (!this.gameActive) {
					this.resizeCanvasToDisplaySize();
					this.frameHandle = requestAnimationFrame(frame);
					return;
				}

				const buttons = this.getButtonsBitmask();
				while (acc >= stepMs) {
					try {
						exports.thinkPlayInstance(gamePtr, buttons);
					}
					catch (err) {
						this.setLoadingStatus(`thinkPlayInstance() trapped: ${err && err.message ? err.message : String(err)}`);
						return;
					}
					thinkCount++;
					acc -= stepMs;
				}

				const { width, height } = this.resizeCanvasToDisplaySize();
				try {
					exports.drawPlayInstance(gamePtr, now / 1000.0, width, height);
				}
				catch (err) {
					this.setLoadingStatus(`drawPlayInstance() trapped: ${err && err.message ? err.message : String(err)}`);
					return;
				}
				drawCount++;

				if (now - lastHud >= 500) {
					const secs = (now - lastHud) / 1000;
					const fps = Math.round(drawCount / secs);
					const tps = Math.round(thinkCount / secs);
					this.hudText = `draw ${fps}fps · think ${tps}hz`;
					drawCount = 0;
					thinkCount = 0;
					lastHud = now;
				}

				this.frameHandle = requestAnimationFrame(frame);
			};

			this.resizeCanvasToDisplaySize();
			this.frameHandle = requestAnimationFrame(frame);

			if (!this.cleanupRegistered) {
				const cleanup = () => {
					if (this.frameHandle) {
						cancelAnimationFrame(this.frameHandle);
						this.frameHandle = 0;
					}
					try {
						exports.destroyPlayInstance(gamePtr);
					}
					catch {
						// ignore
					}
				};
				window.addEventListener("pagehide", cleanup, { once: true });
				window.addEventListener("beforeunload", cleanup, { once: true });
				this.cleanupRegistered = true;
			}
		},
	};
};
