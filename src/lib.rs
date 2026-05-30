use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// PlayerProfile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub name: String,
    pub level: u32,
    pub experience: f64,
    pub tutorials_completed: Vec<String>,
    pub badges_earned: Vec<String>,
    pub pets: Vec<String>,
    pub friends: Vec<String>,
    pub total_builds: u32,
    pub total_ticks: u64,
}

impl PlayerProfile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            level: 1,
            experience: 0.0,
            tutorials_completed: Vec::new(),
            badges_earned: Vec::new(),
            pets: Vec::new(),
            friends: Vec::new(),
            total_builds: 0,
            total_ticks: 0,
        }
    }

    pub fn add_experience(&mut self, xp: f64) {
        self.experience += xp;
    }

    /// Returns `true` if a level-up occurred (may happen multiple times).
    pub fn check_level_up(&mut self) -> bool {
        let mut leveled = false;
        loop {
            let threshold = self.level as f64 * 100.0;
            if self.experience >= threshold {
                self.experience -= threshold;
                self.level += 1;
                leveled = true;
            } else {
                break;
            }
        }
        leveled
    }
}

// ---------------------------------------------------------------------------
// SessionEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    BuildAttempt { structure: String, success: bool },
    LessonComplete { topic: String, score: f64 },
    QuestComplete { quest: String },
    AgentInteraction { agent: String, action: String },
    WorldSave { message: String },
    Collaborate { with: Vec<String> },
    PetAction { pet: String, action: String },
}

impl SessionEvent {
    /// Return a human-readable discriminant name for categorisation.
    pub fn kind(&self) -> &'static str {
        match self {
            SessionEvent::BuildAttempt { .. } => "BuildAttempt",
            SessionEvent::LessonComplete { .. } => "LessonComplete",
            SessionEvent::QuestComplete { .. } => "QuestComplete",
            SessionEvent::AgentInteraction { .. } => "AgentInteraction",
            SessionEvent::WorldSave { .. } => "WorldSave",
            SessionEvent::Collaborate { .. } => "Collaborate",
            SessionEvent::PetAction { .. } => "PetAction",
        }
    }
}

// ---------------------------------------------------------------------------
// PlaySession
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaySession {
    pub id: String,
    pub player: PlayerProfile,
    pub events: Vec<SessionEvent>,
    pub start_tick: u64,
    pub current_tick: u64,
    pub rooms_visited: Vec<String>,
}

impl PlaySession {
    pub fn new(player_name: &str) -> Self {
        let tick = 0;
        Self {
            id: format!("{}-{}", player_name, tick),
            player: PlayerProfile::new(player_name),
            events: Vec::new(),
            start_tick: tick,
            current_tick: tick,
            rooms_visited: Vec::new(),
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.player.total_ticks += 1;
    }

    pub fn record(&mut self, event: SessionEvent) {
        // Side-effects on the player profile based on event type.
        match &event {
            SessionEvent::BuildAttempt { success: true, .. } => {
                self.player.total_builds += 1;
            }
            SessionEvent::LessonComplete { topic, .. }
                if !self.player.tutorials_completed.contains(topic) =>
            {
                self.player.tutorials_completed.push(topic.clone());
            }
            _ => {}
        }
        self.events.push(event);
    }

    pub fn duration_ticks(&self) -> u64 {
        self.current_tick.saturating_sub(self.start_tick)
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn events_by_type(&self) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for e in &self.events {
            *map.entry(e.kind().to_string()).or_insert(0) += 1;
        }
        map
    }
}

// ---------------------------------------------------------------------------
// SessionSummary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub player: String,
    pub duration: u64,
    pub total_events: usize,
    pub builds: u32,
    pub lessons: u32,
    pub quests: u32,
    pub collabs: u32,
    pub top_activity: String,
    pub fun_score: f64,
}

