//! The engineering diagrams: the three-stage water lift
//! and the palace plan.

use std::f32::consts::PI;

use anyhow::Result;
use skia_safe::{Color, PaintCap, PaintJoin};

use super::{
    kit::{halo, label, mono, Kit},
    node::{Anchor, Anim, Node},
    step_param, Sheet,
};
use crate::{
    ease::{Ease, SETTLE},
    film::Shot,
    palette::{hex, INK, INK_SOFT, OCHRE, PAPER, TEAL, TERRACOTTA, WATER},
};

const SOFT_BLUE: Color = hex(0x9CC3CC);
const SAND: Color = hex(0xD8C49C);

/* ------------------------------------------------------------ the three-stage water lift */

/// A draught ox walking its circuit, `phase` seconds into its stride.
fn ox(x: f32, y: f32, phase: f32) -> Node {
    Node::live(move |t| {
        let t = t + phase;
        let hide = hex(0x7B6048);
        let mut parts = vec![
            Node::ellipse(0.0, -34.0, 44.0, 24.0).fill(hide).stroke(INK, 3.0),
            Node::path("M38 -46 q22 -6 26 10 q-6 14 -22 10 Z")?.fill(hide).stroke(INK, 3.0),
            Node::path("M52 -52 q6 -12 14 -10")?.no_fill().stroke(INK, 3.0),
        ];
        for (i, lx) in [-26.0_f32, -10.0, 14.0, 30.0].into_iter().enumerate() {
            parts.push(Node::path(&format!("M{lx} -14 L{} 0", lx + (t * 6.0 + i as f32 * 1.7).sin() * 5.0))?.stroke(INK, 5.0).cap(PaintCap::Round));
        }
        Ok(Node::group(parts).placed(x + (t * 3.0).sin() * 10.0, y, false))
    })
}

/// A leather bag riding its rope from `y0` up toward `y1` and back.
fn bag(x: f32, y0: f32, y1: f32, phase: f32) -> Node {
    Node::live(move |t| {
        let k = ((t * 1.6 + phase).sin() + 1.0) / 2.0;
        let y = y0 - (y0 - y1) * k;
        Ok(Node::group([
            Node::path(&format!("M{x} {} V{y}", y1 - 40.0))?.stroke(INK, 2.0),
            Node::path(&format!("M{} {y} q12 26 24 0 Z", x - 12.0))?.fill(hex(0x8A5A36)).stroke(INK, 2.0),
        ]))
    })
}

/// A stroke whose dashes flow along it at `speed` pixels per second.
fn flow(d: &'static str, color: Color, width: f32, pattern: (f32, f32), speed: f32) -> Node {
    Node::live(move |t| Ok(Node::path(d)?.no_fill().stroke(color, width).dashed(pattern.0, pattern.1).dash_offset(-t * speed)))
}

/// The arrow head SVG's `marker-end` puts on a line ending at `(x, y)` and pointing up.
fn arrow_up(x: f32, y: f32, color: Color) -> Result<Node> {
    Ok(Node::path(&format!("M{x} {} L{} {} L{} {} Z", y - 15.0, x + 15.0, y + 15.0, x - 15.0, y + 15.0))?.fill(color))
}

