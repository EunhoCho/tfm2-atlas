//! Frozen `*V1` value types and enum codes mirroring game-core simulation
//! types. These are the ONLY value shapes that cross the DLL boundary; the
//! host converts to/from its internal types on its side.
//!
//! Freeze rule: fields and enum codes are immutable once published. New game
//! stats/variants get appended codes or `*V2` types plus new slots — existing
//! layouts never change.

/// Borrowed UTF-8 string view. No ownership transfer; valid only as long as
/// the object it was read from (per-call unless documented otherwise).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StrV1 {
    pub ptr: *const u8,
    pub len: usize,
}

impl StrV1 {
    pub const EMPTY: Self = Self {
        ptr: std::ptr::null(),
        len: 0,
    };

    pub fn from_str(s: &str) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }

    /// # Safety
    /// `ptr..ptr+len` must be live UTF-8 memory.
    pub unsafe fn as_str<'a>(&self) -> &'a str {
        if self.ptr.is_null() || self.len == 0 {
            return "";
        }
        std::str::from_utf8(std::slice::from_raw_parts(self.ptr, self.len)).unwrap_or("")
    }
}

macro_rules! stable_code_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident = $code:literal),* $(,)? }) => {
        $(#[$m])*
        #[repr(u32)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant = $code),*
        }

        impl $name {
            pub const fn code(self) -> u32 {
                self as u32
            }

            pub fn from_code(code: u32) -> Option<Self> {
                match code {
                    $($code => Some($name::$variant),)*
                    _ => None,
                }
            }

            /// (variant name, code) pairs — consumed by the contract dump.
            pub const VARIANTS: &'static [(&'static str, u32)] = &[
                $((stringify!($variant), $code)),*
            ];
        }
    };
}

stable_code_enum!(
    /// Mirror of game-core `ChampionCategory`.
    ChampionCategoryV1 {
        Melee = 0,
        Range = 1,
        Magician = 2,
        Util = 3,
        Assassin = 4,
    }
);

stable_code_enum!(
    /// Mirror of game-core `ChampionTag`.
    ChampionTagV1 {
        Ad = 0,
        Ap = 1,
        Heal = 2,
        Shield = 3,
        Dot = 4,
        Cc = 5,
        Range = 6,
        Melee = 7,
        Tank = 8,
        Magic = 9,
    }
);

stable_code_enum!(
    /// Mirror of game-core `ItemTag`.
    ItemTagV1 {
        Ad = 0,
        Ap = 1,
        AttackSpeed = 2,
        Defense = 3,
        MagicResistance = 4,
        Hp = 5,
        DefensePenetration = 6,
        Vamp = 7,
        HealReduce = 8,
        ShieldBreak = 9,
        MoveSpeed = 10,
        AttackRange = 11,
        Shield = 12,
        HpPercentDamage = 13,
        AsDebuff = 14,
        ReflectDamage = 15,
        Toughness = 16,
        MrDebuff = 17,
        RangeDamage = 18,
        MrPenetration = 19,
        CooltimeReduce = 20,
        DotDamage = 21,
        HpRegen = 22,
        MyHpPercentDamage = 23,
        ShareDamage = 24,
        Range = 25,
    }
);

stable_code_enum!(
    /// Mirror of game-core `ItemCategory`.
    ItemCategoryV1 {
        Ad = 0,
        AttackSpeed = 1,
        Defense = 2,
        MagicResistance = 3,
        Magic = 4,
        Hp = 5,
        // Support-style items (ally-facing effects); level 2.
        Support = 6,
    }
);

stable_code_enum!(
    /// Mirror of game-core `AttackType`.
    AttackTypeV1 {
        BaseAttack = 0,
        Skill = 1,
        Dot = 2,
        DotIgnoreShield = 3,
        Item = 4,
        Well = 5,
    }
);

stable_code_enum!(
    /// Mirror of game-core `DamageType`.
    DamageTypeV1 {
        Ad = 0,
        Ap = 1,
        Fixed = 2,
    }
);

