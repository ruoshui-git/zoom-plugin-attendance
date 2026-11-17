// Source snapshot: AttendFlow 405abd5 (2026-11-17). Adapted for zoom-companion desktop UI.
import { DateTime } from 'luxon';
import type {
  LiveStatus,
  MatchStatus,
  MeetingEventType,
  ParticipantLocation,
  SessionMode,
  SessionRecordSource,
} from './models';
import { get, type Readable } from 'svelte/store';
import { pickText, uiLanguage, type UiLanguage } from './i18n-core';

export type BadgeVariant =
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'neutral'
  | 'muted';

/**
 * Display labels for LiveStatus values.
 *
 * Terminology:
 * - `missing` displays as "Absent" / "缺席" — roster-expected person who never joined.
 * - `returnedAfterLongAbsence` displays as "Returned" / "返回" — shorthand used in UI.
 *   "Returned" means the participant left and came back after the return window threshold.
 * - `present` displays as "Seen" / "到过" — participant was observed at least once in session.
 *   Quality status like full attendance is represented by row tags, not this enum value.
 */
const liveStatusLabels: Record<LiveStatus, { zh: string; en: string }> = {
  present: { zh: '到过', en: 'Seen' },
  missing: { zh: '缺席', en: 'Absent' },
  late: { zh: '迟到', en: 'Late' },
  leftEarly: { zh: '早退', en: 'Left Early' },
  returnedAfterLongAbsence: { zh: '返回', en: 'Returned' },
  guest: { zh: '来宾', en: 'Guest' },
  unmatched: { zh: '未匹配', en: 'Unmatched' },
};

const matchStatusLabels: Record<MatchStatus, { zh: string; en: string }> = {
  'not-applicable': { zh: '不适用', en: 'N/A' },
  matched: { zh: '已匹配', en: 'Matched' },
  unmatched: { zh: '未匹配', en: 'Unmatched' },
  guest: { zh: '来宾', en: 'Guest' },
  split: { zh: '多人共用', en: 'Shared' },
};

const participantLocationLabels: Record<
  ParticipantLocation,
  { zh: string; en: string }
> = {
  meeting: { zh: '会议中', en: 'Meeting' },
  'waiting-room': { zh: '等候室', en: 'Waiting Room' },
};

const sessionModeLabels: Record<SessionMode, { zh: string; en: string }> = {
  generic: { zh: '通用模式', en: 'Generic' },
  roster: { zh: '花名册模式', en: 'Roster' },
};

function resolveLanguage(language?: UiLanguage | Readable<UiLanguage>) {
  if (!language) {
    return get(uiLanguage);
  }

  return typeof language === 'string' ? language : get(language);
}

export function getLiveStatusLabel(
  value: LiveStatus | string,
  language?: UiLanguage | Readable<UiLanguage>
) {
  const label = liveStatusLabels[value as LiveStatus];
  return label
    ? pickText(resolveLanguage(language), label.zh, label.en)
    : value;
}

export function getMatchStatusLabel(
  value: MatchStatus,
  language?: UiLanguage | Readable<UiLanguage>
) {
  const label = matchStatusLabels[value];
  return label
    ? pickText(resolveLanguage(language), label.zh, label.en)
    : value;
}

export function getParticipantLocationLabel(
  value?: ParticipantLocation,
  language?: UiLanguage | Readable<UiLanguage>
) {
  if (!value) {
    return '';
  }

  const label = participantLocationLabels[value];
  return label
    ? pickText(resolveLanguage(language), label.zh, label.en)
    : value;
}

export function getSessionModeLabel(
  value: SessionMode,
  language?: UiLanguage | Readable<UiLanguage>
) {
  const label = sessionModeLabels[value];
  return label
    ? pickText(resolveLanguage(language), label.zh, label.en)
    : value;
}

export function getLiveStatusVariant(status: LiveStatus): BadgeVariant {
  switch (status) {
    case 'present':
      return 'info';
    case 'late':
      return 'warning';
    case 'leftEarly':
      return 'warning';
    case 'returnedAfterLongAbsence':
      return 'info';
    case 'missing':
      return 'danger';
    case 'guest':
      return 'neutral';
    case 'unmatched':
      return 'warning';
    default:
      return 'neutral';
  }
}

export function getMatchStatusVariant(status: MatchStatus): BadgeVariant {
  switch (status) {
    case 'matched':
      return 'success';
    case 'unmatched':
      return 'warning';
    case 'guest':
      return 'neutral';
    case 'split':
      return 'info';
    case 'not-applicable':
      return 'muted';
    default:
      return 'neutral';
  }
}

export function getEventTypeVariant(type: MeetingEventType): BadgeVariant {
  switch (type) {
    case 'join':
    case 'joinedBefore':
      return 'success';
    case 'leave':
      return 'danger';
    case 'waitingRoomJoinFromOutside':
    case 'waitingRoomJoinFromMeeting':
    case 'waitingRoomJoinedBefore':
    case 'waitingRoomLeave':
      return 'warning';
    default:
      return 'neutral';
  }
}

export function getRecordSourceLabel(
  source: SessionRecordSource,
  language?: UiLanguage | Readable<UiLanguage>
): { label: string; variant: BadgeVariant } {
  switch (source) {
    case 'meeting_live':
      return {
        label: pickText(resolveLanguage(language), '实时采集', 'Live Capture'),
        variant: 'info',
      };
    case 'main_import':
      return {
        label: pickText(resolveLanguage(language), '导入', 'Imported'),
        variant: 'neutral',
      };
    case 'main_restore':
      return {
        label: pickText(resolveLanguage(language), '恢复', 'Restored'),
        variant: 'warning',
      };
    default:
      return {
        label: source,
        variant: 'neutral',
      };
  }
}

export function getLocationVariant(
  location?: ParticipantLocation
): BadgeVariant {
  if (!location) {
    return 'muted';
  }

  switch (location) {
    case 'meeting':
      return 'success';
    case 'waiting-room':
      return 'warning';
    default:
      return 'muted';
  }
}

export function formatDateTimeForExport(value?: string) {
  if (!value) {
    return '';
  }
  return DateTime.fromISO(value).toFormat('yyyy-LL-dd HH:mm:ss');
}

