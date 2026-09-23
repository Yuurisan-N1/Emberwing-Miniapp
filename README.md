<div align="center">

<img width="100%" alt="header" src="https://capsule-render.vercel.app/api?type=waving&height=210&text=Emberwing%20Bot&fontAlign=50&fontAlignY=36&fontSize=56&desc=Dragons%20%7C%20Arena%20%7C%20Island%20%7C%20Raids%20%7C%20Expeditions%20%7C%20Full%20Automation&descAlign=50&descAlignY=58"/>

<img alt="typing" src="https://readme-typing-svg.demolab.com?font=Inter&size=18&duration=3000&pause=650&center=true&vCenter=true&width=900&lines=Auto+Dragon+Level+Up+%2F+Ability+%2F+Rarity+Up;Auto+Arena+%7C+Sim-Based+Fight+Selection;Auto+Expeditions+%7C+Raids+%7C+Hunts+%7C+Camps;Auto+Island+%7C+Build+%2F+Upgrade+%2F+Collect;Auto+Eggs+%7C+Merge+%2F+Incubate+%2F+Open;Auto+Gear+%7C+Craft+%2F+Equip+%2F+Level+%2F+Fuse"/>

<p>
  <img alt="rust" src="https://img.shields.io/badge/Rust-2021-f74c00?logo=rust&logoColor=white"/>
  <img alt="platform" src="https://img.shields.io/badge/Platform-Emberwing%20Miniapp-111111"/>
  <img alt="author" src="https://img.shields.io/badge/by-Yuurisandesu-111111"/>
</p>

<p>
  <b>Emberwing Bot</b> is a full automation bot for the Emberwing dragon game Telegram Miniapp.<br/>
  It handles a comprehensive cycle of 20 feature modules: daily gift, referrals, quests, achievements, festival events, season pass, island management, villagers, hatchery, dragon management, gear forge, arena combat with simulation-based team selection, arena gold farming, expeditions, raids, hunts, camps, monster battles, area clears, and tavern rolls, all running in a single account loop with presence heartbeat, proxy support, and a live countdown between cycles.<br/>
  Built and distributed by <b>Yuurisandesu</b>.
</p>

</div>

---