stable_code_enum!(
    /// Mirror of game-core `CastingType`.
    CastingTypeV1 {
        Targeting = 0,
        Position = 1,
        Direction = 2,
        None = 3,
    }
);

stable_code_enum!(
    /// Mirror of game-core `CastingTarget`.
    CastingTargetV1 {
        Ally = 0,
        AllyChampion = 1,
        AllyChampionInCc = 2,
        AllyNotSelf = 3,
        AllyOnlySelf = 4,
        Enemy = 5,
        EnemyWithoutTower = 6,
        EnemyChampion = 7,
        EnemyChampionInCc = 8,
        EnemyChampionRecentlyAttacked = 9,
        Both = 10,
        BothWithoutTower = 11,
        BothChampion = 12,
        None = 13,
    }
);

stable_code_enum!(
    /// Mirror of game-core lane `Position`.
    LaneV1 {
        Top = 0,
        Jungle = 1,
        Mid = 2,
        Bottom = 3,
        Support = 4,
    }
);

stable_code_enum!(
    /// Mirror of game-core `CCState` discriminants.
    CcKindV1 {
        Airborne = 0,
        Stun = 1,
        Bind = 2,
        BlockAttack = 3,
        BlockSkill = 4,
        BlockMoveSkill = 5,
        ForceMove = 6,
        Taunt = 7,
        Fear = 8,
        Charm = 9,
        Animation = 10,
    }
);

stable_code_enum!(
    /// Mirror of game-core `BuffType` discriminants.
    BuffDurationV1 {
        Permanent = 0,
        Time = 1,
        WithShield = 2,
    }
);

stable_code_enum!(
    /// `InputTargetV1.kind` values.
    InputTargetKindV1 {
        None = 0,
        Target = 1,
        Dir = 2,
        Pos = 3,
    }
);

stable_code_enum!(
    /// Server-emitted client event target kinds.
    EventTargetKindV1 {
        Broadcast = 0,
        Player = 1,
        Team = 2,
    }
);

stable_code_enum!(
    /// Mirror of game-view `Scene` discriminants. Mods must ignore unknown
    /// codes (new scenes may appear at any level).
    SceneKindV1 {
        Loading = 0,
        Title = 1,
        NewGame = 2,
        DatabaseEdit = 3,
        InGame = 4,
        GameTest = 5,
        Room = 6,
        Lobby = 7,
    }
);

stable_code_enum!(
    /// UI event kinds observable through the stable API. Payload JSON shape
    /// per kind: Click/RightClick/TreeViewSelect {"item"}, CheckboxSelect
    /// {"selected"}, TreeViewRightClick {"item","x","y"}, TreeViewMove
    /// {"item_from","item_to"}, TextEditComplete {"text"}, Custom {"data"
    /// (byte array)}, Remove/Changed {}.
    UiEventKindV1 {
        Click = 0,
        RightClick = 1,
        CheckboxSelect = 2,
        TreeViewSelect = 3,
        TreeViewRightClick = 4,
        TreeViewMove = 5,
        TextEditComplete = 6,
        Remove = 7,
        Custom = 8,
        Changed = 9,
    }
);

stable_code_enum!(
    /// Management record tables addressable through the data record slots.
    /// `Match*` kinds address `MatchInfo` records per match category;
    /// `KnowledgeBase` is keyed by team id; `SoloRankMatch` is the ladder
    /// match record (distinct from `MatchSoloRank`'s `MatchInfo`).
    /// `MatchReplay` (ABI level 3) is keyed by match id and is **read-only**:
    /// `record_set_json` refuses it so mods cannot rewrite match history.
    RecordKindV1 {
        Team = 0,
        Athlete = 1,
        League = 2,
        Tournament = 3,
        Staff = 4,
        MatchNormal = 5,
        MatchPractice = 6,
        MatchTutorial = 7,
        MatchSoloRank = 8,
        LeagueCompetition = 9,
        TournamentCompetition = 10,
        SoloRankMatch = 11,
        KnowledgeBase = 12,
        YearSchedule = 13,
        Match = 14,
        MatchReplay = 15,
    }
);

