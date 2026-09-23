use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::core::constants::CONFIG_FILE;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_loop")]
    pub loop_cycle: LoopCycle,
    #[serde(default = "on")]
    pub presence: Toggle,
    #[serde(default = "off")]
    pub welcome_dm: Toggle,
    #[serde(default = "on")]
    pub daily_gift: Toggle,
    #[serde(default = "default_eggs")]
    pub eggs: EggsCfg,
    #[serde(default = "default_dragons")]
    pub dragons: DragonsCfg,
    #[serde(default = "default_island")]
    pub island: IslandCfg,
    #[serde(default = "default_villagers")]
    pub villagers: VillagersCfg,
    #[serde(default = "default_arena")]
    pub arena: ArenaCfg,
    #[serde(default = "default_expeditions")]
    pub expeditions: ExpeditionsCfg,
    #[serde(default = "default_raids")]
    pub raids: RaidsCfg,
    #[serde(default = "on")]
    pub hunts: Toggle,
    #[serde(default = "on")]
    pub camps: Toggle,
    #[serde(default = "on")]
    pub monsters: Toggle,
    #[serde(default = "on")]
    pub areas: Toggle,
    #[serde(default = "default_gear")]
    pub gear: GearCfg,
    #[serde(default = "on")]
    pub quests: Toggle,
    #[serde(default = "default_achievements")]
    pub achievements: AchievementsCfg,
    #[serde(default = "default_event")]
    pub event: EventCfg,
    #[serde(default = "on")]
    pub pass: Toggle,
    #[serde(default = "on")]
    pub referrals: Toggle,
    #[serde(default = "default_tavern")]
    pub tavern: TavernCfg,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Toggle {
    #[serde(default = "yes")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoopCycle {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_sleep")]
    pub sleep_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EggsCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub merge: bool,
    #[serde(default = "yes")]
    pub incubate: bool,
    #[serde(default = "yes")]
    pub open_free: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DragonsCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub level_up: bool,
    #[serde(default = "yes")]
    pub abilities: bool,
    #[serde(default = "yes")]
    pub rarity_up: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IslandCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub build: bool,
    #[serde(default = "yes")]
    pub upgrade: bool,
    #[serde(default = "yes")]
    pub collect: bool,
    #[serde(default = "yes")]
    pub assign_auto: bool,
    #[serde(default = "yes")]
    pub complete_jobs: bool,
    #[serde(default = "yes")]
    pub den_collect: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VillagersCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub frag_upgrade: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub reroll: bool,
    #[serde(default = "default_rerolls")]
    pub max_rerolls: u32,
    #[serde(default)]
    pub unranked: bool,
    #[serde(default = "default_max_fights")]
    pub max_fights_per_cycle: u32,
    #[serde(default = "default_ratio")]
    pub min_power_ratio: f64,
    #[serde(default = "default_losses")]
    pub stop_after_losses: u32,
    #[serde(default = "default_probe")]
    pub probe_after_holds: u32,
    #[serde(default = "yes")]
    pub farm_unranked: bool,
    #[serde(default = "default_gold_floor")]
    pub gold_floor: f64,
    #[serde(default)]
    pub min_trophy_ev: f64,
    #[serde(default = "default_sim_runs")]
    pub sim_runs: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpeditionsCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_exp_type")]
    pub resource: String,
    #[serde(default = "default_exp_hours")]
    pub hours: u32,
    #[serde(default = "yes")]
    pub attack: bool,
    #[serde(default = "default_edge")]
    pub min_power_ratio: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaidsCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub search: bool,
    #[serde(default = "yes")]
    pub chest: bool,
    #[serde(default = "yes")]
    pub defense: bool,
    #[serde(default = "default_edge")]
    pub min_power_ratio: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GearCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub craft: bool,
    #[serde(default = "yes")]
    pub equip: bool,
    #[serde(default = "yes")]
    pub level_up: bool,
    #[serde(default = "yes")]
    pub fuse: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AchievementsCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub claim_free: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TavernCfg {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_tav_kind")]
    pub kind: String,
    #[serde(default = "default_tav_rolls")]
    pub max_rolls_per_cycle: u32,
}

fn yes() -> bool {
    true
}

fn on() -> Toggle {
    Toggle { enabled: true }
}

fn off() -> Toggle {
    Toggle { enabled: false }
}

fn default_sleep() -> u64 {
    3600
}

fn default_rerolls() -> u32 {
    12
}

fn default_max_fights() -> u32 {
    10
}

fn default_ratio() -> f64 {
    1.0
}

fn default_losses() -> u32 {
    1
}

fn default_gold_floor() -> f64 {
    250.0
}

fn default_sim_runs() -> u32 {
    160
}

fn default_edge() -> f64 {
    1.15
}

fn default_probe() -> u32 {
    8
}

fn default_exp_type() -> String {
    "meat".to_string()
}

fn default_exp_hours() -> u32 {
    6
}

fn default_tav_kind() -> String {
    "d".to_string()
}

fn default_tav_rolls() -> u32 {
    5
}

fn default_loop() -> LoopCycle {
    LoopCycle {
        enabled: true,
        sleep_seconds: default_sleep(),
    }
}

fn default_eggs() -> EggsCfg {
    EggsCfg {
        enabled: true,
        merge: true,
        incubate: true,
        open_free: true,
    }
}

fn default_dragons() -> DragonsCfg {
    DragonsCfg {
        enabled: true,
        level_up: true,
        abilities: true,
        rarity_up: true,
    }
}

fn default_island() -> IslandCfg {
    IslandCfg {
        enabled: true,
        build: true,
        upgrade: true,
        collect: true,
        assign_auto: true,
        complete_jobs: true,
        den_collect: true,
    }
}

fn default_villagers() -> VillagersCfg {
    VillagersCfg {
        enabled: true,
        frag_upgrade: true,
    }
}

fn default_arena() -> ArenaCfg {
    ArenaCfg {
        enabled: true,
        reroll: true,
        max_rerolls: default_rerolls(),
        unranked: false,
        max_fights_per_cycle: default_max_fights(),
        min_power_ratio: default_ratio(),
        stop_after_losses: default_losses(),
        probe_after_holds: default_probe(),
        farm_unranked: true,
        gold_floor: default_gold_floor(),
        min_trophy_ev: 0.0,
        sim_runs: default_sim_runs(),
    }
}

fn default_expeditions() -> ExpeditionsCfg {
    ExpeditionsCfg {
        enabled: true,
        resource: default_exp_type(),
        hours: default_exp_hours(),
        attack: true,
        min_power_ratio: default_edge(),
    }
}

fn default_raids() -> RaidsCfg {
    RaidsCfg {
        enabled: true,
        search: true,
        chest: true,
        defense: true,
        min_power_ratio: default_edge(),
    }
}

fn default_gear() -> GearCfg {
    GearCfg {
        enabled: true,
        craft: true,
        equip: true,
        level_up: true,
        fuse: true,
    }
}

fn default_achievements() -> AchievementsCfg {
    AchievementsCfg { enabled: true }
}

fn default_event() -> EventCfg {
    EventCfg {
        enabled: true,
        claim_free: true,
    }
}

fn default_tavern() -> TavernCfg {
    TavernCfg {
        enabled: true,
        kind: default_tav_kind(),
        max_rolls_per_cycle: default_tav_rolls(),
    }
}

impl Config {
    pub fn sleep_seconds(&self) -> u64 {
        self.loop_cycle.sleep_seconds.max(30)
    }
}

pub fn load_config() -> Result<Config> {
    let path = config_path();

    if !path.exists() {
        let sample = sample_config();
        fs::write(&path, serde_json::to_string_pretty(&sample)?)
            .context("Config file could not be written")?;
        anyhow::bail!("Config file has been created, please edit and rerun");
    }

    let raw = fs::read_to_string(&path).context("Config file could not be read")?;
    let cfg: Config = serde_json::from_str(&raw).context("Config file format is not valid")?;
    Ok(cfg)
}

pub fn config_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push(CONFIG_FILE);
    if !p.exists() {
        let local = PathBuf::from(CONFIG_FILE);
        if local.exists() {
            return local;
        }
    }
    p
}

