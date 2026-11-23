use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode { #[default] Full, Compact }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    #[serde(default)] pub mode: Mode,
    #[serde(default)] pub pinned: bool,
    #[serde(default)] pub full: Option<Rect>,
    #[serde(default)] pub compact: Option<Rect>,
}
impl Default for WindowState { fn default() -> Self { Self { mode: Mode::Full, pinned: false, full: None, compact: None } } }
pub fn load(path: impl AsRef<Path>) -> WindowState { fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default() }
pub fn save(path: impl AsRef<Path>, state: &WindowState) { if let Some(parent)=path.as_ref().parent(){let _=fs::create_dir_all(parent);} if let Ok(b)=serde_json::to_vec_pretty(state){let _=fs::write(path,b);} }
pub const fn compact_default() -> (u32,u32) { (380,260) }
pub const fn compact_min() -> (u32,u32) { (320,200) }
pub const fn full_default() -> (u32,u32) { (1280,820) }
pub fn usable(rect: Rect, monitors: &[Rect]) -> bool {
    monitors.iter().any(|m| {
        let left=rect.x.max(m.x) as i64; let top=rect.y.max(m.y) as i64;
        let right=(rect.x as i64+rect.width as i64).min(m.x as i64+m.width as i64);
        let bottom=(rect.y as i64+40).min(m.y as i64+m.height as i64);
        right-left>=80 && bottom-top>=40
    })
}
pub fn remember_rect(state: &mut WindowState, mode: Mode, rect: Rect) {
    match mode { Mode::Full => state.full=Some(rect), Mode::Compact => state.compact=Some(rect) }
}
pub fn target_rect_for(state: &WindowState, mode: Mode, current_top_left: (i32,i32), monitors: &[Rect]) -> Rect {
    match mode {
        Mode::Compact => state.compact.map(|r|Rect{x:current_top_left.0,y:current_top_left.1,..r}).unwrap_or_else(||{let (width,height)=compact_default();Rect{x:current_top_left.0,y:current_top_left.1,width,height}}),
        Mode::Full => state.full.filter(|r|usable(*r,monitors)).unwrap_or_else(||{let (width,height)=full_default();Rect{x:current_top_left.0,y:current_top_left.1,width,height}}),
    }
}
#[cfg(test)] mod tests {
 use super::*; use std::time::{SystemTime,UNIX_EPOCH};
 fn path()->std::path::PathBuf { std::env::temp_dir().join(format!("window-state-{}-{}.json",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())) }
 #[test] fn default_on_missing_or_corrupt(){let p=path();assert_eq!(load(&p),WindowState::default());fs::write(&p,"{").unwrap();assert_eq!(load(&p),WindowState::default());let _=fs::remove_file(p);}
 #[test] fn round_trip(){let p=path();let s=WindowState{mode:Mode::Compact,pinned:true,full:Some(Rect{x:1,y:2,width:900,height:600}),compact:None};save(&p,&s);assert_eq!(load(&p),s);let _=fs::remove_file(p);}
 #[test] fn offscreen_rect_rejected(){assert!(!usable(Rect{x:0,y:0,width:300,height:200},&[Rect{x:500,y:500,width:1000,height:800}]));}
 #[test] fn partially_visible_rect_kept(){assert!(usable(Rect{x:-300,y:10,width:500,height:300},&[Rect{x:0,y:0,width:1200,height:800}]));}
 #[test] fn remember_rect_updates_requested_mode(){let mut s=WindowState::default();let r=Rect{x:4,y:5,width:6,height:7};remember_rect(&mut s,Mode::Compact,r);assert_eq!(s.compact,Some(r));assert_eq!(s.full,None);}
 #[test] fn target_rect_uses_compact_size_and_current_position(){let mut s=WindowState::default();s.compact=Some(Rect{x:100,y:100,width:400,height:300});assert_eq!(target_rect_for(&s,Mode::Compact,(10,20),&[]),Rect{x:10,y:20,width:400,height:300});}
 #[test] fn target_rect_rejects_offscreen_full_rect(){let mut s=WindowState::default();s.full=Some(Rect{x:9000,y:9000,width:900,height:600});assert_eq!(target_rect_for(&s,Mode::Full,(0,0),&[Rect{x:0,y:0,width:1200,height:800}]),Rect{x:0,y:0,width:1280,height:820});}
}
