Chip's Challenge DX
===================

Modern browser-playable _Chip's Challenge_ remaster with touch, gamepad, mobile, and WebAssembly support. Inspired by the classic *Chip’s Challenge* released for Windows. Built with Rust, WebAssembly, and WebGL.

Features
--------

- Instant browser play with no installation required
- Designed for keyboard, touch, and gamepad controls
- Mobile-friendly interface and responsive controls
- Smooth movement and responsive gameplay
- Retro aesthetic: 2D graphics displayed as 3D models and sprites
- Includes decades of the best community-created level packs
- Built-in level editor for creating and modifying levels
- Supports additional levelsets in the game's native .DAT level format

https://github.com/user-attachments/assets/19000227-1337-484a-8a81-2710e5f87b1f

Play instantly in your browser on desktop or mobile:

<p align="center">
	<a href="https://github.com/CasualX/chipdx/releases/latest"><img src="docs/download-button-left.svg" alt="Download the latest Chip DX release for Windows" width="220" /></a><a href="https://casualhacks.net/chipdx/index.html"><img src="docs/play-button-right.svg" alt="Play Chip DX instantly in your browser on desktop or mobile" width="220" /></a>
</p>

Getting Started
---------------

### [Play in Your Browser](https://casualhacks.net/chipdx/index.html)

1. Play instantly in your browser with no installation required.
2. Supports keyboard, touch, and gamepad controls on desktop and mobile devices.

### Download and Play

Precompiled binaries for Windows are available on the [Releases](https://github.com/CasualX/chipdx/releases) page.

1. Download and extract the latest release.
2. Run `chipplay.exe` to start the game.
3. Select a levelset from the menu to begin playing.
4. Place additional levelset files in the `levelsets` directory to play them.
5. Create your own levels with `chipedit.exe`.

### Build from Source

Other platforms must be built from source code.

1. Install [Rust](https://www.rust-lang.org/tools/install) and clone this repository.
2. Run the game: `cargo run --release --bin chipplay`
3. Level editor: `cargo run --release --bin chipedit`

### Publish from Source

1. Run `cargo run --release -p publish -- target/publish` from the repository root.
2. Find the packaged release files in `target/publish`.

Controls
--------

Playing with a Gamepad is recommended for the best experience, but keyboard controls are also supported.

| Action | Keyboard | Gamepad |
| --- | --- | --- |
| Navigate menus / Move Chip | Arrow keys / WASD | D-pad / Left stick |
| Confirm / Interact | Space | A |
| Back / Close menu | Backspace | B |
| Pause menu | Enter | Start |
| Scouting mode | Shift | Select |
| Toggle music | M | - |
| Toggle fullscreen | F | - |
| Exit fullscreen | Esc | - |

Installing Additional Levelsets
-------------------------------

Visit [https://sets.bitbusters.club/](https://sets.bitbusters.club/) to download additional levelsets.

Place the downloaded `.dat` files in the _levelsets_ folder.

Note that some levelsets are made for specific engines and may not be fully compatible.

Level Editor
------------

The game includes a level editor that can be used to create and play custom levels.

### Editor Mode

| Action | Keyboard |
| --- | --- |
| Load level | F2 |
| Save level | F5 |
| Edit level title | F6 |
| Edit chips required | F7 |
| Edit time limit | F8 |
| Edit hint | F9 |
| Edit author name | F10 |
| Pan camera | Arrow keys |
| Select Terrain Tool | T |
| Select Entity Tool | E |
| Select Connection Tool | C |
| Select Ice Path Tool | I |
| Select Force Path Tool | R |
| Undo | U |
| Redo | Y |
| Toggle Play ↔ Edit | Enter |
| Toggle music | M |
| Toggle fullscreen | F |
| Exit fullscreen | Esc |

#### Terrain Tool

| Action | Keyboard |
| --- | --- |
| Place terrain | Left click |
| Sample terrain | Right click |

#### Entity Tool

| Action | Keyboard |
| --- | --- |
| Select entity | Left click |
| Place entity | Left click |
| Rotate entity | Right click |
| Remove entity | Delete |

#### Connection Tool

| Action | Keyboard |
| --- | --- |
| Place connection | Left click |

### Play Mode

| Action | Keyboard | Gamepad |
| --- | --- | --- |
| Move Chip | Arrow keys | D-pad / Left stick |
| Toggle Play ↔ Edit | Enter | Start |

Community Resources
-------------------

* [Bit Busters Wiki](https://wiki.bitbusters.club/Main_Page)
* [Community levelsets](https://sets.bitbusters.club/)
* [Andrew E.'s Chip's Challenge Page](https://tilde.town/~magical/chip/)
* [pieguy's site](https://davidstolp.com/old/chips/)

License & Credits
-----------------

### Chip's Challenge

*Chip's Challenge* was created by Chuck Sommerville and is owned by Niffler Ltd.  
This project is an independent, open-source remaster inspired by the original game and is not affiliated with or endorsed by Niffler Ltd.

### Source Code

This remaster is released under the GNU General Public License v3.0.  
See the [license](license.md) file for more information.  

### Graphics

Default tileset: *Kayu's Enhanced Interface*

- Sourced from [https://tilde.town/~magical/chip/#kayu](https://tilde.town/~magical/chip/#kayu)

Animated tileset: The Architect's brother created an animated version of this tileset for Tile World.

- Sourced from [Animated MSCC Tileset](https://bitbusters.club/yahoo/Architecture/Animated%20MSCC%20Tileset/)

Additional graphics: *tileworld*

- Sourced from [retrofw/tileworld](https://github.com/retrofw/tileworld)

### Music

Music files written by chaozz of gp32x.com and am-fm's music is used under Creative Commons License.

* Sourced from [retrofw/tileworld](https://github.com/retrofw/tileworld)

### Sound Effects

The sound effects included in this distribution were created by Brian Raiter, with assistance from SoX. Brian Raiter has explicitly placed these files in the public domain.

- Sourced from [SicklySilverMoon/tworld](https://github.com/SicklySilverMoon/tworld)

### Community Level Packs

This remaster is distributed with several community-created level packs.

These packs were designed and maintained by the Chip’s Challenge community and are distributed freely for use with compatible engines such as Tile World.

* [Chip's Challenge Level Pack 1](https://wiki.bitbusters.club/Chip%27s_Challenge_Level_Pack_1)
* [Chip's Challenge Level Pack 2](https://wiki.bitbusters.club/Chip%27s_Challenge_Level_Pack_2)
* [Chip's Challenge Level Pack 3](https://wiki.bitbusters.club/Chip%27s_Challenge_Level_Pack_3)
* [Chip's Challenge Level Pack 4](https://wiki.bitbusters.club/Chip%27s_Challenge_Level_Pack_4)
* [Chip's Challenge Level Pack 5](https://wiki.bitbusters.club/Chip%27s_Challenge_Level_Pack_5)