stable_code_enum!(
    /// Mirror of game-core `Input` discriminants.
    InputKindV1 {
        Move = 0,
        Return = 1,
        Attack = 2,
        Skill = 3,
        Skill2 = 4,
        Ult = 5,
    }
);

stable_code_enum!(
    /// Mirror of game-core `DraftScorePhase`.
    DraftPhaseV1 {
        Ban = 0,
        Pick = 1,
    }
);

stable_code_enum!(
    /// Mirror of game-core `Difficulty`.
    DifficultyV1 {
        Easy = 0,
        Normal = 1,
        Hard = 2,
    }
);

stable_code_enum!(
    /// Result kinds of a draft score hook.
    DraftDecisionKindV1 {
        Pass = 0,
        Add = 1,
        Replace = 2,
    }
);

stable_code_enum!(
    /// Result kinds of a player-input AI hook.
    AiDecisionKindV1 {
        Pass = 0,
        Replace = 1,
    }
);

stable_code_enum!(
    /// Mirror of game-core `GameModeKind`.
    GameModeKindV1 {
        Moba = 0,
        SingleLane = 1,
        DeathMatch = 2,
    }
);

stable_code_enum!(
    /// Setting documents addressable through the JSON get/set slots.
    /// (MapSetting is offline-baked visibility/path data and is deliberately
    /// not exposed; map geometry is the map-customizer hook's domain.)
    SettingTargetV1 {
        GameSetting = 0,
        ItemSetting = 1,
    }
);

stable_code_enum!(
    /// Mirror of common `TextAlignmentX` for draw-surface text.
    TextAlignXV1 {
        Left = 0,
        Center = 1,
        Right = 2,
    }
);

stable_code_enum!(
    /// Mirror of common `TextAlignmentY` for draw-surface text.
    TextAlignYV1 {
        Top = 0,
        Center = 1,
        Bottom = 2,
    }
);

stable_code_enum!(
    /// Movement kind of a mod-spawned projectile: `Target` homes onto
    /// `target_id`; `Linear` flies toward (`target_x`, `target_y`), with
    /// `penetrate` keeping it going through hits.
    ProjectileMoveKindV1 {
        Target = 0,
        Linear = 1,
    }
);

stable_code_enum!(
    /// Management-layer event kinds (payloads are small JSON objects; see
    /// the host docs per kind). Unknown codes must be skipped by mods.
    ManagementEventKindV1 {
        MatchFinished = 0,
        TransferCompleted = 1,
        SeasonRollover = 2,
    }
);

stable_code_enum!(
    /// Raw keyboard input kinds surfaced to mods per frame (hotkeys).
    InputEventKindV1 {
        KeyPressed = 0,
        KeyReleased = 1,
    }
);

stable_code_enum!(
    /// Mirror of the client-side in-game screen (the scene INSIDE the
    /// management client — distinct from the top-level `SceneKindV1`).
    ClientSceneKindV1 {
        Main = 0,
        Lineup = 1,
        StadiumEntrance = 2,
        Match = 3,
        MatchResult = 4,
        LockerRoom = 5,
        InGame = 6,
        Prologue = 7,
        PrologueFirst = 8,
        TutorialMorgad = 9,
        TutorialSerpen = 10,
        Tutorial5v5 = 11,
    }
);

/// Attack profile of a mod-spawned unit (mirror of the internal
/// target-attack action). Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UnitAttackV1 {
    /// Percent of the unit's attack stat added to `attack` (100 = 1x).
    pub attack_ratio: usize,
    /// Flat damage added on top of the ratio part.
    pub attack: usize,
    pub range: usize,
    pub cooltime: usize,
    /// Action animation length in ticks.
    pub duration: usize,
    /// Tick offset within the action when the hit lands.
    pub start_timing: usize,
    pub cancelable: bool,
    /// [`AttackTypeV1`] code.
    pub attack_type: u32,
}

impl Default for UnitAttackV1 {
    /// A plain melee attack calibrated to real game scales (world units are
    /// 1000× screen pixels; a champion radius is 10000, a melee attack range
    /// ~12000, one second = 60 ticks).
    fn default() -> Self {
        Self {
            attack_ratio: 100,
            attack: 0,
            range: 12_000,
            cooltime: 60,
            duration: 24,
            start_timing: 12,
            cancelable: false,
            attack_type: AttackTypeV1::BaseAttack.code(),
        }
    }
}

