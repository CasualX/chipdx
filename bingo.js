const STORAGE_VERSION = "v2";
const RARITY_COUNTS = { common: 9, uncommon: 7, rare: 5, epic: 3 };
const PICK_ORDER = ["epic", "rare", "uncommon", "common"];
const LINES = [
	[0, 1, 2, 3, 4], [5, 6, 7, 8, 9], [10, 11, 12, 13, 14], [15, 16, 17, 18, 19], [20, 21, 22, 23, 24],
	[0, 5, 10, 15, 20], [1, 6, 11, 16, 21], [2, 7, 12, 17, 22], [3, 8, 13, 18, 23], [4, 9, 14, 19, 24],
	[0, 6, 12, 18, 24], [4, 8, 12, 16, 20],
];

const CARD_POOL = {
	common: [
		card("hidden-chip", "Find a chip hidden under a block", "chip", "chips"),
		card("forgotten-chip", "Return to an earlier area for a missed chip", "chip", "chips"),
		card("fire-boots", "Fire ahead before finding fire boots", "fire", "items"),
		card("flippers", "Water ahead before finding flippers", "water", "items"),
		card("skates", "Ice ahead before finding ice skates", "ice", "items"),
		card("suction", "Force floors ahead before finding suction boots", "boots", "items"),
		card("block-wrong", "Push a block into the wrong place", "block", "blocks"),
		card("block-puzzle", "Block puzzle", "block", "blocks"),
		card("block-bridge", "Use a block as a bridge", "water", "blocks"),
		card("block-button", "Use a block to hold a button", "button", "buttons"),
		card("red-button", "Clone a monster", "monster", "buttons"),
		card("monster-timing", "Wait until a monster moves into the right position", "timer", "monsters"),
		card("tele-puzzle", "Teleport puzzle", "teleport", "movement"),
		card("ice-maze", "Ice maze", "ice", "movement"),
		card("force-maze", "Force-floor maze", "map", "movement"),
		card("force-random", "A random force floor sends Chip the wrong way", "map", "movement"),
		card("recessed-return", "Take the wrong path through a recessed wall", "wall", "movement"),
		card("hidden-passage", "Walk through a fake blue wall", "wall", "secrets"),
		card("hint-return", "Return to a hint tile after getting stuck", "hint", "solve"),
		card("cramped", "Level is mostly one-tile-wide corridors", "map", "level"),
		card("maze", "Maze level", "map", "level"),
		card("sokoban", "Sokoban-heavy level", "block", "level"),
		card("pure-puzzle", "Puzzle level with no moving monsters", "hint", "level"),
		card("pure-action", "Monster gauntlet", "monster", "level"),
		card("blob-rng", "Blob RNG level", "monster", "level"),
		card("monster-chase", "A monster chases Chip through the level", "monster", "level"),
		card("pink-ball-dodge", "Pink ball dodge section", "bomb", "movement"),
		card("monster-parade", "Monster parade: a line of monsters follows a fixed route", "monster", "level"),
		card("short", "Finish a level in under one minute", "timer", "level"),
		card("accidental", "Solve something accidentally", "hint", "solve"),
		card("what-button", "“What does this button do?”", "button", "streamer"),
		card("oh", "Streamer says “Ohhhhh.”", "hint", "streamer"),
		card("stupid", "Streamer says “That was stupid”", "hint", "streamer"),
		card("close", "Streamer says “That was close”", "timer", "streamer"),
		card("chat-first", "Chat spots the solution first", "hint", "chat"),
		card("chat-wrong", "Chat confidently gives a wrong solution", "hint", "chat"),
		card("harder", "Streamer says “This is harder than it looks”", "lock", "streamer"),
		card("easier", "Streamer says “That was easier than it looked”", "exit", "streamer"),
		card("trial-error", "Level requires trial and error", "map", "solve"),
		card("return-start", "Return to the starting area before exiting", "map", "solve"),
		card("there-back", "Revisit the same area after changing it", "map", "solve"),
		card("two-streak", "Complete two levels without dying", "heart", "finish"),
		card("toggle-puzzle", "Toggle-wall puzzle", "toggle", "buttons"),
		card("spare-key", "Finish with an unused key", "key", "items"),
		card("thief", "Lose boots to a thief", "thief", "items"),
		card("socket-short", "Reach the socket with chips still needed", "socket", "chips"),
		card("exit-wrong", "Exit is visible from the start but inaccessible", "exit", "chips"),
		card("tele-unexpected", "Teleport somewhere unexpected", "teleport", "movement"),
		card("wrong-ice-path", "Take the wrong path through an ice section", "ice", "movement"),
		card("invisible", "Bump into an invisible wall", "wall", "secrets"),
		card("symmetry", "Visually symmetrical level", "map", "level"),
		card("no-chips", "No chips required", "chip", "level"),
		card("predict-right", "Streamer predicts what happens next and gets it right", "hint", "streamer"),
		card("predict-wrong", "Streamer predicts what happens next and gets it wrong", "hint", "streamer"),
		card("blame-game", "Streamer blames the game", "hint", "streamer"),
		card("teaches-mechanic", "A hint explains the level's gimmick", "hint", "rules"),
		card("one-screen", "One-screen level", "map", "level"),
	],
	uncommon: [
		card("restart", "Restart when the level was already solved", "exit", "restart"),
		card("chip-button", "Chip holds a button for something else", "button", "buttons"),
		card("brown-button", "Free a trapped monster with a brown button", "trap", "buttons"),
		card("blue-button", "Reverse the tanks at least three times", "tank", "buttons"),
		card("lead-monster", "Redirect a monster into a trap or hazard", "monster", "monsters"),
		card("precise-timing", "Slip through a gap between two moving monsters", "timer", "movement"),
		card("mini-rooms", "Level is a series of mini challenge rooms", "map", "level"),
		card("artwork", "Level layout forms a recognizable picture", "map", "level"),
		card("bug", "Die to a bug", "monster", "death"),
		card("fireball", "Die to a fireball", "fire", "death"),
		card("ball", "Die to a pink ball", "bomb", "death"),
		card("tank", "Die to a tank", "tank", "death"),
		card("teeth", "Die to Teeth", "monster", "death"),
		card("glider", "Die to a glider", "water", "death"),
		card("walker", "Die to a walker", "monster", "death"),
		card("blob", "Die to a blob", "monster", "death"),
		card("paramecium", "Die to a paramecium", "monster", "death"),
		card("bombed", "Step on a bomb", "bomb", "death"),
		card("drown", "Drown", "water", "death"),
		card("burn", "Burn in fire", "fire", "death"),
		card("ice-offscreen-death", "Slide on ice into an off-screen hazard and die", "ice", "death"),
		card("force-offscreen-death", "Ride force floors into an off-screen hazard and die", "map", "death"),
		card("misinput-death", "Die because of an accidental misinput", "heart", "death"),
		card("same-mistake", "Make the exact same mistake twice", "exit", "restart"),
		card("realize", "Realize the solution right after restarting", "hint", "restart"),
		card("itemswapper", "Itemswapper: one item leads to the next", "key", "items"),
		card("wrong-lock", "Use a key on the wrong lock", "lock", "items"),
		card("monster-button", "Use a monster to hold a button", "button", "monsters"),
		card("clone", "Clone two different monster types", "monster", "monsters"),
		card("trap-regret", "Release a trap and immediately regret it", "trap", "monsters"),
		card("monster-help", "A monster clears a bomb for Chip", "monster", "monsters"),
		card("monster-ruin", "A monster blocks the route to the exit", "monster", "monsters"),
		card("monster-needed", "Kill a monster that was needed later", "monster", "monsters"),
		card("teeth-chase", "Lead Teeth on a chase around an obstacle", "monster", "monsters"),
		card("teeth-dodge", "Slip past Teeth by timing their half-speed movement", "timer", "movement"),
		card("crowd-dodge", "Dodge through a crowded monster section", "monster", "movement"),
		card("ten-minutes", "Level takes 10+ minutes", "timer", "level"),
		card("no-idea", "Streamer says “I don't know why that worked”", "hint", "streamer"),
		card("lookup-solution", "Streamer looks up the solution", "hint", "streamer"),
		card("ignore-chat", "Streamer ignores chat's correct solution", "hint", "chat"),
		card("blame-rules", "Streamer blames the ruleset", "hint", "rules"),
		card("designer-hates", "Streamer says the level designer hates them", "bomb", "streamer"),
		card("elegant", "Streamer calls the puzzle elegant", "heart", "streamer"),
		card("counting", "Streamer counts tiles, moves, or monsters out loud", "chip", "streamer"),
		card("item-early", "Take an item too early", "boots", "items"),
		card("lock-regret", "Open a lock you wish you'd left closed", "lock", "items"),
		card("optional", "Dismiss an item as optional, then return for it", "key", "items"),
		card("hidden-item", "Find a hidden item", "key", "secrets"),
		card("block-slide", "Send a block sliding across ice or force floors", "block", "blocks"),
		card("block-flick", "Push a block off a tile Chip cannot enter", "block", "blocks"),
		card("block-haul", "Push blocks one by one along the same increasingly long route", "block", "blocks"),
		card("toggle-stranded", "Toggle walls strand Chip on the wrong side", "toggle", "buttons"),
		card("random-retry", "Restart because a random element went badly", "timer", "restart"),
		card("five-attempts", "Call a level easy, then need 5+ attempts", "timer", "restart"),
		card("bad-push", "Restart because of one bad block push", "block", "restart"),
		card("tele-cycle", "Take an accidental extra lap through a teleport loop", "teleport", "movement"),
		card("chat-death", "Streamer tries chat's idea and dies", "heart", "chat"),
		card("cook", "Actually cook the level and restart", "lock", "restart"),
	],
	rare: [
		card("green-button", "A monster presses a green button for Chip", "toggle", "buttons"),
		card("stuck-trap", "Get permanently stuck in a bear trap", "trap", "restart"),
		card("force-softlock", "Get softlocked on force floors", "map", "restart"),
		card("not-cooked", "Think the level is cooked, then salvage it", "lock", "restart"),
		card("hot-block", "Reveal fire under a hot block", "fire", "blocks"),
		card("self-crush", "Get crushed by a block after pushing it yourself", "block", "death"),
		card("miss-chip", "A chip is hidden under something other than a block", "chip", "chips"),
		card("writing", "Level spells a word in tiles", "map", "level"),
		card("huge-chips", "100+ chips required", "chip", "level"),
		card("troll", "Punished for being greedy", "bomb", "community"),
		card("memory", "Streamer writes down a route", "map", "streamer"),
		card("walls-of", "Streamer names the source of a Walls Of level", "map", "streamer"),
		card("skip-boots", "Skip visible boots; cross their hazard another way", "boots", "solve"),
		card("hidden-monster", "Reveal a monster hidden under a block or wall", "monster", "secrets"),
		card("ten-second-death", "Die within 10 seconds of starting", "timer", "death"),
		card("timeout", "Run out of time", "timer", "death"),
		card("boots-hazard", "Lose boots, then walk into that hazard", "thief", "items"),
		card("partial-post", "Use partial posting to reroute a teleport", "hint", "rules"),
		card("button-chain", "One monster presses three different buttons", "button", "monsters"),
		card("fake-exit", "Discover a fake exit", "exit", "community"),
		card("bust", "Use a bust that skips an intended section", "map", "community"),
		card("first-try", "Say “This looks hard,” then finish first try", "exit", "finish"),
		card("under-ten", "Finish with under 10 seconds left", "timer", "finish"),
		card("ruleset", "Level plays differently under Lynx and MS rules", "toggle", "rules"),
		card("never-seen", "Streamer says “I've never seen that before”", "hint", "streamer"),
		card("impossible-clear", "Claim the level is impossible then clear it on the same attempt", "exit", "streamer"),
		card("full-inventory", "Hold all four boots and all four key colors at once", "key", "items"),
	],
	epic: [
		card("one-move", "Die one tile from completing the level", "exit", "death"),
		card("one-second", "Finish with exactly 1 second left", "timer", "finish"),
		card("five-streak", "Complete five levels without dying", "heart", "finish"),
		card("all-boots", "Use all four boot types in one level", "boots", "items"),
		card("clone-army", "Clone at least 20 monsters", "monster", "monsters"),
		card("all-monsters", "All nine monster types appear in one level", "monster", "level"),
		card("block-tour", "One block travels over ice, force floors, and a teleporter", "block", "blocks"),
		card("hundred-clean", "Collect 100+ chips and finish without dying", "chip", "finish"),
		card("tele-occupied", "An occupied teleporter changes the destination", "teleport", "movement"),
		card("ice-loop", "Get stuck in an ice loop", "ice", "movement"),
	],
};

