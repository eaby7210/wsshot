// wsshot X [outfile] — switch to i3 ws X, screenshot it, switch back.
use serde_json::Value;
use std::{io::{Read, Write}, os::unix::net::UnixStream, process::exit, thread::sleep, time::Duration};
use x11rb::{connection::Connection, protocol::xproto::*, rust_connection::RustConnection};

fn ipc(s: &mut UnixStream, ty: u32, payload: &str) -> Value {
    let mut m = b"i3-ipc".to_vec();
    m.extend((payload.len() as u32).to_ne_bytes());
    m.extend(ty.to_ne_bytes());
    m.extend(payload.as_bytes());
    s.write_all(&m).unwrap();
    let mut h = [0u8; 14];
    s.read_exact(&mut h).unwrap();
    let mut b = vec![0u8; u32::from_ne_bytes(h[6..10].try_into().unwrap()) as usize];
    s.read_exact(&mut b).unwrap();
    serde_json::from_slice(&b).unwrap_or(Value::Null)
}

fn windows(v: &Value, out: &mut Vec<u32>) {
    if let Some(w) = v["window"].as_u64() { out.push(w as u32) }
    for k in ["nodes", "floating_nodes"] {
        for c in v[k].as_array().into_iter().flatten() { windows(c, out) }
    }
}

fn find_ws<'a>(v: &'a Value, n: i64) -> Option<&'a Value> {
    if v["type"] == "workspace" && v["num"] == n { return Some(v) }
    ["nodes", "floating_nodes"].iter()
        .flat_map(|k| v[*k].as_array().into_iter().flatten())
        .find_map(|c| find_ws(c, n))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let Some(x) = a.get(1).and_then(|s| s.parse::<i64>().ok()) else {
        eprintln!("usage: wsshot <workspace-number> [outfile]"); exit(2)
    };
    let out = a.get(2).cloned().unwrap_or_else(|| {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let dir = format!("{}/Pictures", std::env::var("HOME").unwrap_or_else(|_| ".".into()));
        let _ = std::fs::create_dir_all(&dir);
        format!("{dir}/wsshot-{x}-{ts}.png")
    });

    let sock = std::env::var("I3SOCK").unwrap_or_else(|_| {
        String::from_utf8(std::process::Command::new("i3").arg("--get-socketpath").output().unwrap().stdout).unwrap().trim().into()
    });
    let mut s = UnixStream::connect(sock).unwrap();
    let wss = ipc(&mut s, 1, "");
    let Some(w) = wss.as_array().unwrap().iter().find(|w| w["num"] == x) else {
        eprintln!("wsshot: workspace {x} does not exist"); exit(1)
    };
    let orig = wss.as_array().unwrap().iter().find(|w| w["focused"] == true).map(|w| w["num"].as_i64().unwrap());
    let r = &w["rect"];
    let g = |k: &str| r[k].as_i64().unwrap() as i16;
    let (gx, gy, gw, gh) = (g("x"), g("y"), g("width") as u16, g("height") as u16);
    let hidden = w["visible"] == false;

    let (conn, scr) = RustConnection::connect(None).unwrap();
    let root = conn.setup().roots[scr].root;

    if hidden {
        let tree = ipc(&mut s, 4, "");
        let mut wins = vec![];
        windows(find_ws(&tree, x).unwrap(), &mut wins);
        ipc(&mut s, 0, &format!("workspace number {x}"));
        for _ in 0..400 { // wait until a window of the ws is mapped
            if wins.iter().any(|&w| conn.get_window_attributes(w).unwrap().reply().map(|a| a.map_state == MapState::VIEWABLE).unwrap_or(false)) { break }
            sleep(Duration::from_millis(1));
        }
        sleep(Duration::from_millis(60)); // ponytail: repaint wait after map
    }
    let img = conn.get_image(ImageFormat::Z_PIXMAP, root, gx, gy, gw, gh, !0).unwrap().reply().unwrap();
    if hidden { if let Some(o) = orig { ipc(&mut s, 0, &format!("workspace number {o}")); } }

    // ponytail: assumes 32bpp BGRX root (true for depth 24/32 little-endian)
    let rgb: Vec<u8> = img.data.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0]]).collect();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(std::fs::File::create(&out).unwrap()), gw as u32, gh as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    enc.write_header().unwrap().write_image_data(&rgb).unwrap();
    println!("{out}");
}