impl SessionSummary {
    pub fn generate(session: &PlaySession) -> Self {
        let mut builds: u32 = 0;
        let mut lessons: u32 = 0;
        let mut quests: u32 = 0;
        let mut collabs: u32 = 0;
        let mut pet_actions: u32 = 0;
        let mut agent_interactions: u32 = 0;

        for e in &session.events {
            match e {
                SessionEvent::BuildAttempt { .. } => builds += 1,
                SessionEvent::LessonComplete { .. } => lessons += 1,
                SessionEvent::QuestComplete { .. } => quests += 1,
                SessionEvent::Collaborate { .. } => collabs += 1,
                SessionEvent::PetAction { .. } => pet_actions += 1,
                SessionEvent::AgentInteraction { .. } => agent_interactions += 1,
                SessionEvent::WorldSave { .. } => {}
            }
        }

        // Top activity = the kind with the highest count.
        let by_type = session.events_by_type();
        let top_activity = by_type
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "none".to_string());

        // fun_score: ratio of distinct activity types actually present out of
        // the 6 "fun" categories (builds, lessons, quests, collabs, pets,
        // agent interactions). Clamped to [0, 1].
        let fun_categories = [builds, lessons, quests, collabs, pet_actions, agent_interactions];
        let active_categories = fun_categories.iter().filter(|&&c| c > 0).count();
        let fun_score = (active_categories as f64 / fun_categories.len() as f64).clamp(0.0, 1.0);

        Self {
            player: session.player.name.clone(),
            duration: session.duration_ticks(),
            total_events: session.event_count(),
            builds,
            lessons,
            quests,
            collabs,
            top_activity,
            fun_score,
        }
    }
}

