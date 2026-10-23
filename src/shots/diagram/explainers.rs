//! The explainer diagrams of `bench/remotion/src/diagrams/Explainers.tsx`.

use std::f32::consts::PI;

use anyhow::{Context, Result};
use skia_safe::{Color, ContourMeasureIter, PaintCap, PaintJoin, Path};

use super::{
    kit::{halo, label, mono, person, Kit},
    node::{Anchor, Anim, Fill, Node, DRAW_SECONDS},
    step_param, Sheet,
};
use crate::{
    ease::{Ease, SETTLE},
    film::Shot,
    palette::{hex, INK, INK_SOFT, LINE, OCHRE, PAPER, TEAL, TERRACOTTA, WATER, WHITE},
    text::{Family, Style},
};

const CARD: Color = hex(0xFBF4E6);

fn draw(seconds: f32) -> Anim {
    Anim::draw(seconds)
}

/// A shape drawn under a spec, as `data-s` + `data-a="draw"` with the default duration.
fn drawn(path: Node, spec: &str) -> Node {
    path.reveal(spec, draw(DRAW_SECONDS))
}

/* ------------------------------------------------------------ Jaivana to scale */

pub fn jaivana_scale(k: &Kit, _: &Shot) -> Result<Sheet> {
    let (m, ground, dx) = (158.0_f32, 880.0_f32, -230.0_f32);
    let bx0 = 470.0 + dx;
    let bx1 = bx0 + 6.15 * m;
    let by = 640.0;
    let rb = 0.89 / 2.0 * m;
    let rm = 0.70 / 2.0 * m;

    let wheel = |cx: f32, r: f32, ghost: bool| -> Result<Node> {
        let cy = ground - r;
        let tyre = Node::circle(cx, cy, r);
        if ghost {
            return Ok(tyre.no_fill().stroke(TERRACOTTA, 4.0).dashed(14.0, 10.0));
        }
        let mut parts = vec![tyre.fill(hex(0x4A3A2C)).stroke(INK, 6.0)];
        for i in 0..12 {
            let a = i as f32 * PI / 6.0;
            parts.push(Node::line(cx, cy, cx + a.cos() * r * 0.9, cy + a.sin() * r * 0.9).stroke(hex(0x8E7A62), 4.0));
        }
        parts.push(Node::circle(cx, cy, r * 0.18).fill(INK));
        Ok(Node::group(parts))
    };

    let mut barrel = vec![
        Node::path(&format!("M{} {} L{} {} L{} {} L{} {} Z", bx0 + 120.0, by + 40.0, bx1 - 140.0, by + 30.0, bx1 - 220.0, ground - 120.0, bx0 + 220.0, ground - 120.0))?
            .fill(hex(0x6B7D72))
            .stroke(INK, 5.0),
        wheel(bx0 + 330.0, 0.685 * m, false)?,
        wheel(bx1 - 300.0, 0.685 * m, false)?,
        Node::path(&format!("M{bx0} {} L{bx1} {} L{bx1} {} L{bx0} {} Z", by - rb, by - rm, by + rm, by + rb))?
            .fill(hex(0x7A4E33))
            .stroke(INK, 6.0)
            .join(PaintJoin::Round),
    ];
    for f in [0.18_f32, 0.5, 0.82] {
        barrel.push(
            Node::path(&format!("M{} {} V{}", bx0 + f * (bx1 - bx0), by - rb + (rb - rm) * f - 6.0, by + rb - (rb - rm) * f + 6.0))?.stroke(hex(0x5A3622), 10.0),
        );
    }
    barrel.push(Node::circle(bx0 - 18.0, by, 30.0).fill(hex(0x7A4E33)).stroke(INK, 6.0));
    barrel.push(Node::ellipse(bx1, by, 14.0, rm).fill(hex(0x2A1A12)).stroke(INK, 5.0));

    let child = Node::group([
        person(1180.0, ground, 1.37 * m, hex(0x8E6F52))?,
        k.centred("child ≈ main wheel", 1180.0, ground + 50.0, label(28.0, INK_SOFT, 600.0))?,
        k.centred("≈ 1.37 m", 1180.0, ground + 84.0, mono(20.0, TERRACOTTA))?,
    ]);
    let ghost = Node::group([
        wheel(1620.0, 1.37 * m, true)?,
        k.centred("transport wheel ≈ 2.74 m", 1620.0, ground - 2.74 * m - 24.0, halo(label(30.0, TERRACOTTA, 700.0), PAPER))?,
    ]);
    let adult = Node::group([
        person(1385.0, ground, 1.75 * m, INK)?,
        k.centred("adult", 1385.0, ground + 50.0, label(28.0, INK_SOFT, 600.0))?,
        k.centred("≈ 1.75 m", 1385.0, ground + 84.0, mono(20.0, TERRACOTTA))?,
    ]);
    Ok(Sheet {
        dark: false,
        tag: "JAIVANA · TO SCALE",
        legend: Some("REPORTED DIMENSIONS · SILHOUETTE SIMPLIFIED"),
        step: 1,
        nodes: vec![
            Node::path(&format!("M60 {ground} H1860"))?.stroke(INK, 4.0),
            Node::group(barrel).reveal("1:t0.15", Anim::up()),
            k.dim((bx0, by - rb - 60.0), (bx1, by - rm - 60.0), "6.15 m barrel", Some("1:t0.9"))?,
            child.reveal("1:0", Anim::up()),
            ghost.reveal("1:1-0.3", Anim::fade()),
            adult.reveal("1:1", Anim::up()),
        ],
    })
}

