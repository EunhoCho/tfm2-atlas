use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use game_core::ChampionTier;
use mod_api::*;

const MOD_ID: &str = "tfm2_atlas_client_055";
const STABLE_BRIDGE_ADDR: &str = "127.0.0.1:28452";
const SYNC_ACTIVE_CHAMPIONS_COMMAND: &str = "SYNC_ACTIVE_CHAMPIONS";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TierSyncState {
    team_id: usize,
    tiers: Vec<(String, String)>,
}

static TIER_SYNC_STATE: OnceLock<Mutex<Option<TierSyncState>>> = OnceLock::new();
static TIER_SYNC_APPLIED_COUNT: AtomicUsize = AtomicUsize::new(0);
static TIER_SYNC_DIRTY: AtomicBool = AtomicBool::new(false);

fn tier_sync_state() -> &'static Mutex<Option<TierSyncState>> {
    TIER_SYNC_STATE.get_or_init(|| Mutex::new(None))
}

fn active_catalog_outbox() -> &'static Mutex<Option<String>> {
    ACTIVE_CATALOG_OUTBOX.get_or_init(|| Mutex::new(None))
}

fn tier_status_outbox() -> &'static Mutex<Option<String>> {
    TIER_STATUS_OUTBOX.get_or_init(|| Mutex::new(None))
}

fn game_positions_from_indices<I>(positions: I) -> Vec<String>
where
    I: IntoIterator<Item = usize>,
{
    let mut labels = Vec::new();
    for position in positions {
        let label = match position {
            0 => "Top",
            1 => "Jungle",
            2 => "Mid",
            3 => "Bottom",
            4 => "Support",
            _ => continue,
        };
        if !labels.iter().any(|existing| existing == label) {
            labels.push(label.to_owned());
        }
        if labels.len() == 2 {
            break;
        }
    }
    labels
}