// ---------------------------------------------------------------------------
// SessionStore
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub sessions: HashMap<String, PlaySession>,
    pub summaries: Vec<SessionSummary>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            summaries: Vec::new(),
        }
    }

    pub fn create_session(&mut self, player: &str) -> &mut PlaySession {
        let session = PlaySession::new(player);
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get_mut(&id).unwrap()
    }

    pub fn end_session(&mut self, id: &str) -> Option<SessionSummary> {
        let session = self.sessions.get(id)?;
        let summary = SessionSummary::generate(session);
        self.summaries.push(summary.clone());
        Some(summary)
    }

    pub fn player_sessions(&self, player: &str) -> Vec<&SessionSummary> {
        self.summaries
            .iter()
            .filter(|s| s.player == player)
            .collect()
    }

    pub fn total_play_time(&self) -> u64 {
        self.summaries.iter().map(|s| s.duration).sum()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- PlayerProfile -------------------------------------------------------

    #[test]
    fn player_new() {
        let p = PlayerProfile::new("Mika");
        assert_eq!(p.name, "Mika");
        assert_eq!(p.level, 1);
        assert_eq!(p.experience, 0.0);
        assert!(p.tutorials_completed.is_empty());
    }

    #[test]
    fn player_add_experience() {
        let mut p = PlayerProfile::new("Ava");
        p.add_experience(42.5);
        assert!((p.experience - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn player_level_up_single() {
        let mut p = PlayerProfile::new("Leo");
        // threshold at level 1 = 100
        p.add_experience(100.0);
        assert!(p.check_level_up());
        assert_eq!(p.level, 2);
        assert!((p.experience).abs() < f64::EPSILON);
    }

    #[test]
    fn player_level_up_multi() {
        let mut p = PlayerProfile::new("Sam");
        // level 1 threshold = 100, level 2 threshold = 200 → need 300 total
        p.add_experience(300.0);
        assert!(p.check_level_up());
        assert_eq!(p.level, 3);
    }

    #[test]
    fn player_no_level_up() {
        let mut p = PlayerProfile::new("Kit");
        p.add_experience(50.0);
        assert!(!p.check_level_up());
        assert_eq!(p.level, 1);
    }

    // -- SessionEvent --------------------------------------------------------

    #[test]
    fn event_kind_names() {
        assert_eq!(
            SessionEvent::BuildAttempt {
                structure: "castle".into(),
                success: true
            }
            .kind(),
            "BuildAttempt"
        );
        assert_eq!(
            SessionEvent::LessonComplete {
                topic: "loops".into(),
                score: 0.9
            }
            .kind(),
            "LessonComplete"
        );
        assert_eq!(
            SessionEvent::QuestComplete {
                quest: "dragons".into()
            }
            .kind(),
            "QuestComplete"
        );
    }

    // -- PlaySession ---------------------------------------------------------

    #[test]
    fn session_new() {
        let s = PlaySession::new("Mika");
        assert!(s.id.contains("Mika"));
        assert_eq!(s.events.len(), 0);
        assert_eq!(s.current_tick, 0);
    }

    #[test]
    fn session_tick() {
        let mut s = PlaySession::new("Mika");
        s.tick();
        s.tick();
        s.tick();
        assert_eq!(s.current_tick, 3);
        assert_eq!(s.player.total_ticks, 3);
    }

    #[test]
    fn session_record() {
        let mut s = PlaySession::new("Mika");
        s.record(SessionEvent::BuildAttempt {
            structure: "tower".into(),
            success: true,
        });
        assert_eq!(s.event_count(), 1);
        assert_eq!(s.player.total_builds, 1);
    }

    #[test]
    fn session_record_failed_build() {
        let mut s = PlaySession::new("Mika");
        s.record(SessionEvent::BuildAttempt {
            structure: "bridge".into(),
            success: false,
        });
        assert_eq!(s.player.total_builds, 0);
    }

    #[test]
    fn session_record_lesson() {
        let mut s = PlaySession::new("Ava");
        s.record(SessionEvent::LessonComplete {
            topic: "variables".into(),
            score: 0.95,
        });
        assert!(s.player.tutorials_completed.contains(&"variables".to_string()));
    }

    #[test]
    fn session_duration() {
        let mut s = PlaySession::new("Leo");
        for _ in 0..10 {
            s.tick();
        }
        assert_eq!(s.duration_ticks(), 10);
    }

    #[test]
    fn session_events_by_type() {
        let mut s = PlaySession::new("Sam");
        s.record(SessionEvent::BuildAttempt {
            structure: "a".into(),
            success: true,
        });
        s.record(SessionEvent::BuildAttempt {
            structure: "b".into(),
            success: false,
        });
        s.record(SessionEvent::QuestComplete {
            quest: "q1".into(),
        });
        let map = s.events_by_type();
        assert_eq!(*map.get("BuildAttempt").unwrap(), 2);
        assert_eq!(*map.get("QuestComplete").unwrap(), 1);
    }

    // -- SessionSummary ------------------------------------------------------

    #[test]
    fn summary_basic() {
        let mut s = PlaySession::new("Mika");
        for _ in 0..5 {
            s.tick();
        }
        s.record(SessionEvent::BuildAttempt {
            structure: "house".into(),
            success: true,
        });
        s.record(SessionEvent::LessonComplete {
            topic: "loops".into(),
            score: 1.0,
        });
        let sum = SessionSummary::generate(&s);
        assert_eq!(sum.player, "Mika");
        assert_eq!(sum.duration, 5);
        assert_eq!(sum.total_events, 2);
        assert_eq!(sum.builds, 1);
        assert_eq!(sum.lessons, 1);
        assert_eq!(sum.quests, 0);
        assert_eq!(sum.collabs, 0);
    }

    #[test]
    fn summary_fun_score_variety() {
        let mut s = PlaySession::new("Ava");
        // Cover all 6 fun categories.
        s.record(SessionEvent::BuildAttempt { structure: "x".into(), success: true });
        s.record(SessionEvent::LessonComplete { topic: "t".into(), score: 1.0 });
        s.record(SessionEvent::QuestComplete { quest: "q".into() });
        s.record(SessionEvent::Collaborate { with: vec!["B".into()] });
        s.record(SessionEvent::PetAction { pet: "Cat".into(), action: "feed".into() });
        s.record(SessionEvent::AgentInteraction { agent: "Tutor".into(), action: "chat".into() });
        let sum = SessionSummary::generate(&s);
        assert!((sum.fun_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn summary_fun_score_low() {
        let mut s = PlaySession::new("Leo");
        // Only builds — 1 out of 6 categories.
        s.record(SessionEvent::BuildAttempt { structure: "x".into(), success: true });
        let sum = SessionSummary::generate(&s);
        let expected = 1.0 / 6.0;
        assert!((sum.fun_score - expected).abs() < 1e-9);
    }

    #[test]
    fn summary_top_activity() {
        let mut s = PlaySession::new("Sam");
        s.record(SessionEvent::BuildAttempt { structure: "a".into(), success: true });
        s.record(SessionEvent::BuildAttempt { structure: "b".into(), success: true });
        s.record(SessionEvent::QuestComplete { quest: "q".into() });
        let sum = SessionSummary::generate(&s);
        assert_eq!(sum.top_activity, "BuildAttempt");
    }

    // -- SessionStore --------------------------------------------------------

    #[test]
    fn store_create_session() {
        let mut store = SessionStore::new();
        let s = store.create_session("Mika");
        assert_eq!(s.player.name, "Mika");
        let id = s.id.clone();
        assert!(store.sessions.contains_key(&id));
    }

    #[test]
    fn store_end_session() {
        let mut store = SessionStore::new();
        let s = store.create_session("Mika");
        let id = s.id.clone();
        for _ in 0..3 {
            store.sessions.get_mut(&id).unwrap().tick();
        }
        let summary = store.end_session(&id).unwrap();
        assert_eq!(summary.player, "Mika");
        assert_eq!(summary.duration, 3);
    }

    #[test]
    fn store_player_sessions() {
        let mut store = SessionStore::new();
        let s1 = store.create_session("Mika");
        let id1 = s1.id.clone();
        store.end_session(&id1);
        let s2 = store.create_session("Ava");
        let id2 = s2.id.clone();
        store.end_session(&id2);
        let s3 = store.create_session("Mika");
        let id3 = s3.id.clone();
        store.end_session(&id3);
        let mika = store.player_sessions("Mika");
        assert_eq!(mika.len(), 2);
        let ava = store.player_sessions("Ava");
        assert_eq!(ava.len(), 1);
    }

    #[test]
    fn store_total_play_time() {
        let mut store = SessionStore::new();
        let s1 = store.create_session("A");
        let id1 = s1.id.clone();
        for _ in 0..5 {
            store.sessions.get_mut(&id1).unwrap().tick();
        }
        store.end_session(&id1);
        let s2 = store.create_session("B");
        let id2 = s2.id.clone();
        for _ in 0..10 {
            store.sessions.get_mut(&id2).unwrap().tick();
        }
        store.end_session(&id2);
        assert_eq!(store.total_play_time(), 15);
    }

    // -- Serde round-trip ----------------------------------------------------

    #[test]
    fn serde_player_profile() {
        let p = PlayerProfile::new("Mika");
        let json = serde_json::to_string(&p).unwrap();
        let p2: PlayerProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p.name, p2.name);
        assert_eq!(p.level, p2.level);
    }

    #[test]
    fn serde_session_event() {
        let e = SessionEvent::Collaborate {
            with: vec!["A".into(), "B".into()],
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: SessionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e.kind(), e2.kind());
    }

    #[test]
    fn serde_play_session() {
        let mut s = PlaySession::new("Mika");
        s.tick();
        s.record(SessionEvent::QuestComplete {
            quest: "dragons".into(),
        });
        let json = serde_json::to_string(&s).unwrap();
        let s2: PlaySession = serde_json::from_str(&json).unwrap();
        assert_eq!(s.id, s2.id);
        assert_eq!(s2.event_count(), 1);
    }

    #[test]
    fn serde_session_store() {
        let mut store = SessionStore::new();
        let s = store.create_session("Ava");
        let id = s.id.clone();
        store.sessions.get_mut(&id).unwrap().tick();
        store.end_session(&id);
        let json = serde_json::to_string(&store).unwrap();
        let s2: SessionStore = serde_json::from_str(&json).unwrap();
        assert_eq!(s2.summaries.len(), 1);
    }
}
