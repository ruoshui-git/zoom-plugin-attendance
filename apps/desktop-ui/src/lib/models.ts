export type SessionMode = 'generic' | 'roster';

export type MeetingScale =
  | 'compact'
  | 'medium'
  | 'large'
  | 'xlarge'
  | 'xxlarge';

export type ClientScale = MeetingScale;

export type ParticipantLocation = 'meeting' | 'waiting-room';

export type MeetingEventType =
  | 'joinedBefore'
  | 'join'
  | 'leave'
  | 'waitingRoomJoinFromOutside'
  | 'waitingRoomJoinFromMeeting'
  | 'waitingRoomJoinedBefore'
  | 'waitingRoomLeave';

export type MatchStatus =
  | 'not-applicable'
  | 'matched'
  | 'unmatched'
  | 'guest'
  | 'split';

/**
 * LiveStatus is a single enum that flattens three orthogonal dimensions:
 *
 *   Dimension        | Values
 *   -----------------+-------------------------------------------
 *   Presence         | inMeeting, notInMeeting, neverSeen
 *   Timeliness       | onTime, late
 *   Departure history| stayedOrStaying, leftEarly, returned
 *
 * Enum values and their meaning:
 *   present                  – currently in meeting, arrived on time, has not left (or left briefly within return window)
 *   late                     – currently in meeting but arrived after the late threshold
 *   missing                  – roster-expected person who has never joined (roster mode only)
 *   leftEarly                – was in meeting at some point but is NOT currently present
 *   returnedAfterLongAbsence – currently present after at least one leave/return gap exceeded returnWindowMinutes
 *   guest                    – in meeting, explicitly marked as guest (not counted in attendance)
 *   unmatched                – in meeting, not matched to any roster person (roster mode only)
 *
 * AttendanceRow also exposes `isCurrentlyPresent` and `wasLate` as separate booleans
 * so the summary bar can derive accurate cross-cutting counts without ambiguity.
 *
 * Summary bar semantics (Meeting.svelte headerSummary):
 *   到场/Here    = isCurrentlyPresent && matched   (does NOT include late + returned)
 *   迟到/Late    = isCurrentlyPresent && matched && wasLate
 *   缺席/Missing = currentStatus === 'missing' && matched   (roster mode only)
 *   早退/Left    = currentStatus === 'leftEarly' && matched
 *   返回/Returned= isCurrentlyPresent && matched && currentStatus === 'returnedAfterLongAbsence'
 *   不计入/Ignored = isCurrentlyPresent && (unmatched || guest)   (roster mode only)
 *   总计/Total   = all matched rows (seen + roster-expected missing, excludes unmatched/guest)
 */
export type LiveStatus =
  | 'present'
  | 'missing'
  | 'late'
  | 'leftEarly'
  | 'returnedAfterLongAbsence'
  | 'guest'
  | 'unmatched'
  // Desktop deviation: waited in the waiting room but was never admitted.
  | 'waitingRoomOnly';

export type ParticipantIdentityResolution =
  | 'new_identity'
  | 'uuid_direct'
  | 'name_rejoin'
  | 'ambiguous';

export interface ZoomParticipantRecord {
  participantUUID: string;
  screenName: string;
  screenNamePinyin: string;
  role: string;
  location: ParticipantLocation;
}

export interface MovementEvent {
  id: string;
  timestamp: string;
  type: MeetingEventType;
  sessionParticipantId?: string;
  participant: ZoomParticipantRecord;
}

export type SessionEvent = Omit<MovementEvent, 'sessionParticipantId'> & {
  sessionParticipantId?: string;
};

export interface RosterObservation {
  id: string;
  timestamp: string;
  participants: ZoomParticipantRecord[];
}

export interface NameChangeEvent {
  id: string;
  timestamp: string;
  sessionParticipantId: string;
  participantUUID: string;
  previousScreenName: string;
  nextScreenName: string;
  role: string;
  location: ParticipantLocation;
  source: 'roster_observation';
}

export interface SessionParticipant {
  sessionParticipantId: string;
  currentParticipantUUID: string;
  participantUUIDs: string[];
  currentScreenName: string;
  screenNames: string[];
  role: string;
  currentLocation?: ParticipantLocation;
  firstSeenAt: string;
  lastSeenAt?: string;
  identityResolution: ParticipantIdentityResolution;
  hasAmbiguousName: boolean;
}

