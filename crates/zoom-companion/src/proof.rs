//! Pure image analysis and local capture path helpers.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Frame { width: u32, height: u32, bgra: Vec<u8> }

fn luma(bgra: &[u8], i: usize) -> u8 {
    ((bgra[i] as u32 * 29 + bgra[i+1] as u32 * 150 + bgra[i+2] as u32 * 77) >> 8) as u8
}

fn downsample_luma(frame: &Frame, cols: u32, rows: u32) -> Vec<u8> {
    if cols == 0 || rows == 0 || frame.width == 0 || frame.height == 0 { return vec![]; }
    let mut out = Vec::with_capacity((cols * rows) as usize);
    for gy in 0..rows { for gx in 0..cols {
        let x0 = gx * frame.width / cols; let x1 = ((gx+1)*frame.width/cols).max(x0+1).min(frame.width);
        let y0 = gy * frame.height / rows; let y1 = ((gy+1)*frame.height/rows).max(y0+1).min(frame.height);
        let (mut sum, mut n) = (0u64, 0u64);
        for y in y0..y1 { for x in x0..x1 { sum += luma(&frame.bgra, ((y*frame.width+x)*4) as usize) as u64; n+=1; } }
        out.push((sum/n.max(1)) as u8);
    }} out
}

fn frames_nearly_identical(a: &Frame, b: &Frame, threshold: f32) -> bool {
    if a.width != b.width || a.height != b.height || a.width == 0 || a.height == 0 { return false; }
    let (aa,bb)=(downsample_luma(a,64,36),downsample_luma(b,64,36));
    aa.iter().zip(&bb).map(|(x,y)|x.abs_diff(*y) as f32).sum::<f32>() / aa.len().max(1) as f32 <= threshold
}

fn count_dark_tiles(frame: &Frame) -> u32 {
    const W: usize=96; const H: usize=54;
    let g=downsample_luma(frame,W as u32,H as u32); let mut seen=vec![false;g.len()]; let mut count=0;
    for start in 0..g.len() { if seen[start] || g[start]>=18 {continue} let mut q=vec![start]; seen[start]=true; let(mut minx,mut maxx,mut miny,mut maxy)=(start%W,start%W,start/W,start/W);
      while let Some(i)=q.pop(){let x=i%W;let y=i/W;minx=minx.min(x);maxx=maxx.max(x);miny=miny.min(y);maxy=maxy.max(y);
        for j in [if x>0{Some(i-1)}else{None},if x+1<W{Some(i+1)}else{None},if y>0{Some(i-W)}else{None},if y+1<H{Some(i+W)}else{None}].into_iter().flatten(){if !seen[j]&&g[j]<18{seen[j]=true;q.push(j)}}
      }
      let bw=maxx-minx+1;let bh=maxy-miny+1;let area=bw*bh;
      if area as f32 >= W as f32*H as f32*0.015 {let ar=bw as f32/bh as f32;if (1.2..=2.2).contains(&ar){count+=1}}
    } count
}

#[derive(Clone, Debug, PartialEq)]
enum Decision { Wait, Accept{frame_index:usize,waited_ms:u64,dark_tiles:u32}, Timeout{frame_index:usize,waited_ms:u64,dark_tiles:u32} }
struct TileWait { frames: Vec<(Frame,u64)>, start_ms: Option<u64> }
impl TileWait {
 fn new()->Self{Self{frames:vec![],start_ms:None}}
 fn push(&mut self,frame:Frame,at_ms:u64)->Decision{
   let start=*self.start_ms.get_or_insert(at_ms);let current=count_dark_tiles(&frame);let prev=self.frames.last().map(|(f,_)|count_dark_tiles(f));self.frames.push((frame,at_ms));let idx=self.frames.len()-1;let waited=at_ms.saturating_sub(start);
   if idx>0 && frames_nearly_identical(&self.frames[idx-1].0,&self.frames[idx].0,2.0) && prev.is_some_and(|p|current>=p){return Decision::Accept{frame_index:idx,waited_ms:waited,dark_tiles:current}}
   if waited>=10_000{return Decision::Timeout{frame_index:idx,waited_ms:waited,dark_tiles:current}} Decision::Wait
 }
}
fn page_repeats(previous_pages:&[Vec<u8>],current:&[u8])->bool{previous_pages.iter().any(|p|p==current)}
fn captures_dir(local_app_data:&Path,instance_id:&str)->PathBuf{let clean:String=instance_id.chars().map(|c|if c.is_ascii_alphanumeric()||c=='-'||c=='_'{c}else{'_'}).collect();local_app_data.join("zoom-companion").join("captures").join(clean)}
fn page_file_name(index:usize,at_ms:u64)->String{format!("page-{:02}-{}.png",index,at_ms)}
fn encode_png(frame:&Frame)->Vec<u8>{
 let mut rgba=Vec::with_capacity(frame.width as usize*frame.height as usize*4);for p in frame.bgra.chunks_exact(4){rgba.extend_from_slice(&[p[2],p[1],p[0],p[3]])}
 let mut out=Vec::new();{let mut enc=png::Encoder::new(&mut out,frame.width,frame.height);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);let mut w=enc.write_header().expect("PNG header");w.write_image_data(&rgba).expect("PNG pixels");}out
}