/// Number of tag slots in [`ChampionBriefV1`] (headroom over today's 10
/// tag codes). Frozen.
pub const CHAMPION_BRIEF_TAG_CAP: usize = 12;

/// Static identity of one draft candidate. `name` borrows host memory and is
/// valid only for the duration of the callback that produced it. Frozen
/// value type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ChampionBriefV1 {
    pub name: StrV1,
    /// [`ChampionCategoryV1`] code; u32::MAX when unknown.
    pub category: u32,
    pub tag_count: u32,
    /// [`ChampionTagV1`] codes; only the first `tag_count` entries are set.
    pub tags: [u32; CHAMPION_BRIEF_TAG_CAP],
    pub stat: StatV1,
    pub growth: StatV1,
}

impl Default for ChampionBriefV1 {
    fn default() -> Self {
        Self {
            name: StrV1::EMPTY,
            category: u32::MAX,
            tag_count: 0,
            tags: [0; CHAMPION_BRIEF_TAG_CAP],
            stat: StatV1::default(),
            growth: StatV1::default(),
        }
    }
}

/// Placement/movement of a mod-spawned projectile. Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProjectileSpawnV1 {
    pub caster_id: usize,
    pub team: usize,
    pub x: u64,
    pub y: u64,
    /// Circle hit shape radius.
    pub radius: u64,
    pub speed: u64,
    /// [`ProjectileMoveKindV1`] code.
    pub move_kind: u32,
    /// Homing target for `Target` movement.
    pub target_id: usize,
    /// Destination for `Linear` movement.
    pub target_x: u64,
    pub target_y: u64,
    /// `Linear` only: keep flying through hits.
    pub penetrate: bool,
    /// [`AttackTypeV1`] code.
    pub attack_type: u32,
    /// [`CastingTypeV1`] code the effect is applied with on hit.
    pub casting_type: u32,
    /// [`CastingTargetV1`] code filtering what the projectile can hit.
    pub casting_target: u32,
}

impl Default for ProjectileSpawnV1 {
    /// Calibrated to real game scales: radius ≈ one champion (10000 world
    /// units), speed 5000/tick matches the game's own basic-attack
    /// projectiles (~300k units per second).
    fn default() -> Self {
        Self {
            caster_id: 0,
            team: 0,
            x: 0,
            y: 0,
            radius: 10_000,
            speed: 5_000,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: 0,
            target_x: 0,
            target_y: 0,
            penetrate: false,
            attack_type: AttackTypeV1::Skill.code(),
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: CastingTargetV1::Enemy.code(),
        }
    }
}

/// Mirror of game-core `Input`. `target` is meaningful for the action kinds,
/// `x`/`y` for `Move`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputV1 {
    /// [`InputKindV1`] code.
    pub kind: u32,
    pub target: InputTargetV1,
    pub x: u64,
    pub y: u64,
}

impl InputV1 {
    pub fn move_to(x: u64, y: u64) -> Self {
        Self {
            kind: InputKindV1::Move.code(),
            x,
            y,
            ..Self::default()
        }
    }

    pub fn return_home() -> Self {
        Self {
            kind: InputKindV1::Return.code(),
            ..Self::default()
        }
    }

    pub fn action(kind: InputKindV1, target: InputTargetV1) -> Self {
        Self {
            kind: kind.code(),
            target,
            ..Self::default()
        }
    }
}

stable_code_enum!(
    /// Result of a mod server command handler.
    CommandResultV1 {
        Pass = 0,
        Handled = 1,
    }
);

/// Ergonomic event target used by the mod-side wrapper; converts to
/// ([`EventTargetKindV1`] code, id) at the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StableEventTarget {
    Broadcast,
    Player(usize),
    Team(usize),
}

