// Source snapshot: AttendFlow 405abd5 (2026-11-17). Adapted for zoom-companion desktop UI.
import type { MeetingEventType } from './models';

/**
 * Human-readable labels for meeting event types.
 * Used in LiveSessionDashboard and SessionRecordDisplay.
 */
export const eventTypeLabels: Record<
  MeetingEventType,
  { zh: string; en: string }
> = {
  joinedBefore: { zh: '启动时已加入', en: 'Already In Meeting At Startup' },
  join: { zh: '进入会议', en: 'Joined Meeting' },
  leave: { zh: '离开会议', en: 'Left Meeting' },
  waitingRoomJoinFromOutside: { zh: '进入等候室', en: 'Entered Waiting Room' },
  waitingRoomJoinFromMeeting: {
    zh: '被移到等候室',
    en: 'Moved To Waiting Room',
  },
  waitingRoomJoinedBefore: {
    zh: '启动时已在等候室',
    en: 'Already In Waiting Room At Startup',
  },
  waitingRoomLeave: { zh: '离开等候室', en: 'Left Waiting Room' },
};

/**
 * Compact event display: action verb + optional location badge.
 * Used in SessionEventsTab for compact event rendering.
 */
export interface CompactEventDisplay {
  action: { zh: string; en: string };
  location: { zh: string; en: string } | null;
  suffix?: { zh: string; en: string };
}

export const compactEventTypeDisplay: Record<
  MeetingEventType,
  CompactEventDisplay
> = {
  joinedBefore: {
    action: { zh: '启动时在', en: 'In' },
    location: { zh: '会议', en: 'Meeting' },
    suffix: { zh: '中', en: 'At Startup' },
  },
  join: {
    action: { zh: '加入', en: 'Joined' },
    location: { zh: '会议', en: 'Meeting' },
  },
  leave: {
    action: { zh: '离开', en: 'Left' },
    location: { zh: '会议', en: 'Meeting' },
  },
  waitingRoomJoinFromOutside: {
    action: { zh: '进入', en: 'Entered' },
    location: { zh: '等候室', en: 'Waiting Room' },
  },
  waitingRoomJoinFromMeeting: {
    action: { zh: '被移到', en: 'Moved to' },
    location: { zh: '等候室', en: 'Waiting Room' },
  },
  waitingRoomJoinedBefore: {
    action: { zh: '启动时在', en: 'In' },
    location: { zh: '等候室', en: 'Waiting Room' },
    suffix: { zh: '中', en: 'At Startup' },
  },
  waitingRoomLeave: {
    action: { zh: '离开', en: 'Left' },
    location: { zh: '等候室', en: 'Waiting Room' },
  },
};

