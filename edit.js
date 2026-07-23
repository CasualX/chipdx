import { createWasmAPI } from "./shade.js";

const KEY_LEFT = 0;
const KEY_RIGHT = 1;
const KEY_UP = 2;
const KEY_DOWN = 3;
const KEY_SHIFT = 4;

const COMMAND = {
	play: 0,
	deleteDown: 1,
	deleteUp: 2,
	undo: 3,
	redo: 4,
	terrain: 5,
	entity: 6,
	connection: 7,
	ice: 8,
	force: 9,
	order: 10,
	expandTop: 11,
	expandBottom: 12,
	expandLeft: 13,
	expandRight: 14,
	cropTop: 15,
	cropBottom: 16,
	cropLeft: 17,
	cropRight: 18,
	zoomIn: 19,
	zoomOut: 20,
	toggleMusic: 21,
	sample: 22,
	cancelLeft: 23,
	rightDown: 24,
	rightUp: 25,
};

const WASM_TOOL_COMMAND = {
	terrain: COMMAND.terrain,
	entity: COMMAND.entity,
	connection: COMMAND.connection,
	ice: COMMAND.ice,
	force: COMMAND.force,
	order: COMMAND.order,
};

const TEXT_FIELD = {
	name: 0,
	author: 1,
	hint: 2,
};

const NUMBER_FIELD = {
	requiredChips: 0,
	timeLimit: 1,
};

