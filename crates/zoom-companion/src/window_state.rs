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
#[cfg(test)] mod tests {
 use super::*; use std::time::{SystemTime,UNIX_EPOCH};
 fn path()->std::path::PathBuf { std::env::temp_dir().join(format!("window-state-{}-{}.json",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())) }
 #[test] fn default_on_missing_or_corrupt(){let p=path();assert_eq!(load(&p),WindowState::default());fs::write(&p,"{").unwrap();assert_eq!(load(&p),WindowState::default());let _=fs::remove_file(p);}
 #[test] fn round_trip(){let p=path();let s=WindowState{mode:Mode::Compact,pinned:true,full:Some(Rect{x:1,y:2,width:900,height:600}),compact:None};save(&p,&s);assert_eq!(load(&p),s);let _=fs::remove_file(p);}
 #[test] fn offscreen_rect_rejected(){assert!(!usable(Rect{x:0,y:0,width:300,height:200},&[Rect{x:500,y:500,width:1000,height:800}]));}
 #[test] fn partially_visible_rect_kept(){assert!(usable(Rect{x:-300,y:10,width:500,height:300},&[Rect{x:0,y:0,width:1200,height:800}]));}
}
