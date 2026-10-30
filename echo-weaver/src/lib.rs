// Copyright 2026 The AutoSplitters Project Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_std]

extern crate alloc;

use alloc::format;
use asr::{
    Address, Process,
    deep_pointer::DeepPointer,
    future::next_tick,
    game_engine::unity::{DictionaryOffsets, mono::Module},
    settings::Gui,
    timer,
    watcher::Watcher,
};
use futures::join;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

asr::async_main!(stable);
asr::panic_handler!();

/// Auto Splitter settings shown in the UI.
#[derive(Gui)]
struct Settings {
    // Loop behavior
    loop_behavior: LoopBehavior,

    /// Split on Elymere Pumphouse completion
    pumphouse: bool,

    /// Split on mended major rifts
    mended_rift: bool,

    /// Split on Loom activation
    loom: bool,
}

#[derive(Gui)]
enum LoopBehavior {
    /// None
    #[default]
    None,
    /// Split
    ///
    /// Split each time the loop is restarted.
    Split,
    /// Reset
    ///
    /// Reset the timer each time the loop is restarted.
    Reset,
}

/// Tracked game runtime information.
struct GameState<'a> {
    process: &'a Process,
    module: Module,
    /// The size of a pointer (usually `Bit64`).
    pointer_size: asr::PointerSize,

    /// Pointer to `PlayerData.instance.loopCount`.
    loop_count_pointer: DeepPointer<2>,
    /// Pointer to `PlayerData.instance.totalCollectedHealthPieces`.
    remnant_count_pointer: DeepPointer<2>,
    /// Pointer to `PlayerData.instance.totalCollectedHealStrengthPieces`.
    vestige_count_pointer: DeepPointer<2>,
    /// Pointer to `PlayerData.instance.totalCollectedTimePieces`.
    dewdrop_count_pointer: DeepPointer<2>,
    /// Pointer to `PlayerData.instance.loopCount`.
    loop_started_pointer: DeepPointer<1>,
    /// Pointer to `PlayerData.instance.keyItems`.
    key_items_pointer: DeepPointer<2>,
    /// Dictionary offsets for `PlayerData.instance.keyItems` (delayed init).
    key_items_offsets: Option<DictionaryOffsets>,
    /// Pointer to `PlayerData.instance.leads`.
    leads_pointer: DeepPointer<2>,
    /// Dictionary offsets for `PlayerData.instance.leads` (delayed init).
    leads_offsets: Option<DictionaryOffsets>,
    /// Pointer to `GameManager.Instance.currentPowerDestination`.
    power_destination_pointer: DeepPointer<2>,
    /// Pointer to `GameManager.InFinalGauntlet.`
    in_gauntlet_pointer: DeepPointer<1>,
    /// Pointer to `GameManager.Instance.endOfGameEchoRift.active`.
    game_complete_pointer: DeepPointer<3>,
    /// Pointer to `GameManager.Instance.RunData.blackboard`.
    run_data_pointer: DeepPointer<3>,
    /// Dictionary offsets for `GameManager.Instance.RunData.blackboard` (delayed init).
    run_data_offsets: Option<DictionaryOffsets>,
    /// Pointer to `GameManager.Player.GlaiveWarping`.
    glaive_warping_pointer: DeepPointer<2>,
    /// Offset of `Value` in ``geode.BlackboardVariable`1``.
    variable_value_offset: u32,

    loop_count: Watcher<i32>,
    remnant_count: Watcher<i32>,
    vestige_count: Watcher<i32>,
    dewdrop_count: Watcher<i32>,
    enth_count: Watcher<i32>,
    leads_count: Watcher<i32>,
    loop_started: Watcher<bool>,
    power_destination: Watcher<i32>,
    rifts_mended: Watcher<i32>,
    in_gauntlet: Watcher<bool>,
    game_complete: Watcher<bool>,
    glaive_warping: Watcher<bool>,
}