impl StableEventTarget {
    pub fn to_parts(self) -> (u32, usize) {
        match self {
            StableEventTarget::Broadcast => (EventTargetKindV1::Broadcast.code(), 0),
            StableEventTarget::Player(id) => (EventTargetKindV1::Player.code(), id),
            StableEventTarget::Team(id) => (EventTargetKindV1::Team.code(), id),
        }
    }
}

/// Mirror of game-core `EntityStat`. Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatV1 {
    pub attack: usize,
    pub magic_power: usize,
    pub hp: usize,
    pub defence: usize,
    pub magic_resistance: usize,
    pub move_speed: usize,
    pub hp_regen: usize,
    pub stack: usize,
    pub crit_chance: usize,
}

pub const BUFF_NAME_CAP: usize = 64;

/// Mirror of game-core `BuffState`. Frozen value type.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuffV1 {
    pub name_buf: [u8; BUFF_NAME_CAP],
    pub name_len: usize,
    /// [`BuffDurationV1`] code.
    pub duration_kind: u32,
    /// Remaining ticks; meaningful only for `Time`.
    pub duration_tick: usize,
    pub attack: i32,
    pub attack_mult: i32,
    pub magic_power: i32,
    pub magic_power_mult: i32,
    pub defence: i32,
    pub defence_mult: i32,
    pub hp: i32,
    pub hp_regen: i32,
    pub magic_resistance: i32,
    pub magic_resistance_mult: i32,
    pub vamp: i32,
    pub hp_mult: i32,
    pub move_speed_mult: i32,
    pub attack_speed_mult: i32,
    pub skill_cooldown_mult: i32,
    pub ult_cooldown_mult: i32,
    pub radius_mult: i32,
    pub crit_chance: i32,
    pub damage_reflect: usize,
    pub damaged_amplify: usize,
    pub damaged_reduce: usize,
    pub defence_penetration: usize,
    pub magic_resistance_penetration: usize,
    pub toughness: usize,
    pub heal_reduce: usize,
    pub range: usize,
    pub base_attack_enemy_max_hp_damage: usize,
    pub self_max_hp_damage: usize,
    pub skill_enemy_max_hp_damage: usize,
    pub dot_amplify: usize,
    pub base_attack_damaged_reduce: usize,
    pub skill_damaged_reduce: usize,
    pub cc_immune: bool,
    pub undying: bool,
    pub ignore_wall: bool,
}

impl Default for BuffV1 {
    fn default() -> Self {
        Self {
            name_buf: [0; BUFF_NAME_CAP],
            name_len: 0,
            duration_kind: BuffDurationV1::Permanent.code(),
            duration_tick: 0,
            attack: 0,
            attack_mult: 0,
            magic_power: 0,
            magic_power_mult: 0,
            defence: 0,
            defence_mult: 0,
            hp: 0,
            hp_regen: 0,
            magic_resistance: 0,
            magic_resistance_mult: 0,
            vamp: 0,
            hp_mult: 0,
            move_speed_mult: 0,
            attack_speed_mult: 0,
            skill_cooldown_mult: 0,
            ult_cooldown_mult: 0,
            radius_mult: 0,
            crit_chance: 0,
            damage_reflect: 0,
            damaged_amplify: 0,
            damaged_reduce: 0,
            defence_penetration: 0,
            magic_resistance_penetration: 0,
            toughness: 0,
            heal_reduce: 0,
            range: 0,
            base_attack_enemy_max_hp_damage: 0,
            self_max_hp_damage: 0,
            skill_enemy_max_hp_damage: 0,
            dot_amplify: 0,
            base_attack_damaged_reduce: 0,
            skill_damaged_reduce: 0,
            cc_immune: false,
            undying: false,
            ignore_wall: false,
        }
    }
}

impl BuffV1 {
    /// Permanent buff with a name (truncated to [`BUFF_NAME_CAP`] bytes).
    pub fn named(name: &str) -> Self {
        let mut buff = Self::default();
        buff.set_name(name);
        buff
    }