fn active_catalog_payload(
    mut champion_ids: Vec<String>,
    positions: &HashMap<String, Vec<String>>,
) -> Option<String> {
    champion_ids.sort();
    champion_ids.dedup();
    if champion_ids.is_empty() {
        return None;
    }
    Some(
        champion_ids
            .iter()
            .map(|champion_id| {
                format!(
                    "{},{}",
                    hex_encode(champion_id),
                    positions
                        .get(champion_id)
                        .map(|positions| positions.join("+"))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join(";"),
    )
}

fn queue_active_catalog_sync(scene: &mut Scene) -> bool {
    let Scene::InGame { data } = scene else {
        return false;
    };
    let db = data.db();
    let champion_ids = db
        .available_champions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if champion_ids.is_empty() {
        return false;
    }
    let positions = db
        .champion_positions
        .iter()
        .filter_map(|(champion_id, positions)| {
            let positions = game_positions_from_indices(positions.iter().copied());
            (!positions.is_empty()).then(|| (champion_id.to_string(), positions))
        })
        .collect::<HashMap<_, _>>();
    let Some(payload) = active_catalog_payload(champion_ids, &positions) else {
        return false;
    };
    if let Ok(mut outbox) = active_catalog_outbox().lock() {
        *outbox = Some(payload);
        true
    } else {
        false
    }
}

fn active_catalog_sync_acknowledged(response: &str) -> bool {
    response.trim_end_matches(['\r', '\n']) == "OK|SYNC_ACTIVE_CHAMPIONS"
}

fn deliver_active_catalog_sync(payload: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect(STABLE_BRIDGE_ADDR) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    if writeln!(stream, "{SYNC_ACTIVE_CHAMPIONS_COMMAND}|{payload}").is_err()
        || stream.flush().is_err()
    {
        return false;
    }
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).is_ok()
        && active_catalog_sync_acknowledged(&response)
}

fn sync_active_catalog_worker() {
    loop {
        let payload = active_catalog_outbox()
            .lock()
            .ok()
            .and_then(|mut outbox| outbox.take());
        if let Some(payload) = payload {
            let delivered = deliver_active_catalog_sync(&payload);
            if !delivered {
                if let Ok(mut outbox) = active_catalog_outbox().lock() {
                    if outbox.is_none() {
                        *outbox = Some(payload);
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn sync_tier_status_worker() {
    loop {
        let command = tier_status_outbox()
            .lock()
            .ok()
            .and_then(|mut outbox| outbox.take());
        if let Some(command) = command {
            let delivered =
                TcpStream::connect(STABLE_BRIDGE_ADDR)
                    .ok()
                    .is_some_and(|mut stream| {
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                        writeln!(stream, "{command}").is_ok() && stream.flush().is_ok()
                    });
            if !delivered {
                if let Ok(mut outbox) = tier_status_outbox().lock() {
                    if outbox.is_none() {
                        *outbox = Some(command);
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn hex_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(encoded: &str) -> Result<String, &'static str> {
    if encoded.len() % 2 != 0 {
        return Err("INVALID_CHAMPION_ENCODING");
    }
    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let high = hex_value(pair[0]).ok_or("INVALID_CHAMPION_ENCODING")?;
        let low = hex_value(pair[1]).ok_or("INVALID_CHAMPION_ENCODING")?;
        bytes.push((high << 4) | low);
    }
    String::from_utf8(bytes).map_err(|_| "INVALID_CHAMPION_ENCODING")
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_tier_sync_response(response: &str) -> Result<Option<TierSyncState>, &'static str> {
    let mut parts = response.trim().split('|');
    if parts.next() != Some("OK") || parts.next() != Some("TIER_SYNC") {
        return Err("INVALID_TIER_SYNC_RESPONSE");
    }
    let team_id = parts.next().unwrap_or_default();
    let payload = parts.next().unwrap_or_default();
    if team_id.is_empty() {
        return Ok(None);
    }
    let team_id = team_id
        .parse::<usize>()
        .map_err(|_| "INVALID_TIER_SYNC_TEAM")?;
    let mut tiers = Vec::new();
    for entry in payload.split(';').filter(|entry| !entry.is_empty()) {
        let (champion_id, tier) = entry.split_once(':').ok_or("INVALID_TIER_SYNC_ENTRY")?;
        if !matches!(tier, "S" | "A" | "B" | "C" | "D" | "NoTier") {
            return Err("INVALID_TIER_SYNC_TIER");
        }
        tiers.push((hex_decode(champion_id)?, tier.to_owned()));
    }
    Ok(Some(TierSyncState { team_id, tiers }))
}

fn champion_tier(value: &str) -> Option<ChampionTier> {
    match value {
        "S" => Some(ChampionTier::S),
        "A" => Some(ChampionTier::A),
        "B" => Some(ChampionTier::B),
        "C" => Some(ChampionTier::C),
        "D" => Some(ChampionTier::D),
        "NoTier" => Some(ChampionTier::NoTier),
        _ => None,
    }
}

fn apply_tier_sync(scene: &mut Scene) {
    if !TIER_SYNC_DIRTY.swap(false, Ordering::AcqRel) {
        return;
    }
    let sync = tier_sync_state()
        .lock()
        .ok()
        .and_then(|state| state.clone());
    let (Scene::InGame { data }, Some(sync)) = (scene, sync) else {
        TIER_SYNC_APPLIED_COUNT.store(0, Ordering::Release);
        TIER_SYNC_DIRTY.store(true, Ordering::Release);
        return;
    };
    let team_id = sync.team_id;
    let expected = sync.tiers.len();
    let mut db = data.db_mut();
    let Some(team) = db.teams.get_mut(&sync.team_id) else {
        TIER_SYNC_APPLIED_COUNT.store(0, Ordering::Release);
        TIER_SYNC_DIRTY.store(true, Ordering::Release);
        if let Ok(mut outbox) = tier_status_outbox().lock() {
            *outbox = Some(format!("SYNC_TIER_STATUS|{team_id}|0|{expected}"));
        }
        return;
    };
    let mut applied = 0;
    for (champion_id, tier) in sync.tiers {
        if let Some(tier) = champion_tier(&tier) {
            team.champion_tiers.insert(champion_id, tier);
            applied += 1;
        }
    }
    TIER_SYNC_APPLIED_COUNT.store(applied, Ordering::Release);
    if let Ok(mut outbox) = tier_status_outbox().lock() {
        *outbox = Some(format!("SYNC_TIER_STATUS|{team_id}|{applied}|{expected}"));
    }
}

fn poll_tier_sync() {
    loop {
        let next = (|| -> Result<Option<TierSyncState>, ()> {
            let mut stream = TcpStream::connect(STABLE_BRIDGE_ADDR).map_err(|_| ())?;
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            stream.write_all(b"GET_TIER_SYNC\n").map_err(|_| ())?;
            let mut response = String::new();
            BufReader::new(stream)
                .read_line(&mut response)
                .map_err(|_| ())?;
            parse_tier_sync_response(&response).map_err(|_| ())
        })()
        .unwrap_or(None);
        if let Ok(mut state) = tier_sync_state().lock() {
            if *state != next {
                *state = next;
                TIER_SYNC_DIRTY.store(true, Ordering::Release);
            }
        }
        thread::sleep(Duration::from_millis(750));
    }
}

static SERVER_STARTED: AtomicBool = AtomicBool::new(false);
static ACTIVE_CATALOG_REFRESH_REQUIRED: AtomicBool = AtomicBool::new(true);
static ACTIVE_CATALOG_LAST_TEAM: AtomicUsize = AtomicUsize::new(usize::MAX);
static ACTIVE_CATALOG_OUTBOX: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static TIER_STATUS_OUTBOX: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn start_companion_workers() {
    if SERVER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    thread::spawn(poll_tier_sync);
    thread::spawn(sync_active_catalog_worker);
    thread::spawn(sync_tier_status_worker);
}

struct ModifierBridgeClient {
    catalog_sync_elapsed: Mutex<f32>,
}

impl Default for ModifierBridgeClient {
    fn default() -> Self {
        Self {
            catalog_sync_elapsed: Mutex::new(0.25),
        }
    }
}

impl ModExtension for ModifierBridgeClient {
    fn post_update(&self, scene: &mut Scene, _ui: &mut GameUI, _assets: &mut Assets, dt: f32) {
        if let Scene::InGame { data } = scene {
            let team_id = data.player_team_id();
            let previous = ACTIVE_CATALOG_LAST_TEAM.swap(team_id, Ordering::AcqRel);
            if previous != team_id {
                ACTIVE_CATALOG_REFRESH_REQUIRED.store(true, Ordering::Release);
            }
        }
        let should_attempt_catalog_sync = ACTIVE_CATALOG_REFRESH_REQUIRED.load(Ordering::Acquire)
            && self
                .catalog_sync_elapsed
                .lock()
                .map(|mut elapsed| {
                    *elapsed += dt.max(0.0);
                    if *elapsed >= 0.25 {
                        *elapsed = 0.0;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
        if should_attempt_catalog_sync && queue_active_catalog_sync(scene) {
            ACTIVE_CATALOG_REFRESH_REQUIRED.store(false, Ordering::Release);
        }
        apply_tier_sync(scene);
    }
}

#[cfg(test)]
mod name_payload_tests {
    use super::*;

    #[test]
    fn tier_sync_parser_supports_mod_champions_and_disabled_profiles() {
        let parsed = parse_tier_sync_response(
            "OK|TIER_SYNC|7|777974686d5f63617373696f70656961:S;62617365:NoTier\n",
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.team_id, 7);
        assert_eq!(
            parsed.tiers,
            vec![
                ("wythm_cassiopeia".to_owned(), "S".to_owned()),
                ("base".to_owned(), "NoTier".to_owned()),
            ]
        );
        assert_eq!(parse_tier_sync_response("OK|TIER_SYNC||\n").unwrap(), None);
    }

    #[test]
    fn champion_ids_round_trip_through_the_wire_encoding() {
        let champion_id = "wythm_cassiopeia_한글";
        assert_eq!(hex_decode(&hex_encode(champion_id)).unwrap(), champion_id);
        assert!(hex_decode("0xz1").is_err());
    }

    #[test]
    fn active_catalog_waits_for_a_non_empty_career_database() {
        let positions = HashMap::new();
        assert_eq!(active_catalog_payload(Vec::new(), &positions), None);

        let mut positions = HashMap::new();
        positions.insert(
            "wythm_cassiopeia".to_owned(),
            vec!["Top".to_owned(), "Mid".to_owned()],
        );
        assert_eq!(
            active_catalog_payload(
                vec!["wythm_cassiopeia".to_owned(), "wythm_cassiopeia".to_owned(),],
                &positions,
            ),
            Some("777974686d5f63617373696f70656961,Top+Mid".to_owned())
        );
    }

    #[test]
    fn active_catalog_delivery_requires_the_stable_bridge_ack() {
        assert!(active_catalog_sync_acknowledged(
            "OK|SYNC_ACTIVE_CHAMPIONS\n"
        ));
        assert!(!active_catalog_sync_acknowledged(""));
        assert!(!active_catalog_sync_acknowledged("ERR|INVALID_PAYLOAD\n"));
        assert!(!active_catalog_sync_acknowledged("OK|OTHER_COMMAND\n"));
    }

}

fn init(_ctx: &GameCtx) -> ModRegistration {
    start_companion_workers();

    let mut reg = ModRegistration::new(MOD_ID);
    reg.set_extension(ModifierBridgeClient::default());
    reg
}

declare_mod!(init);