pub fn water_lift(k: &Kit, shot: &Shot) -> Result<Sheet> {
    let step = step_param(shot, 0)?;
    let lake_y = 900.0_f32;
    let levels = [850.0_f32, 700.0, 550.0, 400.0, 250.0];
    let towers = [960.0_f32, 1130.0, 1300.0, 1470.0];
    let hill = "M520 1080 L520 905 C640 880 820 872 905 860 L985 860 L1010 718 L1150 712 L1180 566 L1320 560 L1350 414 L1490 408 L1520 262 L1880 236 L1920 236 L1920 1080 Z";
    let tank = |x: f32, y: f32, w: f32, h: f32, stroke: f32| Node::rect(x, y, w, h).fill(SOFT_BLUE).stroke(INK, stroke);

    let mut nodes = vec![
        Node::path(hill)?.fill(hex(0xD8C49C)).stroke(hex(0x9C8460), 4.0),
        Node::path(&format!("M40 {lake_y} H520 V1080 H40 Z"))?.fill(hex(0x7FA8B6)),
        Node::live(move |t| Ok(Node::path(&format!("M40 {lake_y} H520"))?.stroke(hex(0x4F7F92), 4.0).dashed(30.0, 18.0).dash_offset(-t * 70.0 * 0.3))),
        k.centred("Maota Lake", 250.0, 980.0, label(36.0, hex(0xF4EBD9), 700.0))?,
        Node::group([
            Node::path("M1700 236 V110 H1880 V236")?.fill(hex(0xEAD8B0)).stroke(INK, 5.0),
            Node::path("M1690 110 H1890 M1710 110 V86 H1735 V110 M1760 110 V86 H1785 V110 M1810 110 V86 H1835 V110 M1858 110 V86 H1880 V110")?.no_fill().stroke(INK, 5.0),
            k.centred("Palace", 1790.0, 70.0, label(32.0, INK, 600.0))?,
        ]),
    ];

    if step == 0 {
        nodes.push(
            Node::group([
                Node::path("M300 860 C 700 600, 1300 300, 1690 170")?.no_fill().stroke(TERRACOTTA, 6.0).dashed(18.0, 14.0),
                k.centred("?", 900.0, 520.0, halo(label(110.0, TERRACOTTA, 800.0), PAPER))?,
            ])
            .reveal("0:0", Anim::fade()),
        );
        let mut markers = Vec::new();
        for (x, y, n) in [(580.0, 820.0, "1"), (1200.0, 620.0, "2"), (1600.0, 300.0, "3")] {
            markers.push(Node::group([Node::circle(x, y, 40.0).fill(OCHRE).stroke(INK, 4.0), k.centred(n, x, y + 15.0, label(44.0, INK, 800.0))?]));
        }
        nodes.push(Node::group(markers).reveal("0:1", Anim::pop()));
    }

    // Stage 1: pulleys and leather bags into two terrace tanks.
    nodes.push(
        Node::group([
            Node::path("M540 905 V780 H640 V905")?.no_fill().stroke(INK, 6.0),
            Node::circle(560.0, 790.0, 14.0).fill(Color::WHITE).stroke(INK, 4.0),
            bag(560.0, 900.0, 800.0, 0.0),
            ox(700.0, 845.0, 0.0),
            tank(600.0, 812.0, 34.0, 30.0, 3.0),
            tank(640.0, 812.0, 34.0, 30.0, 3.0),
            k.centred("STAGE 1", 600.0, 690.0, halo(label(28.0, INK, 700.0), PAPER))?,
            k.centred("PULLEYS · LEATHER BAGS · TWO TANKS", 600.0, 740.0, halo(mono(18.0, INK_SOFT), PAPER))?,
        ])
        .reveal("1:0-0.4", Anim::up()),
    );
    // The clay pipeline between the terraces.
    nodes.push(
        Node::group([
            Node::path("M675 835 C760 845 860 852 945 852")?.no_fill().stroke(hex(0x9C6B45), 12.0),
            flow("M675 835 C760 845 860 852 945 852", SOFT_BLUE, 4.0, (14.0, 12.0), 70.0),
            k.centred("clay pipeline ≈ 125 m", 800.0, 905.0, halo(label(28.0, hex(0x7A4E33), 700.0), SAND))?,
        ])
        .reveal("2:0", Anim::fade()),
    );
    // Stage 2: four towers, each with a pulley, animals and a tank.
    for (i, &x) in towers.iter().enumerate() {
        let spec = if i == 0 { "3:0".to_string() } else { format!("3:0+{:.2}", i as f32 * 0.35) };
        let (low, high) = (levels[i], levels[i + 1]);
        nodes.push(
            Node::group([
                Node::path(&format!("M{} {low} V{} H{} V{}", x - 30.0, high - 30.0, x + 140.0, high + 10.0))?.no_fill().stroke(INK, 6.0),
                tank(x - 26.0, low - 26.0, 52.0, 26.0, 3.0),
                Node::circle(x, high - 30.0, 13.0).fill(Color::WHITE).stroke(INK, 4.0),
                ox(x + 85.0, high - 6.0, i as f32),
            ])
            .reveal(&spec, Anim::up()),
        );
    }
    nodes.push(k.centred("STAGE 2 · FOUR LIFTING TOWERS", 1060.0, 330.0, halo(label(28.0, INK, 700.0), PAPER))?.reveal("3:1", Anim::fade()));
    for (i, &x) in towers.iter().enumerate() {
        let spec = if i == 0 { "4:0".to_string() } else { format!("4:0+{:.1}", i as f32 * 0.3) };
        let (low, high) = (levels[i], levels[i + 1]);
        nodes.push(
            Node::group([
                bag(x, low - 6.0, high - 20.0, i as f32),
                Node::path(&format!("M{} {} V{}", x - 60.0, low - 20.0, high + 20.0))?.stroke(WATER, 6.0),
                arrow_up(x - 60.0, high + 20.0, WATER)?,
                k.text("10–13 m", x - 70.0, (low + high) / 2.0, Anchor::End, halo(label(24.0, WATER, 700.0), SAND))?,
            ])
            .reveal(&spec, Anim::fade()),
        );
    }
    // Stage 3: the Persian wheel.
    let wheel = Node::live(|t| {
        let mut parts = vec![Node::circle(1640.0, 214.0, 36.0).no_fill().stroke(INK, 6.0)];
        for j in 0..4 {
            let a = j as f32 * PI / 2.0;
            parts.push(Node::line(1640.0, 214.0, 1640.0 + a.cos() * 36.0, 214.0 + a.sin() * 36.0).stroke(INK, 4.0));
        }
        Ok(Node::group(parts).rotated(t * 1.4 * 180.0 / PI, 1640.0, 214.0))
    });
    let pots = Node::live(|t| {
        // Down the left side, round the bottom, up the right side, over the wheel.
        let (len, top, left, right) = (140.0_f32, 214.0_f32, 1615.0_f32, 1665.0_f32);
        Ok(Node::group((0..14).map(|i| {
            let u = (i as f32 / 14.0 + t * 0.12) % 1.0;
            let (x, y) = if u < 0.4 {
                (left, top + u / 0.4 * len)
            } else if u < 0.5 {
                let a = PI * ((u - 0.4) / 0.1);
                (1640.0 - a.cos() * 25.0, top + len + a.sin() * 18.0)
            } else if u < 0.9 {
                (right, top + len - (u - 0.5) / 0.4 * len)
            } else {
                let a = PI * ((u - 0.9) / 0.1);
                (1640.0 + a.cos() * 25.0, top - a.sin() * 25.0)
            };
            Node::circle(x, y, 8.0).fill(hex(0xA0623A)).stroke(INK, 2.0)
        })))
    });
    nodes.push(
        Node::group([
            tank(1590.0, 340.0, 100.0, 40.0, 3.0),
            wheel,
            pots,
            k.text("STAGE 3 · PERSIAN WHEEL", 1570.0, 455.0, Anchor::End, halo(label(28.0, INK, 700.0), PAPER))?,
        ])
        .reveal("5:0", Anim::up()),
    );
    nodes.push(
        Node::group([
            Node::path("M1680 190 H1720")?.stroke(WATER, 10.0),
            flow("M1700 190 C1740 190 1760 200 1790 236", SOFT_BLUE, 5.0, (12.0, 10.0), 70.0),
            k.text("POTS DIP, FILL, RISE AND TIP", 1600.0, 500.0, Anchor::End, halo(mono(18.0, INK_SOFT), PAPER))?,
        ])
        .reveal("5:1", Anim::fade()),
    );
    // Step 6: the scale, drawn in the clear sky above the lake.
    nodes.push(
        Node::group([
            Node::path("M255 850 V250 M235 850 H275 M235 250 H275")?.stroke(TERRACOTTA, 5.0),
            k.text("≈ 45 m", 285.0, 520.0, Anchor::Start, halo(label(64.0, TERRACOTTA, 800.0), PAPER))?,
            k.text("THE FOUR TOWERS", 287.0, 562.0, Anchor::Start, halo(mono(20.0, TERRACOTTA), PAPER))?,
        ])
        .reveal("6:payoff-0.3", Anim::fade()),
    );
    let mut storeys = vec![Node::rect(120.0, 250.0, 90.0, 600.0).fill(hex(0xE9DCC1)).stroke(INK, 4.0)];
    for j in 0..14 {
        let y = 250.0 + (j + 1) as f32 * 40.0;
        storeys.push(Node::line(120.0, y, 210.0, y).stroke(INK, 2.0));
    }
    storeys.push(k.centred("15 STOREYS", 165.0, 232.0, halo(mono(18.0, INK), PAPER))?);
    nodes.push(Node::group(storeys).reveal("6:payoff+0.6", Anim::up()));

    Ok(Sheet {
        dark: false,
        tag: "HOW WATER CLIMBED TO THE PALACE",
        legend: Some("BASED ON THE UNESCO NOMINATION DOSSIER (2013) · SCHEMATIC, VERTICAL SCALE APPROX."),
        step,
        nodes,
    })
}