/* ------------------------------------------------------------ family tree */

fn display(size: f32, weight: f32, letter_spacing: f32) -> Style {
    Style { letter_spacing, ..Style::new(Family::Fraunces, size, weight, INK) }
}

struct Card<'a> {
    at: (f32, f32),
    name: &'a str,
    sub: Option<&'a str>,
    spec: &'a str,
    highlight: Option<Color>,
    width: f32,
}

fn card(k: &Kit, p: Card) -> Result<Node> {
    let (x, y) = p.at;
    let frame = Node::rect(x - p.width / 2.0, y - 66.0, p.width, 132.0)
        .rounded(10.0)
        .fill(CARD)
        .stroke(p.highlight.unwrap_or(LINE), if p.highlight.is_some() { 5.0 } else { 2.5 });
    let mut parts = vec![frame, k.centred(p.name, x, y + if p.sub.is_some() { -2.0 } else { 14.0 }, display(46.0, 500.0, -1.0))?];
    if let Some(sub) = p.sub {
        parts.push(k.centred(sub, x, y + 40.0, mono(19.0, TERRACOTTA))?);
    }
    Ok(Node::group(parts).reveal(p.spec, Anim::up()))
}

pub fn family(k: &Kit, shot: &Shot) -> Result<Sheet> {
    let step = step_param(shot, 1)?;
    let (b, bd, mz, ak, ms, jh) = ((960.0, 210.0), (430.0, 490.0), (1080.0, 490.0), (1600.0, 490.0), (430.0, 790.0), (1340.0, 790.0));
    let member = |at, name, sub, spec, highlight, width| card(k, Card { at, name, sub: Some(sub), spec, highlight, width });

    let jodha = Node::group([
        Node::group([k.centred("“Jodha Bai”", mz.0, mz.1 + 14.0, Style { italic: true, color: INK_SOFT, ..display(44.0, 400.0, 0.0) })?])
            .reveal("1:t0.6", Anim::fade()),
        drawn(Node::path(&format!("M{} {} L{} {}", mz.0 - 130.0, mz.1 + 2.0, mz.0 + 130.0, mz.1 - 2.0))?.no_fill().stroke(TERRACOTTA, 7.0).cap(PaintCap::Round), "1:0-1.6"),
    ])
    .hide("1:0-0.1");
    let title = Node::group([
        Node::rect(mz.0 - 196.0, mz.1 - 62.0, 392.0, 124.0).rounded(8.0).fill(CARD),
        k.centred("Mariam-uz-Zamani", mz.0, mz.1 - 2.0, display(40.0, 500.0, 0.0))?,
        k.centred("HER TITLE", mz.0, mz.1 + 40.0, mono(18.0, TERRACOTTA))?,
    ]);
    let daughter = Node::group([
        Node::rect(mz.0 - 200.0, mz.1 - 66.0, 400.0, 132.0).rounded(10.0).fill(CARD).stroke(OCHRE, 5.0),
        k.text("DAUGHTER", mz.0 + 120.0, mz.1 - 84.0, Anchor::Start, mono(17.0, INK_SOFT))?,
    ]);
    let marriage_x = (mz.0 + ak.0) / 2.0;

    Ok(Sheet {
        dark: false,
        tag: "THE KACHHWAHA–MUGHAL FAMILY",
        legend: None,
        step,
        nodes: vec![
            drawn(
                Node::path(&format!("M{} {} V330 H{} V{} M{} 330 H{} V{}", b.0, b.1 + 66.0, bd.0, bd.1 - 66.0, b.0, mz.0, mz.1 - 66.0))?.no_fill().stroke(INK_SOFT, 4.0),
                "1:t0.3",
            ),
            drawn(
                Node::path(&format!("M{} {} H{} M{} {} H{}", mz.0 + 190.0, mz.1 - 10.0, ak.0 - 190.0, mz.0 + 190.0, mz.1 + 10.0, ak.0 - 190.0))?.stroke(TERRACOTTA, 4.0),
                "1:t0.8",
            ),
            k.centred("1562", marriage_x + 5.0, mz.1 + 52.0, mono(17.0, TERRACOTTA))?.reveal("1:t1.0", Anim::fade()),
            member(b, "Raja Bharmal", "RULER OF AMBER", "1:t0.1", None, 380.0)?,
            member(bd, "Bhagwant Das", "SON", "1:t0.3", None, 380.0)?,
            member(ak, "Akbar", "MUGHAL EMPEROR", "1:t0.8", None, 340.0)?,
            daughter.reveal("1:t0.5", Anim::up()),
            jodha,
            k.centred("NOT IN MUGHAL CHRONICLES", mz.0, mz.1 + 116.0, mono(18.0, TERRACOTTA))?.reveal("1:0-1.4", Anim::fade()),
            title.reveal("1:0", Anim::up()),
            drawn(Node::path(&format!("M{marriage_x} {} V{}", mz.1 + 10.0, jh.1 - 66.0))?.no_fill().stroke(INK_SOFT, 4.0), "2:0-0.3"),
            member(jh, "Jahangir", "NEXT EMPEROR · 1605", "2:0", None, 380.0)?,
            Node::rect(mz.0 - 230.0, mz.1 - 96.0, ak.0 - mz.0 + 420.0, jh.1 - mz.1 + 196.0)
                .rounded(20.0)
                .fill(Color::from_argb(36, 224, 169, 59))
                .stroke(OCHRE, 3.0)
                .dashed(12.0, 10.0)
                .reveal("2:1", Anim::fade()),
            drawn(Node::path(&format!("M{} {} V{}", bd.0, bd.1 + 66.0, ms.1 - 66.0))?.no_fill().stroke(TERRACOTTA, 6.0), "3:0"),
            member(ms, "Man Singh", "GRANDSON · GENERAL", "3:1-0.2", Some(TERRACOTTA), 380.0)?,
        ],
    })
}

