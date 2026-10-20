//! 通知图标的像素格：照网页 `packages/mascot/model.js` 的 `render` 一格一格算（2026-10-20 项目主人看了图标说「感觉还是怪怪的，你
//! 真的是照着webui的那个做的吗」：原来借终端字符画那一套算、再照亮度字换成档，明暗和耳根的轮廓都和网页对不上）。只画通知用的
//! 那一种：正面、睁着眼、合着嘴、不动。一格打四根光线，两根以上落在眼睛、锯齿线、肚子上的圈的照它；别的照打中的那几根的平均
//! 亮度分档，打中得太少的不画。

use crate::mascot::{Face, Look, Part};

/// 一格里四根光线的落点：左右、上下都对称，正脸画出来才左右对称。
const SAMPLES: [[f64; 2]; 4] = [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]];
/// 四根里几根落在同一样上才算它。
const ENOUGH: usize = 2;
/// 投票时的先后：一样多的照前面的（同网页）。
const ORDER: [Part; 3] = [Part::Head, Part::Ear, Part::Fin];

/// 一格画什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Px {
    /// 头、耳朵、鳍：第几档亮（0 最暗）。
    Body(Part, usize),
    /// 眼睛。
    Eye,
    /// 锯齿线（嘴）。
    Line,
    /// 肚子上的圈。
    Belly,
}

/// 多少格、头的半径占几格、头的中心在第几行、亮暗分几档（同网页的 `cols`、`rows`、`radius`、`center_row`、`levels`）。
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub radius: f64,
    pub center_row: f64,
    pub levels: usize,
}

type Mat = [[f64; 3]; 3];

/// 摆好的一块。
struct Piece {
    part: Part,
    turn: Mat,
    center: [f64; 3],
    radii: [f64; 3],
}

/// 一格里四根光线落在哪。
#[derive(Default)]
struct Tally {
    parts: [usize; 3],
    lum: f64,
    lit: usize,
    eye: usize,
    line: usize,
    belly: usize,
}

/// 照 `look` 的形状、`face` 的脸画成的格子：`rows` 行、每行 `cols` 格，没打中的是 `None`。
pub fn render(look: &Look, face: &Face, grid: &Grid) -> Vec<Vec<Option<Px>>> {
    let pieces = placed(look);
    let light = normalize(look.light);
    let half = grid.cols as f64 / 2.0;
    let top = grid.levels.saturating_sub(1).max(1);
    (0..grid.rows)
        .map(|r| {
            (0..grid.cols)
                .map(|c| {
                    let mut tally = Tally::default();
                    for [dx, dy] in SAMPLES {
                        let x = (c as f64 + dx - half) / grid.radius;
                        let y = -(r as f64 + dy - grid.center_row) / grid.radius;
                        let Some((part, normal, point)) = shoot(&pieces, x, y) else {
                            continue;
                        };
                        let mark = (part == Part::Head).then(|| on_face(face, point)).flatten();
                        match mark {
                            Some(Px::Eye) => tally.eye += 1,
                            Some(Px::Line) => tally.line += 1,
                            Some(_) => tally.belly += 1,
                            None => {
                                tally.parts[order(part)] += 1;
                                tally.lit += 1;
                                tally.lum += look.ambient
                                    + (1.0 - look.ambient) * dot(normal, light).max(0.0);
                            }
                        }
                    }
                    cell_of(&tally, look.ambient, top)
                })
                .collect()
        })
        .collect()
}

fn order(part: Part) -> usize {
    ORDER.iter().position(|p| *p == part).unwrap_or(0)
}

/// 定这一格：眼睛、锯齿线、肚子上的圈够两根的照它；别的照平均亮度分档，打中得太少的不画。
fn cell_of(t: &Tally, ambient: f64, top: usize) -> Option<Px> {
    if t.eye >= ENOUGH {
        return Some(Px::Eye);
    }
    if t.line >= ENOUGH {
        return Some(Px::Line);
    }
    if t.belly >= ENOUGH {
        return Some(Px::Belly);
    }
    if t.lit + t.eye + t.line + t.belly < ENOUGH || t.lit == 0 {
        return None;
    }
    let bright = (t.lum / t.lit as f64 - ambient) / (1.0 - ambient);
    let level = (bright * top as f64).round().clamp(0.0, top as f64) as usize;
    let best = (0..ORDER.len()).fold(
        0,
        |best, i| if t.parts[i] > t.parts[best] { i } else { best },
    );
    Some(Px::Body(ORDER[best], level))
}