/* ------------------------------------------------------------ palace plan: the walking route */

struct Court {
    id: &'static str,
    name: &'static str,
    sub: &'static str,
    rect: (f32, f32, f32, f32),
    step: u32,
}

const COURTS: [Court; 4] = [
    Court { id: "jaleb", name: "Jaleb Chowk", sub: "FIRST COURTYARD", rect: (130.0, 290.0, 470.0, 400.0), step: 1 },
    Court { id: "diwan", name: "Diwan-i-Aam", sub: "SECOND COURTYARD", rect: (640.0, 250.0, 360.0, 440.0), step: 2 },
    Court { id: "garden", name: "Garden court", sub: "THIRD COURTYARD", rect: (1040.0, 250.0, 410.0, 440.0), step: 4 },
    Court { id: "mansingh", name: "Man Singh Mahal", sub: "ZENANA · OLDEST", rect: (1490.0, 220.0, 330.0, 490.0), step: 5 },
];

/// One route segment per step (index = step - 1).
const ROUTE: [&str; 5] = [
    "M330 1000 C360 880 440 760 470 690 L380 500",
    "M380 500 L615 640 L800 500",
    "M800 500 L1020 500",
    "M1020 500 L1240 500",
    "M1240 500 L1470 500 L1655 470",
];
const HERE: [(f32, f32); 6] = [(0.0, 0.0), (380.0, 500.0), (800.0, 500.0), (1020.0, 500.0), (1240.0, 500.0), (1655.0, 470.0)];