export interface SnapshotParticipant extends ZoomParticipantRecord {
  sessionParticipantId?: string;
  matchedPersonId?: number;
  matchedCanonicalName?: string;
  matchStatus: MatchStatus;
}

export interface ManualSnapshot {
  id: string;
  timestamp: string;
  participants: SnapshotParticipant[];
}

export interface RaiseHandEntry {
  id: string;
  participantUUID: string;
  screenName: string;
  timestamp: string;
  action: 'raise' | 'lower';
  sequence: number;
  receivedAt: number;
}

export interface RaiseHandCapture {
  id: string;
  timestamp: string;
  participantUUIDs: string[];
  screenNames: string[];
  totalCount: number;
}

export interface ReconciliationDecision {
  id: string;
  sessionParticipantId: string;
  zoomName: string;
  zoomNameAtDecision: string;
  action:
    | 'match'
    | 'guest'
    | 'split'
    | 'name_change_keep_match'
    | 'name_change_revert_match';
  personId?: number;
  canonicalName?: string;
  splitPersonIds?: number[];
  splitCanonicalNames?: string[];
  notes?: string;
  createdAt: string;
}

/**
 * A derived attendance row for one participant (or roster person) in a session.
 *
 * Key fields for summary/filter logic:
 *   currentStatus       – the single LiveStatus enum (see LiveStatus docs)
 *   isCurrentlyPresent  – true if participant is in the meeting right now
 *   wasLate             – true if first join was after the late threshold
 *   matchStatus         – roster match result: 'matched' | 'unmatched' | 'guest' | 'split'
 *   isExpected          – true if the matched roster person belongs to the active profile's groups
 *
 * The summary bar in Meeting.svelte uses isCurrentlyPresent + wasLate + matchStatus
 * to derive accurate counts that don't double-count across categories.
 * The status chip filter has been removed; filtering by status is done via
 * hidden presets triggered by summary bar clicks.
 */
export interface AttendanceRow {
  entityId: string;
  sessionParticipantId?: string;
  participantUUID?: string;
  participantUUIDs?: string[];
  zoomName: string;
  zoomNames: string[];
  role?: string;
  personId?: number;
  canonicalName?: string;
  pinyin?: string;
  group?: string;
  currentLocation?: ParticipantLocation;
  matchStatus: MatchStatus;
  isExpected: boolean;
  hasBeenSeen: boolean;
  isCurrentlyPresent: boolean;
  firstJoinAt?: string;
  lastSeenAt?: string;
  currentStatus: LiveStatus;
  wasLate: boolean;
  totalPresenceMinutes: number;
  absenceCount: number;
  sharedName: boolean;
  splitPersonIds?: number[];
  splitCanonicalNames?: string[];
  notes?: string;
}

export interface MeetingProfile {
  id: string;
  name: string;
  description?: string;
  includedSubgroups: string[];
  lateAfterMinutes: number;
  returnWindowMinutes: number;
  meetingIds: string[];
  topicKeywords: string[];
  specialParticipants: string[];
  blacklist: string[];
  groupSortOrder?: string[];
  active: boolean;
  isDefault?: boolean;
}

