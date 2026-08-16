use std::collections::BTreeSet;

use tfm2_atlas_engine::DraftPhase;

use crate::{
    DraftRule, DraftSide, MockDraftAction, MockDraftContextUpdate, MockDraftSet, MockDraftState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockDraftError {
    InvalidSet,
    InvalidPhase,
    ChampionUnavailable,
    ChampionAlreadySelected,
    ChampionFearlessExcluded,
    DraftSlotFull,
    ChampionNotSelected,
}

impl MockDraftError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidSet => "invalid_draft_set",
            Self::InvalidPhase => "invalid_draft_phase",
            Self::ChampionUnavailable => "champion_unavailable",
            Self::ChampionAlreadySelected => "champion_already_selected",
            Self::ChampionFearlessExcluded => "champion_fearless_excluded",
            Self::DraftSlotFull => "draft_slot_full",
            Self::ChampionNotSelected => "champion_not_selected",
        }
    }
}

impl std::fmt::Display for MockDraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MockDraftError {}

#[derive(Debug, Clone, Default)]
pub struct MockDraftSession {
    state: MockDraftState,
    history: Vec<MockDraftState>,
}

impl MockDraftSession {
    pub fn state(&self) -> &MockDraftState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut MockDraftState {
        &mut self.state
    }

    pub fn set_context(&mut self, update: MockDraftContextUpdate) -> Result<(), MockDraftError> {
        if let Some(current_set) = update.current_set {
            if !(1..=5).contains(&current_set) {
                return Err(MockDraftError::InvalidSet);
            }
            self.state.current_set = current_set;
        }
        if let Some(opponent_team_id) = update.opponent_team_id {
            self.state.opponent_team_id = Some(opponent_team_id);
        }
        if let Some(player_side) = update.player_side {
            self.state.sets[self.state.current_set - 1].our_side = player_side;
        }
        if let Some(rule) = update.rule {
            self.state.rule = rule;
        }
        if let Some(phase) = update.recommendation_phase {
            if phase == DraftPhase::Waiting {
                return Err(MockDraftError::InvalidPhase);
            }
            self.state.last_own_phase = phase;
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        mut action: MockDraftAction,
        active_champions: &[String],
    ) -> Result<(), MockDraftError> {
        action.champion_id = action.champion_id.trim().to_owned();
        if action.phase == DraftPhase::Waiting {
            return Err(MockDraftError::InvalidPhase);
        }
        if action.champion_id.is_empty()
            || !active_champions.iter().any(|id| id == &action.champion_id)
        {
            return Err(MockDraftError::ChampionUnavailable);
        }
        let set_index = self.state.current_set - 1;
        if selected_champions(&self.state.sets[set_index]).contains(&action.champion_id) {
            return Err(MockDraftError::ChampionAlreadySelected);
        }
        if carryover_excluded(&self.state, action.side).contains(&action.champion_id) {
            return Err(MockDraftError::ChampionFearlessExcluded);
        }
        let list = slot_mut(&mut self.state.sets[set_index], action.side, action.phase)?;
        let capacity = if action.phase == DraftPhase::Ban {
            3
        } else {
            5
        };
        if list.len() >= capacity {
            return Err(MockDraftError::DraftSlotFull);
        }
        self.history.push(self.state.clone());
        let list = slot_mut(&mut self.state.sets[set_index], action.side, action.phase)?;
        list.push(action.champion_id);
        if action.side == self.state.current_our_side() {
            self.state.last_own_phase = action.phase;
        }
        Ok(())
    }

    pub fn remove(&mut self, action: &MockDraftAction) -> Result<(), MockDraftError> {
        let set_index = self.state.current_set - 1;
        let list = slot_mut(&mut self.state.sets[set_index], action.side, action.phase)?;
        let Some(index) = list.iter().position(|id| id == &action.champion_id) else {
            return Err(MockDraftError::ChampionNotSelected);
        };
        self.history.push(self.state.clone());
        let list = slot_mut(&mut self.state.sets[set_index], action.side, action.phase)?;
        list.remove(index);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.state = previous;
        true
    }

    pub fn reset_current_set(&mut self) -> bool {
        let index = self.state.current_set - 1;
        let set = &self.state.sets[index];
        if selected_champions(set).is_empty() {
            return false;
        }
        self.history.push(self.state.clone());
        let our_side = self.state.sets[index].our_side;
        self.state.sets[index] = MockDraftSet::empty(self.state.current_set);
        self.state.sets[index].our_side = our_side;
        true
    }
}

fn slot_mut(
    set: &mut MockDraftSet,
    side: DraftSide,
    phase: DraftPhase,
) -> Result<&mut Vec<String>, MockDraftError> {
    match (side, phase) {
        (DraftSide::Blue, DraftPhase::Ban) => Ok(&mut set.blue_bans),
        (DraftSide::Red, DraftPhase::Ban) => Ok(&mut set.red_bans),
        (DraftSide::Blue, DraftPhase::Pick) => Ok(&mut set.blue_picks),
        (DraftSide::Red, DraftPhase::Pick) => Ok(&mut set.red_picks),
        (_, DraftPhase::Waiting) => Err(MockDraftError::InvalidPhase),
    }
}

fn selected_champions(set: &MockDraftSet) -> BTreeSet<String> {
    set.blue_bans
        .iter()
        .chain(&set.red_bans)
        .chain(&set.blue_picks)
        .chain(&set.red_picks)
        .cloned()
        .collect()
}

pub fn carryover_excluded(state: &MockDraftState, side: DraftSide) -> BTreeSet<String> {
    let previous = state
        .sets
        .iter()
        .filter(|set| set.set_number < state.current_set);
    match state.rule {
        DraftRule::Classic => BTreeSet::new(),
        DraftRule::Fearless => previous
            .flat_map(|set| {
                let current_is_ours = side == state.current_our_side();
                let previous_team_side = if current_is_ours {
                    set.our_side
                } else {
                    opposite_side(set.our_side)
                };
                match previous_team_side {
                    DraftSide::Blue => set.blue_picks.iter(),
                    DraftSide::Red => set.red_picks.iter(),
                }
            })
            .cloned()
            .collect(),
        DraftRule::HardFearless => previous
            .flat_map(|set| set.blue_picks.iter().chain(&set.red_picks))
            .cloned()
            .collect(),
    }
}

fn opposite_side(side: DraftSide) -> DraftSide {
    match side {
        DraftSide::Blue => DraftSide::Red,
        DraftSide::Red => DraftSide::Blue,
    }
}
