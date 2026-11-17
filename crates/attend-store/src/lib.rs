use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
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
        let s = Self { path };
        s.with(|c| { c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS meetings(instance_id TEXT PRIMARY KEY, meeting_number TEXT NOT NULL, topic TEXT NOT NULL DEFAULT '', started_at INTEGER NOT NULL, ended_at INTEGER, end_reason TEXT, is_host INTEGER NOT NULL DEFAULT 0, last_seq INTEGER NOT NULL DEFAULT 0, meta_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS events(instance_id TEXT NOT NULL REFERENCES meetings(instance_id) ON DELETE CASCADE, seq INTEGER NOT NULL, at INTEGER NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(instance_id,seq)); CREATE TABLE IF NOT EXISTS docs(key TEXT PRIMARY KEY, revision INTEGER NOT NULL, json TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS states(instance_id TEXT PRIMARY KEY REFERENCES meetings(instance_id) ON DELETE CASCADE, revision INTEGER NOT NULL, json TEXT NOT NULL, updated_at INTEGER NOT NULL);")?; c.execute("INSERT OR IGNORE INTO schema_migrations VALUES(1,unixepoch())",[])?; Ok(()) })?;
        Ok(s)
    }
    fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let c = Connection::open(&self.path)?;
        c.execute_batch("PRAGMA foreign_keys=ON;")?;
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
        self.with(|c| {
            Ok(c.execute("UPDATE meetings SET ended_at=COALESCE((SELECT MAX(events.at) FROM events WHERE events.instance_id=meetings.instance_id),?1), end_reason='companion_offline' WHERE ended_at IS NULL", [ended_at as i64])?)
        })
    }
    pub fn import_jsonl(&self, path: &Path) -> Result<usize> {
        let mut n = 0;
        for line in fs::read_to_string(path)?.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(id) = v["instanceId"].as_str() else {
                continue;
            };
            let kind = v["kind"].as_str().unwrap_or("unknown");
            if matches!(kind, "instance_started" | "instance_resumed") {
                let m = &v["meeting"];
                self.upsert_meeting(m, None, v["seq"].as_u64().unwrap_or(0))?;
            }
            if self.meeting(id)?.is_none() {
                let meta = serde_json::json!({"instanceId":id,"meetingNumber":"","topic":"","startedAt":v["at"],"isHost":false});
                self.upsert_meeting(&meta, None, 0)?;
            }
            let mut payload = v.clone();
            if let Some(o) = payload.as_object_mut() {
                for k in ["type", "seq", "at", "instanceId", "kind"] {
                    o.remove(k);
                }
            }
            self.append_event(
                id,
                v["seq"].as_u64().unwrap_or(0),
                v["at"].as_u64().unwrap_or(0),
                kind,
                &payload,
            )?;
            if kind == "self_changed" {
                if let Some(mut meta) = self.meeting(id)? {
                    meta["isHost"] = v["isHost"].clone();
                    meta["myself"] = v["myself"].clone();
                    self.upsert_meeting(&meta, None, v["seq"].as_u64().unwrap_or(0))?;
                }
            }
            if kind == "instance_ended" {
                if let Some(meta) = self.meeting(id)? {
                    self.upsert_meeting(
                        &meta,
                        Some((
                            v["at"].as_u64().unwrap_or(0),
                            v["reason"].as_str().unwrap_or("ended").into(),
                        )),
                        v["seq"].as_u64().unwrap_or(0),
                    )?;
                }
            }
            n += 1;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
        assert_eq!(s.import_jsonl(&log).unwrap(), 3);
        assert_eq!(s.events("pz-1-100").unwrap().len(), 3);
        let m = s.meeting("pz-1-100").unwrap().unwrap();
        assert_eq!(m["topic"], "Class");
        assert_eq!(m["endReason"], "ended");
        clean(&p);
        let _ = fs::remove_dir_all(dir);
    }
}
