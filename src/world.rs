use crate::items::Item;
use macroquad::prelude::*;
use std::collections::{HashMap, VecDeque};
pub const LIMIT: f32 = 58.;
const GRID: i32 = LIMIT as i32;
#[derive(Clone, Copy)]
pub struct Block {
    pub min: Vec2,
    pub max: Vec2,
}
impl Block {
    pub fn contains(&self, p: Vec2, radius: f32) -> bool {
        p.x > self.min.x - radius
            && p.x < self.max.x + radius
            && p.y > self.min.y - radius
            && p.y < self.max.y + radius
    }
}
pub struct Prop {
    pub name: &'static str,
    pub pos: Vec3,
    pub scale: Vec3,
    pub angle: f32,
}
/// A named spot residents can walk to. `source` is what can be made or gathered there:
/// public sources serve anyone, others only the resident who works there.
pub struct Place {
    pub name: &'static str,
    pub pos: Vec2,
    pub source: Option<Item>,
    pub public: bool,
}
pub struct Town {
    pub props: Vec<Prop>,
    pub blocks: Vec<Block>,
    pub destinations: Vec<Vec2>,
    pub places: Vec<Place>,
    pub houses: Vec<Vec2>,
    /// Untextured details (anvils, signs, tables): centre, size, colour.
    pub cubes: Vec<(Vec3, Vec3, Color)>,
    /// Precomputed walkable cells for path finding.
    walkable: Vec<bool>,
}
impl Town {
    pub fn new() -> Self {
        let mut town = Self {
            props: vec![],
            blocks: vec![],
            destinations: vec![
                vec2(-5., -6.),
                vec2(5., -6.),
                vec2(-5., 6.),
                vec2(5., 6.),
                vec2(-20., 0.),
                vec2(20., 0.),
                vec2(0., -22.),
                vec2(0., 23.),
                vec2(20., 20.),
                vec2(-20., -20.),
                vec2(0., -38.),
                vec2(0., 44.),
                vec2(-36., 0.),
                vec2(36., 0.),
                vec2(-30., 36.),
                vec2(30., -36.),
            ],
            places: vec![],
            houses: vec![],
            cubes: vec![],
            walkable: vec![],
        };
        // Old market quarter.
        for (x, z) in [
            (-12., -12.),
            (-12., 0.),
            (-12., 12.),
            (12., -12.),
            (12., 0.),
            (12., 12.),
            (-24., -12.),
            (-24., 12.),
            (24., -12.),
            (24., 12.),
            (-12., -25.),
            (12., -25.),
            (-12., 25.),
            (12., 25.),
        ] {
            let p = vec2(x, z);
            town.house(p, town.houses.len());
            town.houses.push(p);
        }
        // Outer districts: more homes.
        for (x, z) in [
            (-14., -42.),
            (14., -42.),
            (-24., 26.),
            (24., -26.),
            (-14., 40.),
            (14., 40.),
            (-48., -22.),
            (48., -20.),
            (-48., 20.),
            (48., 22.),
            (-26., -40.),
            (26., -40.),
        ] {
            let p = vec2(x, z);
            town.house(p, town.houses.len());
            town.houses.push(p);
        }
        town.prop("fountain-round", vec3(0., 0.02, 0.), vec3(2., 2., 2.), 0.);
        town.prop("fountain-center", vec3(0., 0.02, 0.), vec3(2., 2., 2.), 0.);
        town.blocks.push(Block {
            min: vec2(-1.9, -1.9),
            max: vec2(1.9, 1.9),
        });
        for (x, z, a) in [
            (-6., -11., 0.),
            (6., -11., std::f32::consts::PI),
            (-6., 11., 0.),
            (6., 11., std::f32::consts::PI),
        ] {
            town.prop(
                if x < 0. { "stall-red" } else { "stall-green" },
                vec3(x, 0., z),
                vec3(2.4, 2.4, 2.4),
                a,
            );
            town.solid(vec2(x, z), 1.2);
        }
        town.temple();
        town.barracks();
        // Bakery, scriptorium, infirmary.
        town.house(vec2(-40., -46.), 2);
        town.house(vec2(-38., -28.), 4);
        town.house(vec2(38., 30.), 5);
        town.smithy();
        town.tavern();
        // Mill.
        town.prop("windmill", vec3(-46., 0., 40.), vec3(3., 3., 3.), 0.6);
        town.solid(vec2(-46., 40.), 3.);
        // Kitchen gardens.
        for i in 0..6 {
            let x = 24. + i as f32 * 2.4;
            town.prop("fence", vec3(x, 0., 48.), vec3(2.4, 2., 2.), 0.);
            town.prop("hedge", vec3(x, 0., 36.5), vec3(2.4, 1.2, 1.2), 0.);
        }
        // River and pier.
        town.blocks.push(Block {
            min: vec2(-LIMIT, 52.),
            max: vec2(LIMIT, LIMIT + 2.),
        });
        town.prop("watermill", vec3(-22., 0., 53.5), vec3(3., 3., 3.), 0.);
        town.prop("planks-half", vec3(0., 0.05, 51.), vec3(4., 2., 4.), 0.);
        town.prop("cart", vec3(-19., 0., 7.), vec3(2., 2., 2.), 0.7);
        town.blocks.push(Block {
            min: vec2(-20.3, 5.7),
            max: vec2(-17.7, 8.3),
        });
        // Trees along the town wall (the river closes the south side).
        for i in 0..40 {
            let t = i as f32 / 40.;
            let p = match i / 10 {
                0 => vec2(-55. + t * 4. * 110., -55.),
                1 => vec2(55., -55. + (t - 0.25) * 4. * 105.),
                2 => vec2(-55., -55. + (t - 0.5) * 4. * 105.),
                _ => vec2(-50. + (t - 0.75) * 4. * 100., 49.5),
            };
            if (i / 10 == 3 && (p.x.abs() < 6. || (p.x - 30.).abs() < 10.))
                || (i / 10 != 3 && p.y.abs() < 5.)
            {
                continue;
            }
            town.prop(
                if i % 2 == 0 {
                    "tree"
                } else {
                    "tree-high-round"
                },
                vec3(p.x, 0., p.y),
                vec3(2., 2., 2.),
                i as f32,
            );
            town.solid(p, 0.3);
        }
        for x in [-5., 5.] {
            for z in [-44., -32., -20., -8., 8., 20., 32., 44.] {
                town.prop("lantern", vec3(x, 0., z), vec3(2., 2., 2.), 0.);
            }
        }
        for z in [-5., 5.] {
            for x in [-44., -32., -20., 20., 32., 44.] {
                town.prop("lantern", vec3(x, 0., z), vec3(2., 2., 2.), 0.);
            }
        }
        town.places = vec![
            Place {
                name: "рынок",
                pos: vec2(0., 5.),
                source: None,
                public: false,
            },
            Place {
                name: "храм",
                pos: vec2(0., -41.),
                source: Some(Item::Candle),
                public: false,
            },
            Place {
                name: "казарма",
                pos: vec2(40., -38.),
                source: None,
                public: false,
            },
            Place {
                name: "пекарня",
                pos: vec2(-40., -40.),
                source: Some(Item::Bread),
                public: false,
            },
            Place {
                name: "скрипторий",
                pos: vec2(-38., -22.),
                source: Some(Item::Book),
                public: false,
            },
            Place {
                name: "лекарня",
                pos: vec2(38., 24.),
                source: Some(Item::Medicine),
                public: false,
            },
            Place {
                name: "кузница",
                pos: vec2(-39., 0.),
                source: Some(Item::Tool),
                public: false,
            },
            Place {
                name: "трактир",
                pos: vec2(38., 0.),
                source: Some(Item::Ale),
                public: false,
            },
            Place {
                name: "мельница",
                pos: vec2(-40., 36.),
                source: Some(Item::Flour),
                public: false,
            },
            Place {
                name: "огороды",
                pos: vec2(30., 42.),
                source: Some(Item::Herbs),
                public: true,
            },
            Place {
                name: "пристань",
                pos: vec2(0., 49.),
                source: Some(Item::Fish),
                public: true,
            },
        ];
        town.build_grid();
        town
    }
    /// Places a wall-like piece (plane near local x=0.45, spanning local z) along segment a-b.
    fn edge(&mut self, name: &'static str, a: Vec2, b: Vec2, y: f32, h: f32, mirror: bool) {
        let d = b - a;
        let l = d.length();
        let angle = d.x.atan2(d.y);
        let mid = (a + b) / 2. - vec2(angle.cos(), -angle.sin()) * 0.45 * l;
        self.prop(
            name,
            vec3(mid.x, y, mid.y),
            vec3(l, h, if mirror { -l } else { l }),
            angle,
        );
    }
    fn cube(&mut self, center: Vec3, size: Vec3, color: Color) {
        self.cubes.push((center, size, color));
    }
    fn fence(&mut self, a: Vec2, b: Vec2) {
        let n = ((b - a).length() / 2.4).round().max(1.) as usize;
        for i in 0..n {
            let (p, q) = (
                a.lerp(b, i as f32 / n as f32),
                a.lerp(b, (i + 1) as f32 / n as f32),
            );
            self.edge("fence", p, q, 0., 2.6, false);
        }
        self.blocks.push(Block {
            min: a.min(b) - Vec2::splat(0.15),
            max: a.max(b) + Vec2::splat(0.15),
        });
    }
    /// Stone hall with an arched portal, round windows, a bell tower and a colonnade.
    fn temple(&mut self) {
        let (x0, x1, z0, z1) = (-5.6, 5.6, -53.2, -44.8);
        self.blocks.push(Block {
            min: vec2(x0, z0),
            max: vec2(x1, z1),
        });
        for floor in 0..3 {
            let y = floor as f32 * 2.8;
            for i in 0..4 {
                let a = vec2(x0 + i as f32 * 2.8, z1);
                let name = match (floor, i) {
                    (0, 1 | 2) => "wall-arch-top",
                    (0, _) => "wall-detail-cross",
                    (1, 1 | 2) => "wall-window-round",
                    _ => "wall",
                };
                self.edge(name, a, a + vec2(2.8, 0.), y, 2.8, floor == 0 && i == 1);
                let b = vec2(x0 + i as f32 * 2.8, z0);
                let name = if floor == 1 {
                    "wall-window-round"
                } else {
                    "wall"
                };
                self.edge(name, b + vec2(2.8, 0.), b, y, 2.8, false);
            }
            for i in 0..3 {
                let z = z0 + i as f32 * 2.8;
                let name = if floor == 1 {
                    "wall-window-round"
                } else {
                    "wall"
                };
                self.edge(name, vec2(x0, z), vec2(x0, z + 2.8), y, 2.8, false);
                self.edge(name, vec2(x1, z + 2.8), vec2(x1, z), y, 2.8, false);
            }
        }
        self.prop(
            "roof-high-point",
            vec3(0., 8.4, -49.),
            vec3(10.6, 5., 8.),
            0.,
        );
        // Bell tower rising from the roof.
        let c = [
            vec2(-1.4, -50.4),
            vec2(1.4, -50.4),
            vec2(1.4, -47.6),
            vec2(-1.4, -47.6),
        ];
        for floor in 0..3 {
            for k in 0..4 {
                let name = if floor == 2 {
                    "wall-window-round"
                } else {
                    "wall"
                };
                self.edge(
                    name,
                    c[(k + 1) % 4],
                    c[k],
                    8.4 + floor as f32 * 2.8,
                    2.8,
                    false,
                );
            }
        }
        self.prop(
            "roof-high-point",
            vec3(0., 16.8, -49.),
            vec3(3.4, 4.5, 3.4),
            0.,
        );
        self.cube(vec3(0., 21.7, -49.), vec3(0.15, 1.2, 0.15), GOLD);
        self.cube(vec3(0., 21.9, -49.), vec3(0.7, 0.15, 0.15), GOLD);
        for (a, b) in [(x0, x0 + 2.8), (x1 - 2.8, x1)] {
            self.edge("banner-red", vec2(a, z1), vec2(b, z1), 3.4, 4., false);
        }
        for x in [-7.4, -3.4, 3.4, 7.4] {
            self.prop("pillar-stone", vec3(x, 0., -43.4), vec3(3., 4.4, 3.), 0.);
            self.cube(
                vec3(x, 4.5, -43.4),
                vec3(0.7, 0.25, 0.7),
                Color::new(0.72, 0.7, 0.64, 1.),
            );
            self.solid(vec2(x, -43.4), 0.35);
        }
    }
    /// Barracks with a fenced drill yard, a gate to the south and flags on poles.
    fn barracks(&mut self) {
        self.house(vec2(40., -46.), 1);
        let (x0, x1, z0, z1) = (33.6, 46.4, -43., -34.4);
        self.fence(vec2(x0, z0), vec2(x0, z1));
        self.fence(vec2(x1, z1), vec2(x1, z0));
        self.fence(vec2(x0, z1), vec2(37.6, z1));
        self.fence(vec2(42.4, z1), vec2(x1, z1));
        for (x, banner) in [
            (37.6, "banner-red"),
            (42.4, "banner-red"),
            (x0, "banner-green"),
            (x1, "banner-green"),
        ] {
            self.prop("pillar-wood", vec3(x, 0., z1), vec3(2., 4.6, 2.), 0.);
            self.prop(banner, vec3(x - 0.8, 2.6, z1 + 0.5), vec3(2., 2., 2.), 0.);
            self.solid(vec2(x, z1), 0.2);
        }
        // Weapon rack and a straw dummy in the yard.
        self.prop("poles", vec3(36., 0., -42.), vec3(2.4, 1.8, 2.4), 0.);
        let straw = Color::new(0.78, 0.68, 0.38, 1.);
        self.cube(
            vec3(44., 0.75, -40.5),
            vec3(0.12, 1.5, 0.12),
            Color::new(0.42, 0.28, 0.16, 1.),
        );
        self.cube(vec3(44., 1.35, -40.5), vec3(0.5, 0.6, 0.35), straw);
        self.cube(vec3(44., 1.8, -40.5), vec3(0.3, 0.3, 0.3), straw);
        self.solid(vec2(44., -40.5), 0.3);
    }
    /// Smithy: an awning on posts over an anvil and a glowing forge.
    fn smithy(&mut self) {
        self.house(vec2(-46., 0.), 7);
        self.prop(
            "overhang",
            vec3(-43.2 + 0.45 * 5., 0., 0.),
            vec3(5., 3.2, 5.6),
            std::f32::consts::PI,
        );
        for z in [-2.5, 2.5] {
            self.prop("pillar-wood", vec3(-41.5, 0., z), vec3(2., 2.2, 2.), 0.);
            self.solid(vec2(-41.5, z), 0.2);
        }
        let iron = Color::new(0.24, 0.24, 0.26, 1.);
        self.cube(
            vec3(-41.6, 0.25, 0.9),
            vec3(0.4, 0.5, 0.4),
            Color::new(0.38, 0.26, 0.15, 1.),
        );
        self.cube(vec3(-41.6, 0.58, 0.9), vec3(0.32, 0.16, 0.7), iron);
        self.cube(vec3(-41.6, 0.62, 1.35), vec3(0.18, 0.08, 0.3), iron);
        self.solid(vec2(-41.6, 0.9), 0.4);
        self.cube(
            vec3(-42.5, 0.45, -1.3),
            vec3(1.1, 0.9, 1.1),
            Color::new(0.45, 0.43, 0.4, 1.),
        );
        self.cube(
            vec3(-42.5, 0.92, -1.3),
            vec3(0.8, 0.06, 0.8),
            Color::new(1., 0.42, 0.1, 1.),
        );
        self.solid(vec2(-42.5, -1.3), 0.6);
        for z in [-3., 3.] {
            self.prop("planks", vec3(-41., 0., z), vec3(2., 2., 2.), 0.);
        }
        self.prop("cart-high", vec3(-41., 0., 6.), vec3(2., 2., 2.), 1.2);
        self.solid(vec2(-41., 6.), 1.1);
    }
    /// Tavern: a hanging sign by the road and tables with benches outside.
    fn tavern(&mut self) {
        self.house(vec2(46., 0.), 3);
        self.prop("banner-red", vec3(43.1, 3., 0.), vec3(2., 2., 2.), 1.57);
        let wood = Color::new(0.42, 0.28, 0.16, 1.);
        let ale = Color::new(0.9, 0.7, 0.25, 1.);
        self.prop("pillar-wood", vec3(42.4, 0., 3.4), vec3(2., 3.4, 2.), 0.);
        self.solid(vec2(42.4, 3.4), 0.2);
        self.cube(vec3(41.7, 3.1, 3.4), vec3(1.5, 0.1, 0.1), wood);
        self.cube(
            vec3(41.4, 2.5, 3.4),
            vec3(1.1, 0.8, 0.08),
            Color::new(0.55, 0.38, 0.2, 1.),
        );
        self.cube(vec3(41.35, 2.48, 3.34), vec3(0.34, 0.42, 0.1), ale);
        self.cube(vec3(41.6, 2.5, 3.34), vec3(0.1, 0.22, 0.1), ale);
        for z in [-5.5, 6.5] {
            self.cube(vec3(40.5, 0.78, z), vec3(1.8, 0.08, 1.), wood);
            for (dx, dz) in [(-0.75, -0.4), (0.75, -0.4), (-0.75, 0.4), (0.75, 0.4)] {
                self.cube(vec3(40.5 + dx, 0.37, z + dz), vec3(0.08, 0.74, 0.08), wood);
            }
            self.cube(vec3(40.2, 0.9, z), vec3(0.14, 0.18, 0.14), ale);
            self.cube(vec3(40.9, 0.9, z + 0.2), vec3(0.14, 0.18, 0.14), ale);
            self.solid(vec2(40.5, z), 0.9);
            for side in [-1., 1.] {
                self.prop(
                    "stall-bench",
                    vec3(40.5, 0., z + side * 0.95),
                    vec3(2., 2., 2.),
                    1.57,
                );
            }
        }
    }
    fn solid(&mut self, p: Vec2, r: f32) {
        self.blocks.push(Block {
            min: p - Vec2::splat(r),
            max: p + Vec2::splat(r),
        });
    }
    pub fn place(&self, name: &str) -> Option<&Place> {
        self.places.iter().find(|p| p.name == name)
    }
    pub fn place_names(&self) -> Vec<String> {
        self.places.iter().map(|p| p.name.to_string()).collect()
    }
    /// The named place nearest to `p`, if within `radius`.
    pub fn place_at(&self, p: Vec2, radius: f32) -> Option<&Place> {
        self.places
            .iter()
            .filter(|pl| pl.pos.distance(p) < radius)
            .min_by(|a, b| a.pos.distance(p).total_cmp(&b.pos.distance(p)))
    }
    /// Street point in front of house `i` (houses face +z).
    pub fn door(&self, i: usize) -> Vec2 {
        let h = self.houses[i % self.houses.len()];
        self.nearest_walkable(h + vec2(0., 3.6))
    }
    fn build_grid(&mut self) {
        let n = (2 * GRID + 1) as usize;
        let mut walkable = vec![false; n * n];
        for x in -GRID..=GRID {
            for z in -GRID..=GRID {
                walkable[((x + GRID) as usize) * n + (z + GRID) as usize] =
                    self.free(vec2(x as f32, z as f32), 0.38);
            }
        }
        self.walkable = walkable;
    }
    fn cell_free(&self, c: (i32, i32)) -> bool {
        if c.0.abs() > GRID || c.1.abs() > GRID {
            return false;
        }
        let n = (2 * GRID + 1) as usize;
        self.walkable[((c.0 + GRID) as usize) * n + (c.1 + GRID) as usize]
    }
    pub fn nearest_walkable(&self, p: Vec2) -> Vec2 {
        let c = (p.x.round() as i32, p.y.round() as i32);
        for r in 0..6 {
            for dx in -r..=r {
                for dz in -r..=r {
                    let n = (c.0 + dx, c.1 + dz);
                    if (dx.abs() == r || dz.abs() == r) && self.cell_free(n) {
                        return vec2(n.0 as f32, n.1 as f32);
                    }
                }
            }
        }
        p
    }
    fn prop(&mut self, name: &'static str, pos: Vec3, scale: Vec3, angle: f32) {
        self.props.push(Prop {
            name,
            pos,
            scale,
            angle,
        });
    }
    fn house(&mut self, p: Vec2, seed: usize) {
        let width = 5.6;
        let floors = if seed.is_multiple_of(3) { 3 } else { 2 };
        self.blocks.push(Block {
            min: p - vec2(width / 2., width / 2.),
            max: p + vec2(width / 2., width / 2.),
        });
        for floor in 0..floors {
            for side in 0..4 {
                for segment in 0..2 {
                    let a = side as f32 * std::f32::consts::FRAC_PI_2;
                    let local = vec3(1.4, 0., -1.4 + segment as f32 * 2.8);
                    let pos = Quat::from_rotation_y(a) * local + vec3(p.x, floor as f32 * 2.8, p.y);
                    let name = if floor == 0 && segment == 0 {
                        "wall-door"
                    } else if floor == 0 {
                        "wall-window-stone"
                    } else if seed.is_multiple_of(2) {
                        "wall-wood-window-glass"
                    } else {
                        "wall-wood-window-shutters"
                    };
                    self.prop(name, pos, vec3(2.8, 2.8, 2.8), a);
                }
            }
        }
        self.prop(
            "roof-high-point",
            vec3(p.x, floors as f32 * 2.8, p.y),
            vec3(5.6, 3.5, 5.6),
            0.,
        );
        self.prop(
            "chimney",
            vec3(p.x + 1.7, floors as f32 * 2.8 + 1., p.y + 1.4),
            vec3(2., 3., 2.),
            0.,
        );
    }
    pub fn free(&self, p: Vec2, r: f32) -> bool {
        p.x.abs() < LIMIT - r
            && p.y.abs() < LIMIT - r
            && !self.blocks.iter().any(|b| b.contains(p, r))
    }
    pub fn move_actor(&self, p: Vec2, delta: Vec2, r: f32) -> Vec2 {
        let steps = (delta.length() / 0.15).ceil().max(1.) as usize;
        let d = delta / steps as f32;
        let mut out = p;
        for _ in 0..steps {
            if self.free(out + vec2(d.x, 0.), r) {
                out.x += d.x;
            }
            if self.free(out + vec2(0., d.y), r) {
                out.y += d.y;
            }
        }
        out
    }
    pub fn clear_line(&self, a: Vec2, b: Vec2) -> bool {
        let steps = ((b - a).length() / 0.12).ceil().max(1.) as usize;
        (0..=steps).all(|i| self.free(a.lerp(b, i as f32 / steps as f32), 0.))
    }
    pub fn path(&self, start: Vec2, end: Vec2) -> VecDeque<Vec2> {
        let cell = |p: Vec2| (p.x.round() as i32, p.y.round() as i32);
        let start_cell = cell(start);
        let end_cell = cell(self.nearest_walkable(end));
        let mut queue = VecDeque::from([start_cell]);
        let mut previous = HashMap::from([(start_cell, start_cell)]);
        while let Some(current) = queue.pop_front() {
            if current == end_cell {
                break;
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (current.0 + dx, current.1 + dz);
                if !previous.contains_key(&n) && self.cell_free(n) {
                    previous.insert(n, current);
                    queue.push_back(n);
                }
            }
        }
        let mut result = VecDeque::new();
        if !previous.contains_key(&end_cell) {
            return result;
        }
        let mut at = end_cell;
        while at != start_cell {
            result.push_front(vec2(at.0 as f32, at.1 as f32));
            at = previous[&at];
        }
        result
    }
}
pub fn sees(town: &Town, position: Vec2, facing: Vec2, target: Vec2) -> bool {
    let delta = target - position;
    delta.length() < 14.
        && (delta.length() < 1.5 || facing.dot(delta.normalize_or_zero()) > 0.35)
        && town.clear_line(position, target)
}
pub struct Assets {
    meshes: HashMap<String, Vec<Mesh>>,
}
impl Assets {
    pub async fn load(root: &std::path::Path, town: &Town) -> Result<Self, String> {
        let dir = root.join("assets/kenney/Models/OBJ format");
        let texture = load_texture(
            dir.join("Textures/colormap.png")
                .to_str()
                .ok_or("asset path")?,
        )
        .await
        .map_err(|e| e.to_string())?;
        texture.set_filter(FilterMode::Nearest);
        let mut meshes = HashMap::new();
        for prop in &town.props {
            if meshes.contains_key(prop.name) {
                continue;
            }
            let (models, _) = tobj::load_obj(
                dir.join(format!("{}.obj", prop.name)),
                &tobj::LoadOptions {
                    triangulate: true,
                    single_index: true,
                    ..Default::default()
                },
            )
            .map_err(|e| e.to_string())?;
            let mut parts = vec![];
            for model in models {
                let m = model.mesh;
                if m.positions.len() / 3 > u16::MAX as usize {
                    return Err("Mesh exceeds u16 indices".into());
                }
                let vertices = (0..m.positions.len() / 3)
                    .map(|i| {
                        let normal = if m.normals.len() > i * 3 + 2 {
                            vec3(m.normals[i * 3], m.normals[i * 3 + 1], m.normals[i * 3 + 2])
                        } else {
                            Vec3::Y
                        };
                        let light =
                            0.62 + 0.38 * normal.dot(vec3(-0.4, 0.8, 0.3).normalize()).max(0.);
                        Vertex::new(
                            m.positions[i * 3],
                            m.positions[i * 3 + 1],
                            m.positions[i * 3 + 2],
                            m.texcoords.get(i * 2).copied().unwrap_or(0.),
                            1. - m.texcoords.get(i * 2 + 1).copied().unwrap_or(0.),
                            Color::new(light, light * 0.97, light * 0.91, 1.),
                        )
                    })
                    .collect();
                parts.push(Mesh {
                    vertices,
                    indices: m.indices.iter().map(|i| *i as u16).collect(),
                    texture: Some(texture.clone()),
                });
            }
            meshes.insert(prop.name.to_string(), parts);
        }
        Ok(Self { meshes })
    }
    pub fn draw(&self, town: &Town) {
        for prop in &town.props {
            let transform = Mat4::from_scale_rotation_translation(
                prop.scale,
                Quat::from_rotation_y(prop.angle),
                prop.pos,
            );
            unsafe {
                get_internal_gl().quad_gl.push_model_matrix(transform);
            }
            for mesh in &self.meshes[prop.name] {
                draw_mesh(mesh);
            }
            unsafe {
                get_internal_gl().quad_gl.pop_model_matrix();
            }
        }
        for &(center, size, color) in &town.cubes {
            draw_cube(center, size, None, color);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn walls_block_sight_and_movement() {
        let t = Town::new();
        assert!(!t.clear_line(vec2(-17., 0.), vec2(-7., 0.)));
        let p = t.move_actor(vec2(-17., 0.), vec2(20., 0.), 0.3);
        assert!(p.x < -15.);
        assert!(t.clear_line(vec2(0., 5.), vec2(0., 20.)));
    }
    #[test]
    fn paths_avoid_buildings() {
        let t = Town::new();
        let path = t.path(vec2(-20., 0.), vec2(20., 0.));
        assert!(!path.is_empty());
        assert!(path.iter().all(|p| t.free(*p, 0.38)));
        assert_eq!(*path.back().unwrap(), vec2(20., 0.));
    }
    #[test]
    fn every_place_is_reachable_from_the_market() {
        let t = Town::new();
        let start = t.nearest_walkable(vec2(0., 5.));
        for pl in &t.places {
            let end = t.nearest_walkable(pl.pos);
            assert!(end.distance(pl.pos) < 2., "{} is buried", pl.name);
            assert!(
                start == end || !t.path(start, pl.pos).is_empty(),
                "{} unreachable",
                pl.name
            );
        }
    }
    #[test]
    fn perception_has_field_of_view() {
        let t = Town::new();
        assert!(sees(&t, vec2(0., 10.), Vec2::Y, vec2(0., 15.)));
        assert!(!sees(&t, vec2(0., 10.), Vec2::Y, vec2(0., 5.)));
    }
}