#[cfg(test)] mod tests{
 use super::*;
 fn solid(w:u32,h:u32,v:u8)->Frame{let mut bgra=vec![v; (w*h*4) as usize];for p in bgra.chunks_exact_mut(4){p[3]=255;}Frame{width:w,height:h,bgra}}
 fn rect(f:&mut Frame,x0:u32,y0:u32,x1:u32,y1:u32,v:u8){for y in y0..y1{for x in x0..x1{let i=((y*f.width+x)*4)as usize;f.bgra[i..i+3].fill(v);}}}
 #[test]fn identical_noise_moved_and_size(){let a=solid(192,108,100);let mut b=a.clone();for p in b.bgra.chunks_exact_mut(4){p[..3].fill(101);}assert!(frames_nearly_identical(&a,&b,2.));let mut c=a.clone();rect(&mut c,10,10,70,40,255);assert!(!frames_nearly_identical(&a,&c,2.));assert!(!frames_nearly_identical(&a,&solid(190,108,100),2.));}
 #[test]fn dark_gallery_counts_and_bar(){for n in [0,2,5]{let mut f=solid(960,540,100);for i in 0..n{let x=20+(i%3)*310;let y=20+(i/3)*170;rect(&mut f,x,y,x+270,y+150,0);}assert_eq!(count_dark_tiles(&f),n)}let mut f=solid(960,540,100);rect(&mut f,0,200,960,220,0);assert_eq!(count_dark_tiles(&f),0)}
 #[test]fn tile_wait_state_machine(){let mut w=TileWait::new();assert_eq!(w.push(solid(192,108,100),0),Decision::Wait);assert!(matches!(w.push(solid(192,108,100),500),Decision::Accept{..}));let mut w=TileWait::new();let mut a=solid(960,540,100);for (t,n) in [(0,3),(500,2),(1000,1)]{a=solid(960,540,100);for i in 0..n{rect(&mut a,20+i*300,20,280+i*300,170,0);}assert_eq!(w.push(a.clone(),t),Decision::Wait);}assert!(matches!(w.push(a,1500),Decision::Accept{..}));let mut w=TileWait::new();for t in (0..=10_000).step_by(500){let f=solid(192,108,(100+(t/500%2)*30)as u8);let d=w.push(f,t);if t==10_000{assert!(matches!(d,Decision::Timeout{..}))}}
 }
 #[test]fn repeats_paths_and_names(){assert!(page_repeats(&[vec![1,2],vec![3]],&[3]));assert_eq!(captures_dir(Path::new("C:/Users/u/AppData/Local"),"a/b"),Path::new("C:/Users/u/AppData/Local/zoom-companion/captures/a_b"));assert_eq!(page_file_name(1,42),"page-01-42.png");}
 #[test]fn png_round_trip_dimensions(){let f=solid(4,3,90);let bytes=encode_png(&f);let decoder=png::Decoder::new(bytes.as_slice());let reader=decoder.read_info().unwrap();assert_eq!((reader.info().width,reader.info().height),(4,3));}
}