/* ------------------------------------------------------------ seeing through the jali */

pub fn jali_view(k: &Kit, _: &Shot) -> Result<Sheet> {
    let ground = 860.0_f32;
    let eye = (1330.0_f32, 392.0_f32);
    let crowd: [(f32, f32); 8] = [(240.0, 0.0), (330.0, 1.0), (420.0, 0.0), (520.0, 1.0), (610.0, 0.0), (700.0, 1.0), (800.0, 0.0), (890.0, 1.0)];

    let people = crowd.iter().map(|&(x, v)| person(x, ground, 150.0 + v * 14.0, hex(0x6E5A47))).collect::<Result<Vec<_>>>()?;
    let gate = Node::group([
        Node::path(&format!("M1180 {ground} V520 H1700 V{ground}"))?.fill(hex(0xE6CFA2)).stroke(INK, 5.0),
        Node::path(&format!("M1360 {ground} V700 Q1440 620 1520 700 V{ground}"))?.fill(hex(0x4A3A2C)),
        Node::path("M1220 520 V330 H1660 V520")?.fill(hex(0xEEDDBA)).stroke(INK, 5.0),
        Node::path("M1200 330 Q1440 230 1680 330")?.fill(hex(0xE0C48E)).stroke(INK, 5.0),
        k.centred("Ganesh Pol", 1440.0, 600.0, label(30.0, INK, 600.0))?,
        k.centred("Suhag Mandir", 1440.0, 300.0, halo(label(30.0, INK, 600.0), hex(0xE0C48E)))?,
    ]);
    let mut lattice: Vec<Node> = (0..6)
        .flat_map(|r| (0..2).map(move |_| Node::circle(1222.0, 350.0 + r as f32 * 28.0, 9.0).no_fill().stroke(INK, 3.0)))
        .collect();
    lattice.push(Node::rect(1212.0, 338.0, 20.0, 176.0).filled(Fill::Cross { color: INK, cell: 12.0, width: 2.0 }).stroke(INK, 4.0));

    let mut nodes = vec![
        Node::path(&format!("M60 {ground} H1180"))?.stroke(INK, 4.0),
        k.centred("DIWAN-I-AAM COURTYARD", 560.0, ground + 56.0, mono(22.0, INK_SOFT))?,
        Node::group(people).reveal("1:t0.2", Anim::up()),
        gate.reveal("1:t0.1", Anim::up()),
        Node::group(lattice).reveal("1:t0.5", Anim::fade()),
        Node::group([person(1330.0, 520.0, 150.0, TERRACOTTA)?]).reveal("1:t0.6", Anim::up()),
    ];
    for (i, &(x, _)) in crowd.iter().enumerate() {
        let spec = if i == 0 { "1:0".to_string() } else { format!("1:0+{:.2}", i as f32 * 0.06) };
        nodes.push(
            Node::path(&format!("M{} {} L{x} {}", eye.0, eye.1, ground - 140.0))?.no_fill().stroke(OCHRE, 3.0).reveal(&spec, draw(0.6)),
        );
    }
    nodes.push(k.centred("From inside: a clear view down", 760.0, 330.0, halo(label(34.0, TEAL, 700.0), PAPER))?.reveal("1:0+0.5", Anim::fade()));
    for (i, x) in [420.0_f32, 700.0].into_iter().enumerate() {
        let lift = i as f32 * 40.0;
        let sight = if i == 0 { "1:1" } else { "1:1+0.15" };
        nodes.push(
            Node::path(&format!("M{x} {} L1190 {}", ground - 160.0, 430.0 + lift))?.no_fill().stroke(TERRACOTTA, 4.0).dashed(10.0, 9.0).reveal(sight, draw(0.5)),
        );
        nodes.push(
            Node::path(&format!("M1178 {} l24 24 M1202 {} l-24 24", 418.0 + lift, 418.0 + lift))?
                .stroke(TERRACOTTA, 6.0)
                .reveal(&format!("1:1+{}", 0.45 + i as f32 * 0.15), Anim::pop()),
        );
    }
    nodes.push(k.centred("From outside: only stone lattice", 760.0, 420.0, halo(label(34.0, TERRACOTTA, 700.0), PAPER))?.reveal("1:1+0.6", Anim::fade()));
    Ok(Sheet { dark: false, tag: "SUHAG MANDIR · THE SCREENED GALLERY", legend: Some("SIMPLIFIED SECTION"), step: 1, nodes })
}