/// Subset of leads counted in the UI, taken from `space-game.LeadThreadInfo`.
#[derive(derive_more::TryFrom)]
#[cfg_attr(feature = "debug", derive(derive_more::Debug))]
#[try_from(repr)]
#[repr(i32)]
enum Leads {
    // GoldenPath > GoldenPathVivary
    GPVivaryManualLinkage = 0x8C,
    GPLinkOnlyAfterMending = 0x9E,
    // GoldenPath > Pumphouse
    GPLoomNeedsPower = 0x8B,
    // GoldenPath > Rifts
    GPAnomalousRiftsBreadcrumb = 0x8D,
    GPMendRiftsNavene = 0x8E,
    GPMendRiftsTwins = 0x9D,
    // GoldenPath > ParadiseDoor
    GPParadiseDoorBreadcrumb = 0x8F,
    GPTakeEnergyToDoor = 0x90,
    // SaveEveryone > VosShortcut
    VosShortcut = 0xB6,
    // SaveEveryone > NovaShortcut
    BugQuizAnswer0 = 0xBC,
    BugQuizAnswer1 = 0xBD,
    BugQuizAnswer2 = 0xBE,
    BugQuizAnswer3 = 0xBF,
    BugQuizAnswer4 = 0xC0,
    BugQuizAnswer5 = 0xC1,
    BugQuizAnswer6 = 0xC2,
    BugQuizAnswer7 = 0xC3,
    BugQuizAnswer8 = 0xC4,
    BugQuizAnswer9 = 0xC5,
    BugQuizAnswer10 = 0xC6,
    // SaveEveryone > TetsaShortcut
    TetsaShortcut = 0xB7,
    // SaveEveryone > VerrikShortcut
    VerrikShortcut = 0xAC,
    // SaveEveryone > TheHistorian
    TakeSpindleToEnd = 0xB8,
    // SaveEveryone > IvoLinkingRoom
    IvoLinkingRoomDone = 0xC7,
    // SaveEveryone > TownRecluseShortcut
    SaveTownRecluse = 0xCF,
    // EscapeTheLoom > BulkheadDoor
    ClosingDoorBreadcrumb = 0x04,
    ClosingDoorTimeCode = 0x05,
    ClosingDoorShortcut = 0x06,
    // EscapeTheLoom > LoomElevator
    LoomElevator = 0x07,
    LoomElevatorPasscode = 0x08,
    // EscapeTheLoom > GlaiveTraining
    GlaiveFound = 0x1A,
    GlaiveRecallLearned = 0x0A,
    GlaiveProjectShutDown = 0x77,
    // EscapeTheLoom > SystemsOffice
    GlaiveWarpBreadcrumb = 0x0B,
    GlaiveWarpTaught = 0x0C,
    GlaiveWarpLearned = 0x0D,
    // FixPower > Pumphouse
    LakePumphouse = 0x36,
    PumphouseSwitches = 0x54,
    PumphouseRotation1 = 0x55,
    PumphouseRotation2 = 0x56,
    PumphouseRotation3 = 0x57,
    PumphouseRotation4 = 0x5F,
    PumphouseDone = 0x58,
    // FixPower > Reservoir
    ReservoirBreadcrumb = 0x79,
    GrappleTaught = 0x72,
    GrappleLearned = 0x73,
    PumpHouseManualControl = 0x26,
    // LakeTowers > LakeTowerWest
    LakeTowerWestKickoff = 0x78,
    // LakeTowers > LakeTowerCenter
    LakeWestTowerNote = 0x2B,
    // LakeTowers > LakeTowerEast
    LakeCenterTowerNote = 0x2A,
    // LakeTowers > LakeTowerSpillways
    LakeEastTowerNote = 0x2C,
    LakeTowersConclusion = 0x2E,
    // ExploreChurch > ChurchCourtyard
    CourtyardGateLocked = 0x41,
    VosCourtyardKey = 0x42,
    ChurchCourtyardAccess = 0x51,
    // ExploreChurch > ChurchMain
    ChurchIntro = 0x59,
    ChurchRift2 = 0x86,
    // ExploreChurch > PrisonBlock
    PrisonCell2 = 0x5A,
    WeaverLinkingTaught1 = 0x1F,
    // ExploreChurch > NavenesCell
    WeaverLinkingTaught2 = 0x5B,
    WeaverLinkingTaught3 = 0x5C,
    WeaverLinkingLearned = 0x20,
    // SolveSky > SkyAnchorNode
    SkyAnchorBreadcrumb = 0x69,
    OverclockTaught1 = 0x52,
    OverclockTaught2 = 0xA0,
    SkyAnchorDone = 0x7C,
    // SolveSky > SkyLab
    VosFindTetsa = 0x3D,
    TetsaExperimentPower = 0x32,
    TetsaPowerBeacon1 = 0x33,
    TetsaPowerBeacon2 = 0x34,
    TetsaPowerBeacon3 = 0x35,
    // SolveSky > QuietGarden
    TreeJump = 0x9F,
    // ExploreSpillways > SpillwaysAnchorNode
    SpillwaysAnchorBreadcrumb = 0x6A,
    SpillwaysAnchorNavene = 0x7F,
    GrappleToGlaive = 0x82,
    TwinSightTaught = 0x83,
    SpillwaysAnchorDone = 0x7D,
    // ExploreSpillways > SpillwaysAccess
    SpillwaysElevatorBreadcrumb = 0x5D,
    UnderlakeElevatorUpCode = 0x2F,
    UnderlakeElevatorBreadcrumb = 0x80,
    HighClearanceSpillways = 0x9C,
    UnderlakeElevatorDownCode = 0x30,
    // MarrowAnchorNode > MarrowAnchorNode
    MarrowAnchorBreadcrumb = 0x6B,
    UnbindTaught1 = 0x7A,
    UnbindTaught2 = 0xAE,
    MarrowAnchorDone = 0x7E,
    // MarrowAnchorNode > Paternost
    PaternostAutoMove = 0x75,
    PaternostReveal = 0x84,
    PaternostControl = 0x76,
    // MarrowAnchorNode > ControlRoom
    HighClearanceControlRoom = 0x9B,
    ControlRoomResolution = 0xAD,
    // TopOfSky > TowerOfInitiating
    LearnPath1 = 0xAF,
    LearnPath2 = 0xB0,
    LearnPath3 = 0xB1,
    // TopOfSky > TowerOfProving
    TetsaTippyTopNudge = 0xB2,
    RiftImpossible = 0xB3,
    PostUnbindingNudgeToTippy = 0xB4,
    TippyTopWrapup = 0xB5,
    // SpillwaysRedRoom > SpillwaysRedOffice
    UnlitTorch = 0x87,
    LightingTorches = 0x88,
    RedOfficeInside1 = 0x89,
    // Vos > VosErrands
    VosChecklist = 0x70,
    VosErrandLake = 0x71,
    VosErrand1 = 0x6C,
    VosErrand2 = 0x6D,
    VosErrand3 = 0x6E,
    VosComplete = 0x6F,
    VosClosure = 0xD5,
    // Nova > NovaChurch
    NovaCourtyardBreadcrumb = 0x45,
    NovaUpperChurchBreadcrumb = 0x63,
    NovaChurchMapRoomBreadcrumb = 0x64,
    NovaCatacombBreadcrumb = 0x65,
    NovaZRoomMarrowBreadcrumb = 0x66,
    NovaQuestCompleted = 0x68,
    // Tetsa > TetsaSky
    TetsaQuestIntro = 0xA1,
    TetsaQuestWontListen = 0xA2,
    TetsaFurtherIn = 0xA3,
    TetsaShowGlaivePower = 0xA4,
    TetsaClosure = 0xD3,
    // Tetsa > BlipSpots
    TetsaBlipsIntro = 0xA5,
    TetsaBlipsLake = 0xA6,
    TetsaBlipsDepths = 0xA7,
    TetsaBlipsSpillways = 0xA8,
    TetsaBlipsControlRoom = 0xA9,
    TetsaBlipsStart = 0xAA,
    TetsaBlipsCourtyard = 0xAB,
    TetsaBlipsVivary = 0xD4,
    // Tetsa > TetsaHangar
    DeadTetsa = 0x43,
    TetsaFindTetsa = 0x3E,
    // Verrik > VerrikNaveneGrave
    VerrikNaveneGrave = 0x95,
    VerrikNaveneGraveSeen = 0x9A,
    // Verrik > VerrikMainHall
    VerrikMainHall = 0x91,
    // Verrik > VerrikPrisonBlock
    VerrikPrisonBlock = 0x92,
    VerrikPrisonWeaverTrapped = 0x96,
    VerrikPrisonEscape = 0x97,
    VerrikPrisonVerrikTrapped = 0x98,
    // Verrik > VerrikCourtyard
    VerrikCourtyard = 0x93,
    VerrikCourtyardConvoSeen = 0x99,
    // Verrik > VerrikControlRoom
    VerrikControlRoom = 0x94,
    // Zaramondus > TheMapmaker
    ZIntroduced = 0x48,
    ZSecretMaps = 0xBA,
    AllMaps = 0xBB,
    // Ivo > IvoMain
    IvoIntro = 0xC8,
    IvoFirstSeedPlanted = 0xC9,
    IvoChild = 0xCA,
    IvoSmart = 0xCB,
    IvoGrumpy = 0xCC,
    IvoLinkingRoomGoal = 0xCD,
    IvoPickForm = 0xCE,
    IvoClosure = 0xD2,
    // Recluse > TownRecluseHouse
    TownRecluseBreadcrumb = 0xD0,
    TownRecluseClosure = 0xD1,
}

