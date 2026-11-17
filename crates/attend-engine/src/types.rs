use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionMode {
    Generic,
    Roster,
}
impl Default for SessionMode {
    fn default() -> Self {
        Self::Generic
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ParticipantLocation {
    Meeting,
    WaitingRoom,
}
impl Default for ParticipantLocation {
    fn default() -> Self {
        Self::Meeting
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MovementType {
    JoinedBefore,
    Join,
    Leave,
    WaitingRoomJoinFromOutside,
    WaitingRoomJoinFromMeeting,
    WaitingRoomJoinedBefore,
    WaitingRoomLeave,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterPerson {
    pub person_id: u64,
    pub canonical_name: String,
    #[serde(default)]
    pub pinyin: String,
    #[serde(default)]
    pub group: String,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AliasMapping {
    #[serde(default)]
    pub id: Option<u64>,
    pub zoom_name: String,
    pub person_id: u64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingProfile {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub included_subgroups: Vec<String>,
    #[serde(default = "late_default")]
    pub late_after_minutes: u32,
    #[serde(default = "return_default")]
    pub return_window_minutes: u32,
    #[serde(default)]
    pub meeting_ids: Vec<String>,
    #[serde(default)]
    pub topic_keywords: Vec<String>,
    #[serde(default)]
    pub special_participants: Vec<String>,
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub group_sort_order: Vec<String>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub is_default: bool,
}
fn late_default() -> u32 {
    10
}
fn return_default() -> u32 {
    8
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DecisionAction {
    Match,
    Guest,
    Split,
    NameChangeKeepMatch,
    NameChangeRevertMatch,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationDecision {
    pub id: String,
    pub session_participant_id: String,
    #[serde(default)]
    pub zoom_name: String,
    #[serde(default)]
    pub zoom_name_at_decision: String,
    pub action: DecisionAction,
    #[serde(default)]
    pub person_id: Option<u64>,
    #[serde(default)]
    pub canonical_name: Option<String>,
    #[serde(default)]
    pub split_person_ids: Vec<u64>,
    #[serde(default)]
    pub split_canonical_names: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MatchStatus {
    NotApplicable,
    Matched,
    Unmatched,
    Guest,
    Split,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AttendanceStatus {
    Present,
    Missing,
    Late,
    LeftEarly,
    ReturnedAfterLongAbsence,
    Guest,
    Unmatched,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceTag {
    Seen,
    FullPresent,
    Late,
    LeftEarly,
    Returned,
    Missing,
    Ignored,
    Guest,
    Unmatched,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZoomParticipant {
    #[serde(rename = "participantUUID")]
    pub participant_uuid: String,
    pub screen_name: String,
    #[serde(default)]
    pub screen_name_pinyin: String,
    #[serde(default = "attendee")]
    pub role: String,
    #[serde(default)]
    pub location: ParticipantLocation,
}
fn attendee() -> String {
    "attendee".into()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovementEvent {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub kind: MovementType,
    #[serde(default)]
    pub session_participant_id: Option<String>,
    pub participant: ZoomParticipant,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NameChangeEvent {
    pub id: String,
    pub timestamp: String,
    pub session_participant_id: String,
    pub participant_uuid: String,
    pub previous_screen_name: String,
    pub next_screen_name: String,
    pub role: String,
    pub location: ParticipantLocation,
    #[serde(default = "rename_source")]
    pub source: String,
}
fn rename_source() -> String {
    "roster_observation".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceRow {
    pub entity_id: String,
    #[serde(default)]
    pub session_participant_id: Option<String>,
    #[serde(default, rename = "participantUUID")]
    pub participant_uuid: Option<String>,
    #[serde(default, rename = "participantUUIDs")]
    pub participant_uuids: Vec<String>,
    pub zoom_name: String,
    #[serde(default)]
    pub zoom_names: Vec<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub person_id: Option<u64>,
    #[serde(default)]
    pub canonical_name: Option<String>,
    #[serde(default)]
    pub pinyin: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub current_location: Option<ParticipantLocation>,
    pub match_status: MatchStatus,
    pub is_expected: bool,
    pub has_been_seen: bool,
    pub is_currently_present: bool,
    #[serde(default)]
    pub first_join_at: Option<String>,
    #[serde(default)]
    pub first_waiting_room_at: Option<String>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    pub current_status: AttendanceStatus,
    pub was_late: bool,
    pub total_presence_minutes: u64,
    #[serde(default)]
    pub waiting_room_minutes: u64,
    pub absence_count: u32,
    #[serde(default)]
    pub returned_after_long_absence: bool,
    #[serde(default)]
    pub tags: Vec<AttendanceTag>,
    #[serde(default)]
    pub shared_name: bool,
    #[serde(default)]
    pub split_person_ids: Vec<u64>,
    #[serde(default)]
    pub split_canonical_names: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceSummary {
    pub seen: u32,
    pub full_present: u32,
    pub late: u32,
    pub returned: u32,
    pub left_early: u32,
    pub absent: u32,
    pub ignored: u32,
    pub total: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceGrade {
    Excellent,
    Good,
    Moderate,
    Poor,
    Absent,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceScore {
    pub percentage: u8,
    pub grade: AttendanceGrade,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionParticipant {
    pub session_participant_id: String,
    #[serde(rename = "currentParticipantUUID")]
    pub current_participant_uuid: String,
    #[serde(rename = "participantUUIDs")]
    pub participant_uuids: Vec<String>,
    pub current_screen_name: String,
    pub screen_names: Vec<String>,
    pub role: String,
    pub current_location: ParticipantLocation,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub identity_resolution: String,
    pub has_ambiguous_name: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NameChange {
    pub id: String,
    pub timestamp: String,
    pub session_participant_id: String,
    #[serde(rename = "participantUUID")]
    pub participant_uuid: String,
    pub previous_screen_name: String,
    pub next_screen_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RaiseHandEntry {
    pub id: String,
    #[serde(rename = "participantUUID")]
    pub participant_uuid: String,
    pub screen_name: String,
    pub timestamp: String,
    pub action: String,
    pub sequence: u64,
    pub received_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RaiseHandCapture {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "participantUUIDs")]
    pub participant_uuids: Vec<String>,
    pub screen_names: Vec<String>,
    pub total_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub meeting_id: String,
    pub meeting_topic: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    pub mode: SessionMode,
    #[serde(default)]
    pub selected_profile_id: Option<String>,
    pub guessed_profile_id: Option<String>,
    pub participants: Vec<SessionParticipant>,
    pub movement_events: Vec<MovementEvent>,
    pub name_change_events: Vec<NameChange>,
    pub latest_roster: Vec<ZoomParticipant>,
    pub rows: Vec<AttendanceRow>,
    pub summary: AttendanceSummary,
    pub raise_hand_entries: Vec<RaiseHandEntry>,
    pub raise_hand_captures: Vec<RaiseHandCapture>,
}

/// Persisted AttendFlow-shaped input to the derivation phase. Ingestion owns this
/// record; derivation receives it unchanged so the desktop app never replays SDK
/// batching semantics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub origin_session_key: String,
    #[serde(default)]
    pub session_revision: u64,
    #[serde(default)]
    pub record_source: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub needs_export: bool,
    #[serde(default)]
    pub mode: SessionMode,
    #[serde(default)]
    pub meeting_uuid: Option<String>,
    #[serde(default)]
    pub meeting_id: Option<String>,
    #[serde(default)]
    pub meeting_topic: Option<String>,
    #[serde(default)]
    pub privilege_acquired_at: String,
    #[serde(default)]
    pub guessed_profile_id: Option<String>,
    #[serde(default)]
    pub selected_profile_id: Option<String>,
    #[serde(default)]
    pub selected_included_subgroups: Vec<String>,
    #[serde(default)]
    pub selected_group_sort_order: Option<Vec<String>>,
    #[serde(default)]
    pub late_threshold_minutes: u32,
    #[serde(default)]
    pub return_window_minutes: u32,
    #[serde(default)]
    pub late_overrides: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub return_overrides: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub session_participants: Vec<SessionParticipant>,
    #[serde(default)]
    pub movement_events: Vec<MovementEvent>,
    #[serde(default)]
    pub name_change_events: Vec<SessionNameChangeEvent>,
    #[serde(default)]
    pub latest_roster: Vec<ZoomParticipant>,
    #[serde(default)]
    pub roster_observations: Vec<serde_json::Value>,
    #[serde(default)]
    pub manual_snapshots: Vec<serde_json::Value>,
    #[serde(default)]
    pub reconciliation_decisions: Vec<ReconciliationDecision>,
    #[serde(default)]
    pub raise_hand_entries: Vec<RaiseHandEntry>,
    #[serde(default)]
    pub raise_hand_captures: Vec<RaiseHandCapture>,
    #[serde(default)]
    pub derived_rows: Vec<AttendanceRow>,
    #[serde(default)]
    pub exported_at: Option<String>,
    #[serde(default)]
    pub last_submitted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionNameChangeEvent {
    pub id: String,
    pub timestamp: String,
    pub session_participant_id: String,
    #[serde(rename = "participantUUID")]
    pub participant_uuid: String,
    pub previous_screen_name: String,
    pub next_screen_name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub location: ParticipantLocation,
    #[serde(default = "rename_source")]
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Derived {
    pub rows: Vec<AttendanceRow>,
    pub summary: AttendanceSummary,
}
