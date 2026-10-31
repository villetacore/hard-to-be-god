//! The player's divine powers, their visual effects and how witnesses react.
use crate::{
    gameplay::{self, PlayerState},
    items::Item,
    npc::Npc,
    world::{Town, sees},
};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Power {
    Heal,
    Bread,
    Lightning,
    Flight,
    Whisper,
    Leap,
}
impl Power {
    pub const ALL: [Power; 6] = [
        Power::Heal,
        Power::Bread,
        Power::Lightning,
        Power::Flight,
        Power::Whisper,
        Power::Leap,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Power::Heal => "Исцеление",
            Power::Bread => "Хлеб",
            Power::Lightning => "Молния",
            Power::Flight => "Полёт",
            Power::Whisper => "Внушение",
            Power::Leap => "Скачок",
        }
    }
    /// Grace spent per use (per second for flight).
    pub fn cost(self) -> f32 {
        match self {
            Power::Heal => 30.,
            Power::Bread => 20.,
            Power::Lightning => 45.,
            Power::Flight => 6.,
            Power::Whisper => 25.,
            Power::Leap => 25.,
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Bolt,
    Glow,
    Burst,
}
struct Effect {
    kind: Kind,
    pos: Vec3,
    start: f64,
    until: f64,
}
pub struct Powers {
    effects: Vec<Effect>,
    last_flight_witness: f64,
    pub flash_until: f64,
}
pub const GRACE_REGEN: f32 = 4.;
/// What happened when a power was used.
pub enum Outcome {
    Done(String),
    /// Whisper needs the words first: open the input for this resident.
    Whisper(usize),
    Failed(String),
}
/// Where the player's gaze meets the ground, within `range`.
pub fn ground_point(eye: Vec3, front: Vec3, range: f32) -> Vec2 {
    if front.y < -0.02 {
        let t = (eye.y / -front.y).min(range);
        let p = eye + front * t;
        vec2(p.x, p.z)
    } else {
        let flat = vec2(front.x, front.z).normalize_or_zero();
        vec2(eye.x, eye.z) + flat * range * 0.6
    }
}
impl Powers {
    pub fn new() -> Self {
        Self {
            effects: vec![],
            last_flight_witness: -100.,
            flash_until: 0.,
        }
    }
    fn effect(&mut self, kind: Kind, pos: Vec3, now: f64, secs: f64) {
        self.effects.push(Effect {
            kind,
            pos,
            start: now,
            until: now + secs,
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub fn cast(
        &mut self,
        power: Power,
        npcs: &mut [Npc],
        town: &Town,
        bag: &mut PlayerState,
        player: &mut Vec2,
        eye: Vec3,
        front: Vec3,
        now: f64,
    ) -> Outcome {
        if power == Power::Flight {
            bag.flying = !bag.flying;
            return Outcome::Done(if bag.flying {
                "Вы поднимаетесь в воздух.".into()
            } else {
                "Вы плавно опускаетесь на землю.".into()
            });
        }
        if bag.grace < power.cost() {
            return Outcome::Failed(format!("Не хватает благодати: нужно {:.0}.", power.cost()));
        }
        let flat = vec2(front.x, front.z).normalize_or_zero();
        let outcome = match power {
            Power::Heal => {
                let Some(id) = gameplay::aimed(town, npcs, *player, flat, 8., 0.8, true) else {
                    return Outcome::Failed("Посмотрите на жителя, чтобы исцелить его.".into());
                };
                let name = npcs[id].name;
                let n = &mut npcs[id];
                let was_down = n.health <= 0;
                n.health = 100;
                n.down_until = 0.;
                n.fear = (n.fear - 0.5).max(0.);
                n.hurt_until = 0.;
                n.epoch += 1;
                let spot = n.pos;
                self.effect(Kind::Glow, vec3(spot.x, 0., spot.y), now, 2.5);
                witness(
                    npcs,
                    town,
                    spot,
                    *player,
                    &format!("Путник коснулся {name} — и тот исцелился! Это чудо."),
                    25,
                    0.,
                );
                let n = &mut npcs[id];
                n.faith = (n.faith + 20).min(100);
                n.trust = (n.trust + 20).min(100);
                n.remember(if was_down {
                    "Я лежал без чувств, а путник поднял меня одним касанием!".into()
                } else {
                    "Путник исцелил меня. Боль ушла — это чудо!".into()
                });
                n.heard = Some("Путник только что исцелил меня чудом.".into());
                Outcome::Done(format!("{name} исцелён."))
            }
            Power::Bread => {
                bag.items.add(Item::Bread, 3);
                self.effect(Kind::Burst, vec3(player.x, 1.2, player.y), now, 1.5);
                witness(
                    npcs,
                    town,
                    *player,
                    *player,
                    "Путник сотворил хлеб из воздуха!",
                    15,
                    0.,
                );
                Outcome::Done("В ваших руках появились три каравая.".into())
            }
            Power::Lightning => {
                let spot = town.nearest_walkable(ground_point(eye, front, 30.));
                self.effect(Kind::Bolt, vec3(spot.x, 0., spot.y), now, 0.6);
                self.effect(Kind::Burst, vec3(spot.x, 0.3, spot.y), now, 1.);
                self.flash_until = now + 0.15;
                let struck = witness(
                    npcs,
                    town,
                    spot,
                    *player,
                    "Путник поднял руку — и с ясного неба ударила молния!",
                    20,
                    0.35,
                );
                for n in npcs
                    .iter_mut()
                    .filter(|n| n.health > 0 && n.pos.distance(spot) < 4.)
                {
                    n.fear = 1.;
                    n.remember("Молния ударила совсем рядом со мной!".into());
                }
                Outcome::Done(format!("Гром прокатился над городом. Очевидцев: {struck}."))
            }
            Power::Whisper => {
                let Some(id) = gameplay::aimed(town, npcs, *player, flat, 16., 0.85, false) else {
                    return Outcome::Failed(
                        "Посмотрите на жителя, чтобы внушить ему мысль.".into(),
                    );
                };
                return Outcome::Whisper(id);
            }
            Power::Leap => {
                let to = town.nearest_walkable(ground_point(eye, front, 25.));
                if !town.free(to, 0.3) {
                    return Outcome::Failed("Туда не переместиться.".into());
                }
                let from = *player;
                self.effect(Kind::Burst, vec3(from.x, 1., from.y), now, 1.);
                witness(
                    npcs,
                    town,
                    from,
                    from,
                    "Путник исчез во вспышке света!",
                    15,
                    0.1,
                );
                *player = to;
                self.effect(Kind::Burst, vec3(to.x, 1., to.y), now, 1.);
                witness(
                    npcs,
                    town,
                    to,
                    to,
                    "Путник появился из ниоткуда во вспышке света!",
                    15,
                    0.1,
                );
                Outcome::Done("Вы переноситесь в одно мгновение.".into())
            }
            Power::Flight => unreachable!(),
        };
        bag.grace -= power.cost();
        outcome
    }
    /// Plants the player's words as an inner voice in a resident's head.
    pub fn whisper(
        &mut self,
        npcs: &mut [Npc],
        id: usize,
        words: &str,
        bag: &mut PlayerState,
        now: f64,
    ) -> String {
        if bag.grace < Power::Whisper.cost() {
            return "Не хватает благодати.".into();
        }
        bag.grace -= Power::Whisper.cost();
        let n = &mut npcs[id];
        n.remember(format!("Я услышал голос прямо у себя в голове: «{words}»"));
        n.heard = Some(format!(
            "Голос, не мой, звучит прямо у меня в голове: «{words}». Это не моя мысль."
        ));
        n.heard_from = None;
        n.faith = (n.faith + 8).min(100);
        n.needs_think = true;
        n.epoch += 1;
        self.effect(Kind::Glow, vec3(n.pos.x, 1.6, n.pos.y), now, 1.2);
        format!("{} слышит ваш голос в своей голове.", n.name)
    }
    /// Per-frame upkeep: grace, flight cost and witnesses of flight.
    pub fn tick(
        &mut self,
        npcs: &mut [Npc],
        town: &Town,
        bag: &mut PlayerState,
        player: Vec2,
        dt: f32,
        now: f64,
    ) -> Option<String> {
        self.effects.retain(|e| e.until > now);
        if bag.flying {
            bag.grace -= Power::Flight.cost() * dt;
            if bag.grace <= 0. {
                bag.grace = 0.;
                bag.flying = false;
                return Some("Благодать иссякла — вы опускаетесь на землю.".into());
            }
            if bag.height > 2. && now - self.last_flight_witness > 8. {
                self.last_flight_witness = now;
                witness(
                    npcs,
                    town,
                    player,
                    player,
                    "Путник парит в воздухе, как птица!",
                    8,
                    0.,
                );
            }
        } else {
            bag.grace = (bag.grace + GRACE_REGEN * dt).min(100.);
        }
        None
    }
    pub fn draw(&self, now: f64) {
        for e in &self.effects {
            let t = ((now - e.start) / (e.until - e.start)) as f32;
            match e.kind {
                Kind::Bolt => {
                    // A jagged bolt from the sky, regenerated every few frames.
                    let seed = (now * 30.) as i32;
                    let mut p = e.pos + vec3(0., 40., 0.);
                    for i in 0..14 {
                        let k = (seed * 31 + i * 17).rem_euclid(97) as f32 / 97. - 0.5;
                        let l = (seed * 13 + i * 29).rem_euclid(89) as f32 / 89. - 0.5;
                        let next = if i == 13 {
                            e.pos
                        } else {
                            vec3(
                                e.pos.x + k * 2.5,
                                40. - (i + 1) as f32 * 40. / 14.,
                                e.pos.z + l * 2.5,
                            )
                        };
                        for o in [-0.06, 0., 0.06] {
                            draw_line_3d(
                                p + vec3(o, 0., 0.),
                                next + vec3(o, 0., 0.),
                                Color::new(1., 1., 0.85, 1.),
                            );
                        }
                        p = next;
                    }
                    draw_sphere(e.pos, 0.6 * (1. - t), None, Color::new(1., 0.95, 0.7, 0.8));
                }
                Kind::Glow => {
                    let c = Color::new(1., 0.86, 0.45, 0.22 * (1. - t));
                    draw_cylinder(e.pos, 0.45, 0.3, 2.4 + t * 2., None, c);
                    for i in 0..8 {
                        let a = i as f32 * 0.785 + t * 6.;
                        draw_sphere(
                            e.pos
                                + vec3(a.cos() * 0.8, t * 3. + (i % 3) as f32 * 0.4, a.sin() * 0.8),
                            0.07,
                            None,
                            Color::new(1., 0.9, 0.5, 1. - t),
                        );
                    }
                }
                Kind::Burst => {
                    for i in 0..12 {
                        let a = i as f32 * std::f32::consts::FRAC_PI_6;
                        let r = 0.3 + t * 2.2;
                        draw_sphere(
                            e.pos + vec3(a.cos() * r, (i % 4) as f32 * 0.3 * t, a.sin() * r),
                            0.09 * (1. - t) + 0.02,
                            None,
                            Color::new(1., 0.92, 0.6, 1. - t),
                        );
                    }
                }
            }
        }
    }
}
/// Everyone who can see `spot` (or the player doing it) remembers the miracle and
/// believes a little more. Returns how many saw it.
pub fn witness(
    npcs: &mut [Npc],
    town: &Town,
    spot: Vec2,
    player: Vec2,
    what: &str,
    faith: i32,
    fear: f32,
) -> usize {
    let mut count = 0;
    for n in npcs.iter_mut().filter(|n| n.health > 0) {
        let saw = sees(town, n.pos, n.facing, spot)
            || sees(town, n.pos, n.facing, player)
            || n.pos.distance(spot) < 3.;
        if !saw {
            continue;
        }
        count += 1;
        n.remember(what.to_string());
        n.faith = (n.faith + faith).min(100);
        n.fear = (n.fear + fear).min(1.);
        n.facing = (spot - n.pos).normalize_or_zero();
        n.needs_think = true;
        if n.heard.is_none() {
            n.heard = Some(format!("Я своими глазами видел: {what}"));
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn healing_is_a_miracle_for_witnesses() {
        let town = Town::new();
        let mut ns = crate::npc::population(&town);
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[0].pos = vec2(0., 12.);
        ns[0].health = 0;
        ns[1].pos = vec2(3., 14.);
        ns[1].facing = vec2(-1., -0.5).normalize();
        let mut bag = PlayerState::default();
        let mut player = vec2(0., 15.);
        let mut p = Powers::new();
        let faith = ns[1].faith;
        let out = p.cast(
            Power::Heal,
            &mut ns,
            &town,
            &mut bag,
            &mut player,
            vec3(0., 1.7, 15.),
            vec3(0., 0., -1.),
            1.,
        );
        assert!(matches!(out, Outcome::Done(_)));
        assert_eq!(ns[0].health, 100);
        assert!(ns[1].faith > faith);
        assert!(ns[1].memory.back().unwrap().contains("исцелился"));
        assert_eq!(bag.grace, 100. - Power::Heal.cost());
        assert!(ns[5].memory.is_empty(), "far residents saw nothing");
    }
    #[test]
    fn powers_cost_grace_and_flight_drains_it() {
        let town = Town::new();
        let mut ns = crate::npc::population(&town);
        let mut bag = PlayerState::default();
        bag.grace = 10.;
        let mut player = vec2(0., 15.);
        let mut p = Powers::new();
        let out = p.cast(
            Power::Bread,
            &mut ns,
            &town,
            &mut bag,
            &mut player,
            vec3(0., 1.7, 15.),
            vec3(0., 0., -1.),
            1.,
        );
        assert!(matches!(out, Outcome::Failed(_)));
        bag.grace = 100.;
        p.cast(
            Power::Flight,
            &mut ns,
            &town,
            &mut bag,
            &mut player,
            vec3(0., 1.7, 15.),
            vec3(0., 0., -1.),
            1.,
        );
        assert!(bag.flying);
        for i in 0..400 {
            p.tick(&mut ns, &town, &mut bag, player, 0.05, 1. + i as f64 * 0.05);
        }
        assert!(!bag.flying, "flight ends when grace runs out");
    }
    #[test]
    fn whisper_becomes_an_inner_voice() {
        let town = Town::new();
        let mut ns = crate::npc::population(&town);
        let mut bag = PlayerState::default();
        let mut p = Powers::new();
        p.whisper(&mut ns, 3, "Иди в храм", &mut bag, 1.);
        assert!(ns[3].heard.as_deref().unwrap().contains("Иди в храм"));
        assert!(ns[3].needs_think);
    }
}