fn sample_config() -> serde_json::Value {
    serde_json::json!({
        "loop_cycle": { "enabled": true, "sleep_seconds": 3600 },
        "presence": { "enabled": true },
        "welcome_dm": { "enabled": false },
        "daily_gift": { "enabled": true },
        "eggs": { "enabled": true, "merge": true, "incubate": true, "open_free": true },
        "dragons": { "enabled": true, "level_up": true, "abilities": true, "rarity_up": true },
        "island": { "enabled": true, "build": true, "upgrade": true, "collect": true, "assign_auto": true, "complete_jobs": true, "den_collect": true },
        "villagers": { "enabled": true, "frag_upgrade": true },
        "arena": { "enabled": true, "reroll": true, "max_rerolls": 12, "unranked": false, "max_fights_per_cycle": 10, "min_power_ratio": 1.0, "stop_after_losses": 1, "probe_after_holds": 8, "farm_unranked": true, "gold_floor": 250, "min_trophy_ev": 0, "sim_runs": 160 },
        "expeditions": { "enabled": true, "resource": "meat", "hours": 6, "attack": true, "min_power_ratio": 1.15 },
        "raids": { "enabled": true, "search": true, "chest": true, "defense": true, "min_power_ratio": 1.15 },
        "hunts": { "enabled": true },
        "camps": { "enabled": true },
        "monsters": { "enabled": true },
        "areas": { "enabled": true },
        "gear": { "enabled": true, "craft": true, "equip": true, "level_up": true, "fuse": true },
        "quests": { "enabled": true },
        "achievements": { "enabled": true },
        "event": { "enabled": true, "claim_free": true },
        "pass": { "enabled": true },
        "referrals": { "enabled": true },
        "tavern": { "enabled": true, "kind": "d", "max_rolls_per_cycle": 5 }
    })
}