/// 每一块摆好：正面不转，照 `tilt` 绕 z 歪；左右各一个的照 x 镜像再放一个。
fn placed(look: &Look) -> Vec<Piece> {
    let mut out = Vec::new();
    for shape in &look.shapes {
        let c = shape.center;
        out.push(Piece {
            part: shape.part,
            turn: rot_z(shape.tilt),
            center: c,
            radii: shape.radii,
        });
        if shape.mirror {
            out.push(Piece {
                part: shape.part,
                turn: rot_z(-shape.tilt),
                center: [-c[0], c[1], c[2]],
                radii: shape.radii,
            });
        }
    }
    out
}

/// 一根光线从 `(x, y)` 朝 −z 打进去，停在最近的面上：打中哪一块、那里的法线、那一点。
fn shoot(pieces: &[Piece], x: f64, y: f64) -> Option<(Part, [f64; 3], [f64; 3])> {
    let origin = [x, y, 10.0];
    let dir = [0.0, 0.0, -1.0];
    let mut best: Option<(f64, Part, [f64; 3])> = None;
    for p in pieces {
        let o = div(mul_t(&p.turn, sub(origin, p.center)), p.radii);
        let d = div(mul_t(&p.turn, dir), p.radii);
        let a = dot(d, d);
        let b = 2.0 * dot(o, d);
        let c = dot(o, o) - 1.0;
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            continue;
        }
        let t = (-b - disc.sqrt()) / (2.0 * a);
        if t <= 0.0 || best.is_some_and(|(bt, _, _)| bt <= t) {
            continue;
        }
        let on = [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t];
        best = Some((t, p.part, normalize(mul(&p.turn, div(on, p.radii)))));
    }
    best.map(|(t, part, normal)| (part, normal, [x, y, 10.0 - t]))
}

/// 头上这一点落没落在脸上（只看正面）：眼睛、锯齿线、肚子上的圈。
fn on_face(face: &Face, p: [f64; 3]) -> Option<Px> {
    if p[2] <= 0.0 {
        return None;
    }
    let q = [p[0], p[1]];
    // 眼睛是竖着的长方形：左右各 `eye_width`、上下照两头（2026-10-20 项目主人：「眼睛现在吉祥物应该是双竖线，长方形」；原来照
    // 线段量，两头是圆的，正对着看是三角；网页同一个判法，miyu-web 5e8bd89）。
    let eye = |[a, b]: &[[f64; 2]; 2]| {
        (q[0] - a[0]).abs() < face.eye_width && q[1] >= a[1].min(b[1]) && q[1] <= a[1].max(b[1])
    };
    if face.eyes.iter().any(eye) {
        return Some(Px::Eye);
    }
    if face
        .line
        .windows(2)
        .any(|w| segment(q, w[0], w[1]) < face.line_width)
    {
        return Some(Px::Line);
    }
    let m = &face.belly;
    let dx = (q[0] - m.center[0]).abs() - (m.half[0] - m.round);
    let dy = (q[1] - m.center[1]).abs() - (m.half[1] - m.round);
    let outside = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0);
    (outside <= m.round && outside >= m.round - m.ring).then_some(Px::Belly)
}

/// 点到线段的距离。
fn segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [p[0] - a[0], p[1] - a[1]];
    let len = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len > 0.0 {
        ((ap[0] * ab[0] + ap[1] * ab[1]) / len).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (ap[0] - ab[0] * t).hypot(ap[1] - ab[1] * t)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn div(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] / b[0], a[1] / b[1], a[2] / b[2]]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let len = dot(a, a).sqrt();
    if len > 0.0 {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        a
    }
}

/// `m · v`
fn mul(m: &Mat, v: [f64; 3]) -> [f64; 3] {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

/// `mᵀ · v`：旋转的逆。
fn mul_t(m: &Mat, v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

fn rot_z(a: f64) -> Mat {
    [
        [a.cos(), -a.sin(), 0.0],
        [a.sin(), a.cos(), 0.0],
        [0.0, 0.0, 1.0],
    ]
}