    /// Timed buff with a name.
    pub fn timed(name: &str, ticks: usize) -> Self {
        let mut buff = Self::named(name);
        buff.duration_kind = BuffDurationV1::Time.code();
        buff.duration_tick = ticks;
        buff
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let mut len = bytes.len().min(BUFF_NAME_CAP);
        // Never cut a UTF-8 sequence in half.
        while len > 0 && !name.is_char_boundary(len) {
            len -= 1;
        }
        self.name_buf[..len].copy_from_slice(&bytes[..len]);
        self.name_len = len;
    }

    pub fn name(&self) -> &str {
        let len = self.name_len.min(BUFF_NAME_CAP);
        std::str::from_utf8(&self.name_buf[..len]).unwrap_or("")
    }
}

/// Mirror of game-core `CCState`. Frozen value type. Which fields are
/// meaningful depends on `kind` ([`CcKindV1`]); the rest stay zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CcV1 {
    /// [`CcKindV1`] code.
    pub kind: u32,
    pub tick: u64,
    /// ForceMove/Fear/Charm direction.
    pub dx: i64,
    pub dy: i64,
    /// ForceMove speed.
    pub speed: u64,
    /// Taunt target entity id.
    pub target: usize,
    /// Animation name (Animation kind only).
    pub name_buf: [u8; BUFF_NAME_CAP],
    pub name_len: usize,
}

impl Default for CcV1 {
    fn default() -> Self {
        Self {
            kind: CcKindV1::Stun.code(),
            tick: 0,
            dx: 0,
            dy: 0,
            speed: 0,
            target: 0,
            name_buf: [0; BUFF_NAME_CAP],
            name_len: 0,
        }
    }
}

impl CcV1 {
    pub fn of_kind(kind: CcKindV1, tick: u64) -> Self {
        Self {
            kind: kind.code(),
            tick,
            ..Self::default()
        }
    }

    pub fn stun(tick: u64) -> Self {
        Self::of_kind(CcKindV1::Stun, tick)
    }

    pub fn name(&self) -> &str {
        let len = self.name_len.min(BUFF_NAME_CAP);
        std::str::from_utf8(&self.name_buf[..len]).unwrap_or("")
    }

    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let mut len = bytes.len().min(BUFF_NAME_CAP);
        while len > 0 && !name.is_char_boundary(len) {
            len -= 1;
        }
        self.name_buf[..len].copy_from_slice(&bytes[..len]);
        self.name_len = len;
    }
}

/// Mirror of game-core `InputTarget`. Which fields are meaningful depends on
/// `kind` ([`InputTargetKindV1`]).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputTargetV1 {
    /// [`InputTargetKindV1`] code.
    pub kind: u32,
    pub target_id: usize,
    pub dir_x: i64,
    pub dir_y: i64,
    pub x: u64,
    pub y: u64,
}

impl InputTargetV1 {
    pub const NONE: Self = Self {
        kind: 0,
        target_id: 0,
        dir_x: 0,
        dir_y: 0,
        x: 0,
        y: 0,
    };

    pub fn target(target_id: usize) -> Self {
        Self {
            kind: InputTargetKindV1::Target.code(),
            target_id,
            ..Self::NONE
        }
    }

    pub fn pos(x: u64, y: u64) -> Self {
        Self {
            kind: InputTargetKindV1::Pos.code(),
            x,
            y,
            ..Self::NONE
        }
    }

    pub fn dir(dir_x: i64, dir_y: i64) -> Self {
        Self {
            kind: InputTargetKindV1::Dir.code(),
            dir_x,
            dir_y,
            ..Self::NONE
        }
    }
}

/// Mirror of the kill-log entry exposed to mods.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KillLogV1 {
    pub tick: usize,
    pub killer_team: usize,
    /// [`LaneV1`] code of the killer.
    pub killer_position: u32,
    /// [`LaneV1`] code of the victim.
    pub killed_position: u32,
    pub assist_count: u32,
    /// [`LaneV1`] codes, first `assist_count` entries valid (max 4).
    pub assist_positions: [u32; 4],
}

/// Mirror of the projectile info exposed to mods.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProjectileInfoV1 {
    pub x: u64,
    pub y: u64,
    pub caster_id: usize,
    pub team: usize,
    pub is_end: bool,
}
