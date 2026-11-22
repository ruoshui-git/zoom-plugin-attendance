use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

fn upsert_meeting(c: &Connection, meta: &Value, ended: Option<(u64, String)>, seq: u64) -> Result<()> {
    let id = meta["instanceId"].as_str().context("meeting.instanceId missing")?;
    c.execute("INSERT INTO meetings(instance_id,meeting_number,topic,started_at,ended_at,end_reason,is_host,last_seq,meta_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(instance_id) DO UPDATE SET meeting_number=excluded.meeting_number,topic=CASE WHEN excluded.topic='' THEN meetings.topic ELSE excluded.topic END,ended_at=COALESCE(excluded.ended_at,meetings.ended_at),end_reason=COALESCE(excluded.end_reason,meetings.end_reason),is_host=excluded.is_host,last_seq=MAX(meetings.last_seq,excluded.last_seq),meta_json=CASE WHEN excluded.meeting_number='' THEN meetings.meta_json ELSE excluded.meta_json END", params![id,meta["meetingNumber"].as_str().unwrap_or(""),meta["topic"].as_str().unwrap_or(""),meta["startedAt"].as_u64().unwrap_or(0) as i64,ended.as_ref().map(|x|x.0 as i64),ended.as_ref().map(|x|x.1.as_str()),meta["isHost"].as_bool().unwrap_or(false) as i32,seq as i64,meta.to_string()])?;
    Ok(())
}
fn meeting_conn(c: &Connection, id: &str) -> Result<Option<Value>> {
    Ok(c.query_row("SELECT meta_json,ended_at,end_reason,last_seq FROM meetings WHERE instance_id=?1", [id], |r| {
        let mut v: Value = serde_json::from_str(&r.get::<_, String>(0)?).unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.insert("endedAt".into(), r.get::<_, Option<i64>>(1)?.map(|n| Value::from(n as u64)).unwrap_or(Value::Null));
            o.insert("endReason".into(), r.get::<_, Option<String>>(2)?.map(Value::from).unwrap_or(Value::Null));
            o.insert("lastSeq".into(), Value::from(r.get::<_, i64>(3)? as u64));
        }
        Ok(v)
    }).optional()?)
}
#[derive(Debug, Clone)]
pub struct MeetingRow {
    pub instance_id: String,
    pub meeting_number: String,
    pub topic: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub end_reason: Option<String>,
    pub is_host: bool,
    pub last_seq: u64,
}
#[derive(Debug, Clone)]
pub struct EventRow {
    pub seq: u64,
    pub at: u64,
    pub kind: String,
    pub payload: Value,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }
        let connection = Connection::open(&path)?;
        let s = Self { path, connection: Arc::new(Mutex::new(connection)) };
        s.with(|c| { c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS meetings(instance_id TEXT PRIMARY KEY, meeting_number TEXT NOT NULL, topic TEXT NOT NULL DEFAULT '', started_at INTEGER NOT NULL, ended_at INTEGER, end_reason TEXT, is_host INTEGER NOT NULL DEFAULT 0, last_seq INTEGER NOT NULL DEFAULT 0, meta_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS events(instance_id TEXT NOT NULL REFERENCES meetings(instance_id) ON DELETE CASCADE, seq INTEGER NOT NULL, at INTEGER NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(instance_id,seq)); CREATE TABLE IF NOT EXISTS docs(key TEXT PRIMARY KEY, revision INTEGER NOT NULL, json TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS states(instance_id TEXT PRIMARY KEY REFERENCES meetings(instance_id) ON DELETE CASCADE, revision INTEGER NOT NULL, json TEXT NOT NULL, updated_at INTEGER NOT NULL);")?; c.execute("INSERT OR IGNORE INTO schema_migrations VALUES(1,unixepoch())",[])?; Ok(()) })?;
        Ok(s)
    }
    fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let c = self.connection.lock().map_err(|_| anyhow::anyhow!("store connection lock poisoned"))?;
        f(&c)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn upsert_meeting(
        &self,
        meta: &Value,
        ended: Option<(u64, String)>,
        seq: u64,
    ) -> Result<()> {
        let id = meta["instanceId"]
            .as_str()
            .context("meeting.instanceId missing")?;
        let number = meta["meetingNumber"].as_str().unwrap_or("");
        let topic = meta["topic"].as_str().unwrap_or("");
        let start = meta["startedAt"].as_u64().unwrap_or(0);
        let host = meta["isHost"].as_bool().unwrap_or(false);
        let json = meta.to_string();
        self.with(|c| { c.execute("INSERT INTO meetings(instance_id,meeting_number,topic,started_at,ended_at,end_reason,is_host,last_seq,meta_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(instance_id) DO UPDATE SET meeting_number=excluded.meeting_number,topic=CASE WHEN excluded.topic='' THEN meetings.topic ELSE excluded.topic END,ended_at=COALESCE(excluded.ended_at,meetings.ended_at),end_reason=COALESCE(excluded.end_reason,meetings.end_reason),is_host=excluded.is_host,last_seq=MAX(meetings.last_seq,excluded.last_seq),meta_json=CASE WHEN excluded.meeting_number='' THEN meetings.meta_json ELSE excluded.meta_json END",params![id,number,topic,start as i64,ended.as_ref().map(|x|x.0 as i64),ended.as_ref().map(|x|x.1.as_str()),host as i32,seq as i64,json])?; Ok(()) })
    }
    pub fn append_event(
        &self,
        id: &str,
        seq: u64,
        at: u64,
        kind: &str,
        payload: &Value,
    ) -> Result<()> {
        self.with(|c| { c.execute("INSERT OR IGNORE INTO events(instance_id,seq,at,kind,payload) VALUES(?1,?2,?3,?4,?5)",params![id,seq as i64,at as i64,kind,payload.to_string()])?; c.execute("UPDATE meetings SET last_seq=MAX(last_seq,?2) WHERE instance_id=?1",params![id,seq as i64])?; Ok(()) })
    }
    pub fn meetings(&self) -> Result<Vec<MeetingRow>> {
        self.with(|c| { let mut st=c.prepare("SELECT instance_id,meeting_number,topic,started_at,ended_at,end_reason,is_host,last_seq FROM meetings ORDER BY started_at DESC")?; let rows=st.query_map([],|r| Ok(MeetingRow{instance_id:r.get(0)?,meeting_number:r.get(1)?,topic:r.get(2)?,started_at:r.get::<_,i64>(3)? as u64,ended_at:r.get::<_,Option<i64>>(4)?.map(|n|n as u64),end_reason:r.get(5)?,is_host:r.get::<_,i32>(6)?!=0,last_seq:r.get::<_,i64>(7)? as u64}))?; Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?) })
    }
    pub fn meeting_summaries(&self) -> Result<Vec<Value>> {
        self.with(|c| {
            let mut st = c.prepare("SELECT m.instance_id,m.meeting_number,m.topic,m.started_at,m.ended_at,m.end_reason,m.is_host,m.last_seq,m.meta_json,s.json FROM meetings m LEFT JOIN states s ON s.instance_id=m.instance_id ORDER BY m.started_at DESC")?;
            let rows = st.query_map([], |r| {
                let meta: Value = serde_json::from_str(&r.get::<_, String>(8)?).unwrap_or(Value::Null);
                let state: Value = r.get::<_, Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
                Ok(serde_json::json!({"instanceId":r.get::<_,String>(0)?,"meetingNumber":r.get::<_,String>(1)?,"topic":r.get::<_,String>(2)?,"startedAt":r.get::<_,i64>(3)? as u64,"endedAt":r.get::<_,Option<i64>>(4)?.map(|x|x as u64),"endReason":r.get::<_,Option<String>>(5)?,"isHost":r.get::<_,i32>(6)?!=0,"lastSeq":r.get::<_,i64>(7)? as u64,"source":meta["source"],"mode":state["mode"],"exportedAt":state["exportedAt"],"needsExport":state["needsExport"]}))
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    pub fn meeting(&self, id: &str) -> Result<Option<Value>> {
        self.with(|c| {
            Ok(c.query_row(
                "SELECT meta_json,ended_at,end_reason,last_seq FROM meetings WHERE instance_id=?1",
                [id],
                |r| {
                    let mut v: Value =
                        serde_json::from_str(&r.get::<_, String>(0)?).unwrap_or(Value::Null);
                    if let Some(o) = v.as_object_mut() {
                        o.insert(
                            "endedAt".into(),
                            r.get::<_, Option<i64>>(1)?
                                .map(|n| Value::from(n as u64))
                                .unwrap_or(Value::Null),
                        );
                        o.insert(
                            "endReason".into(),
                            r.get::<_, Option<String>>(2)?
                                .map(Value::from)
                                .unwrap_or(Value::Null),
                        );
                        o.insert("lastSeq".into(), Value::from(r.get::<_, i64>(3)? as u64));
                    }
                    Ok(v)
                },
            )
            .optional()?)
        })
    }
    pub fn events(&self, id: &str) -> Result<Vec<EventRow>> {
        self.with(|c| {
            let mut st = c.prepare(
                "SELECT seq,at,kind,payload FROM events WHERE instance_id=?1 ORDER BY seq",
            )?;
            let rows = st.query_map([id], |r| {
                Ok(EventRow {
                    seq: r.get::<_, i64>(0)? as u64,
                    at: r.get::<_, i64>(1)? as u64,
                    kind: r.get(2)?,
                    payload: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or(Value::Null),
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }
    pub fn get_doc(&self, key: &str) -> Result<(u64, Value)> {
        self.with(|c| {
            Ok(
                c.query_row("SELECT revision,json FROM docs WHERE key=?1", [key], |r| {
                    Ok((
                        r.get::<_, i64>(0)? as u64,
                        serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or(Value::Null),
                    ))
                })
                .optional()?
                .unwrap_or((0, Value::Array(vec![]))),
            )
        })
    }
    pub fn put_doc(
        &self,
        key: &str,
        revision: u64,
        value: &Value,
        now: u64,
    ) -> Result<Result<u64, u64>> {
        self.with(|c| { let tx=c.unchecked_transaction()?; let actual:i64=tx.query_row("SELECT revision FROM docs WHERE key=?1",[key],|r|r.get(0)).optional()?.unwrap_or(0); if actual as u64 != revision { return Ok(Err(actual as u64)); } let next=revision+1; tx.execute("INSERT INTO docs(key,revision,json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,json=excluded.json,updated_at=excluded.updated_at",params![key,next as i64,value.to_string(),now as i64])?; tx.commit()?; Ok(Ok(next)) })
    }
    /// Compare revisions and update several configuration documents in one SQLite transaction.
    pub fn put_docs_atomic(
        &self,
        docs: &[(String, u64, Value)],
        now: u64,
    ) -> Result<Result<Vec<u64>, (String, u64)>> {
        self.with(|c| {
            let tx=c.unchecked_transaction()?;
            for (key,revision,_) in docs {
                let actual:i64=tx.query_row("SELECT revision FROM docs WHERE key=?1",[key],|r|r.get(0)).optional()?.unwrap_or(0);
                if actual as u64 != *revision { return Ok(Err((key.clone(),actual as u64))); }
            }
            let mut revisions=Vec::with_capacity(docs.len());
            for (key,revision,value) in docs {
                let next=revision+1;
                tx.execute("INSERT INTO docs(key,revision,json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,json=excluded.json,updated_at=excluded.updated_at",params![key,next as i64,value.to_string(),now as i64])?;
                revisions.push(next);
            }
            tx.commit()?;
            Ok(Ok(revisions))
        })
    }
    pub fn get_state(&self, id: &str) -> Result<(u64, Value)> {
        self.with(|c| {
            Ok(c.query_row(
                "SELECT revision,json FROM states WHERE instance_id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)? as u64,
                        serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or(Value::Null),
                    ))
                },
            )
            .optional()?
            .unwrap_or((0, serde_json::json!({"mode":"generic"}))))
        })
    }
    pub fn put_state(
        &self,
        id: &str,
        revision: u64,
        value: &Value,
        now: u64,
    ) -> Result<Result<u64, u64>> {
        self.with(|c| { let tx=c.unchecked_transaction()?; let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM meetings WHERE instance_id=?1)",[id],|r|r.get(0))?; if !exists{return Ok(Err(u64::MAX));} let actual:i64=tx.query_row("SELECT revision FROM states WHERE instance_id=?1",[id],|r|r.get(0)).optional()?.unwrap_or(0); if actual as u64!=revision{return Ok(Err(actual as u64));} let next=revision+1; tx.execute("INSERT INTO states VALUES(?1,?2,?3,?4) ON CONFLICT(instance_id) DO UPDATE SET revision=excluded.revision,json=excluded.json,updated_at=excluded.updated_at",params![id,next as i64,value.to_string(),now as i64])?; tx.commit()?; Ok(Ok(next)) })
    }
    /// Atomically persist a meeting state update together with alias changes.
    /// Both revisions are checked inside the same SQLite transaction.
    pub fn put_state_and_doc_atomic(
        &self,
        id: &str,
        state_revision: u64,
        state: &Value,
        doc_key: &str,
        doc_revision: u64,
        doc: &Value,
        now: u64,
    ) -> Result<Result<(u64, u64), (String, u64)>> {
        self.with(|c| {
            let tx = c.unchecked_transaction()?;
            let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM meetings WHERE instance_id=?1)", [id], |r| r.get(0))?;
            if !exists { return Ok(Err(("meeting".into(), u64::MAX))); }
            let actual_state: i64 = tx.query_row("SELECT revision FROM states WHERE instance_id=?1", [id], |r| r.get(0)).optional()?.unwrap_or(0);
            if actual_state as u64 != state_revision { return Ok(Err(("state".into(), actual_state as u64))); }
            let actual_doc: i64 = tx.query_row("SELECT revision FROM docs WHERE key=?1", [doc_key], |r| r.get(0)).optional()?.unwrap_or(0);
            if actual_doc as u64 != doc_revision { return Ok(Err((doc_key.into(), actual_doc as u64))); }
            let next_state = state_revision + 1;
            let next_doc = doc_revision + 1;
            tx.execute("INSERT INTO states(instance_id,revision,json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(instance_id) DO UPDATE SET revision=excluded.revision,json=excluded.json,updated_at=excluded.updated_at", params![id, next_state as i64, state.to_string(), now as i64])?;
            tx.execute("INSERT INTO docs(key,revision,json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,json=excluded.json,updated_at=excluded.updated_at", params![doc_key, next_doc as i64, doc.to_string(), now as i64])?;
            tx.commit()?;
            Ok(Ok((next_state, next_doc)))
        })
    }
    pub fn delete_meeting(&self, id: &str) -> Result<bool> {
        self.with(|c| {
            Ok(c.execute(
                "DELETE FROM meetings WHERE instance_id=?1 AND ended_at IS NOT NULL",
                [id],
            )? != 0)
        })
    }
    /// Remove every local record and its cascading events/states.
    pub fn clear_records(&self) -> Result<()> {
        self.with(|c| {
            c.execute("DELETE FROM meetings", [])?;
            c.execute("DELETE FROM docs", [])?;
            Ok(())
        })
    }
    /// Close records left open by a process shutdown, using the last persisted event time.
    pub fn close_stale_meetings(&self, ended_at: u64) -> Result<usize> {
        self.close_open_meetings_except(ended_at, None)
    }
    /// Closes every open meeting except `keep` (the live or resumable instance) at its
    /// last event time, with end reason `companion_offline`. Only one meeting is live.
    pub fn close_open_meetings_except(&self, ended_at: u64, keep: Option<&str>) -> Result<usize> {
        self.with(|c| {
            Ok(c.execute("UPDATE meetings SET ended_at=COALESCE((SELECT MAX(events.at) FROM events WHERE events.instance_id=meetings.instance_id),?1), end_reason='companion_offline' WHERE ended_at IS NULL AND instance_id IS NOT ?2", params![ended_at as i64, keep])?)
        })
    }
    pub fn import_jsonl(&self, path: &Path) -> Result<usize> {
        let mut events = Vec::new();
        for line in fs::read_to_string(path)?.lines() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v["instanceId"].as_str().is_some() { events.push(v); }
            }
        }
        if events.is_empty() { return Ok(0); }
        let max_seq = events.iter().filter_map(|v| v["seq"].as_u64()).max().unwrap_or(0);
        self.with(|c| {
            let tx = c.unchecked_transaction()?;
            let id = events[0]["instanceId"].as_str().unwrap();
            // Compare with events actually stored (not meetings.last_seq, which the live path
            // bumps before appending), so a crash between the two never hides an event.
            let stored: Option<i64> = tx.query_row("SELECT MAX(seq) FROM events WHERE instance_id=?1", [id], |r| r.get(0))?;
            if stored.unwrap_or(0) as u64 >= max_seq { return Ok(0); }
            let mut n = 0;
            for v in events {
            let id = v["instanceId"].as_str().unwrap();
            let kind = v["kind"].as_str().unwrap_or("unknown");
            let seq = v["seq"].as_u64().unwrap_or(0);
            if matches!(kind, "instance_started" | "instance_resumed") {
                upsert_meeting(&tx, &v["meeting"], None, seq)?;
            }
            let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM meetings WHERE instance_id=?1)", [id], |r| r.get(0))?;
            if !exists {
                let meta = serde_json::json!({"instanceId":id,"meetingNumber":"","topic":"","startedAt":v["at"],"isHost":false});
                upsert_meeting(&tx, &meta, None, 0)?;
            }
            let mut payload = v.clone();
            if let Some(o) = payload.as_object_mut() {
                for k in ["type", "seq", "at", "instanceId", "kind"] {
                    o.remove(k);
                }
            }
            n += tx.execute(
                "INSERT OR IGNORE INTO events(instance_id,seq,at,kind,payload) VALUES(?1,?2,?3,?4,?5)",
                params![id, seq as i64,
                v["at"].as_u64().unwrap_or(0) as i64, kind, payload.to_string()])?;
            tx.execute("UPDATE meetings SET last_seq=MAX(last_seq,?2) WHERE instance_id=?1", params![id, seq as i64])?;
            if kind == "self_changed" {
                if let Some(mut meta) = meeting_conn(&tx, id)? {
                    meta["isHost"] = v["isHost"].clone();
                    meta["myself"] = v["myself"].clone();
                    upsert_meeting(&tx, &meta, None, seq)?;
                }
            }
            if kind == "instance_ended" {
                if let Some(meta) = meeting_conn(&tx, id)? {
                    upsert_meeting(&tx, &meta,
                        Some((
                            v["at"].as_u64().unwrap_or(0),
                            v["reason"].as_str().unwrap_or("ended").into(),
                        )),
                        seq,
                    )?;
                }
            }
            }
            tx.commit()?;
            Ok(n)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    fn temp(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("attend-store-{label}-{}.db", std::process::id()))
    }
    fn clean(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(path.with_extension("db-wal"));
        let _ = fs::remove_file(path.with_extension("db-shm"));
    }
    #[test]
    fn migrations_and_revision_conflicts() {
        let p = temp("revision");
        clean(&p);
        let s = Store::open(&p).unwrap();
        let (rev, val) = s.get_doc("roster").unwrap();
        assert_eq!(rev, 0);
        assert_eq!(val, Value::Array(vec![]));
        assert_eq!(
            s.put_doc("roster", 0, &json!([{"name":"Ada"}]), 10)
                .unwrap(),
            Ok(1)
        );
        assert_eq!(s.put_doc("roster", 0, &json!([]), 11).unwrap(), Err(1));
        assert_eq!(s.get_doc("roster").unwrap().0, 1);
        clean(&p)
    }
    #[test]
    fn meeting_summaries_include_state_mode_and_null_without_state() {
        let p = temp("summary-mode"); clean(&p);
        let s = Store::open(&p).unwrap();
        s.upsert_meeting(&json!({"instanceId":"mode-1","meetingNumber":"1","startedAt":1}), None, 0).unwrap();
        s.upsert_meeting(&json!({"instanceId":"mode-2","meetingNumber":"2","startedAt":2}), None, 0).unwrap();
        s.put_state("mode-1", 0, &json!({"mode":"roster"}), 3).unwrap();
        let summaries = s.meeting_summaries().unwrap();
        assert_eq!(summaries.iter().find(|m| m["instanceId"] == "mode-1").unwrap()["mode"], "roster");
        assert!(summaries.iter().find(|m| m["instanceId"] == "mode-2").unwrap()["mode"].is_null());
        clean(&p)
    }
    #[test]
    fn imports_jsonl_idempotently() {
        let p = temp("import");
        clean(&p);
        let dir = std::env::temp_dir().join(format!("attend-store-log-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let log = dir.join("meeting.jsonl");
        let text = r#"{"type":"event","seq":1,"at":100,"instanceId":"pz-1-100","kind":"instance_started","meeting":{"instanceId":"pz-1-100","meetingNumber":"1","topic":"Class","startedAt":100,"isHost":false}}\n{"type":"event","seq":2,"at":120,"instanceId":"pz-1-100","kind":"users","action":"add","users":[{"userId":2,"name":"Ada"}]}\n{"type":"event","seq":3,"at":200,"instanceId":"pz-1-100","kind":"instance_ended","reason":"ended"}"#;
        fs::write(&log, text.replace("\\n", "\n")).unwrap();
        let s = Store::open(&p).unwrap();
        assert_eq!(s.import_jsonl(&log).unwrap(), 3);
        assert_eq!(s.import_jsonl(&log).unwrap(), 0);
        assert_eq!(s.events("pz-1-100").unwrap().len(), 3);
        let m = s.meeting("pz-1-100").unwrap().unwrap();
        assert_eq!(m["topic"], "Class");
        assert_eq!(m["endReason"], "ended");
        clean(&p);
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn imports_only_appended_events() {
        let p = temp("append"); clean(&p);
        let dir = std::env::temp_dir().join(format!("attend-store-append-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir); let log = dir.join("meeting.jsonl");
        fs::write(&log, "{\"seq\":1,\"at\":1,\"instanceId\":\"x\",\"kind\":\"instance_started\",\"meeting\":{\"instanceId\":\"x\",\"meetingNumber\":\"1\",\"startedAt\":1}}\n").unwrap();
        let s = Store::open(&p).unwrap(); assert_eq!(s.import_jsonl(&log).unwrap(), 1);
        fs::OpenOptions::new().append(true).open(&log).unwrap().write_all(b"{\"seq\":2,\"at\":2,\"instanceId\":\"x\",\"kind\":\"users\"}\n").unwrap();
        assert_eq!(s.import_jsonl(&log).unwrap(), 1); assert_eq!(s.events("x").unwrap().len(), 2);
        clean(&p); let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn import_recovers_an_event_missing_despite_a_higher_last_seq() {
        // The live path updates meetings.last_seq before appending the event; a crash in
        // between must not make the next startup skip that event from the log.
        let p = temp("recover"); clean(&p);
        let dir = std::env::temp_dir().join(format!("attend-store-recover-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir); let log = dir.join("meeting.jsonl");
        fs::write(&log, "{\"seq\":1,\"at\":1,\"instanceId\":\"y\",\"kind\":\"instance_started\",\"meeting\":{\"instanceId\":\"y\",\"meetingNumber\":\"1\",\"startedAt\":1}}
{\"seq\":2,\"at\":2,\"instanceId\":\"y\",\"kind\":\"users\"}
").unwrap();
        let s = Store::open(&p).unwrap();
        s.upsert_meeting(&json!({"instanceId":"y","meetingNumber":"1","topic":"","startedAt":1,"isHost":false}), None, 2).unwrap();
        s.append_event("y", 1, 1, "instance_started", &json!({})).unwrap();
        assert_eq!(s.import_jsonl(&log).unwrap(), 1);
        assert_eq!(s.events("y").unwrap().len(), 2);
        clean(&p); let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn shared_store_is_safe_across_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Store>();
        let p = temp("threads"); clean(&p); let store = Store::open(&p).unwrap();
        let handles: Vec<_> = (0..2).map(|i| { let s=store.clone(); std::thread::spawn(move || { for _ in 0..100 { let _ = s.put_doc("shared", 0, &json!({"thread":i}), 1).unwrap(); let _=s.get_doc("shared").unwrap(); } }) }).collect();
        for h in handles { h.join().unwrap(); }
        clean(&p);
    }
}
