import { afterAll, describe, it } from 'vitest';
import { writeFileSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { db } from '../../../database/db';
import { computeAttendanceSummary, deriveAttendanceTags } from '../../../lib/attendance-status';
import { deriveAttendanceRows } from '../../../lib/session-engine';
import { getExampleConfigBundle } from '../../../lib/example-config-data';
import { ZoomSdkMock } from '../../../lib/__tests__/zoom-sdk-mock';
import { clearDatabase } from '../../../lib/__tests__/recording-test-helpers';
import { mountMeetingComponent, resetMeetingTestState, teardownMeetingTestState, waitForCondition, waitForMeetingReady, type MountedMeeting } from './meeting-test-helpers';
const fixtureRoot = process.env.ATTEND_ENGINE_FIXTURES!;
const outputRoot = process.env.ATTEND_ENGINE_GOLDENS!;
const mock = new ZoomSdkMock();
let mounted: MountedMeeting | null = null;
const walk = (dir: string): string[] => readdirSync(dir, {withFileTypes:true}).flatMap(e => e.isDirectory() ? walk(path.join(dir,e.name)) : e.name.endsWith('.json') ? [path.join(dir,e.name)] : []);
describe('temporary AttendFlow parity golden generator', () => {
  for (const fixturePath of walk(fixtureRoot).sort()) {
    it(path.relative(fixtureRoot,fixturePath), async () => {
      await resetMeetingTestState();
      const recording = JSON.parse(readFileSync(fixturePath,'utf8'));
      mock.loadInlineRecording(recording);
      await mock.setupMock();
      const mountedMeeting = await mountMeetingComponent();
      mounted = mountedMeeting.app;
      await waitForMeetingReady(mountedMeeting.target);
      await mock.playEvents();
      await mock.waitForIdle();
      // Repository recalculation is debounced by 1 second; wait for Dexie to settle after it.
      await new Promise(resolve => setTimeout(resolve, 1200));
      await waitForCondition(async () => {
        const session = await db.sessions.toCollection().first();
        if (!session) throw new Error('session not written');
      }, {timeoutMs:10000,stepMs:100});
      const session = await db.sessions.toCollection().first();
      if (!session) throw new Error('session missing after flush');
      const referenceTime = session.endedAt ?? session.rosterObservations.at(-1)?.timestamp ?? new Date().toISOString();
      const summary = computeAttendanceSummary(session.derivedRows, session.mode, {isEnded: !!session.endedAt});
      const rosterVariants = ['zh-CN', 'en'].flatMap((language) => {
        const seed = getExampleConfigBundle(language as 'zh-CN' | 'en');
        const rosterPeople = seed.rosterPeople.map((person, index) => ({
          personId: index + 1, canonicalName: person.canonicalName, pinyin: person.pinyin,
          group: person.group, active: person.active, createdAt: session.startedAt, updatedAt: session.startedAt,
        }));
        const aliases = seed.aliasMappings.map((alias) => ({
          zoomName: alias.zoomName,
          personId: rosterPeople[seed.rosterPeople.findIndex((person) => person.key === alias.personKey)]?.personId ?? 0,
          notes: alias.notes ?? '', lastUsedAt: undefined,
        })).filter((alias) => alias.personId !== 0);
        const profiles = seed.profiles;
        const picked = session.sessionParticipants[0];
        const actions = ['match', 'guest', 'split', 'name_change_keep_match'] as const;
        return actions.map((action) => {
          const variant = {...session, mode: 'roster' as const,
            selectedProfileId: profiles[0]?.id, reconciliationDecisions: picked ? [{
              id: `golden-${action}`, sessionParticipantId: picked.sessionParticipantId,
              zoomName: picked.currentScreenName, zoomNameAtDecision: picked.currentScreenName,
              action, personId: action === 'match' || action === 'name_change_keep_match' ? rosterPeople[0]?.personId : undefined,
              canonicalName: action === 'match' || action === 'name_change_keep_match' ? rosterPeople[0]?.canonicalName : undefined,
              splitPersonIds: action === 'split' ? rosterPeople.slice(0, 2).map((person) => person.personId) : [],
              splitCanonicalNames: action === 'split' ? rosterPeople.slice(0, 2).map((person) => person.canonicalName) : [],
              createdAt: session.startedAt,
            }] : [],
          };
          const rows = deriveAttendanceRows(variant, rosterPeople, aliases, profiles)
            .map((row) => ({...row, tags: deriveAttendanceTags(row, 'roster')}));
          return {language, action, config: {rosterPeople, aliasMappings: aliases, profiles},
            derivedRows: rows, summary: computeAttendanceSummary(rows, 'roster', {isEnded: !!session.endedAt})};
        });
      });
      const result = {
        fixture: path.relative(fixtureRoot,fixturePath).replaceAll('\\','/'),
        mode: session.mode,
        config: { roster: await db.rosterPeople.toArray(), aliases: await db.aliasMappings.toArray(), profiles: await db.meetingProfiles.toArray() },
        derivedRows: session.derivedRows.map(row => ({...row, tags: deriveAttendanceTags(row, session.mode)})),
        summary,
        referenceTime,
        sessionRecord: {...session,
          endedAt: session.endedAt ?? null,
          selectedProfileId: session.selectedProfileId ?? null,
          guessedProfileId: session.guessedProfileId ?? null,
          selectedGroupSortOrder: session.selectedGroupSortOrder ?? null,
        },
        rosterVariants,
        sessionParticipants: session.sessionParticipants,
        movementEvents: session.movementEvents.map(e => ({type:e.type,timestamp:e.timestamp,sessionParticipantId:e.sessionParticipantId})),
        nameChangeEvents: session.nameChangeEvents,
      };
      const target = path.join(outputRoot, path.basename(fixturePath));
      writeFileSync(target, JSON.stringify(result, null, 2) + '\n');
      await teardownMeetingTestState(mounted); mounted=null; mock.teardown();
    }, 20000);
  }
  afterAll(async () => { if (mounted) await teardownMeetingTestState(mounted); mock.teardown(); await clearDatabase(); });
});