/* ------------------------------------------------------------ Ganesh Pol's bent entrance */

const ROUTE: &str = "M420 450 H860 V700 H1060 V660 H1500";

/// The walker's token running the whole route once, starting at `at` and taking `seconds`.
fn walker(at: f32, seconds: f32) -> Result<Node> {
    let route = Path::from_svg(ROUTE).context("route path")?;
    let measure = ContourMeasureIter::new(&route, false, None).next().context("empty route")?;
    Ok(Node::live(move |t| {
        let along = Ease::PowerInOut(1).apply((t - at) / seconds) * measure.length();
        let (at, _) = measure.pos_tan(along).context("a point on the route")?;
        Ok(Node::circle(at.x, at.y, 16.0).fill(TEAL).stroke(Color::WHITE, 4.0))
    })
    .enter(at, Anim::Fade { duration: 0.2, ease: SETTLE }))
}

pub fn ganesh_route(k: &Kit, shot: &Shot) -> Result<Sheet> {
    let step = step_param(shot, 1)?;
    let passage = ["M760 260 H1160 V600", "M1160 720 V820 H760 V500", "M760 400 V260", "M960 260 V610"]
        .into_iter()
        .map(|d| Ok(Node::path(d)?.stroke(INK, 22.0).cap(PaintCap::Square)))
        .collect::<Result<Vec<_>>>()?;
    let halls = Node::group([
        Node::rect(771.0, 271.0, 178.0, 538.0).fill(hex(0xF6EBD3)),
        Node::rect(971.0, 271.0, 178.0, 538.0).fill(hex(0xF6EBD3)),
        k.centred("HALL 1", 860.0, 300.0, mono(18.0, INK_SOFT))?,
        k.centred("HALL 2", 1060.0, 300.0, mono(18.0, INK_SOFT))?,
    ]);
    let numbered = |x: f32, y: f32, n: &str, spec: &str| -> Result<Node> {
        Ok(Node::group([Node::circle(x, y, 30.0).fill(OCHRE), k.centred(n, x, y + 11.0, label(30.0, INK, 800.0))?]).reveal(spec, Anim::pop()))
    };
    let attackers = [(820.0, 430.0), (835.0, 470.0), (800.0, 455.0), (850.0, 680.0), (830.0, 720.0)];
    let defenders = [(900.0, 360.0), (900.0, 600.0), (1010.0, 760.0)];

    let mut nodes = vec![
        Node::rect(120.0, 240.0, 560.0, 600.0).fill(hex(0xEADCC0)),
        Node::rect(1240.0, 240.0, 560.0, 600.0).fill(hex(0xE3D8BC)),
        k.centred("PUBLIC COURT", 400.0, 210.0, mono(22.0, INK_SOFT))?,
        k.centred("PRIVATE PALACE", 1520.0, 210.0, mono(22.0, INK_SOFT))?,
        Node::group(passage).no_fill().reveal("1:t0.1", Anim::fade()),
        halls.reveal("1:t0.1", Anim::fade()),
        Node::path("M420 450 H940")?.no_fill().stroke(TERRACOTTA, 7.0).dashed(16.0, 12.0).reveal("1:0", draw(0.8)),
        Node::path("M918 428 l44 44 M962 428 l-44 44")?.stroke(TERRACOTTA, 9.0).reveal("1:0+0.75", Anim::pop()),
        k.centred("No straight line through", 600.0, 410.0, halo(label(34.0, TERRACOTTA, 700.0), hex(0xEADCC0)))?.reveal("1:0+0.9", Anim::fade()),
        Node::path("M420 450 H860 V700")?.no_fill().stroke(TEAL, 9.0).join(PaintJoin::Round).reveal("2:0", draw(1.0)),
        Node::path("M860 700 H1060 V660 H1500")?.no_fill().stroke(TEAL, 9.0).join(PaintJoin::Round).reveal("2:1", draw(1.0)),
        numbered(860.0, 450.0, "1", "2:0+0.6")?,
        numbered(860.0, 700.0, "2", "2:1")?,
    ];
    if step >= 2 {
        let at = if step == 2 { shot.cue_at(0, None) } else { 0.0 };
        let seconds = if step == 2 { (shot.cue_at(1, None) - at + 1.2).max(1.5) } else { 2.4 };
        nodes.push(walker(at, seconds)?);
    }
    nodes.push(
        Node::group(
            attackers
                .iter()
                .map(|&(x, y)| Node::circle(x, y, 13.0).fill(TERRACOTTA).stroke(Color::WHITE, 3.0))
                .chain([k.centred("attackers bunch at the turns", 600.0, 560.0, halo(label(32.0, TERRACOTTA, 700.0), hex(0xEADCC0)))?]),
        )
        .reveal("3:0", Anim::fade()),
    );
    let mut guards = defenders
        .iter()
        .map(|&(x, y)| Ok(Node::path(&format!("M{x} {} L{} {} L{} {} Z", y - 18.0, x + 18.0, y + 14.0, x - 18.0, y + 14.0))?.fill(INK)))
        .collect::<Result<Vec<_>>>()?;
    guards.push(k.centred("defenders waiting", 1500.0, 560.0, halo(label(32.0, INK, 700.0), hex(0xE3D8BC)))?);
    nodes.push(Node::group(guards).reveal("3:1", Anim::fade()));
    Ok(Sheet {
        dark: false,
        tag: "GANESH POL · PLAN",
        legend: Some("SCHEMATIC PLAN · THE PASSAGE TURNS TWICE (UNESCO NOMINATION, 2013)"),
        step,
        nodes,
    })
}

