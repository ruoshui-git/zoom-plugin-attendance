use std::{sync::{atomic::{AtomicBool, AtomicI32, Ordering}, Condvar, Mutex}, time::Duration};
use crate::proof::{self, Decision, Frame, TileWait};

pub(crate) type ProofResult<T> = Result<T, String>;
pub(crate) trait ProofDeps {
    fn view_type(&mut self) -> ProofResult<i32>;
    fn switch_view(&mut self, v: i32) -> ProofResult<()>;
    fn next_page(&mut self) -> ProofResult<()>;
    fn capture(&mut self) -> Result<Frame, String>;
    fn sleep(&mut self, ms: u64);
    fn participants(&self) -> Vec<(u32, String, bool)>;
    fn save_png(&mut self, file: &str, png: &[u8]) -> ProofResult<()>;
}
#[derive(Clone, Debug)] pub(crate) struct ProofPage { pub index: usize, pub file: String, pub at: u64, pub waited_ms: u64, pub dark_tiles: u32, pub timed_out: bool, pub camera_on_count: usize, pub participants: Vec<(u32,String,bool)> }
#[derive(Clone, Debug)] pub(crate) struct ProofCapture { pub original_view: i32, pub pages: Vec<ProofPage> }
pub(crate) fn run_capture<D: ProofDeps>(deps: &mut D, _instance_id: &str, mut now: impl FnMut() -> u64) -> ProofResult<ProofCapture> {
    let original = deps.view_type()?;
    let result = (|| {
        if original != 1 { deps.switch_view(1)?; }
        deps.sleep(1500);
        let mut pages = Vec::new(); let mut signatures: Vec<Vec<u8>> = Vec::new();
        for index in 0..20 {
            let mut wait = TileWait::new(); let mut at_ms = 0;
            let (frame, waited_ms, dark_tiles, timed_out) = loop {
                let f = deps.capture()?;
                match wait.push(f, at_ms) {
                    Decision::Wait => { deps.sleep(500); at_ms += 500; }
                    Decision::Accept { waited_ms, dark_tiles, .. } => break (wait.into_frame().unwrap(), waited_ms, dark_tiles, false),
                    Decision::Timeout { waited_ms, dark_tiles, .. } => break (wait.into_frame().unwrap(), waited_ms, dark_tiles, true),
                }
            };
            let signature = proof::downsample_luma(&frame,64,36);
            if proof::page_repeats(&signatures, &signature) { break; }
            let at=now(); let file=proof::page_file_name(index+1,at);
            deps.save_png(&file,&proof::encode_png(&frame))?;
            let participants=deps.participants(); let camera_on_count=participants.iter().filter(|p|p.2).count();
            signatures.push(signature); pages.push(ProofPage{index:index+1,file,at,waited_ms,dark_tiles,timed_out,camera_on_count,participants});
            if index < 19 { deps.next_page()?; deps.sleep(800); }
        }
        Ok(ProofCapture { original_view: original, pages })
    })();
    let restored=deps.switch_view(original);
    match result { Err(e)=>Err(e), Ok(v)=>restored.map(|_|v) }
}

pub(crate) struct ProofShared {
    pub(crate) view: AtomicI32,
    pub(crate) busy: AtomicBool,
    results: Mutex<Vec<(String, i32, i64)>>,
    ready: Condvar,
}
impl Default for ProofShared {
    fn default() -> Self { Self { view: AtomicI32::new(-1), busy: AtomicBool::new(false), results: Mutex::new(Vec::new()), ready: Condvar::new() } }
}
impl ProofShared {
    pub(crate) fn push_result(&self, op: impl Into<String>, code: i32, value: i64) {
        self.results.lock().unwrap().push((op.into(), code, value));
        self.ready.notify_all();
    }
    pub(crate) fn wait_result(&self, op: &str, timeout: Duration) -> Option<(i32, i64)> {
        let end = std::time::Instant::now() + timeout;
        let mut entries = self.results.lock().unwrap();
        loop {
            if let Some(i) = entries.iter().position(|(name, _, _)| name == op) { let (_, code, value) = entries.remove(i); return Some((code, value)); }
            let remaining = end.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() { return None; }
            let (next, wait) = self.ready.wait_timeout(entries, remaining).unwrap();
            entries = next;
            if wait.timed_out() && !entries.iter().any(|(name, _, _)| name == op) { return None; }
        }
    }
    /// Drops answers left over from an earlier run.
    pub(crate) fn clear_results(&self) { self.results.lock().unwrap().clear(); }
    pub(crate) fn observe_view(&self, value: i32) { self.view.store(value, Ordering::SeqCst); }
}
#[cfg(test)] mod tests {
    use super::*;
    struct Fake { view:i32, switches:Vec<i32>, frames:Vec<Frame>, pos:usize, fail:bool, saved:Vec<String>, next:usize }
    impl ProofDeps for Fake {
        fn view_type(&mut self)->ProofResult<i32>{Ok(self.view)}
        fn switch_view(&mut self,v:i32)->ProofResult<()> { self.switches.push(v); self.view=v; Ok(()) }
        fn next_page(&mut self)->ProofResult<()> { self.next+=1; Ok(()) }
        fn capture(&mut self)->Result<Frame,String>{if self.fail{return Err("capture failed".into())} let f=self.frames[self.next.min(self.frames.len()-1)].clone(); self.pos+=1; Ok(f)}
        fn sleep(&mut self,_:u64){}
        fn participants(&self)->Vec<(u32,String,bool)>{vec![(1,"A".into(),true)]}
        fn save_png(&mut self,f:&str,_:&[u8])->ProofResult<()>{self.saved.push(f.into());Ok(())}
    }
    fn solid(v:u8)->Frame { Frame{width:16,height:9,bgra:(0..16*9).flat_map(|_|[v,v,v,255]).collect()} }
    fn fake(view:i32,frames:Vec<Frame>)->Fake { Fake{view,switches:vec![],frames,pos:0,fail:false,saved:vec![],next:0} }
    #[test] fn three_pages_then_repeat_saves_three_and_restores_view(){let mut d=fake(0,vec![solid(20),solid(80),solid(140),solid(20)]);let c=run_capture(&mut d,"m",||42).unwrap();assert_eq!(c.pages.len(),3);assert_eq!(d.saved.len(),3);assert_eq!(d.view,0);}
    #[test] fn capture_error_restores_view(){let mut d=fake(0,vec![solid(50)]);d.fail=true;assert!(run_capture(&mut d,"m",||42).is_err());assert_eq!(d.view,0);}
    #[test] fn already_gallery_skips_switch(){let mut d=fake(1,vec![solid(90),solid(90)]);let _=run_capture(&mut d,"m",||42).unwrap();assert_eq!(d.switches,vec![1]);}
    #[test] fn stops_at_twenty_pages(){let mut d=fake(1,(0..20).map(|n|solid(n as u8*10)).collect());let c=run_capture(&mut d,"m",||42).unwrap();assert_eq!(c.pages.len(),20);assert_eq!(d.saved.len(),20);}
    #[test] fn wait_result_returns_matching_operation_or_times_out() {
        let shared = ProofShared::default();
        shared.push_result("other", 9, 1);
        shared.push_result("wanted", 1, 42);
        assert_eq!(shared.wait_result("wanted", Duration::from_millis(10)), Some((1,42)));
        assert_eq!(shared.wait_result("missing", Duration::from_millis(1)), None);
    }
}
