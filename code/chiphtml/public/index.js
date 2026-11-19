import { createWasmAPI } from "./shade.js";

const PREFS_KEY = "chipdx-prefs";
const LEVELSET_KEY_PREFIX = "chipdx-custom-levelset:";
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

function levelSetKey(filename) {
	const extension = /\.paks$/i.test(filename) ? "paks" : "dat";
	const stem = filename.replace(/\.(dat|ccl|paks)$/i, "").trim() || "Local Levelset";
	return `${LEVELSET_KEY_PREFIX}${stem}.${extension}`;
}

function encodeLevelSet(bytes) {
	let binary = "";
	for (let offset = 0; offset < bytes.length; offset += 0x8000) {
		binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
	}
	return btoa(binary);
}

window.chipGame = function chipGame() {
	return {
		loadingStatus: "Starting...",
		loadingProgress: 0,
		hudText: "",
		customLevelSets: [],
		toastMessage: "",
		toastIsError: false,
		toastTimer: 0,
		levelSetImporting: false,
		isSingleLevel: false,
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
		storagePersistenceStatus: "idle",
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
		gameStarting: false,
		gameThinkStartTime: 0,
		replayStartDelayMs: 0,
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
			window.addEventListener("hashchange", () => window.location.reload());
			window.addEventListener("storage", (event) => {
				if (event.key === null || event.key.startsWith(LEVELSET_KEY_PREFIX)) self.refreshCustomLevelSets();
			});
			document.addEventListener("fullscreenchange", () => self.updateDisplayModeCta());
			document.addEventListener("visibilitychange", () => self.syncPageActivity());
			if (screen && screen.orientation && screen.orientation.addEventListener) {
				screen.orientation.addEventListener("change", () => self.syncViewportLayout());
			}

			this.updateDisplayModeCta();
			this.syncPageActivity();
			this.load().catch((err) => {
				console.error(err);
				this.setLoadingStatus("");
				this.showToast(`Could not load the game — ${err?.message ?? err}. Reload to try again.`, true);
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
			return !this.loadingStatus || this.loadingProgress >= 1 ? this.loadingStatus : `Loading game: ${this.loadingStatus}`;
		},

		get isReadyToPlay() {
			return this.loadingProgress >= 1 && !this.gameActive && !this.gameStarting && !this.levelSetImporting;
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

		get canRequestPersistentStorage() {
			return ["available", "denied", "error"].includes(this.storagePersistenceStatus);
		},

		async refreshStoragePersistenceStatus() {
			if (["checking", "requesting"].includes(this.storagePersistenceStatus)) return;
			const storage = navigator.storage;
			if (typeof storage?.persisted !== "function" || typeof storage?.persist !== "function") {
				this.storagePersistenceStatus = "unsupported";
				this.showToast("Persistent storage is unavailable in this browser.", true);
				return;
			}
			this.storagePersistenceStatus = "checking";
			try {
				this.storagePersistenceStatus = await storage.persisted() ? "granted" : "available";
			}
			catch {
				this.storagePersistenceStatus = "error";
				this.showToast("Could not check persistent storage permission.", true);
			}
		},

		async requestPersistentStorage(event) {
			// The browser decides whether to check the box.
			event.target.checked = false;
			if (!this.canRequestPersistentStorage) return;
			this.storagePersistenceStatus = "requesting";
			try {
				const granted = await navigator.storage.persist();
				this.storagePersistenceStatus = granted ? "granted" : "denied";
				if (!granted) this.showToast("The browser declined the persistent storage request.", true);
			}
			catch {
				this.storagePersistenceStatus = "error";
				this.showToast("Could not request persistent storage.", true);
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
			const hash = window.location.hash.startsWith("#?") ? window.location.hash.slice(2) : "";
			const hashParams = new URLSearchParams(hash);
			const searchParams = new URLSearchParams(window.location.search);
			const compressed = hashParams.get("levelc") ?? searchParams.get("levelc");
			if (compressed !== null) {
				return { value: compressed, compressed: true };
			}
			const plain = hashParams.get("level") ?? searchParams.get("level");
			if (plain !== null) {
				return { value: plain, compressed: false };
			}
			return null;
		},

		getCustomLevelLink() {
			const hash = window.location.hash.startsWith("#?") ? window.location.hash.slice(2) : "";
			const hashParams = new URLSearchParams(hash);
			const searchParams = new URLSearchParams(window.location.search);
			return hashParams.get("link") ?? searchParams.get("link");
		},

		getReplayIndex() {
			const hash = window.location.hash.startsWith("#?") ? window.location.hash.slice(2) : "";
			const hashParams = new URLSearchParams(hash);
			const searchParams = new URLSearchParams(window.location.search);
			const value = hashParams.get("replay") ?? searchParams.get("replay");
			if (value === null || value.trim() === "") return null;
			const index = Number(value);
			return Number.isInteger(index) && index >= 0 && index <= 0x7fffffff ? index : null;
		},

		async fetchCustomLevel(link) {
			if (!link.trim()) {
				throw new Error("The custom level link is empty");
			}
			let url;
			try {
				url = new URL(link, window.location.href);
			}
			catch {
				throw new Error("The custom level link is not a valid URL");
			}
			if (url.protocol !== "http:" && url.protocol !== "https:") {
				throw new Error("The custom level link must use http:// or https://");
			}

			this.setLoadingStatus("Fetching custom level...", 0.05);
			let response;
			try {
				response = await fetch(url.href, {
					credentials: "omit",
					headers: {
						Accept: "application/json, text/plain;q=0.9, */*;q=0.1",
					},
				});
			}
			catch (err) {
				const detail = err && err.message ? `: ${err.message}` : "";
				throw new Error(`Could not fetch the custom level link${detail}. The server must allow cross-origin requests.`);
			}
			if (!response.ok) {
				throw new Error(`Custom level fetch failed: ${response.status} ${response.statusText}`);
			}

			return {
				value: await response.text(),
				compressed: false,
			};
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
			this.showToast(String(event.error?.message ?? event.message ?? event), true);
		},

		handleUnhandledRejection(event) {
			console.error("unhandledrejection", event.reason);
			this.showToast(String(event.reason?.message ?? event.reason ?? event), true);
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
			if (!this.gameActive) return;
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
			if (!this.gameActive) return;
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

		openLevelSetManager() {
			this.$refs.levelSetManager.showModal();
			this.refreshCustomLevelSets();
			this.refreshStoragePersistenceStatus();
		},

		refreshCustomLevelSets() {
			try {
				const sets = [];
				for (const key of Object.keys(localStorage)) {
					if (!key.startsWith(LEVELSET_KEY_PREFIX)) continue;
					const raw = localStorage.getItem(key);
					if (raw === null) continue;
					const set = { key, name: key.slice(LEVELSET_KEY_PREFIX.length), data: null };
					try {
						const saved = JSON.parse(raw);
						if (typeof saved?.name === "string" && typeof saved.data === "string") {
							set.name = saved.name;
							set.data = saved.data;
						}
					}
					catch {
						// Keep a damaged entry visible so it can be removed.
					}
					sets.push(set);
				}
				this.customLevelSets = sets.sort((a, b) => a.name.localeCompare(b.name));
			}
			catch {
				this.showToast("Could not read saved levelsets from this browser.", true);
			}
		},

		showToast(message, isError = false) {
			clearTimeout(this.toastTimer);
			this.toastMessage = message;
			this.toastIsError = isError;
			const duration = isError ? Math.max(6500, Math.min(15000, message.length * 45)) : 3000;
			this.toastTimer = setTimeout(() => {
				this.toastMessage = "";
				this.toastTimer = 0;
			}, duration);
		},

		async addCustomLevelSets(event) {
			const input = event.target;
			const files = Array.from(input.files);
			input.value = "";
			if (!files.length || this.levelSetImporting) return;

			this.levelSetImporting = true;
			const errors = [];
			let added = 0;
			try {
				for (const file of files) {
					try {
						const bytes = new Uint8Array(await file.arrayBuffer());
						this.loadPlayLevelSet(0, file.name, bytes);
						const key = levelSetKey(file.name);
						const value = JSON.stringify({ name: file.name, data: encodeLevelSet(bytes) });
						if (localStorage.getItem(key) !== null) {
							throw new Error("A levelset with this name is already saved. Remove it first to import another copy.");
						}
						localStorage.setItem(key, value);
						added++;
					}
					catch (err) {
						const message = err?.name === "QuotaExceededError" ? "This browser’s storage is full." : err?.message ?? String(err);
						errors.push(`${file.name} — ${message}`);
					}
				}
				const messages = added ? [`Added ${added} ${added === 1 ? "levelset" : "levelsets"}.`, ...errors] : errors;
				this.showToast(messages.join("\n"), errors.length > 0);
				this.refreshCustomLevelSets();
			}
			finally {
				this.levelSetImporting = false;
			}
		},

		removeCustomLevelSet(set) {
			if (!window.confirm(`Remove "${set.name}" from your custom levelsets?`)) return;
			try {
				localStorage.removeItem(set.key);
				this.showToast("Levelset removed.");
				this.refreshCustomLevelSets();
				this.$refs.addLevelSetsButton.focus();
			}
			catch (err) {
				this.showToast(`Could not remove ${set.name} — ${err?.message ?? err}`, true);
			}
		},

		loadPlayLevelSet(instance, filename, bytes) {
			const data = this.allocWasmBytes(bytes);
			let name;
			try {
				name = this.allocWasmBytes(new TextEncoder().encode(filename));
				this.lastWasmError = null;
				const result = this.wasmExports.loadPlayLevelSet(instance, data.ptr, data.len, name.ptr, name.len);
				if (result !== 0) {
					throw this.lastWasmError ?? new Error("The levelset file could not be loaded");
				}
			}
			finally {
				if (name) this.wasmExports.freeBytes(name.ptr, name.capacity);
				this.wasmExports.freeBytes(data.ptr, data.capacity);
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
			if (!this.isReadyToPlay) return;
			this.gameStarting = true;
			try {
				if (!this.wasmGamePtr) {
					this.preparePlayInstance();
				}
				this.savePrefs();
				if (this.prefs.fullscreenEnabled) {
					await this.requestPreferredDisplayMode();
				}
				if (this.prefs.audioEnabled) {
					await this.enableAudio();
				}
				this.gameThinkStartTime = performance.now() + this.replayStartDelayMs;
				this.gameActive = true;
			}
			catch (err) {
				this.showToast(err?.message ?? String(err), true);
			}
			finally {
				this.gameStarting = false;
			}
		},

		preparePlayInstance() {
			this.refreshCustomLevelSets();
			const exports = this.wasmExports;
			const gamePtr = exports.createPlayInstance();
			if (!gamePtr) throw new Error("Could not create the game instance");
			try {
				for (const set of this.customLevelSets) {
					try {
						if (set.data === null) throw new Error("The saved file is invalid");
						const binary = atob(set.data);
						const bytes = new Uint8Array(binary.length);
						for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
						this.loadPlayLevelSet(gamePtr, set.name, bytes);
					}
					catch (err) {
						throw new Error(`Could not load ${set.name} — ${err.message}`);
					}
				}
				exports.launchPlayInstance(gamePtr);
				this.wasmGamePtr = gamePtr;
			}
			catch (err) {
				exports.destroyPlayInstance(gamePtr);
				throw err;
			}
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
			const customLevelLink = this.getCustomLevelLink();
			let customLevelPayload = this.getCustomLevelPayload();
			this.isSingleLevel = customLevelLink !== null || customLevelPayload !== null;
			if (customLevelLink !== null) {
				customLevelPayload = await this.fetchCustomLevel(customLevelLink);
			}
			const replayIndex = customLevelPayload === null ? null : this.getReplayIndex();
			this.replayStartDelayMs = replayIndex === null ? 0 : 1000;
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
				this.lastWasmError = new Error(readUtf8(message_ptr, message_len) || "Unknown error");
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

			if (customLevelPayload !== null) {
				this.setLoadingStatus("Loading custom level...", 0.8);
				this.lastWasmError = null;
				const payload = this.allocWasmBytes(encoder.encode(customLevelPayload.value));
				try {
					this.wasmGamePtr = exports.createCustomPlayLevel(payload.ptr, payload.len, customLevelPayload.compressed, replayIndex ?? -1);
				}
				finally {
					exports.freeBytes(payload.ptr, payload.capacity);
				}
				if (!this.wasmGamePtr) {
					throw this.lastWasmError ?? new Error("Could not create the custom level instance");
				}
			}

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

				if (!this.userSelectedControlScheme) {
					this.syncControlSchemeFromEnvironment();
				}

				if (!this.gameActive) {
					last = now;
					acc = 0;
					this.resizeCanvasToDisplaySize();
					this.frameHandle = requestAnimationFrame(frame);
					return;
				}

				const gamePtr = this.wasmGamePtr;
				const dt = Math.min(250, now - last);
				last = now;
				if (now < this.gameThinkStartTime) {
					acc = 0;
				}
				else {
					acc += dt;
				}

				const buttons = this.getButtonsBitmask();
				while (acc >= stepMs) {
					try {
						exports.thinkPlayInstance(gamePtr, buttons);
					}
					catch (err) {
						this.showToast(`Could not update the game — ${err?.message ?? err}`, true);
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
					this.showToast(`Could not draw the game — ${err?.message ?? err}`, true);
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
						exports.destroyPlayInstance(this.wasmGamePtr);
						this.wasmGamePtr = 0;
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