/* ------------------------------------------------------------ mirror light */

pub fn mirror_light(k: &Kit, _: &Shot) -> Result<Sheet> {
    let (cx, cy, r) = (960.0_f32, 800.0_f32, 560.0_f32);
    let glow = || Fill::Fade(Color::from_argb(230, 0xFF, 0xD9, 0x8A));
    let tiles: Vec<(f32, f32, f32)> = (0..27)
        .map(|i| {
            let a = PI * (0.06 + 0.88 * i as f32 / 26.0);
            (a, cx - a.cos() * r, cy - a.sin() * r * 0.95)
        })
        .collect();

    let mut nodes = vec![
        Node::path(&format!("M{} {cy} A{} {} 0 0 1 {} {cy}", cx - r - 40.0, r + 40.0, (r + 40.0) * 0.95, cx + r + 40.0))?.no_fill().stroke(hex(0x5C4A3A), 26.0),
    ];
    for &(a, x, y) in &tiles {
        nodes.push(
            Node::rect(x - 17.0, y - 11.0, 34.0, 22.0).rounded(4.0).fill(hex(0x9AA7AE)).stroke(hex(0xD6E0E4), 2.0).rotated(a * 180.0 / PI - 90.0, x, y),
        );
    }
    nodes.push(
        Node::group([
            Node::circle(cx, cy - 40.0, 150.0).filled(glow()),
            Node::path(&format!(
                "M{cx} {} C{} {} {} {} {cx} {} C{} {} {} {} {cx} {} Z",
                cy - 92.0, cx + 26.0, cy - 54.0, cx + 18.0, cy - 22.0, cy - 18.0, cx - 18.0, cy - 22.0, cx - 26.0, cy - 54.0, cy - 92.0
            ))?
            .fill(hex(0xFFC75A)),
            Node::rect(cx - 34.0, cy - 18.0, 68.0, 22.0).rounded(6.0).fill(hex(0xB07A3A)),
        ])
        .reveal("1:0-0.2", Anim::pop()),
    );
    for (i, &(_, x, y)) in tiles.iter().enumerate() {
        nodes.push(
            Node::path(&format!("M{cx} {} L{x} {y}", cy - 55.0))?
                .no_fill()
                .stroke(Color::from_argb(115, 255, 210, 120), 2.0)
                .reveal(&format!("1:0+{:.2}", 0.25 + i as f32 * 0.03), draw(0.4)),
        );
    }
    for (i, &(_, x, y)) in tiles.iter().enumerate() {
        nodes.push(
            Node::group([Node::circle(x, y, 26.0).filled(glow()), Node::circle(x, y, 6.0).fill(hex(0xFFE7AE))])
                .reveal(&format!("1:1+{:.2}", i as f32 * 0.035), Anim::pop()),
        );
    }
    nodes.push(k.centred("one lamp", cx, cy + 120.0, label(36.0, WHITE, 600.0))?.reveal("1:0+0.4", Anim::fade()));
    nodes.push(k.centred("every mirror holds its own flame", cx, 170.0, label(40.0, hex(0xFFD98A), 700.0))?.reveal("1:1+0.8", Anim::fade()));
    Ok(Sheet { dark: true, tag: "SHEESH MAHAL · ONE FLAME, MANY REFLECTIONS", legend: Some("SIMPLIFIED: CONVEX GLASS PIECES SET IN PLASTER"), step: 1, nodes })
}