impl<'a> GameState<'a> {
    /// Creates a new GameState.
    async fn create(process: &'a Process) -> Self {
        let module = Module::wait_attach_auto_detect(process).await;
        let pointer_size = module.get_pointer_size();

        // Load images.
        let (space_game_image, geode_image) = join!(
            module.wait_get_image(process, "space-game"),
            module.wait_get_image(process, "geode"),
        );
        #[cfg(feature = "debug")]
        asr::print_message("Loaded images");

        // Load classes.
        let (
            echo_rift_class,
            game_manager_class,
            player_controller_class,
            player_data_class,
            blackboard_class,
            variable_class,
        ) = join!(
            space_game_image.wait_get_class(process, &module, "EchoRift"),
            space_game_image.wait_get_class(process, &module, "GameManager"),
            space_game_image.wait_get_class(process, &module, "PlayerController"),
            space_game_image.wait_get_class(process, &module, "PlayerData"),
            geode_image.wait_get_class(process, &module, "geode.Blackboard"),
            geode_image.wait_get_class(process, &module, "geode.BlackboardVariable`1"),
        );
        #[cfg(feature = "debug")]
        asr::print_message("Loaded classes");

        // Load class static tables and offsets.
        let (
            rift_active_offset,
            game_manager_table,
            in_gauntlet_offset,
            loop_started_offset,
            game_manager_instance_offset,
            player_offset,
            run_data_offset,
            power_destination_offset,
            end_of_game_rift_offset,
            glaive_warping_offset,
            player_data_table,
            player_data_instance_offset,
            key_items_offset,
            leads_offset,
            loop_count_offset,
            remnant_count_offset,
            vestige_count_offset,
            dewdrop_count_offset,
            blackboard_offset,
            variable_value_offset,
        ) = join!(
            echo_rift_class.wait_get_field_offset(process, &module, "active"),
            game_manager_class.wait_get_static_table(process, &module),
            game_manager_class.wait_get_field_offset(process, &module, "InFinalGauntlet"),
            game_manager_class.wait_get_field_offset(process, &module, "LoopHasStarted"),
            game_manager_class.wait_get_field_offset(process, &module, "Instance"),
            game_manager_class.wait_get_field_offset(process, &module, "Player"),
            game_manager_class.wait_get_field_offset(process, &module, "RunData"),
            game_manager_class.wait_get_field_offset(process, &module, "currentPowerDestination"),
            game_manager_class.wait_get_field_offset(process, &module, "endOfGameEchoRift"),
            player_controller_class.wait_get_field_offset(process, &module, "glaiveWarping"),
            player_data_class.wait_get_static_table(process, &module),
            player_data_class.wait_get_field_offset(process, &module, "instance"),
            player_data_class.wait_get_field_offset(process, &module, "keyItems"),
            player_data_class.wait_get_field_offset(process, &module, "leads"),
            player_data_class.wait_get_field_offset(process, &module, "loopCount"),
            player_data_class.wait_get_field_offset(process, &module, "totalCollectedHealthPieces"),
            player_data_class.wait_get_field_offset(
                process,
                &module,
                "totalCollectedHealStrengthPieces"
            ),
            player_data_class.wait_get_field_offset(process, &module, "totalCollectedTimePieces"),
            blackboard_class.wait_get_field_offset(process, &module, "blackboard"),
            variable_class.wait_get_field_offset(process, &module, "Value"),
        );
        #[cfg(feature = "debug")]
        asr::print_message("Loaded static tables and offsets");

        Self {
            process,
            module,
            pointer_size,
            loop_count_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[player_data_instance_offset as u64, loop_count_offset as u64],
            ),
            remnant_count_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[
                    player_data_instance_offset as u64,
                    remnant_count_offset as u64,
                ],
            ),
            vestige_count_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[
                    player_data_instance_offset as u64,
                    vestige_count_offset as u64,
                ],
            ),
            dewdrop_count_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[
                    player_data_instance_offset as u64,
                    dewdrop_count_offset as u64,
                ],
            ),
            key_items_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[player_data_instance_offset as u64, key_items_offset as u64],
            ),
            key_items_offsets: None,
            leads_pointer: DeepPointer::new(
                player_data_table,
                pointer_size,
                &[player_data_instance_offset as u64, leads_offset as u64],
            ),
            leads_offsets: None,
            loop_started_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[loop_started_offset as u64],
            ),
            power_destination_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[
                    game_manager_instance_offset as u64,
                    power_destination_offset as u64,
                ],
            ),
            in_gauntlet_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[in_gauntlet_offset as u64],
            ),
            game_complete_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[
                    game_manager_instance_offset as u64,
                    end_of_game_rift_offset as u64,
                    rift_active_offset as u64,
                ],
            ),
            glaive_warping_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[player_offset as u64, glaive_warping_offset as u64],
            ),
            run_data_pointer: DeepPointer::new(
                game_manager_table,
                pointer_size,
                &[
                    game_manager_instance_offset as u64,
                    run_data_offset as u64,
                    blackboard_offset as u64,
                ],
            ),
            run_data_offsets: None,
            variable_value_offset,
            loop_count: Watcher::new(),
            remnant_count: Watcher::new(),
            vestige_count: Watcher::new(),
            dewdrop_count: Watcher::new(),
            enth_count: Watcher::new(),
            leads_count: Watcher::new(),
            loop_started: Watcher::new(),
            power_destination: Watcher::new(),
            rifts_mended: Watcher::new(),
            in_gauntlet: Watcher::new(),
            game_complete: Watcher::new(),
            glaive_warping: Watcher::new(),
        }
    }

    /// Returns the current number of mended major rifts.
    fn get_rifts_mended(
        process: &Process,
        module: &Module,
        pointer_size: asr::PointerSize,
        offsets: &DictionaryOffsets,
        variable_value_offset: u32,
        at: Address,
    ) -> Result<i32, asr::Error> {
        match pointer_size {
            asr::PointerSize::Bit16 => Self::get_rifts_mended_impl::<asr::Address16>(
                process,
                module,
                offsets,
                variable_value_offset,
                at,
            ),
            asr::PointerSize::Bit32 => Self::get_rifts_mended_impl::<asr::Address32>(
                process,
                module,
                offsets,
                variable_value_offset,
                at,
            ),
            asr::PointerSize::Bit64 => Self::get_rifts_mended_impl::<asr::Address64>(
                process,
                module,
                offsets,
                variable_value_offset,
                at,
            ),
        }
    }

    /// Internal implementation of `get_rifts_mended` handling different `Address` types.
    fn get_rifts_mended_impl<AddressType: bytemuck::CheckedBitPattern + Into<Address>>(
        process: &Process,
        module: &Module,
        offsets: &DictionaryOffsets,
        variable_value_offset: u32,
        at: Address,
    ) -> Result<i32, asr::Error> {
        const RIFTS_MENDED: &[u16] = &[
            'e' as u16, 'c' as u16, 'h' as u16, 'o' as u16, 'R' as u16, 'i' as u16, 'f' as u16,
            't' as u16, 's' as u16, 'M' as u16, 'e' as u16, 'n' as u16, 'd' as u16, 'e' as u16,
            'd' as u16,
        ];
        const KEY_LEN: usize = RIFTS_MENDED.len();

        let entries = module
            .read_dictionary::<AddressType, AddressType, 2048>(process, *offsets, at)
            .map_err(|e| {
                #[cfg(feature = "debug")]
                asr::print_limited::<1024>(&format_args!("read_dictionary: {e:?}"));
                e
            })?;
        for (key_addr, value_addr) in entries {
            if let Ok(key) = module.read_string_object::<KEY_LEN>(process, key_addr.into())
                && key.matches(RIFTS_MENDED)
                && let Ok(v) =
                    process.read::<i32>(Into::<Address>::into(value_addr) + variable_value_offset)
            {
                return Ok(v);
            }
        }
        // Return 0 if the entry isn't present.
        Ok(0)
    }

    /// Logs newly seen RunData keys for debugging.
    #[cfg(feature = "debug")]
    fn log_new_run_data_keys(
        process: &Process,
        module: &Module,
        pointer_size: asr::PointerSize,
        offsets: &DictionaryOffsets,
        at: Address,
    ) {
        match pointer_size {
            asr::PointerSize::Bit16 => {
                Self::log_new_run_data_keys_impl::<asr::Address16>(process, module, offsets, at)
            }
            asr::PointerSize::Bit32 => {
                Self::log_new_run_data_keys_impl::<asr::Address32>(process, module, offsets, at)
            }
            asr::PointerSize::Bit64 => {
                Self::log_new_run_data_keys_impl::<asr::Address64>(process, module, offsets, at)
            }
        }
    }

    /// Internal implementation of `getlog_new_run_data_keys_rifts_mended` handling different
    /// `Address` types.
    #[cfg(feature = "debug")]
    fn log_new_run_data_keys_impl<AddressType: bytemuck::CheckedBitPattern + Into<Address>>(
        process: &Process,
        module: &Module,
        offsets: &DictionaryOffsets,
        at: Address,
    ) {
        use spin::{LazyLock, Mutex};
        static SEEN_KEYS: LazyLock<Mutex<hashbrown::HashSet<alloc::vec::Vec<u16>>>> =
            LazyLock::new(|| Mutex::new(hashbrown::HashSet::new()));
        const MAX_KEY_LEN: usize = 1024;

        let entries = module
            .read_dictionary::<AddressType, AddressType, 2048>(process, *offsets, at)
            .map_err(|e| {
                #[cfg(feature = "debug")]
                asr::print_limited::<1024>(&format_args!("read_dictionary: {e:?}"));
                e
            });
        for (key_addr, _) in entries.unwrap_or_default() {
            if let Ok(key) = module.read_string_object::<MAX_KEY_LEN>(process, key_addr.into())
                && SEEN_KEYS.lock().insert(key.to_vec())
                && let Ok(str) = alloc::string::String::from_utf16(key.as_slice())
            {
                const MAX_MSG_LEN: usize = MAX_KEY_LEN + 10;
                asr::print_limited::<MAX_MSG_LEN>(&format_args!("New key: {str}"));
            }
        }
    }

    /// Returns the number of currently held Calcified Enth.
    fn get_enth(
        process: &Process,
        module: &Module,
        offsets: &DictionaryOffsets,
        at: Address,
    ) -> Result<i32, asr::Error> {
        let entries = module
            .read_dictionary::<i32, i32, 60>(process, *offsets, at)
            .map_err(|e| {
                #[cfg(feature = "debug")]
                asr::print_limited::<1024>(&format_args!("read_dictionary (keyItems): {e:?}"));
                e
            })?;
        Ok(entries
            .into_iter()
            .find_map(|(key, value)| match key {
                0xE => Some(value),
                _ => None,
            })
            .unwrap_or(0))
    }

    /// Returns the number of collected leads that are counted by the in-game UI.
    fn get_leads(
        process: &Process,
        module: &Module,
        offsets: &DictionaryOffsets,
        at: Address,
    ) -> Result<i32, asr::Error> {
        let entries = module
            .read_dictionary::<i32, i32, 214>(process, *offsets, at)
            .map_err(|e| {
                #[cfg(feature = "debug")]
                asr::print_limited::<1024>(&format_args!("read_dictionary (leads): {e:?}"));
                e
            })?;

        // In debug builds, log new leads.
        #[cfg(feature = "debug")]
        {
            use spin::{LazyLock, Mutex};
            static SEEN_LEADS: LazyLock<Mutex<hashbrown::HashSet<i32>>> =
                LazyLock::new(|| Mutex::new(hashbrown::HashSet::new()));

            let mut guard = SEEN_LEADS.lock();
            for (key, _) in &entries {
                if guard.insert(*key) {
                    if let Ok(lead) = Leads::try_from(*key) {
                        asr::print_limited::<64>(&format_args!("New lead: {lead:?}"));
                    } else {
                        asr::print_limited::<14>(&format_args!("New lead: 0x{key:02x}"));
                    }
                }
            }
        }

        Ok(entries
            .into_iter()
            .filter(|(key, _)| Leads::try_from(*key).is_ok())
            .count() as i32)
    }

    /// Updates timer counters and splits.
    fn update(&mut self, settings: &Settings) {
        // Read the current loop count.
        if let Some(pair) = self
            .loop_count
            .update(self.loop_count_pointer.deref::<i32>(self.process).ok())
        {
            timer::set_variable_int("loop", pair.current);
            // Reset the timer when the save data is reset.
            if pair.changed_to(&1) {
                #[cfg(feature = "debug")]
                asr::print_message("Save data reset");
                timer::reset();
            }
        }

        // Start the timer (if not already started) when a loop starts.
        // Based on settings, reset or split when a loop ends.
        if let Some(pair) = self
            .loop_started
            .update(self.loop_started_pointer.deref::<bool>(self.process).ok())
        {
            if pair.changed_to(&true) {
                #[cfg(feature = "debug")]
                asr::print_message("Loop started");
                timer::start();
            } else if pair.changed_to(&false) {
                #[cfg(feature = "debug")]
                asr::print_message("Loop churned");
                match settings.loop_behavior {
                    LoopBehavior::None => {}
                    LoopBehavior::Split => timer::split(),
                    LoopBehavior::Reset => timer::reset(),
                }
            }
        }

        // Check if the end of the game has been reached.
        if let Some(pair) = self
            .game_complete
            .update(self.game_complete_pointer.deref::<bool>(self.process).ok())
            && pair.changed_to(&true)
        {
            #[cfg(feature = "debug")]
            asr::print_message("Game complete");
            timer::split();
        }

        // Pause game time while glaive warping to remove load times. As of 1.0.9, this matches how
        // the game removes load times for plinths.
        if let Some(pair) = self
            .glaive_warping
            .update(self.glaive_warping_pointer.deref::<bool>(self.process).ok())
        {
            if pair.changed_to(&true) {
                #[cfg(feature = "debug")]
                asr::print_message("Pausing game time for glaive warp");
                timer::pause_game_time();
            } else if pair.changed_to(&false) {
                #[cfg(feature = "debug")]
                asr::print_message("Resuming game time after glaive warp");
                timer::resume_game_time();
            }
        }

        // Check for power being routed to the colony (pumphouse complete).
        if let Some(pair) = self.power_destination.update(
            self.power_destination_pointer
                .deref::<i32>(self.process)
                .ok(),
        ) && pair.changed_to(&2)
            && settings.pumphouse
        {
            #[cfg(feature = "debug")]
            asr::print_message("Pumphouse complete");
            timer::split();
        }

        // Check for the final gauntlet starting (loom complete).
        if let Some(pair) = self
            .in_gauntlet
            .update(self.in_gauntlet_pointer.deref::<bool>(self.process).ok())
            && pair.changed_to(&true)
            && settings.loom
        {
            #[cfg(feature = "debug")]
            asr::print_message("Gauntlet started");
            timer::split();
        }

        // Check for major rift completion in the RunData. Since this can be expensive, we skip it
        // if the setting is disabled.
        if (settings.mended_rift || cfg!(feature = "debug"))
            && let Ok(addr) = self.run_data_pointer.deref_offsets(self.process)
            && !addr.is_null()
        {
            // Dictionary offsets are initialized on first use since the objects may be null when
            // the game is first launched.
            if self.run_data_offsets.is_none() {
                self.run_data_offsets = self.module.get_dictionary_offsets(self.process, addr);
            }
            if let Some(offsets) = &self.run_data_offsets {
                if settings.mended_rift
                    && let Some(pair) = self.rifts_mended.update(
                        Self::get_rifts_mended(
                            self.process,
                            &self.module,
                            self.pointer_size,
                            offsets,
                            self.variable_value_offset,
                            addr,
                        )
                        .ok(),
                    )
                    && pair.increased()
                    && pair.current <= 3
                // Try not to split for the Loom rift.
                {
                    #[cfg(feature = "debug")]
                    asr::print_message("Rift mended");
                    timer::split();
                }

                #[cfg(feature = "debug")]
                Self::log_new_run_data_keys(
                    self.process,
                    &self.module,
                    self.pointer_size,
                    offsets,
                    addr,
                );
            }
        }

        // Update misc variables.
        let mut update_summary = false;
        if let Some(pair) = self
            .remnant_count
            .update(self.remnant_count_pointer.deref::<i32>(self.process).ok())
        {
            timer::set_variable_int("remnants", pair.current);
            update_summary = true;
        }
        if let Some(pair) = self
            .vestige_count
            .update(self.vestige_count_pointer.deref::<i32>(self.process).ok())
        {
            timer::set_variable_int("vestiges", pair.current);
            update_summary = true;
        }
        if let Some(pair) = self
            .dewdrop_count
            .update(self.dewdrop_count_pointer.deref::<i32>(self.process).ok())
        {
            timer::set_variable_int("dewdrops", pair.current);
            update_summary = true;
        }
        if let Ok(addr) = self.key_items_pointer.deref_offsets(self.process)
            && !addr.is_null()
        {
            if self.key_items_offsets.is_none() {
                self.key_items_offsets = self.module.get_dictionary_offsets(self.process, addr);
            }
            if let Some(offsets) = &self.key_items_offsets
                && let Some(pair) = self
                    .enth_count
                    .update(Self::get_enth(self.process, &self.module, offsets, addr).ok())
            {
                timer::set_variable_int("enth", pair.current);
                update_summary = true;
            }
        }
        if let Ok(addr) = self.leads_pointer.deref_offsets(self.process)
            && !addr.is_null()
        {
            if self.leads_offsets.is_none() {
                self.leads_offsets = self.module.get_dictionary_offsets(self.process, addr);
            }
            if let Some(offsets) = &self.leads_offsets
                && let Some(pair) = self
                    .leads_count
                    .update(Self::get_leads(self.process, &self.module, offsets, addr).ok())
            {
                timer::set_variable_int("leads", pair.current);
                update_summary = true;
            }
        }
        if update_summary {
            timer::set_variable(
                "collectibles",
                &format!(
                    "{}/{}/{}/{}/{}",
                    self.remnant_count.pair.map(|p| p.current).unwrap_or(0),
                    self.vestige_count.pair.map(|p| p.current).unwrap_or(0),
                    self.dewdrop_count.pair.map(|p| p.current).unwrap_or(0),
                    self.enth_count.pair.map(|p| p.current).unwrap_or(0),
                    self.leads_count.pair.map(|p| p.current).unwrap_or(0),
                ),
            );
        }
    }
}

async fn main() {
    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("Echo Weaver.exe").await;

        process
            .until_closes(async {
                let mut game_state = GameState::create(&process).await;
                #[cfg(feature = "debug")]
                asr::print_message("Initialization complete.");
                loop {
                    settings.update();
                    game_state.update(&settings);
                    next_tick().await;
                }
            })
            .await;
    }
}