pub fn palace_plan(k: &Kit, shot: &Shot) -> Result<Sheet> {
    let step = step_param(shot, 0)?;
    let overview = step == 0 || step == 6 || shot.param("highlight") == Some("all");
    let box_at = |x: f32, y: f32, w: f32, h: f32, name: &str, fill: Color| -> Result<Node> {
        Ok(Node::group([Node::rect(x, y, w, h).fill(fill).stroke(INK, 3.0), k.centred(name, x + w / 2.0, y + h / 2.0 + 6.0, mono(17.0, INK))?]))
    };

    let mut nodes = vec![
        Node::path("M0 860 C300 840 500 900 800 880 C1100 860 1400 900 1920 870 V1080 H0 Z")?.fill(hex(0x9EC0C9)),
        k.centred("MAOTA LAKE", 1150.0, 985.0, mono(26.0, hex(0x3F6E7E)))?,
        Node::rect(560.0, 915.0, 170.0, 74.0).fill(hex(0x9DB386)).stroke(hex(0x5F7A4A), 3.0),
        k.centred("KESAR KYARI", 645.0, 1022.0, mono(16.0, hex(0x3F6E7E)))?,
        Node::path("M1840 74 L1852 110 L1840 102 L1828 110 Z")?.fill(INK),
        k.centred("N", 1840.0, 140.0, mono(18.0, INK))?,
        k.text("JAIGARH FORT · 152 m HIGHER ↗", 1800.0, 170.0, Anchor::End, mono(17.0, INK_SOFT))?,
    ];
    for (i, c) in COURTS.iter().enumerate() {
        let on = overview || c.step == step || (c.id == "diwan" && step == 3);
        let (x, y, w, h) = c.rect;
        let court = Node::group([
            Node::rect(x, y, w, h)
                .rounded(6.0)
                .fill(if on { Color::from_argb(107, 224, 169, 59) } else { hex(0xEADFC8) })
                .stroke(if on { INK } else { hex(0xB4A07C) }, if on { 5.0 } else { 3.0 }),
            k.centred(c.name, x + w / 2.0, y + 52.0, label(if on { 38.0 } else { 30.0 }, if on { INK } else { INK_SOFT }, if on { 700.0 } else { 600.0 }))?,
            k.centred(c.sub, x + w / 2.0, y + 84.0, mono(16.0, if on { TERRACOTTA } else { hex(0xA08E70) }))?,
        ]);
        nodes.push(if step == 0 { court.enter(0.2 + i as f32 * 0.25, Anim::Up { y: 14.0, duration: 0.6 }) } else { court });
    }
    nodes.push(box_at(680.0, 600.0, 230.0, 70.0, "DIWAN-I-AAM HALL", hex(0xC9AE84))?);
    nodes.push(box_at(1075.0, 610.0, 340.0, 66.0, "SHEESH MAHAL", hex(0xD9D3C6))?);
    nodes.push(box_at(1075.0, 330.0, 340.0, 56.0, "SUKH NIWAS", hex(0xD9D3C6))?);
    nodes.push(Node::path("M1150 470 H1340 M1245 420 V560 M1150 520 H1340")?.stroke(hex(0x5F8D9D), 7.0));
    nodes.push(Node::rect(1615.0, 430.0, 86.0, 86.0).fill(hex(0xD9C39A)).stroke(INK, 3.0));
    nodes.push(k.centred("BARADARI", 1658.0, 548.0, mono(15.0, INK))?);
    nodes.push(Node::rect(998.0, 455.0, 44.0, 90.0).fill(TERRACOTTA));
    nodes.push(k.centred("GANESH POL", 1020.0, 440.0, halo(mono(17.0, TERRACOTTA), hex(0xEADFC8)))?);
    nodes.push(Node::rect(446.0, 676.0, 50.0, 30.0).fill(TERRACOTTA));
    nodes.push(k.centred("SURAJ POL", 471.0, 738.0, halo(mono(17.0, TERRACOTTA), PAPER))?);
    nodes.push(Node::rect(604.0, 612.0, 30.0, 30.0).fill(hex(0x8A6A44)));
    nodes.push(k.text("SHILA DEVI", 560.0, 668.0, Anchor::End, mono(14.0, INK_SOFT))?);

    if shot.param("mark") == Some("shila") {
        let ring = Node::live(|t| {
            let r = 120.0 + (46.0 - 120.0) * Ease::BackOut(1.6).apply((t - 0.6) / 0.6);
            Ok(Node::circle(619.0, 627.0, r).no_fill().stroke(TERRACOTTA, 6.0))
        });
        nodes.push(
            Node::group([
                ring,
                Node::path("M590 660 L380 790")?.stroke(TERRACOTTA, 3.0),
                k.text("Singh Pol → Shila Devi", 370.0, 815.0, Anchor::End, halo(label(30.0, TERRACOTTA, 800.0), PAPER))?,
            ])
            .enter(0.6, Anim::Fade { duration: 0.4, ease: SETTLE }),
        );
    }
    for (i, d) in ROUTE.iter().enumerate() {
        let seg = Node::path(d)?.no_fill().stroke(TEAL, 10.0).cap(PaintCap::Round).join(PaintJoin::Round);
        nodes.push(match (step, i as u32 + 1) {
            (6, _) => seg.enter(0.3 + i as f32 * 0.5, Anim::Draw { duration: 0.55, ease: Ease::None }),
            (0, _) => seg.undrawn(),
            (s, n) if n < s => seg,
            (s, n) if n == s => seg.enter(0.3, Anim::Draw { duration: 1.3, ease: Ease::PowerInOut(2) }),
            _ => seg.undrawn(),
        });
    }
    if !overview {
        let (x, y) = HERE[step.min(5) as usize];
        nodes.push(
            Node::group([
                Node::circle(x, y, 24.0).fill(TEAL).stroke(Color::WHITE, 5.0),
                k.centred("YOU ARE HERE", x, y - 40.0, halo(label(26.0, TEAL, 800.0), hex(0xEADFC8)))?,
            ])
            .enter(1.5, Anim::Pop { from: 0.0, duration: 0.5, ease: Ease::BackOut(2.0) }),
        );
    }
    Ok(Sheet { dark: false, tag: "AMBER PALACE · THE ROUTE", legend: Some("SCHEMATIC · NOT TO SCALE · AFTER THE UNESCO NOMINATION PLAN"), step: 1, nodes })
}