/* ------------------------------------------------------------ Sukh Niwas cooling */

/// A dashed stroke whose pattern flows along its path at `speed` pixels per second.
fn flowing(d: &'static str, color: Color, width: f32, pattern: (f32, f32), speed: f32) -> Node {
    Node::live(move |t| Ok(Node::path(d)?.no_fill().stroke(color, width).dashed(pattern.0, pattern.1).cap(PaintCap::Round).dash_offset(-t * speed)))
}

pub fn cooling(k: &Kit, shot: &Shot) -> Result<Sheet> {
    let step = step_param(shot, 1)?;
    let ground = 820.0_f32;
    let soft_blue = hex(0x9CC3CC);
    let slab = |color, width| Ok::<_, anyhow::Error>(Node::path("M310 400 L470 760")?.stroke(color, width).cap(PaintCap::Round));

    let mut nodes = vec![
        Node::path(&format!("M200 {ground} H1560"))?.stroke(INK, 5.0),
        Node::path(&format!("M200 {ground} V280 H300 V{ground}"))?.fill(hex(0xE3D2B0)).stroke(INK, 5.0),
        Node::path("M300 280 H1380 V330 H300")?.fill(hex(0xE3D2B0)).stroke(INK, 5.0),
        Node::path(&format!("M1380 330 V560 M1380 {ground} V700"))?.stroke(INK, 8.0),
        k.centred("SUKH NIWAS", 760.0, 260.0, mono(22.0, INK_SOFT))?,
        k.centred("GARDEN", 1690.0, ground - 20.0, mono(22.0, INK_SOFT))?,
        Node::group([
            slab(hex(0xF4F1EA), 34.0)?,
            slab(hex(0xC9C3B6), 34.0)?.dashed(6.0, 10.0).opacity(0.7),
            k.text("pierced marble slab", 560.0, 430.0, Anchor::Start, halo(label(30.0, INK, 600.0), PAPER))?,
        ])
        .reveal("1:0-0.2", Anim::up()),
        Node::group([
            flowing("M318 392 L476 758", WATER, 10.0, (22.0, 14.0), 60.0),
            k.text("water cascades over it", 560.0, 480.0, Anchor::Start, halo(label(30.0, WATER, 700.0), PAPER))?,
        ])
        .reveal("1:1", Anim::fade()),
    ];
    for i in 0..3 {
        let f = i as f32;
        let hot = if i == 0 { "2:0-0.6".to_string() } else { format!("2:0+{}", f * 0.2) };
        nodes.push(drawn(
            Node::path(&format!(
                "M1340 {y} C1000 {y} 700 {} 430 {}",
                520.0 + f * 60.0,
                560.0 + f * 50.0,
                y = 470.0 + f * 70.0
            ))?
            .no_fill()
            .stroke(TERRACOTTA, 7.0)
            .cap(PaintCap::Round),
            &hot,
        ));
        nodes.push(drawn(
            Node::path(&format!("M430 {} C620 {} 900 {} 1340 {}", 590.0 + f * 50.0, 640.0 + f * 40.0, 650.0 + f * 40.0, 640.0 + f * 50.0))?
                .no_fill()
                .stroke(TEAL, 7.0)
                .cap(PaintCap::Round),
            &format!("2:0+{}", 0.8 + f * 0.2),
        ));
    }
    nodes.push(k.centred("hot air in", 1060.0, 430.0, halo(label(30.0, TERRACOTTA, 700.0), PAPER))?.reveal("2:0-0.4", Anim::fade()));
    nodes.push(k.centred("cooler air out", 1060.0, 790.0, halo(label(30.0, TEAL, 700.0), PAPER))?.reveal("2:0+1.2", Anim::fade()));
    let mut vapour = (0..4)
        .map(|i| {
            let f = i as f32;
            Ok(Node::path(&format!("M{} {} q 12 -20 0 -40 q -12 -20 0 -40", 380.0 + f * 26.0, 700.0 - f * 70.0))?.no_fill().stroke(soft_blue, 4.0))
        })
        .collect::<Result<Vec<_>>>()?;
    vapour.push(k.text("evaporation absorbs heat", 300.0, 880.0, Anchor::Start, label(26.0, INK_SOFT, 600.0))?);
    nodes.push(Node::group(vapour).reveal("2:0+0.5", Anim::fade()));

    nodes.push(drawn(Node::path(&format!("M470 {} H1720", ground - 8.0))?.stroke(WATER, 14.0).cap(PaintCap::Round), "3:0"));
    nodes.push(
        Node::live(move |t| Ok(Node::path(&format!("M470 {} H1720", ground - 8.0))?.no_fill().stroke(hex(0xBFE0E8), 5.0).dashed(18.0, 16.0).dash_offset(-t * 90.0)))
            .reveal("3:0+0.6", Anim::fade()),
    );
    nodes.push(k.centred("marble channel through the floor", 900.0, ground + 50.0, label(28.0, WATER, 700.0))?.reveal("3:0+0.3", Anim::fade()));
    let mut garden = vec![
        Node::path(&format!("M1690 {} V720", ground - 8.0))?.stroke(WATER, 6.0),
    ];
    for kk in [-1.0_f32, 0.0, 1.0] {
        garden.push(Node::path(&format!("M1690 720 q {} -60 {} 10", kk * 40.0, kk * 80.0))?.no_fill().stroke(hex(0x7FB4C4), 5.0));
    }
    for x in [1600.0_f32, 1780.0] {
        garden.push(Node::group([Node::path(&format!("M{x} {ground} V760"))?.stroke(hex(0x6B5B3E), 6.0), Node::circle(x, 740.0, 34.0).fill(hex(0x6E8F5E))]));
    }
    nodes.push(Node::group(garden).reveal("3:1", Anim::up()));
    Ok(Sheet { dark: false, tag: "SUKH NIWAS · HOW THE ROOM WAS COOLED", legend: Some("SIMPLIFIED SECTION · NOT TO SCALE"), step, nodes })
}