export interface RosterPerson {
  personId: number;
  canonicalName: string;
  pinyin: string;
  group: string;
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface AliasMapping {
  id?: number;
  zoomName: string;
  personId: number;
  notes: string;
  lastUsedAt?: string;
}

export type SessionRecordSource =
  | 'meeting_live'
  | 'main_import'
  | 'main_restore';

export type SessionDisposition =
  | 'accepted'
  | 'suppressed'
  | 'imported_authoritative';

export type AliasMutationStatus =
  | 'pending'
  | 'submitted'
  | 'committed'
  | 'rejected';

export type AliasMutationReviewStatus =
  | 'pending_review'
  | 'approved'
  | 'dismissed';

export type AliasMutationOp = 'upsert' | 'delete';

export interface SessionRecord {
  id?: number;
  sessionId: string;
  originSessionKey: string;
  sessionRevision: number;
  recordSource: SessionRecordSource;
  startedAt: string;
  endedAt?: string;
  isActive: boolean;
  needsExport: boolean;
  mode: SessionMode;
  meetingUUID?: string;
  meetingId?: string;
  meetingTopic?: string;
  privilegeAcquiredAt: string;
  guessedProfileId?: string;
  selectedProfileId?: string;
  selectedIncludedSubgroups: string[]; // Captured from profile when selected
  selectedGroupSortOrder?: string[]; // Captured from profile when selected
  movementEvents: MovementEvent[];
  sessionParticipants: SessionParticipant[];
  rosterObservations: RosterObservation[];
  nameChangeEvents: NameChangeEvent[];
  latestRoster: ZoomParticipantRecord[];
  manualSnapshots: ManualSnapshot[];
  reconciliationDecisions: ReconciliationDecision[];
  derivedRows: AttendanceRow[];
  exportedAt?: string;
  raiseHandEntries: RaiseHandEntry[];
  raiseHandCaptures: RaiseHandCapture[];
  lastSubmittedAt?: string;
  lateThresholdMinutes: number;
  returnWindowMinutes: number;
}

export interface SessionExportBundle {
  version: 3;
  exportedAt: string;
  session: SessionRecord;
}

export interface SessionPolicy {
  originSessionKey: string;
  effectiveDisposition: SessionDisposition;
  lastAcceptedRevision: number;
  blockedReason?: string;
  updatedAt: string;
}

export interface SessionSyncAck {
  originSessionKey: string;
  ackedRevision: number;
  cleanupEligibleRevision?: number;
  updatedAt: string;
}

export interface AliasMutationOutboxItem {
  mutationId: string;
  op: AliasMutationOp;
  zoomName: string;
  personId?: number;
  notes?: string;
  baseConfigVersion: number;
  createdAt: string;
  status: AliasMutationStatus;
  submittedAt?: string;
  resolvedAt?: string;
  rejectionCode?: string;
  rejectionMessage?: string;
}

export interface AliasMutationReviewItem {
  mutationId: string;
  sourceClient: 'meeting';
  op: AliasMutationOp;
  zoomName: string;
  personId?: number;
  notes?: string;
  baseConfigVersion: number;
  submittedAt: string;
  status: AliasMutationReviewStatus;
  rejectionCode: string;
  reviewedAt?: string;
}

export interface ProcessedAliasMutation {
  mutationId: string;
  processedAt: string;
}

export interface ConfigSnapshotMessage {
  configVersion: number;
  rosterPeople: RosterPerson[];
  aliasMappings: AliasMapping[];
  meetingProfiles: MeetingProfile[];
  meetingScale: MeetingScale;
  clientScale: ClientScale;
  filterDefaultsClosed: boolean;
}

export interface AliasMutationMessage {
  mutationId: string;
  op: AliasMutationOp;
  zoomName: string;
  personId?: number;
  notes?: string;
  baseConfigVersion: number;
  createdAt: string;
}

export interface AliasMutationResultMessage {
  mutationId: string;
  status: 'committed' | 'rejected';
  configVersionAfterCommit?: number;
  rejectionCode?: string;
  rejectionMessage?: string;
}

export interface SessionUpsertMessage {
  session: SessionRecord;
}

export interface SessionAckMessage {
  originSessionKey: string;
  ackedRevision: number;
}

export interface SessionCleanupEligibleMessage {
  originSessionKey: string;
  ackedRevision: number;
}

export interface ConfigDirtyState {
  roster: boolean;
  aliases: boolean;
  profiles: boolean;
}

export const DEFAULT_RETURN_WINDOW_MINUTES = 8;
export const DEFAULT_LATE_AFTER_MINUTES = 10;

export const FACTORY_DEFAULT_PROFILE: MeetingProfile = {
  id: 'default-all',
  name: 'System Initial Attendance Profile/系统初始考勤配制',
  description:
    'Profile that is used when no other attendance configuration is available/无其它考勤配置时使用的默认考勤配置',
  includedSubgroups: [],
  lateAfterMinutes: DEFAULT_LATE_AFTER_MINUTES,
  returnWindowMinutes: DEFAULT_RETURN_WINDOW_MINUTES,
  meetingIds: [],
  topicKeywords: [],
  specialParticipants: [],
  blacklist: [],
  groupSortOrder: [],
  active: true,
  isDefault: true,
};

export const DIRTY_STATE_DEFAULT: ConfigDirtyState = {
  roster: false,
  aliases: false,
  profiles: false,
};