function card(id, text, icon, group) {
	return { id, text, icon, group };
}

function hashSeed(value) {
	let hash = 2166136261;
	for (let i = 0; i < value.length; i += 1) {
		hash ^= value.charCodeAt(i);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function mulberry32(seed) {
	return function random() {
		let value = seed += 0x6d2b79f5;
		value = Math.imul(value ^ value >>> 15, value | 1);
		value ^= value + Math.imul(value ^ value >>> 7, value | 61);
		return ((value ^ value >>> 14) >>> 0) / 4294967296;
	};
}

function shuffled(items, seed) {
	const output = items.slice();
	const random = mulberry32(hashSeed(seed));
	for (let i = output.length - 1; i > 0; i -= 1) {
		const j = Math.floor(random() * (i + 1));
		[output[i], output[j]] = [output[j], output[i]];
	}
	return output;
}

function normalizeSeed(value) {
	const seed = String(value || "").trim().toUpperCase();
	return /^[A-Z0-9_-]{3,48}$/.test(seed) ? seed : "";
}

function randomSeed(prefix = "CC") {
	const values = new Uint32Array(2);
	if (window.crypto && typeof window.crypto.getRandomValues === "function") {
		window.crypto.getRandomValues(values);
	} else {
		values[0] = Math.floor(Math.random() * 0xffffffff) ^ Date.now();
		values[1] = Math.floor(Math.random() * 0xffffffff);
	}
	return `${prefix}-${values[0].toString(36).toUpperCase().padStart(7, "0")}-${values[1].toString(36).toUpperCase().padStart(7, "0")}`;
}

function boardSeedFromHash() {
	const rawHash = window.location.hash.slice(1).replace(/^\?/, "");
	const params = new URLSearchParams(rawHash);
	return params.has("board") ? params.get("board") : null;
}

function storageKey(seed) {
	return `chipdx-bingo:${STORAGE_VERSION}:${seed}`;
}

function loadSession(seed) {
	try {
		const value = JSON.parse(window.localStorage.getItem(storageKey(seed)) || "null");
		if (!value || typeof value.shuffleSeed !== "string" || !Array.isArray(value.markedIds)) return null;
		const shuffleSeed = normalizeSeed(value.shuffleSeed);
		if (!shuffleSeed) return null;
		return {
			shuffleSeed,
			markedIds: [...new Set(value.markedIds.filter(id => typeof id === "string"))],
		};
	} catch (_error) {
		return null;
	}
}

async function copyText(value) {
	if (navigator.clipboard && typeof navigator.clipboard.writeText === "function") {
		try {
			await navigator.clipboard.writeText(value);
			return true;
		} catch (_error) {
			// Fall back to the selection-based API below.
		}
	}

	const copyField = document.createElement("textarea");
	copyField.value = value;
	copyField.setAttribute("readonly", "");
	copyField.style.position = "fixed";
	copyField.style.opacity = "0";

	try {
		document.body.appendChild(copyField);
		copyField.select();
		return document.execCommand("copy");
	} catch (_error) {
		return false;
	} finally {
		copyField.remove();
	}
}

window.addEventListener("hashchange", () => {
	window.location.reload();
});

window.bingoBoard = function bingoBoard() {
	return {
		ready: false,
		hasBoard: false,
		boardSeed: "",
		playerSeed: "",
		squares: [],
		markedIds: [],
		copyFeedback: "",
		seedError: "",
		copyTimer: null,

		init() {
			const rawSeed = boardSeedFromHash();
			if (rawSeed === null || rawSeed === "") {
				this.ready = true;
				return;
			}

			const normalized = normalizeSeed(rawSeed);
			if (!normalized) {
				this.seedError = "That invite seed is invalid. Generate a fresh board to keep playing.";
				this.ready = true;
				return;
			}

			this.boardSeed = normalized;
			const session = loadSession(normalized);
			this.playerSeed = session ? session.shuffleSeed : randomSeed("PLAYER");
			this.markedIds = session ? session.markedIds : [];
			this.buildSquares();
			this.hasBoard = true;
			this.ready = true;
			this.saveSession();
		},

		get inviteUrl() {
			const url = new URL(window.location.href);
			url.hash = `?board=${encodeURIComponent(this.boardSeed)}`;
			return url.href;
		},

		get isBingo() {
			return LINES.some(line => line.every(position => this.isMarked(this.squares[position])));
		},

		buildSquares() {
			const selected = [];
			const groupCounts = new Map();
			for (const rarity of PICK_ORDER) {
				const count = RARITY_COUNTS[rarity];
				const candidates = shuffled(CARD_POOL[rarity], `${this.boardSeed}:pick:${rarity}`);
				const picks = [];
				const overflow = [];
				for (const candidate of candidates) {
					if ((groupCounts.get(candidate.group) || 0) < 2 && picks.length < count) {
						picks.push(candidate);
						groupCounts.set(candidate.group, (groupCounts.get(candidate.group) || 0) + 1);
					} else {
						overflow.push(candidate);
					}
				}
				while (picks.length < count && overflow.length > 0) picks.push(overflow.shift());
				selected.push(...picks.map(item => ({ ...item, rarity, free: false })));
			}
			const ordered = shuffled(selected, `${this.boardSeed}:order:${this.playerSeed}`);
			ordered.splice(12, 0, { id: "free", text: "CHIP!", icon: "free", rarity: "free", free: true });
			this.squares = ordered;
			const validIds = new Set(selected.map(item => item.id));
			this.markedIds = this.markedIds.filter(id => validIds.has(id));
		},

		isMarked(square) {
			return Boolean(square && (square.free || this.markedIds.includes(square.id)));
		},

		squareClasses(square) {
			return [
				`rarity-${square.rarity}`,
				!square.free && this.isMarked(square) ? "marked" : "",
				square.free ? "free" : "",
			].filter(Boolean).join(" ");
		},

		toggleSquare(square) {
			if (square.free) return;
			this.markedIds = this.markedIds.includes(square.id)
				? this.markedIds.filter(id => id !== square.id)
				: [...this.markedIds, square.id];
			this.saveSession();
		},

		generateBoard() {
			window.location.hash = `?board=${encodeURIComponent(randomSeed())}`;
		},

		async copyInvite() {
			this.copyFeedback = await copyText(this.inviteUrl) ? "Link copied!" : "Copy failed";
			window.clearTimeout(this.copyTimer);
			this.copyTimer = window.setTimeout(() => { this.copyFeedback = ""; }, 1800);
		},

		clearProgress() {
			if (this.markedIds.length === 0) return;
			if (!window.confirm("Clear every mark on this card? Your layout will stay the same.")) return;
			this.markedIds = [];
			this.saveSession();
		},

		newPlayerCard() {
			if (!window.confirm("Reshuffle this card and clear all marks?")) return;
			this.playerSeed = randomSeed("PLAYER");
			this.markedIds = [];
			this.buildSquares();
			this.saveSession();
		},

		saveSession() {
			if (!this.boardSeed) return;
			try {
				window.localStorage.setItem(storageKey(this.boardSeed), JSON.stringify({
					shuffleSeed: this.playerSeed,
					markedIds: this.markedIds,
				}));
			} catch (_error) {
				// Private browsing or locked-down storage: the card still works for this tab.
			}
		},

		destroy() {
			window.clearTimeout(this.copyTimer);
		},
	};
};
