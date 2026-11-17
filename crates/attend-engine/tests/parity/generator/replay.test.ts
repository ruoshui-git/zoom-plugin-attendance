import { afterAll, describe, it } from 'vitest';
import { writeFileSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { db } from '../../../database/db';
import { computeAttendanceSummary, deriveAttendanceTags } from '../../../lib/attendance-status';
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
      const summary = computeAttendanceSummary(session.derivedRows, session.mode, {isEnded: !!session.endedAt});
      const result = {
        fixture: path.relative(fixtureRoot,fixturePath).replaceAll('\\','/'),
        mode: session.mode,
        config: { roster: await db.rosterPeople.toArray(), aliases: await db.aliasMappings.toArray(), profiles: await db.meetingProfiles.toArray() },
        derivedRows: session.derivedRows.map(row => ({...row, tags: deriveAttendanceTags(row, session.mode)})),
        summary,
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