window.chipEditor = function chipEditor() {
	return {
		statusText: "Loading...",
		wasmExports: null,
		wasmMemory: null,
		editorPtr: 0,
		lastWasmError: null,
		savedLevel: "",
		currentFileName: "level.json",
		frameHandle: 0,
		pageActive: true,
		pressedKeys: new Set(),
		activePointerId: null,
		activeMouseButtons: new Set(),
		activeTool: "terrain",
		lastEditTool: "terrain",
		activeToolAction: null,
		panPointerPoint: null,
		touchPointers: new Map(),
		touchSupported: false,
		editorPlaying: false,
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
		padTouchId: null,
		shareDialogVisible: false,
		shareUrl: "",
		shareLinkText: "",
		initialized: false,
		gamepadPressed: {
			left: false,
			right: false,
			up: false,
			down: false,
		},

		async init() {
			if (this.initialized) return;
			this.initialized = true;
			this.touchSupported = this.detectTouchSupport();
			window.addEventListener("error", (event) => this.fail(event.error ?? event.message ?? event));
			window.addEventListener("unhandledrejection", (event) => this.fail(event.reason ?? event));
			window.addEventListener("beforeunload", (event) => this.onBeforeUnload(event));
			window.addEventListener("keydown", (event) => this.onKeyDown(event));
			window.addEventListener("keyup", (event) => this.onKeyUp(event));
			window.addEventListener("blur", () => this.clearAllInputs());
			document.addEventListener("visibilitychange", () => {
				this.pageActive = document.visibilityState === "visible";
				if (!this.pageActive) this.clearAllInputs();
			});

			await this.loadWasm();
			this.savedLevel = this.saveLevelFromWasm();
			this.setStatus("Ready");
			this.startLoop();
		},

		setStatus(text) {
			this.statusText = text;
		},

		fail(err) {
			console.error(err);
			this.setStatus(String(err && err.message ? err.message : err));
		},

		onBeforeUnload(event) {
			if (!this.hasUnsavedChanges()) return;
			event.preventDefault();
			event.returnValue = "";
		},

		hasUnsavedChanges() {
			if (!this.editorPtr) return false;
			try {
				return this.saveLevelFromWasm() !== this.savedLevel;
			}
			catch {
				return true;
			}
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
			return { width, height };
		},

		detectTouchSupport() {
			if (navigator.maxTouchPoints > 0) return true;
			if (window.matchMedia && window.matchMedia("(pointer: coarse)").matches) return true;
			return "ontouchstart" in window;
		},

		get touchControlsEnabled() {
			return this.touchSupported && this.editorPlaying && !this.shareDialogVisible;
		},

		canvasPoint(event) {
			const canvas = this.$refs.canvas;
			const rect = canvas.getBoundingClientRect();
			const scaleX = canvas.width / Math.max(1, rect.width);
			const scaleY = canvas.height / Math.max(1, rect.height);
			return {
				x: Math.floor((event.clientX - rect.left) * scaleX),
				y: Math.floor((event.clientY - rect.top) * scaleY),
			};
		},

		canvasDeltaFromClient(dx, dy) {
			const canvas = this.$refs.canvas;
			const rect = canvas.getBoundingClientRect();
			const scaleX = canvas.width / Math.max(1, rect.width);
			const scaleY = canvas.height / Math.max(1, rect.height);
			return {
				x: dx * scaleX,
				y: dy * scaleY,
			};
		},

		updateMouse(event) {
			if (!this.editorPtr) return;
			const point = this.canvasPoint(event);
			this.wasmExports.setEditorMouse(this.editorPtr, point.x, point.y);
		},

		pointerIsOverEditorChrome(event) {
			const element = document.elementFromPoint(event.clientX, event.clientY);
			return !!element && !!element.closest("#toolbar");
		},

		onPointerDown(event) {
			if (event.pointerType === "touch") {
				this.onTouchPointerDown(event);
				return;
			}
			if (event.pointerType === "mouse" && event.button !== 0 && event.button !== 2) return;
			event.preventDefault();
			this.activePointerId = event.pointerId;
			this.$refs.canvas.setPointerCapture?.(event.pointerId);
			this.updateMouse(event);
			if (this.beginPanPointer(event) || this.beginActiveToolAction()) return;
			const button = event.button === 2 ? 1 : 0;
			this.activeMouseButtons.add(button);
			this.commandMouseButton(button, true);
		},

		onPointerMove(event) {
			if (event.pointerType === "touch") {
				this.onTouchPointerMove(event);
				return;
			}
			if (this.activePointerId !== null && event.pointerId !== this.activePointerId) return;
			if (this.updatePanPointer(event)) return;
			this.updateMouse(event);
		},

		onPointerUp(event) {
			if (event.pointerType === "touch") {
				this.onTouchPointerUp(event);
				return;
			}
			if (this.activePointerId !== null && event.pointerId !== this.activePointerId) return;
			event.preventDefault();
			this.updateMouse(event);
			if (this.endPanPointer(event.pointerId)) return;
			if (this.endActiveToolAction()) {
				this.releasePointerCapture(event.pointerId);
				this.activePointerId = null;
				return;
			}
			const button = event.button === 2 ? 1 : 0;
			if (button === 0 && this.pointerIsOverEditorChrome(event)) {
				this.cancelLeftDrag();
			}
			else {
				this.commandMouseButton(button, false);
			}
			this.activeMouseButtons.delete(button);
			this.releasePointerIfIdle(event.pointerId);
		},

		onPointerCancel(event) {
			if (event.pointerType === "touch") {
				this.onTouchPointerCancel(event);
				return;
			}
			if (this.activePointerId !== null && event.pointerId !== this.activePointerId) return;
			if (this.panPointerPoint) {
				this.endPanPointer(event.pointerId);
				return;
			}
			if (this.activeToolAction) {
				this.cancelActiveToolAction();
				this.activePointerId = null;
				return;
			}
			if (this.activeMouseButtons.has(0)) {
				this.cancelLeftDrag();
			}
			if (this.activeMouseButtons.has(1)) {
				this.commandMouseButton(1, false);
			}
			this.activeMouseButtons.clear();
			this.releasePointerIfIdle(event.pointerId);
		},

		releasePointerIfIdle(pointerId) {
			if (this.activeMouseButtons.size > 0) return;
			this.releasePointerCapture(pointerId);
			this.activePointerId = null;
		},

		onTouchPointerDown(event) {
			event.preventDefault();
			this.$refs.canvas.setPointerCapture?.(event.pointerId);
			this.touchPointers.set(event.pointerId, {
				clientX: event.clientX,
				clientY: event.clientY,
			});

			if (this.touchPointers.size === 1) {
				this.activePointerId = event.pointerId;
				this.updateMouse(event);
				if (this.beginPanPointer(event) || this.beginActiveToolAction()) return;
				this.activeMouseButtons.add(0);
				this.commandMouseButton(0, true);
				return;
			}

			if (this.activeMouseButtons.has(0)) {
				this.cancelLeftDrag();
				this.activeMouseButtons.delete(0);
			}
			if (this.panPointerPoint) {
				this.endPanPointer(this.panPointerPoint.pointerId);
			}
			if (this.activeToolAction) {
				this.cancelActiveToolAction();
			}
			this.activePointerId = null;
		},

		onTouchPointerMove(event) {
			if (!this.touchPointers.has(event.pointerId)) return;
			event.preventDefault();
			this.touchPointers.set(event.pointerId, {
				clientX: event.clientX,
				clientY: event.clientY,
			});

			if (this.touchPointers.size >= 2) {
				return;
			}

			if (this.activePointerId === event.pointerId) {
				if (this.updatePanPointer(event)) return;
				this.updateMouse(event);
			}
		},

		onTouchPointerUp(event) {
			if (!this.touchPointers.has(event.pointerId)) return;
			event.preventDefault();
			const wasDrawing = this.activePointerId === event.pointerId && this.activeMouseButtons.has(0);
			const wasPanning = this.activePointerId === event.pointerId && !!this.panPointerPoint;
			const wasToolAction = this.activePointerId === event.pointerId && !!this.activeToolAction;
			if (wasPanning) {
				this.endPanPointer(event.pointerId);
			}
			else if (wasToolAction) {
				this.updateMouse(event);
				this.endActiveToolAction();
				this.activePointerId = null;
			}
			else if (wasDrawing) {
				this.updateMouse(event);
				if (this.pointerIsOverEditorChrome(event)) {
					this.cancelLeftDrag();
				}
				else {
					this.commandMouseButton(0, false);
				}
				this.activeMouseButtons.delete(0);
				this.activePointerId = null;
			}
			this.touchPointers.delete(event.pointerId);
			this.releasePointerCapture(event.pointerId);
		},

		onTouchPointerCancel(event) {
			if (!this.touchPointers.has(event.pointerId)) return;
			if (this.activePointerId === event.pointerId && this.panPointerPoint) {
				this.endPanPointer(event.pointerId);
			}
			if (this.activePointerId === event.pointerId && this.activeToolAction) {
				this.cancelActiveToolAction();
				this.activePointerId = null;
			}
			if (this.activePointerId === event.pointerId && this.activeMouseButtons.has(0)) {
				this.cancelLeftDrag();
				this.activeMouseButtons.delete(0);
				this.activePointerId = null;
			}
			this.touchPointers.delete(event.pointerId);
			this.releasePointerCapture(event.pointerId);
		},

		beginPanPointer(event) {
			if (this.activeTool !== "pan" || event.button === 2) return false;
			this.panPointerPoint = {
				pointerId: event.pointerId,
				clientX: event.clientX,
				clientY: event.clientY,
			};
			return true;
		},

		updatePanPointer(event) {
			if (!this.panPointerPoint || this.panPointerPoint.pointerId !== event.pointerId) return false;
			const clientDelta = {
				x: event.clientX - this.panPointerPoint.clientX,
				y: event.clientY - this.panPointerPoint.clientY,
			};
			const canvasDelta = this.canvasDeltaFromClient(clientDelta.x, clientDelta.y);
			this.panView(canvasDelta.x, canvasDelta.y);
			this.panPointerPoint.clientX = event.clientX;
			this.panPointerPoint.clientY = event.clientY;
			return true;
		},

		endPanPointer(pointerId) {
			if (!this.panPointerPoint || this.panPointerPoint.pointerId !== pointerId) return false;
			this.panPointerPoint = null;
			this.releasePointerCapture(pointerId);
			this.activePointerId = null;
			return true;
		},

		beginActiveToolAction() {
			if (this.activeTool === "sample") {
				this.activeToolAction = "sample";
				this.command(COMMAND.sample);
				return true;
			}
			if (this.activeTool === "rotate") {
				this.activeToolAction = "rotate";
				this.command(COMMAND.rightDown);
				return true;
			}
			if (this.activeTool === "delete") {
				this.activeToolAction = "delete";
				this.command(COMMAND.deleteDown);
				return true;
			}
			return false;
		},

		endActiveToolAction() {
			if (this.activeToolAction === "sample") {
				this.activeToolAction = null;
				return true;
			}
			if (this.activeToolAction === "rotate") {
				this.command(COMMAND.rightUp);
				this.activeToolAction = null;
				return true;
			}
			if (this.activeToolAction === "delete") {
				this.command(COMMAND.deleteUp);
				this.activeToolAction = null;
				return true;
			}
			return false;
		},

		cancelActiveToolAction() {
			this.endActiveToolAction();
		},

		selectTool(tool) {
			if (tool === "pan") {
				if (this.activeTool === "pan") {
					this.activeTool = this.lastEditTool;
					if (WASM_TOOL_COMMAND[this.activeTool] !== undefined) {
						this.command(WASM_TOOL_COMMAND[this.activeTool]);
					}
				}
				else {
					this.activeTool = "pan";
				}
				return true;
			}

			this.activeTool = tool;
			this.lastEditTool = tool;
			if (WASM_TOOL_COMMAND[tool] !== undefined) {
				this.command(WASM_TOOL_COMMAND[tool]);
			}
			return true;
		},

		releasePointerCapture(pointerId) {
			try {
				this.$refs.canvas.releasePointerCapture?.(pointerId);
			}
			catch {
				// ignore
			}
		},

		commandMouseButton(button, pressed) {
			if (!this.editorPtr) return;
			this.wasmExports.setEditorMouseButton(this.editorPtr, button, pressed);
		},

		cancelLeftDrag() {
			if (!this.editorPtr) return;
			this.wasmExports.editorCommand(this.editorPtr, COMMAND.cancelLeft);
		},

		panView(deltaX, deltaY) {
			if (!this.editorPtr) return;
			this.wasmExports.panEditorView(this.editorPtr, deltaX, deltaY);
		},

		allocWasmBytes(bytes) {
			const capacity = Math.max(1, bytes.length);
			const ptr = this.wasmExports.allocBytes(capacity);
			if (!ptr) throw new Error("WASM allocBytes() returned null");
			new Uint8Array(this.wasmMemory.buffer, ptr, bytes.length).set(bytes);
			return { ptr, len: bytes.length, capacity };
		},

		readUtf8(ptr, len) {
			if (!this.wasmMemory || !ptr || !len) return "";
			return new TextDecoder().decode(new Uint8Array(this.wasmMemory.buffer, ptr, len));
		},

		withWasmString(value, fn) {
			const bytes = new TextEncoder().encode(value);
			const data = this.allocWasmBytes(bytes);
			try {
				return fn(data.ptr, data.len);
			}
			finally {
				this.wasmExports.freeBytes(data.ptr, data.capacity);
			}
		},

		saveLevelFromWasm() {
			return this.readOwnedWasmString((lenPtr) => this.wasmExports.saveEditorLevel(this.editorPtr, lenPtr));
		},

		sharePayloadFromWasm() {
			return this.readOwnedWasmString((lenPtr) => this.wasmExports.shareEditorLevel(this.editorPtr, lenPtr));
		},

		readOwnedWasmString(createPtr) {
			const lenBuf = this.allocWasmBytes(new Uint8Array(4));
			let ptr = 0;
			try {
				this.lastWasmError = null;
				ptr = createPtr(lenBuf.ptr);
				if (this.lastWasmError) throw this.lastWasmError;
				if (!ptr) throw new Error("WASM string export returned null");
				const len = new Uint32Array(this.wasmMemory.buffer, lenBuf.ptr, 1)[0];
				return this.readUtf8(ptr, len);
			}
			finally {
				if (ptr) {
					const len = new Uint32Array(this.wasmMemory.buffer, lenBuf.ptr, 1)[0];
					this.wasmExports.freeBytes(ptr, len);
				}
				this.wasmExports.freeBytes(lenBuf.ptr, lenBuf.capacity);
			}
		},

		loadLevelIntoWasm(json) {
			this.withWasmString(json, (ptr, len) => {
				this.lastWasmError = null;
				const result = this.wasmExports.loadEditorLevel(this.editorPtr, ptr, len);
				if (this.lastWasmError) throw this.lastWasmError;
				if (result !== 0) throw new Error("Level could not be loaded");
			});
		},

		async loadWasm() {
			this.setStatus("Initializing WebGL...");
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

			const resultError = (messagePtr, messageLen) => {
				this.lastWasmError = new Error(readUtf8(messagePtr, messageLen) || "Unknown WASM error");
			};

			const readFile = (pathPtr, pathLen, contentPtr, contentLenPtr) => {
				if (!wasmMemory) return -1;
				const path = readUtf8(pathPtr, pathLen);
				let content = null;
				try {
					content = localStorage.getItem(path);
				}
				catch {
					return -1;
				}
				if (content === null) return -1;
				const bytes = encoder.encode(content);
				const lenView = new Uint32Array(wasmMemory.buffer, contentLenPtr, 1);
				if (!contentPtr) {
					lenView[0] = bytes.length;
					return 0;
				}
				const cap = lenView[0] >>> 0;
				new Uint8Array(wasmMemory.buffer, contentPtr, cap).set(bytes.subarray(0, cap));
				lenView[0] = Math.min(cap, bytes.length);
				return 0;
			};

			const writeFile = (pathPtr, pathLen, contentPtr, contentLen) => {
				if (!wasmMemory) return -1;
				const path = readUtf8(pathPtr, pathLen);
				const content = decoder.decode(new Uint8Array(wasmMemory.buffer, contentPtr, contentLen));
				try {
					localStorage.setItem(path, content);
					return 0;
				}
				catch {
					return -1;
				}
			};

			const imports = {
				webgl: shade,
				env: {
					randomBytes,
					playSound: () => {},
					playMusic: () => {},
					registerSound: () => {},
					registerMusic: () => {},
					setTitle: () => {},
					requestLevelSetFile: () => {},
					resultError,
					readFile,
					writeFile,
				},
			};

			this.setStatus("Loading WASM...");
			const response = await fetch("./chipwasm.wasm");
			if (!response.ok) {
				throw new Error(`Failed to fetch chipwasm.wasm: ${response.status} ${response.statusText}`);
			}
			const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), imports);
			shade.bindInstance(instance);
			wasmMemory = instance.exports.memory;
			this.wasmExports = instance.exports;
			this.wasmMemory = wasmMemory;
			this.editorPtr = this.wasmExports.createEditorInstance();
			if (!this.editorPtr) throw new Error("createEditorInstance() returned null");

			const cleanup = () => {
				if (this.frameHandle) cancelAnimationFrame(this.frameHandle);
				if (this.editorPtr) this.wasmExports.destroyEditorInstance(this.editorPtr);
				this.editorPtr = 0;
			};
			window.addEventListener("pagehide", cleanup, { once: true });
		},

		startLoop() {
			const stepMs = 1000 / 60;
			let last = performance.now();
			let acc = 0;
			const frame = (now) => {
				if (!this.editorPtr) return;
				const dt = Math.min(250, now - last);
				last = now;
				if (this.pageActive) acc += dt;
				const { width, height } = this.resizeCanvasToDisplaySize();
				this.syncGamepadInputs();
				while (acc >= stepMs) {
					this.wasmExports.thinkEditorInstance(this.editorPtr);
					acc -= stepMs;
				}
				this.syncEditorPlayState();
				this.wasmExports.drawEditorInstance(this.editorPtr, now / 1000.0, width, height);
				this.frameHandle = requestAnimationFrame(frame);
			};
			this.frameHandle = requestAnimationFrame(frame);
		},

		syncEditorPlayState() {
			if (!this.editorPtr || !this.wasmExports.isEditorPlaying) return;
			const wasPlaying = this.editorPlaying;
			this.editorPlaying = !!this.wasmExports.isEditorPlaying(this.editorPtr);
			if (wasPlaying && !this.editorPlaying) {
				this.clearTouchInputs();
				this.clearGamepadInputs();
			}
		},

		syncGamepadInputs() {
			if (!this.editorPtr || !this.pageActive || !this.editorPlaying) {
				this.clearGamepadInputs();
				return;
			}

			const next = this.getGamepadDirectionState();
			for (const [dir, key] of [["left", KEY_LEFT], ["right", KEY_RIGHT], ["up", KEY_UP], ["down", KEY_DOWN]]) {
				if (this.gamepadPressed[dir] === next[dir]) continue;
				this.gamepadPressed[dir] = next[dir];
				this.setEditorKey(key, next[dir]);
			}
		},

		getGamepadDirectionState() {
			const pads = navigator.getGamepads ? navigator.getGamepads() : [];
			const next = { left: false, right: false, up: false, down: false };
			const deadzone = 0.5;

			for (const gamepad of pads) {
				if (!gamepad || !gamepad.connected) continue;
				const buttons = gamepad.buttons ?? [];
				const axes = gamepad.axes ?? [];

				next.up ||= !!buttons[12]?.pressed;
				next.down ||= !!buttons[13]?.pressed;
				next.left ||= !!buttons[14]?.pressed;
				next.right ||= !!buttons[15]?.pressed;

				const lx = axes[0] ?? 0;
				const ly = axes[1] ?? 0;
				next.left ||= lx <= -deadzone;
				next.right ||= lx >= deadzone;
				next.up ||= ly <= -deadzone;
				next.down ||= ly >= deadzone;
			}

			return next;
		},

		clearGamepadInputs() {
			for (const [dir, key] of [["left", KEY_LEFT], ["right", KEY_RIGHT], ["up", KEY_UP], ["down", KEY_DOWN]]) {
				if (!this.gamepadPressed[dir]) continue;
				this.gamepadPressed[dir] = false;
				this.setEditorKey(key, false);
			}
		},

		clearAllInputs() {
			this.clearHeldKeys();
			this.clearTouchInputs();
			this.clearGamepadInputs();
		},

		clearHeldKeys() {
			this.pressedKeys.clear();
			if (!this.editorPtr) return;
			for (const key of [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_SHIFT]) {
				this.wasmExports.setEditorKey(this.editorPtr, key, false);
			}
			this.wasmExports.editorCommand(this.editorPtr, COMMAND.deleteUp);
		},

		setEditorKey(key, pressed) {
			if (!this.editorPtr) return;
			this.wasmExports.setEditorKey(this.editorPtr, key, pressed);
		},

		onKeyDown(event) {
			if (event.code === "Escape" && this.shareDialogVisible) {
				this.closeShareDialog();
				event.preventDefault();
				return;
			}
			if (this.shouldCapturePlayKeyEvent(event)) {
				if (event.repeat || this.pressedKeys.has(event.code)) {
					if (this.isHandledKey(event.code)) event.preventDefault();
					return;
				}
				this.pressedKeys.add(event.code);
				if (this.handleKey(event, true)) event.preventDefault();
				return;
			}
			if (this.isTextEntryEvent(event)) return;
			if (event.repeat || this.pressedKeys.has(event.code)) {
				if (this.isHandledKey(event.code) || this.isActionKey(event.code)) {
					event.preventDefault();
				}
				return;
			}
			this.pressedKeys.add(event.code);
			if (this.handleKey(event, true)) event.preventDefault();
		},

		onKeyUp(event) {
			if (this.shouldCapturePlayKeyEvent(event)) {
				this.pressedKeys.delete(event.code);
				if (this.handleKey(event, false)) event.preventDefault();
				return;
			}
			if (this.isTextEntryEvent(event)) return;
			this.pressedKeys.delete(event.code);
			if (this.handleKey(event, false)) event.preventDefault();
		},

		shouldCapturePlayKeyEvent(event) {
			if (!this.editorPlaying || this.shareDialogVisible) return false;
			if (!this.isHandledKey(event.code)) return false;
			const target = event.target;
			if (!target || target === document.body || target === document.documentElement) return true;
			if (target.closest("#share-dialog")) return false;
			return true;
		},

		handleKey(event, pressed) {
			const code = event.code;
			if (code === "ArrowLeft" || code === "KeyA") return this.keyInput(KEY_LEFT, pressed);
			if (code === "ArrowRight" || code === "KeyD") return this.keyInput(KEY_RIGHT, pressed);
			if (code === "ArrowUp" || code === "KeyW") return this.keyInput(KEY_UP, pressed);
			if (code === "ArrowDown" || code === "KeyS") return this.keyInput(KEY_DOWN, pressed);
			if (code === "ShiftLeft" || code === "ShiftRight") return this.keyInput(KEY_SHIFT, pressed);
			if (code === "Delete") {
				this.command(pressed ? COMMAND.deleteDown : COMMAND.deleteUp);
				return true;
			}
			if (!pressed) return this.isHandledKey(code);

			if (code === "Enter") return this.command(COMMAND.play);
			if (code === "KeyU") return this.command(COMMAND.undo);
			if (code === "KeyY") return this.command(COMMAND.redo);
			if (code === "KeyT") return this.selectTool("terrain");
			if (code === "KeyE") return this.selectTool("entity");
			if (code === "KeyC") return this.selectTool("connection");
			if (code === "KeyI") return this.selectTool("ice");
			if (code === "KeyR") return this.selectTool("force");
			if (code === "KeyO") return this.selectTool("order");
			if (code === "KeyM") return this.command(COMMAND.toggleMusic);
			if (code === "KeyQ") return this.selectTool("sample");
			if (code === "NumpadAdd" || code === "Equal") return this.command(COMMAND.zoomIn);
			if (code === "NumpadSubtract" || code === "Minus") return this.command(COMMAND.zoomOut);
			if (code === "F2") return this.openFile();
			if (code === "F5") return this.downloadLevel();
			if (code === "F6") return this.editTextField("name", "Level Title");
			if (code === "F7") return this.editNumberField("requiredChips", "Required Chips");
			if (code === "F8") return this.editNumberField("timeLimit", "Time Limit");
			if (code === "F9") return this.editTextField("hint", "Level Hint");
			if (code === "F10") return this.editTextField("author", "Author");
			if (code === "KeyF") return this.toggleFullscreen();
			if (code === "Numpad8") return this.command(event.shiftKey ? COMMAND.expandTop : COMMAND.cropTop);
			if (code === "Numpad2") return this.command(event.shiftKey ? COMMAND.expandBottom : COMMAND.cropBottom);
			if (code === "Numpad4") return this.command(event.shiftKey ? COMMAND.expandLeft : COMMAND.cropLeft);
			if (code === "Numpad6") return this.command(event.shiftKey ? COMMAND.expandRight : COMMAND.cropRight);
			return false;
		},

		isHandledKey(code) {
			return code.startsWith("Arrow") ||
				["KeyA", "KeyD", "KeyW", "KeyS", "ShiftLeft", "ShiftRight", "Delete"].includes(code);
		},

		isActionKey(code) {
			return [
				"Enter",
				"KeyU",
				"KeyY",
				"KeyT",
				"KeyE",
				"KeyC",
				"KeyI",
				"KeyR",
				"KeyO",
				"KeyM",
				"KeyQ",
				"NumpadAdd",
				"Equal",
				"NumpadSubtract",
				"Minus",
				"F2",
				"F5",
				"F6",
				"F7",
				"F8",
				"F9",
				"F10",
				"KeyF",
				"Numpad8",
				"Numpad2",
				"Numpad4",
				"Numpad6",
			].includes(code);
		},

		isTextEntryEvent(event) {
			const target = event.target;
			if (!target || target === document.body || target === document.documentElement) return false;
			if (target.closest("#game")) return false;
			return !!target.closest("input, textarea, select, button, a, [contenteditable='true']");
		},

		keyInput(key, pressed) {
			this.setEditorKey(key, pressed);
			return true;
		},

		command(command) {
			this.wasmExports.editorCommand(this.editorPtr, command);
			this.syncEditorPlayState();
			this.setStatus("Edited");
			return true;
		},

		keyForPadDirection(dir) {
			if (dir === "left") return KEY_LEFT;
			if (dir === "right") return KEY_RIGHT;
			if (dir === "up") return KEY_UP;
			if (dir === "down") return KEY_DOWN;
			return null;
		},

		setPadDirection(dir) {
			if (this.padDir === dir) return;
			if (this.padDir) {
				this.touchPressed[this.padDir] = false;
				this.setEditorKey(this.keyForPadDirection(this.padDir), false);
			}
			this.padDir = dir;
			if (this.padDir) {
				this.touchPressed[this.padDir] = true;
				this.setEditorKey(this.keyForPadDirection(this.padDir), true);
			}
		},

		getPadDirectionFromTouch(touch) {
			const pad = this.$refs.touchPad;
			if (!pad) return null;
			const rect = pad.getBoundingClientRect();
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
			if (!this.touchControlsEnabled) return;
			this.touchPressed[id] = true;
			if (id === "b" || id === "start") {
				this.command(COMMAND.play);
				this.clearTouchInputs();
			}
		},

		releaseTouchButton(id) {
			this.touchPressed[id] = false;
		},

		onPadTouchStart(event) {
			if (!this.touchControlsEnabled || this.padTouchId !== null) return;
			const touch = event.changedTouches[0];
			if (!touch) return;
			this.padTouchId = touch.identifier;
			this.setPadDirection(this.getPadDirectionFromTouch(touch));
		},

		onPadTouchMove(event) {
			if (!this.touchControlsEnabled || this.padTouchId === null) return;
			for (const touch of event.changedTouches) {
				if (touch.identifier === this.padTouchId) {
					this.setPadDirection(this.getPadDirectionFromTouch(touch));
					return;
				}
			}
		},

		onPadTouchEnd(event) {
			if (this.padTouchId === null) return;
			for (const touch of event.changedTouches) {
				if (touch.identifier === this.padTouchId) {
					this.padTouchId = null;
					this.setPadDirection(null);
					return;
				}
			}
		},

		clearTouchInputs() {
			this.setPadDirection(null);
			this.padTouchId = null;
			for (const id of ["a", "b", "start", "select"]) {
				this.touchPressed[id] = false;
			}
		},

		runToolbarCommand(command) {
			if (this.isToolbarActionDisabled(command)) return false;
			if (command === "open") this.openFile();
			else if (command === "save") this.downloadLevel();
			else if (command === "share") this.openShareDialog();
			else if (command === "terrain") this.selectTool("terrain");
			else if (command === "entity") this.selectTool("entity");
			else if (command === "connection") this.selectTool("connection");
			else if (command === "ice") this.selectTool("ice");
			else if (command === "force") this.selectTool("force");
			else if (command === "order") this.selectTool("order");
			else if (command === "sample") this.selectTool("sample");
			else if (command === "rotate") this.selectTool("rotate");
			else if (command === "delete") this.selectTool("delete");
			else if (command === "pan") this.selectTool("pan");
			else if (command === "zoom-in") this.command(COMMAND.zoomIn);
			else if (command === "zoom-out") this.command(COMMAND.zoomOut);
			else if (command === "meta") this.editMetadata();
			else if (command === "fullscreen") this.toggleFullscreen();
			else if (COMMAND[command] !== undefined) this.command(COMMAND[command]);
			if (command === "play") this.focusCanvas();
		},

		focusCanvas() {
			const active = document.activeElement;
			if (active && active instanceof HTMLElement && active !== document.body) {
				active.blur();
			}
		},

		isToolbarActionDisabled(command) {
			if (!this.editorPlaying) return false;
			return !["open", "save", "share", "play", "fullscreen"].includes(command);
		},

		onToolbarKeyDown(event) {
			if (event.repeat && (event.code === "Enter" || event.code === "Space")) {
				event.preventDefault();
			}
		},

		openFile() {
			this.$refs.fileInput.value = "";
			this.$refs.fileInput.click();
			return true;
		},

		async openSelectedFile(event) {
			const input = event.target;
			const file = input.files && input.files[0];
			input.value = "";
			if (!file) return;
			const text = await file.text();
			this.loadLevelIntoWasm(text);
			this.savedLevel = this.saveLevelFromWasm();
			this.currentFileName = file.name || "level.json";
			this.setStatus(`Opened ${this.currentFileName}`);
		},

		downloadLevel() {
			const level = this.saveLevelFromWasm();
			const blob = new Blob([level], { type: "application/json" });
			const url = URL.createObjectURL(blob);
			const link = document.createElement("a");
			link.href = url;
			link.download = this.currentFileName || "level.json";
			document.body.appendChild(link);
			link.click();
			link.remove();
			URL.revokeObjectURL(url);
			this.savedLevel = level;
			this.setStatus(`Saved ${link.download}`);
			return true;
		},

		createShareUrl() {
			const payload = this.sharePayloadFromWasm();
			const playUrl = new URL("./", window.location.href);
			playUrl.search = "";
			playUrl.hash = "";
			playUrl.hash = `?levelc=${payload}`;
			return playUrl.toString();
		},

		openShareDialog() {
			this.shareUrl = this.createShareUrl();
			this.shareLinkText = this.shareUrl.length > 96 ? `${this.shareUrl.slice(0, 72)}...${this.shareUrl.slice(-18)}` : this.shareUrl;
			this.shareDialogVisible = true;
			this.$nextTick(() => this.$refs.shareLink?.focus());
			this.setStatus("Share URL ready");
			return true;
		},

		closeShareDialog() {
			this.shareDialogVisible = false;
		},

		async copyShareUrl() {
			if (!this.shareUrl) return;
			try {
				await navigator.clipboard.writeText(this.shareUrl);
				this.setStatus("Share URL copied");
			}
			catch {
				this.fallbackCopyText(this.shareUrl);
				this.setStatus("Share URL copied");
			}
		},

		fallbackCopyText(text) {
			const input = document.createElement("textarea");
			input.value = text;
			input.setAttribute("readonly", "");
			input.style.position = "fixed";
			input.style.left = "-9999px";
			document.body.appendChild(input);
			input.select();
			document.execCommand("copy");
			input.remove();
		},

		openShareUrl() {
			if (this.shareUrl) window.open(this.shareUrl, "_blank", "noopener,noreferrer");
		},

		currentLevelDto() {
			return JSON.parse(this.saveLevelFromWasm());
		},

		editMetadata() {
			this.editTextField("name", "Level Title");
			this.editNumberField("requiredChips", "Required Chips");
			this.editNumberField("timeLimit", "Time Limit");
			this.editTextField("hint", "Level Hint");
			this.editTextField("author", "Author");
			return true;
		},

		editTextField(name, label) {
			const dto = this.currentLevelDto();
			const current = dto[name] ?? "";
			const value = window.prompt(label, current);
			if (value === null) return true;
			this.withWasmString(value, (ptr, len) => {
				this.wasmExports.setEditorTextField(this.editorPtr, TEXT_FIELD[name], ptr, len);
			});
			this.setStatus("Metadata updated");
			return true;
		},

		editNumberField(name, label) {
			const dto = this.currentLevelDto();
			const current = name === "requiredChips" ? dto.required_chips : dto.time_limit;
			const value = window.prompt(label, String(current ?? 0));
			if (value === null) return true;
			const parsed = Number.parseInt(value, 10);
			if (!Number.isFinite(parsed)) {
				this.setStatus("Invalid number");
				return true;
			}
			this.wasmExports.setEditorNumberField(this.editorPtr, NUMBER_FIELD[name], parsed);
			this.setStatus("Metadata updated");
			return true;
		},

		async toggleFullscreen() {
			if (document.fullscreenElement) {
				await document.exitFullscreen?.();
			}
			else {
				await document.documentElement.requestFullscreen?.();
				const orientation = screen && screen.orientation;
				if (orientation && orientation.lock) {
					try {
						await orientation.lock("landscape");
					}
					catch {
						// ignore
					}
				}
			}
			return true;
		},
	};
};