## Table of Contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the Bot](#running-the-bot)
- [Download Prebuilt Binary](#download-prebuilt-binary)
- [Features](#features)
- [File Structure](#file-structure)
- [Disclaimer](#disclaimer)

---

## Requirements

- Rust `1.70+` (includes `cargo`) -- only needed if building from source
- Python `3.10+` -- only needed to run the downloader script
- Git

---

## Installation

### Install Rust

**Linux / macOS:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Windows:**

Download and run the installer from https://rustup.rs, then restart your terminal.

**Termux (Android):**

```bash
pkg update && pkg install proot-distro
proot-distro install ubuntu
proot-distro login ubuntu
```

Then inside Ubuntu:

```bash
apt update && apt install -y curl git build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Clone the Repository

```bash
git clone https://github.com/Yuurisan-N1/Emberwing-Miniapp.git
cd Emberwing-Miniapp
```

---

## Configuration

### 1. Account (data.txt)

Fill `data.txt` with your Telegram WebApp `initData`. This bot runs a single account -- only the first line is used:

```
user=%7B%22id%22...&hash=abc123
```

> `initData` can be obtained from the browser DevTools when opening Emberwing on Telegram Web.

### 2. Proxy (proxy.txt)

Fill `proxy.txt` with one proxy (optional, leave empty to run without proxy):

```
host:port
host:port:user:pass
http://user:pass@host:port
socks5://user:pass@host:port
```

### 3. Bot Settings (config.json)

`config.json` is **auto-generated on first run** with all defaults filled in. Every feature module has its own section with an `enabled` toggle and per-action flags. Set `loop_cycle.enabled` to `true` to keep the bot running indefinitely between cycles. Key settings per module:

| Module | Notable Config |
|---|---|
| `loop_cycle` | `enabled`, `sleep_seconds` |
| `presence` | `enabled` -- heartbeat ping to keep session alive |
| `arena` | `max_fights_per_cycle`, `max_rerolls`, `min_power_ratio`, `stop_after_losses`, `gold_floor`, `sim_runs`, `farm_unranked` |
| `expeditions` | `resource` (meat/logs/etc), `hours`, `attack`, `min_power_ratio` |
| `raids` | `search`, `chest`, `defense`, `min_power_ratio` |
| `tavern` | `kind` (d/s/etc), `max_rolls_per_cycle` |
| `dragons` | `level_up`, `abilities`, `rarity_up` |
| `eggs` | `merge`, `incubate`, `open_free` |
| `island` | `build`, `upgrade`, `collect`, `assign_auto`, `complete_jobs`, `den_collect` |
| `gear` | `craft`, `equip`, `level_up`, `fuse` |

---

## Running the Bot

### Using run.sh (Linux / macOS / Termux)

`run.sh` builds and runs in one command. Make it executable first:

```bash
chmod +x run.sh
```

Then choose a mode:

```bash
./run.sh direct    # build and run in foreground (default)
./run.sh nohup     # build and run in background, logs saved to emberwing-bot.log
./run.sh screen    # build and run in a detached screen session
./run.sh tmux      # build and run in a detached tmux session
./run.sh logs      # tail the log file
./run.sh stop      # stop the running bot process
```

Attach to a background session anytime:

```bash
# screen
screen -r emberwing-bot

# tmux
tmux attach -t emberwing-bot
```

### Using make (Linux / macOS)

```bash
make release   # optimized release build
make start     # build release and run immediately
make run       # build debug and run
make check     # check for errors without building
make fmt       # format the code
make clean     # remove all build artifacts
make size      # show release binary size
```

### Manual (all platforms)

```bash
cargo build --release
```

Then run:

```bash
# Linux / macOS / Termux
./target/release/emberwing-bot

# Windows
.\target\release\emberwing-bot.exe
```

---

## Download Prebuilt Binary

If you do not want to build from source, prebuilt binaries are available two ways.

### Option 1 - Downloader Script

```bash
pip install requests colorama
python bot.py
```

The script shows a numbered menu with all available platforms. Enter the number for your platform and the binary downloads with a live progress bar, set to executable automatically on Linux and Android.

### Option 2 - Manual Download

Download the latest binaries from the Actions page:
https://github.com/Yuurisan-N1/Emberwing-Miniapp/actions/workflows/build.yml

Open the latest successful run and scroll to the Artifacts section. All binaries are retained for 90 days per build.

| Artifact | Platform |
|---|---|
| `emberwing-bot-linux-x86_64` | Linux x86_64 |
| `emberwing-bot-linux-aarch64` | Linux ARM64 |
| `emberwing-bot-linux-armv7` | Linux ARMv7 |
| `emberwing-bot-windows-x86_64` | Windows x86_64 |
| `emberwing-bot-macos-aarch64` | macOS Apple Silicon |
| `emberwing-bot-android-aarch64` | Android ARM64 (Termux) |
| `emberwing-bot-android-armv7` | Android ARMv7 (Termux) |

**Linux / macOS / Android after downloading:**

```bash
chmod +x emberwing-bot-linux-x86_64
./emberwing-bot-linux-x86_64
```

**Windows:**

```bash
.\emberwing-bot-windows-x86_64.exe
```

---

## Features

### Daily Gift
The bot checks whether the daily gift can be claimed. If available, it claims the reward and logs the day number and reward. If already claimed, the time until next reset is logged.

### Referrals
The bot claims all pending referral bonuses and logs the amount received.

### Quests
The bot collects all completable main quests, side quests, and quest chain steps and claims them in sequence.

### Achievements
The bot claims all newly unlocked achievement rewards.

### Festival and Events
The bot claims all available free festival and top-up event rewards for the current event season.

### Season Pass
The bot claims all unlocked free season pass rewards.

### Island Management
The bot collects all ready building outputs, completes all finished jobs, auto-assigns idle villagers, builds available buildings, upgrades existing ones, collects dragon den output, and applies any available speedups.

### Villagers
The bot upgrades villager fragments when enough are available.

### Hatchery
The bot merges egg pairs of the same tier and element up to the maximum tier cap, incubates unmounted eggs, and opens any free-ready eggs.

### Dragons
The bot levels up all dragons that have enough XP, activates available abilities, and upgrades rarity when fragment requirements are met.

### Gear Forge
The bot crafts all available gear recipes, equips the strongest available gear to each dragon, levels up equipped gear, fuses duplicate gear pieces, salvages and dismantles excess items.

### Arena Farm
If the gold balance drops below the configured `gold_floor` or the team is outmatched by the unranked pool, the bot fights unranked opponents to recover gold before the main ranked session begins.

### Arena Combat
The bot selects the optimal 3-dragon team using a built-in combat simulator that runs configurable `sim_runs` Monte Carlo simulations per opponent. It rerolls opponents up to `max_rerolls` times to find a winnable matchup above `min_power_ratio`. The session stops after `stop_after_losses` consecutive losses or after `max_fights_per_cycle` fights. Expected trophy value is tracked and the bot avoids fights with negative trophy expectation when configured.

### Expeditions
The bot collects all completed expedition returns, starts new expeditions in available slots for the configured resource type and duration, and optionally attacks enemy expeditions above the `min_power_ratio` threshold.

### Raids
The bot opens the daily raid chest if the win requirement is met, collects all completed defense rewards, and searches for new raid targets above the `min_power_ratio` threshold.

### Hunts, Camps, Monsters, and Areas
The bot runs all available hunt boss fights, clears active camps, fights available monsters, and progresses through unlocked map areas.

### Tavern
The bot spends tickets on tavern rolls up to `max_rolls_per_cycle` per cycle. The `kind` setting controls which ticket type is spent. Remaining pity progress is logged when no tickets are available.

### Presence
A background heartbeat task pings the server at a regular interval to keep the session alive while the bot is running between steps.

### Single Account
This bot runs one account per instance. Only the first line of `data.txt` is used. If the file contains extra lines they are noted in the log. To run multiple accounts, run multiple instances in separate folders each with their own `data.txt`.

### Proxy Support
A single proxy is loaded from `proxy.txt`. If network errors accumulate during a cycle, the bot rotates to the next proxy in the pool automatically. Running without proxy is fully supported.

### Loop Cycle
Set `loop_cycle.enabled` to `true` in `config.json` to keep the bot running indefinitely. After each cycle completes, a live `HH:MM:SS` countdown shows the wait before the next one. Set it to `false` for a single-run mode that exits after one cycle.

---

## File Structure

```text
Emberwing-Miniapp/
├── .github/
│   └── workflows/
│       └── build.yml                    # CI: 7-target matrix build, artifacts retained 90 days
├── src/
│   ├── main.rs                          # Entry point, cycle loop, step dispatch
│   ├── core/
│   │   ├── config/                      # Config loading and structs
│   │   ├── network/                     # HTTP client, proxy pool
│   │   ├── state/                       # Snapshot, state model
│   │   └── ui/                          # Banner, logger
│   └── features/
│       ├── auth/                        # Session, account loading, presence, referrals
│       ├── combat/
│       │   ├── arena/                   # Arena logic, sim, policy, farm, econ, memory
│       │   ├── expeditions/             # Expedition collect and start
│       │   ├── hunts/                   # Hunt, camps, monsters, areas
│       │   └── raids/                   # Raid chest, defense, search
│       ├── entities/
│       │   ├── dragons/                 # Dragon level, ability, rarity
│       │   ├── eggs/                    # Egg merge, incubate, open
│       │   ├── gear/                    # Forge craft, equip, level, fuse
│       │   ├── island/                  # Build, upgrade, collect, jobs, den
│       │   ├── tavern/                  # Tavern rolls
│       │   └── villagers/              # Villager fragment upgrade
│       ├── progression/
│       │   ├── achievements/            # Achievement claim
│       │   ├── daily_gift/              # Daily gift claim
│       │   ├── pass/                    # Season pass rewards
│       │   └── quests/                  # Quest and chain claim
│       └── system/
│           ├── event/                   # Festival and top-up event
│           └── notices/                 # Server notices
├── assets/
│   ├── icon.ico                         # Windows binary icon
│   └── sim_model.json                   # Arena combat simulation model weights
├── lab/
│   ├── collect.py                       # Data collection script for sim model
│   ├── fit.py                           # Model fitting script
│   └── sim.py                           # Simulation development script
├── Cargo.toml                           # Project manifest and dependencies
├── Cargo.lock
├── build.rs                             # Build script (embeds icon into Windows binary)
├── Makefile                             # make targets: build, run, release, start, clean, size
├── run.sh                               # Run helper: direct, nohup, screen, tmux, logs, stop
├── bot.py                               # Interactive downloader script (7 platforms)
├── config.json                          # Auto-generated on first run
├── data.txt                             # Single account initData
└── proxy.txt                            # Proxy (optional)
```

---

## Disclaimer

This tool is built for educational and technical exploration purposes. Use it wisely and at your own responsibility.

---

<div align="center">
<img width="100%" alt="footer" src="https://capsule-render.vercel.app/api?type=waving&height=120&section=footer"/>
</div>