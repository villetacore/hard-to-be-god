use crate::{
    ai::{Action, Decision, HOME, Line, Request},
    items::{Inventory, Item},
    world::{Town, sees},
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
/// Who an NPC's speech or action is aimed at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Player,
    Npc(usize),
}
pub struct Npc {
    pub name: &'static str,
    pub role: &'static str,
    pub persona: &'static str,
    pub long_goal: &'static str,
    /// Name of the place where this resident works.
    pub work: &'static str,
    pub home: Vec2,
    pub pos: Vec2,
    pub facing: Vec2,
    pub color: Color,
    pub trust: i32,
    pub memory: VecDeque<String>,
    pub path: VecDeque<Vec2>,
    /// Where the current action leads; the engine keeps the path towards it.
    pub dest: Option<Vec2>,
    repath_after: f64,
    pub action: Action,
    pub target: Option<Target>,
    pub item: Option<Item>,
    /// Destination name for `go`.
    pub place: Option<String>,
    pub next_plan: f64,
    /// Set when the current plan ran out once; the next expiry hands over to routine.
    pub plan_stale: bool,
    pub wants_routine: bool,
    /// Time when a timed step (gathering, working) completes.
    pub busy_until: f64,
    pub speech: String,
    pub speech_until: f64,
    /// Words meant for someone far away, said on arrival.
    pub queued_speech: Option<String>,
    pub visible_player: bool,
    pub last_seen_player: Option<Vec2>,
    pub thought: String,
    /// Near-term objective, kept until the model changes it.
    pub goal: String,
    /// What this NPC believes about the player, written by the model.
    pub opinion: String,
    /// Conversation with the player, oldest first.
    pub talk: VecDeque<Line>,
    pub inventory: Inventory,
    /// Belief that the player is a god, 0..100.
    pub faith: i32,
    /// 0 = full, 1 = starving.
    pub hunger: f32,
    /// Affinity towards every resident by index, -100..100.
    pub relations: Vec<i32>,
    pub health: i32,
    pub fear: f32,
    pub hurt_until: f64,
    pub down_until: f64,
    pub threat_until: f64,
    pub attack_after: f64,
    pub personal_space_until: f64,
    pub needs_think: bool,
    pub heard: Option<String>,
    pub heard_from: Option<usize>,
    pub social_after: f64,
    /// Bumped by events that make an in-flight decision stale.
    pub epoch: u64,
}
const TALK_LINES: usize = 10;
/// What the world looks like to one NPC when it is asked to think.
pub struct Outlook<'a> {
    pub time: String,
    pub names: &'a [&'static str],
    pub residents: Vec<String>,
    pub places: Vec<String>,
    /// The named place the NPC is standing at, if any.
    pub here: Option<String>,
}
fn relation_word(v: i32) -> &'static str {
    match v {
        50.. => "близкий человек",
        20..=49 => "друг",
        -9..=19 => "знакомый",
        -29..=-10 => "неприязнь",
        _ => "враг",
    }
}
impl Npc {
    pub fn remember(&mut self, event: String) {
        if self.memory.back() != Some(&event) {
            self.memory.push_back(event);
        }
        while self.memory.len() > 16 {
            self.memory.pop_front();
        }
    }
    pub fn add_talk(&mut self, player: bool, text: String) {
        self.talk.push_back(Line { player, text });
        while self.talk.len() > TALK_LINES {
            self.talk.pop_front();
        }
    }
    pub fn befriend(&mut self, other: usize, delta: i32) {
        if let Some(v) = self.relations.get_mut(other) {
            *v = (*v + delta).clamp(-100, 100);
        }
    }
    pub fn request(&self, id: usize, utterance: Option<String>, world: &Outlook) -> Request {
        let mut observations = vec![match &world.here {
            _ if self.home.distance(self.pos) < 3. => format!("Я, {}, у своего дома.", self.name),
            Some(place) => format!("Я, {}, сейчас здесь: {place}.", self.name),
            None => format!("Я, {}, на улице Велиграда.", self.name),
        }];
        if self.visible_player {
            observations.push("Вижу путника-чужака неподалёку.".into());
        } else {
            observations.push("Путника сейчас не вижу.".into());
        }
        let hunger = match self.hunger {
            h if h > 0.8 => " Я очень голоден.",
            h if h > 0.55 => " Я проголодался.",
            _ => "",
        };
        observations.push(format!(
            "Моё здоровье: {} из 100. Мой страх: {:.0}%.{hunger}",
            self.health,
            self.fear * 100.,
        ));
        observations.push(format!(
            "Моя прошлая мысль: «{}». Сейчас я {}.",
            self.thought,
            self.action.label()
        ));
        if let Some(heard) = &self.heard
            && utterance.is_some()
        {
            observations.push(heard.clone());
        }
        let mut relations: Vec<(i32, String)> = self
            .relations
            .iter()
            .enumerate()
            .filter(|&(j, &v)| {
                j != id && (v.abs() >= 10 || world.residents.iter().any(|r| r == world.names[j]))
            })
            .map(|(j, &v)| {
                (
                    v,
                    format!("{} — {} ({v:+})", world.names[j], relation_word(v)),
                )
            })
            .collect();
        relations.sort_by_key(|(v, _)| -v.abs());
        // The current utterance is sent separately, so leave it out of the history.
        let mut conversation: Vec<Line> = self.talk.iter().cloned().collect();
        if let Some(u) = &utterance
            && conversation
                .last()
                .is_some_and(|l| l.player && &l.text == u)
        {
            conversation.pop();
        }
        Request {
            npc: id,
            generation: self.epoch,
            name: self.name.into(),
            role: self.role.into(),
            persona: self.persona.into(),
            long_goal: self.long_goal.into(),
            goal: self.goal.clone(),
            work: self.work.into(),
            time: world.time.clone(),
            observations,
            // Dialogue with the player already travels as chat turns.
            memories: self
                .memory
                .iter()
                .filter(|m| {
                    !m.starts_with("Путник сказал мне") && !m.starts_with("Мои слова путнику")
                })
                .cloned()
                .collect(),
            inventory: self.inventory.describe(),
            relations: relations.into_iter().take(8).map(|(_, s)| s).collect(),
            faith: self.faith,
            opinion: self.opinion.clone(),
            residents: world.residents.clone(),
            known: world
                .names
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != id)
                .map(|(_, n)| n.to_string())
                .collect(),
            places: world.places.clone(),
            sees_player: self.visible_player,
            conversation,
            // Without a player line, what the NPC just heard becomes the thing to answer.
            addressed: if utterance.is_none() {
                self.heard.clone()
            } else {
                None
            },
            utterance,
            trust: self.trust,
        }
    }
    /// Takes on a decision. Targets, items and places are already resolved by the caller.
    pub fn apply(
        &mut self,
        d: Decision,
        target: Option<Target>,
        dialogue: bool,
        town: &Town,
        player: Vec2,
        now: f64,
    ) {
        if self.health <= 0 {
            return;
        }
        if !d.thought.is_empty() {
            self.thought = d.thought.clone();
        }
        if !d.goal.is_empty() {
            self.goal = d.goal.clone();
        }
        if !d.about_player.is_empty() {
            self.opinion = d.about_player.clone();
        }
        self.needs_think = false;
        self.plan_stale = false;
        self.wants_routine = false;
        self.heard = None;
        self.heard_from = None;
        self.queued_speech = None;
        if dialogue || target == Some(Target::Player) {
            self.trust = (self.trust + d.trust_delta).clamp(-100, 100);
        }
        self.speech = d.speech;
        self.speech_until = now + 14.;
        if !self.speech.is_empty() && (target == Some(Target::Player) || dialogue) {
            self.remember(format!("Мои слова путнику: «{}»", self.speech));
            self.add_talk(false, self.speech.clone());
        }
        self.action = d.action;
        self.target = target;
        self.item = Item::from_name(&d.item);
        self.place = (!d.place.is_empty()).then(|| d.place.clone());
        self.busy_until = 0.;
        if self.action == Action::Guard && self.role != "стражник" {
            self.action = Action::Alarm;
        }
        // Danger the NPC has witnessed overrides calm plans, but not chosen reactions.
        if now < self.threat_until
            && !matches!(
                self.action,
                Action::Flee | Action::Guard | Action::Alarm | Action::Bow | Action::Pray
            )
        {
            self.action = if self.role == "стражник" {
                Action::Guard
            } else {
                Action::Flee
            };
        }
        if matches!(self.action, Action::Follow | Action::Bow) || dialogue {
            self.target = self.target.or(Some(Target::Player));
        }
        if matches!(self.action, Action::Approach | Action::Follow | Action::Bow)
            && self.target == Some(Target::Player)
            && !self.visible_player
            && !dialogue
        {
            // Cannot act on someone it does not see; look around instead.
            self.action = Action::Idle;
        }
        self.dest = None;
        self.path.clear();
        if self.action == Action::Flee {
            let from = self.last_seen_player.unwrap_or(player);
            self.dest = town
                .destinations
                .iter()
                .max_by(|a, b| {
                    a.distance_squared(from)
                        .total_cmp(&b.distance_squared(from))
                })
                .copied();
        }
        self.next_plan = now
            + match self.action {
                Action::Wander | Action::Guard => 0.,
                Action::Flee => 6.,
                Action::Bow | Action::Use => 6.,
                Action::Alarm => 4.,
                Action::Idle => 18.,
                Action::Approach | Action::Talk => 20.,
                Action::Follow => 40.,
                // Errands end when done; the timer is only a safety net.
                Action::Go | Action::Ask | Action::Give | Action::Fetch | Action::Pray => 90.,
                Action::Work => 70.,
            };
    }
    pub fn home_or(&self, town: &Town, place: &str) -> Option<Vec2> {
        if place == HOME {
            Some(self.home)
        } else {
            town.place(place).map(|p| p.pos)
        }
    }
    pub fn update(&mut self, town: &Town, player: Vec2, dt: f32, now: f64, talking: bool) {
        if self.health <= 0 {
            self.path.clear();
            self.thought = "Мне нужно прийти в себя.".into();
            if now >= self.down_until {
                self.health = 40;
                self.fear = 0.6;
                self.next_plan = 0.;
                self.needs_think = true;
            } else {
                return;
            }
        }
        self.fear = (self.fear - dt * 0.006).max(0.);
        self.hunger = (self.hunger + dt / 700.).min(1.);
        if now >= self.threat_until {
            self.threat_until = 0.;
            if self.action == Action::Guard {
                self.action = Action::Idle;
                self.next_plan = now + 4.;
                self.needs_think = true;
            }
        }
        let visible = sees(town, self.pos, self.facing, player);
        if visible && !self.visible_player {
            self.remember("Видел путника на улице.".into());
            self.needs_think = true;
        }
        self.visible_player = visible;
        if visible {
            self.last_seen_player = Some(player);
        }
        if talking && now >= self.threat_until {
            self.path.clear();
            self.facing = (player - self.pos).normalize_or_zero();
            return;
        }
        if now < self.threat_until && self.action != Action::Alarm {
            self.action = if self.role == "стражник" {
                Action::Guard
            } else {
                Action::Flee
            };
        }
        if now > self.next_plan && self.action != Action::Guard {
            if !self.plan_stale {
                // The plan ran out: keep it a little longer and ask for a new decision.
                self.plan_stale = true;
                self.needs_think = true;
                self.next_plan = now + 12.;
            } else {
                self.plan_stale = false;
                self.wants_routine = true;
                self.next_plan = now + 30.;
            }
        }
        if let Some(dest) = self.dest {
            if dest.distance(self.pos) <= 0.9 {
                self.path.clear();
            } else if (self.path.is_empty()
                || self.path.back().is_some_and(|end| end.distance(dest) > 1.5))
                && now >= self.repath_after
            {
                self.path = town.path(self.pos, dest);
                self.repath_after = now + if self.path.is_empty() { 2. } else { 0.6 };
            }
        } else {
            self.path.clear();
        }
        if let Some(goal) = self.path.front().copied() {
            let d = goal - self.pos;
            if d.length() < 0.12 {
                self.path.pop_front();
            } else {
                self.facing = d.normalize();
                let speed = match self.action {
                    Action::Flee | Action::Guard => 2.6,
                    Action::Follow | Action::Fetch | Action::Ask | Action::Give => 1.9,
                    _ => 1.3,
                };
                self.pos =
                    town.move_actor(self.pos, self.facing * (speed * dt).min(d.length()), 0.32);
            }
        }
    }

    pub fn draw(&self, time: f32) {
        if self.health <= 0 {
            draw_cube(
                vec3(self.pos.x, 0.22, self.pos.y),
                vec3(0.5, 0.4, 1.6),
                None,
                self.color,
            );
            return;
        }
        let p = vec3(self.pos.x, 0., self.pos.y);
        let yaw = self.facing.x.atan2(self.facing.y);
        let mut transform = Mat4::from_rotation_translation(Quat::from_rotation_y(yaw), p);
        let praying = self.action == Action::Pray && self.path.is_empty();
        if self.action == Action::Bow || praying {
            // Lean the whole figure forward from the hips; kneel when praying.
            transform = transform
                * Mat4::from_translation(vec3(0., if praying { 0.35 } else { 0.7 }, 0.))
                * Mat4::from_rotation_x(if praying { 0.3 } else { 0.55 })
                * Mat4::from_translation(vec3(0., -0.7, 0.));
        }
        unsafe {
            get_internal_gl().quad_gl.push_model_matrix(transform);
        }
        let step = if self.path.is_empty() {
            0.
        } else {
            (time * 7.).sin() * 0.16
        };
        let dark = Color::new(0.16, 0.14, 0.12, 1.);
        draw_cube(vec3(-0.14, 0.3, step), vec3(0.21, 0.6, 0.24), None, dark);
        draw_cube(vec3(0.14, 0.3, -step), vec3(0.21, 0.6, 0.24), None, dark);
        draw_cube(vec3(0., 0.96, 0.), vec3(0.57, 0.75, 0.34), None, self.color);
        draw_cube(vec3(0., 0.7, 0.), vec3(0.59, 0.09, 0.36), None, dark);
        let skin = if (time as f64) < self.hurt_until {
            RED
        } else {
            Color::new(0.76, 0.56, 0.38, 1.)
        };
        draw_cube(
            vec3(-0.37, 0.95, -step),
            vec3(0.16, 0.62, 0.2),
            None,
            self.color,
        );
        draw_cube(
            vec3(0.37, 0.95, step),
            vec3(0.16, 0.62, 0.2),
            None,
            self.color,
        );
        // Carried goods are shown in the hand.
        if matches!(self.action, Action::Give | Action::Fetch)
            && let Some(item) = self.item
            && self.inventory.has(item)
        {
            draw_cube(
                vec3(0.37, 0.72, 0.22),
                vec3(0.2, 0.18, 0.2),
                None,
                item.color(),
            );
        }
        draw_cube(vec3(0., 1.54, 0.), vec3(0.36, 0.4, 0.34), None, skin);
        for x in [-0.09, 0.09] {
            draw_cube(vec3(x, 1.57, 0.175), vec3(0.035, 0.04, 0.015), None, dark);
        }
        self.draw_attire(dark);
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }
    pub fn is_woman(&self) -> bool {
        self.name == "Ада" || self.role.ends_with('а')
    }
    /// Role-specific clothing and tools drawn over the base figure (local space, facing +z).
    fn draw_attire(&self, dark: Color) {
        let iron = Color::new(0.55, 0.57, 0.6, 1.);
        let wood = Color::new(0.42, 0.28, 0.16, 1.);
        let cloth = Color::new(0.88, 0.85, 0.76, 1.);
        let robe = Color::new(0.12, 0.1, 0.11, 1.);
        let hair = |c| draw_cube(vec3(0., 1.75, -0.02), vec3(0.4, 0.13, 0.38), None, c);
        match self.role {
            "стражник" => {
                draw_cube(vec3(0., 1.78, 0.), vec3(0.42, 0.16, 0.4), None, iron);
                draw_cube(vec3(0., 1.88, 0.), vec3(0.06, 0.08, 0.36), None, iron);
                draw_cube(vec3(0., 1.6, 0.18), vec3(0.04, 0.16, 0.03), None, iron);
                draw_cube(
                    vec3(0., 1., 0.18),
                    vec3(0.5, 0.5, 0.02),
                    None,
                    Color::new(0.6, 0.12, 0.1, 1.),
                );
                draw_cube(vec3(0.47, 1.2, 0.18), vec3(0.06, 2.3, 0.06), None, wood);
                draw_cube(vec3(0.47, 2.42, 0.18), vec3(0.1, 0.22, 0.04), None, iron);
            }
            "священник" | "проповедник" => {
                let c = if self.role == "священник" {
                    robe
                } else {
                    Color::new(0.4, 0.33, 0.24, 1.)
                };
                draw_cube(vec3(0., 0.55, 0.), vec3(0.6, 0.85, 0.38), None, c);
                draw_cube(vec3(0., 1.02, 0.), vec3(0.6, 0.66, 0.37), None, c);
                draw_cube(vec3(0., 1.74, -0.06), vec3(0.42, 0.2, 0.36), None, c);
                draw_cube(vec3(0., 1.6, -0.17), vec3(0.42, 0.4, 0.06), None, c);
                draw_cube(vec3(0., 1.0, 0.19), vec3(0.05, 0.28, 0.02), None, GOLD);
                draw_cube(vec3(0., 1.06, 0.19), vec3(0.18, 0.05, 0.02), None, GOLD);
                if self.role == "проповедник" {
                    draw_cube(vec3(-0.47, 1.2, 0.18), vec3(0.06, 2.1, 0.06), None, wood);
                }
            }
            "пекарша" | "кузнец" | "трактирщик" | "служанка" => {
                let apron = match self.role {
                    "кузнец" => Color::new(0.3, 0.2, 0.13, 1.),
                    _ => cloth,
                };
                draw_cube(vec3(0., 0.72, 0.18), vec3(0.46, 0.75, 0.02), None, apron);
                if self.role == "кузнец" {
                    hair(dark);
                    draw_cube(vec3(0.42, 0.6, 0.2), vec3(0.06, 0.4, 0.06), None, wood);
                    draw_cube(vec3(0.42, 0.42, 0.2), vec3(0.2, 0.1, 0.12), None, iron);
                } else if self.is_woman() {
                    draw_cube(vec3(0., 1.76, -0.02), vec3(0.42, 0.14, 0.4), None, cloth);
                } else {
                    hair(dark);
                }
            }
            "лекарь" | "писец" | "переписчик" => {
                draw_cube(vec3(0., 0.55, 0.), vec3(0.6, 0.85, 0.38), None, self.color);
                if self.is_woman() {
                    hair(Color::new(0.45, 0.28, 0.14, 1.));
                    draw_cube(
                        vec3(0., 1.4, -0.2),
                        vec3(0.3, 0.5, 0.06),
                        None,
                        Color::new(0.45, 0.28, 0.14, 1.),
                    );
                } else {
                    draw_cube(vec3(0., 1.79, 0.), vec3(0.38, 0.1, 0.36), None, dark);
                }
                draw_cube(vec3(0.37, 0.72, 0.18), vec3(0.18, 0.24, 0.08), None, wood);
            }
            "купец" => {
                draw_cube(
                    vec3(0., 1.8, 0.),
                    vec3(0.5, 0.06, 0.48),
                    None,
                    Color::new(0.3, 0.12, 0.3, 1.),
                );
                draw_cube(
                    vec3(0., 1.88, 0.),
                    vec3(0.36, 0.14, 0.34),
                    None,
                    Color::new(0.3, 0.12, 0.3, 1.),
                );
                draw_cube(vec3(0., 0.72, 0.), vec3(0.62, 0.08, 0.38), None, GOLD);
                draw_cube(vec3(-0.2, 0.62, 0.2), vec3(0.12, 0.14, 0.06), None, GOLD);
            }
            "нищий" => {
                hair(Color::new(0.5, 0.48, 0.44, 1.));
                draw_cube(
                    vec3(0., 1.42, 0.15),
                    vec3(0.3, 0.16, 0.06),
                    None,
                    Color::new(0.5, 0.48, 0.44, 1.),
                );
                draw_cube(
                    vec3(0.05, 1.05, 0.18),
                    vec3(0.18, 0.16, 0.02),
                    None,
                    Color::new(0.38, 0.33, 0.25, 1.),
                );
            }
            "мальчишка" => {
                hair(Color::new(0.75, 0.55, 0.25, 1.));
                draw_cube(
                    vec3(0., 1.84, 0.),
                    vec3(0.3, 0.08, 0.3),
                    None,
                    Color::new(0.35, 0.3, 0.2, 1.),
                );
            }
            "рыбак" => {
                draw_cube(
                    vec3(0., 1.79, 0.),
                    vec3(0.6, 0.04, 0.58),
                    None,
                    Color::new(0.75, 0.65, 0.35, 1.),
                );
                draw_cube(
                    vec3(0., 1.86, 0.),
                    vec3(0.34, 0.12, 0.32),
                    None,
                    Color::new(0.75, 0.65, 0.35, 1.),
                );
                draw_cube(vec3(0.46, 1.4, 0.3), vec3(0.04, 1.8, 0.04), None, wood);
            }
            "мельник" => {
                hair(dark);
                draw_cube(vec3(0., 1.84, 0.), vec3(0.4, 0.1, 0.38), None, cloth);
                draw_cube(vec3(-0.2, 1.1, -0.26), vec3(0.4, 0.5, 0.2), None, cloth);
            }
            "знахарка" => {
                let shawl = Color::new(0.2, 0.25, 0.18, 1.);
                draw_cube(vec3(0., 1.72, -0.02), vec3(0.44, 0.24, 0.42), None, shawl);
                draw_cube(vec3(0., 1.24, -0.05), vec3(0.66, 0.2, 0.42), None, shawl);
                draw_cube(vec3(-0.47, 1., 0.18), vec3(0.06, 2., 0.06), None, wood);
            }
            _ if self.is_woman() => {
                let scarf = Color::new(
                    0.75 - self.color.r * 0.3,
                    0.3 + self.color.g * 0.2,
                    0.28,
                    1.,
                );
                draw_cube(vec3(0., 1.74, -0.02), vec3(0.42, 0.2, 0.4), None, scarf);
                draw_cube(vec3(0., 1.54, -0.18), vec3(0.42, 0.42, 0.06), None, scarf);
                draw_cube(vec3(0., 0.5, 0.), vec3(0.6, 0.62, 0.38), None, self.color);
            }
            _ => hair(dark),
        }
    }
}
type Def = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [(Item, u32)],
    i32,
);
/// name, role, persona, life goal, workplace, starting goods, faith.
const PEOPLE: [Def; 22] = [
    (
        "Марта",
        "торговка",
        "Продаёшь травы на рынке. Добрая, но пугливая. Твой сын болен, нужны деньги на лекарство. Дружишь с лекарем Ильёй, терпеть не можешь купца Елисея, который сбивает цены.",
        "накопить 15 монет на лекарство для больного сына",
        "рынок",
        &[(Item::Herbs, 4), (Item::Coin, 3)],
        5,
    ),
    (
        "Бран",
        "стражник",
        "Охраняешь рынок. Служака, дорожишь порядком, боишься капитана Рогволда. Не доверяешь незнакомцам.",
        "выслужиться перед капитаном и стать десятником",
        "рынок",
        &[(Item::Coin, 5)],
        0,
    ),
    (
        "Илья",
        "лекарь",
        "Лечишь людей в лекарне и тайно прячешь запрещённые книги. Осторожен и умён. Марта носит тебе травы, повитуха Марфа помогает прятать книги.",
        "сохранить запрещённые книги от сожжения",
        "лекарня",
        &[(Item::Medicine, 3), (Item::Herbs, 1), (Item::Book, 1)],
        5,
    ),
    (
        "Томас",
        "кузнец",
        "Куёшь инструменты, ценишь честность и прямоту. Ненавидишь поборы стражи. Твоя жена — Дарья.",
        "поднять людей против поборов стражи",
        "кузница",
        &[(Item::Tool, 3), (Item::Coin, 4)],
        0,
    ),
    (
        "Ада",
        "писец",
        "Переписываешь книги в скриптории. Умна и мечтательна, боишься выдать свою образованность. Дружишь со служанкой Златой.",
        "сбежать из Велиграда в вольный Арканар",
        "скрипторий",
        &[(Item::Book, 2), (Item::Candle, 1), (Item::Coin, 2)],
        5,
    ),
    (
        "Орен",
        "трактирщик",
        "Держишь трактир. Любишь слухи и прибыль, всем наливаешь и всё про всех знаешь. Злата служит у тебя.",
        "разбогатеть и купить второй трактир",
        "трактир",
        &[(Item::Ale, 6), (Item::Bread, 2), (Item::Coin, 10)],
        5,
    ),
    (
        "Кирилл",
        "священник",
        "Ты отец Кирилл, служишь в храме. Искренне веришь и давно ждёшь знамения свыше, но боишься ложных пророков и гнева капитана.",
        "дождаться настоящего знамения и укрепить веру прихожан",
        "храм",
        &[(Item::Candle, 5), (Item::Book, 1)],
        20,
    ),
    (
        "Рогволд",
        "стражник",
        "Ты капитан городской стражи. Жёсткий и властный, не доверяешь чужакам и смутьянам. Берёшь мзду с торговцев.",
        "держать город в страхе и выявить смутьянов",
        "казарма",
        &[(Item::Coin, 15)],
        0,
    ),
    (
        "Ждан",
        "стражник",
        "Молодой стражник, добрый и неуверенный. Влюблён в пекаршу Милу. Боишься капитана Рогволда.",
        "скопить на свадьбу с Милой",
        "казарма",
        &[(Item::Coin, 2)],
        5,
    ),
    (
        "Мила",
        "пекарша",
        "Печёшь хлеб в пекарне. Весёлая и бережливая. Тебе нравится стражник Ждан, муку берёшь у мельника Гордея.",
        "открыть свою лавку на рынке",
        "пекарня",
        &[(Item::Bread, 6), (Item::Flour, 1), (Item::Coin, 3)],
        5,
    ),
    (
        "Гордей",
        "мельник",
        "Мелешь зерно на мельнице. Скупой и ворчливый, считаешь каждую монету. Твоя дочь — Варя.",
        "выдать дочь Варю за богатого жениха",
        "мельница",
        &[(Item::Flour, 6), (Item::Coin, 8)],
        0,
    ),
    (
        "Варя",
        "огородница",
        "Растишь травы на огородах. Мечтательная, тайно влюблена в стражника Ждана. Твой отец — мельник Гордей.",
        "чтобы Ждан тебя заметил",
        "огороды",
        &[(Item::Herbs, 3)],
        10,
    ),
    (
        "Сенька",
        "мальчишка",
        "Беспризорный мальчишка, вечно голодный и шустрый. Таскаешь еду с рынка и всех знаешь. Боишься стражи.",
        "наесться досыта и найти своего отца",
        "рынок",
        &[],
        15,
    ),
    (
        "Фома",
        "рыбак",
        "Ловишь рыбу у пристани. Простой и суеверный, веришь в приметы.",
        "накопить на новую лодку",
        "пристань",
        &[(Item::Fish, 4), (Item::Coin, 2)],
        15,
    ),
    (
        "Агафья",
        "знахарка",
        "Старая знахарка, живёшь у огородов. Люди шепчутся, что ты ведьма. Во всём видишь знаки и предзнаменования.",
        "доказать, что ты не ведьма",
        "огороды",
        &[(Item::Herbs, 5), (Item::Candle, 1)],
        25,
    ),
    (
        "Лука",
        "проповедник",
        "Бродячий проповедник. Пламенный и одержимый, ищешь мессию и готов пойти за любым, кто явит чудо.",
        "найти мессию и стать его первым учеником",
        "храм",
        &[(Item::Book, 1)],
        35,
    ),
    (
        "Дарья",
        "жена кузнеца",
        "Жена кузнеца Томаса, торгуешь его инструментами на рынке. Практичная и острая на язык, боишься, что муж попадёт в беду со стражей.",
        "уберечь мужа от беды",
        "рынок",
        &[(Item::Tool, 2), (Item::Coin, 3)],
        5,
    ),
    (
        "Елисей",
        "купец",
        "Богатый купец. Хитрый и высокомерный, хочешь подмять всю торговлю травами, а Марта тебе мешает. Платишь капитану Рогволду.",
        "прибрать к рукам всю торговлю на рынке",
        "рынок",
        &[(Item::Coin, 25), (Item::Book, 1), (Item::Tool, 1)],
        0,
    ),
    (
        "Прохор",
        "нищий",
        "Бывший солдат, спившийся нищий. Ворчишь и клянчишь эль у трактира, но в душе честен и помнишь войну.",
        "выпросить кружку эля и вернуть былую честь",
        "трактир",
        &[(Item::Coin, 1)],
        10,
    ),
    (
        "Злата",
        "служанка",
        "Служанка в трактире Орена. Бойкая, мечтаешь вырваться из города вместе с подругой Адой.",
        "накопить денег и уехать с Адой",
        "трактир",
        &[(Item::Coin, 4), (Item::Ale, 1)],
        5,
    ),
    (
        "Никон",
        "переписчик",
        "Работаешь в скриптории и тайно доносишь капитану Рогволду о вольнодумцах. Вежливый и скользкий.",
        "выявить еретиков и получить награду от капитана",
        "скрипторий",
        &[(Item::Book, 1), (Item::Coin, 6)],
        0,
    ),
    (
        "Марфа",
        "повитуха",
        "Помогаешь лекарю Илье в лекарне и прячешь его книги. Тёплая и мудрая, всех в городе знаешь с рождения.",
        "сберечь Илью и его книги",
        "лекарня",
        &[(Item::Medicine, 1), (Item::Herbs, 2)],
        10,
    ),
];
/// Who feels what about whom at the start: (from, to, affinity).
const BONDS: [(&str, &str, i32); 34] = [
    ("Марта", "Илья", 40),
    ("Илья", "Марта", 40),
    ("Марта", "Елисей", -40),
    ("Елисей", "Марта", -20),
    ("Бран", "Рогволд", 20),
    ("Ждан", "Мила", 60),
    ("Мила", "Ждан", 50),
    ("Варя", "Ждан", 50),
    ("Варя", "Мила", -20),
    ("Томас", "Дарья", 70),
    ("Дарья", "Томас", 70),
    ("Томас", "Рогволд", -50),
    ("Томас", "Бран", -20),
    ("Ада", "Злата", 60),
    ("Злата", "Ада", 60),
    ("Орен", "Злата", 20),
    ("Кирилл", "Лука", -10),
    ("Лука", "Кирилл", 10),
    ("Рогволд", "Елисей", 30),
    ("Елисей", "Рогволд", 20),
    ("Никон", "Рогволд", 40),
    ("Рогволд", "Никон", 20),
    ("Илья", "Марфа", 60),
    ("Марфа", "Илья", 60),
    ("Гордей", "Варя", 40),
    ("Варя", "Гордей", 30),
    ("Мила", "Гордей", 10),
    ("Сенька", "Мила", 20),
    ("Сенька", "Бран", -30),
    ("Орен", "Прохор", -10),
    ("Агафья", "Варя", 20),
    ("Никон", "Ада", -20),
    ("Дарья", "Рогволд", -30),
    ("Ждан", "Рогволд", 10),
];
pub fn names(npcs: &[Npc]) -> Vec<&'static str> {
    npcs.iter().map(|n| n.name).collect()
}
pub fn population(town: &Town) -> Vec<Npc> {
    let count = PEOPLE.len();
    let mut people: Vec<Npc> = PEOPLE
        .iter()
        .enumerate()
        .map(
            |(i, &(name, role, persona, long_goal, work, goods, faith))| {
                let hue = i as f32 / count as f32;
                let color = Color::new(
                    0.3 + 0.35 * (hue * std::f32::consts::TAU).sin().abs(),
                    0.28 + 0.3 * (hue * std::f32::consts::TAU + 2.).sin().abs(),
                    0.25 + 0.3 * (hue * std::f32::consts::TAU + 4.).sin().abs(),
                    1.,
                );
                let workplace = town.place(work).map(|p| p.pos).unwrap_or(Vec2::ZERO);
                let offset = vec2((i % 3) as f32 - 1., (i % 2) as f32 * 1.5 - 0.5) * 1.6;
                Npc {
                    name,
                    role,
                    persona,
                    long_goal,
                    work,
                    home: town.door(i),
                    pos: town.nearest_walkable(workplace + offset),
                    facing: vec2(0., 1.),
                    color,
                    trust: 0,
                    memory: VecDeque::new(),
                    path: VecDeque::new(),
                    dest: None,
                    repath_after: 0.,
                    action: Action::Idle,
                    target: None,
                    item: None,
                    place: None,
                    next_plan: 8. + i as f64 * 1.5,
                    plan_stale: false,
                    wants_routine: false,
                    busy_until: 0.,
                    speech: String::new(),
                    speech_until: 0.,
                    queued_speech: None,
                    visible_player: false,
                    last_seen_player: None,
                    thought: "Нужно заняться своими делами.".into(),
                    goal: String::new(),
                    opinion: String::new(),
                    talk: VecDeque::new(),
                    inventory: Inventory::of(goods),
                    faith,
                    hunger: 0.2 + (i % 5) as f32 * 0.1,
                    relations: vec![0; count],
                    health: 100,
                    fear: 0.,
                    hurt_until: 0.,
                    down_until: 0.,
                    threat_until: 0.,
                    attack_after: 0.,
                    personal_space_until: 0.,
                    needs_think: false,
                    heard: None,
                    heard_from: None,
                    social_after: 0.,
                    epoch: 0,
                }
            },
        )
        .collect();
    let index = |name: &str| PEOPLE.iter().position(|p| p.0 == name);
    for (from, to, v) in BONDS {
        if let (Some(a), Some(b)) = (index(from), index(to)) {
            people[a].relations[b] = v;
        }
    }
    people
}
fn full_health() -> i32 {
    100
}
#[derive(Serialize, Deserialize)]
pub struct SavedNpc {
    pub name: String,
    pub trust: i32,
    pub memories: VecDeque<String>,
    #[serde(default = "full_health")]
    pub health: i32,
    #[serde(default)]
    pub fear: f32,
    #[serde(default)]
    pub thought: String,
    #[serde(default)]
    pub opinion: String,
    #[serde(default)]
    pub talk: VecDeque<Line>,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub inventory: Option<Inventory>,
    #[serde(default)]
    pub faith: Option<i32>,
    #[serde(default)]
    pub hunger: f32,
    #[serde(default)]
    pub relations: HashMap<String, i32>,
}

#[derive(Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub npcs: Vec<SavedNpc>,
}
pub fn save(path: &std::path::Path, npcs: &[Npc]) -> Result<(), String> {
    let names = names(npcs);
    let data = Save {
        version: 4,
        npcs: npcs
            .iter()
            .map(|n| SavedNpc {
                name: n.name.into(),
                trust: n.trust,
                memories: n.memory.clone(),
                health: n.health,
                fear: n.fear,
                thought: n.thought.clone(),
                opinion: n.opinion.clone(),
                talk: n.talk.clone(),
                goal: n.goal.clone(),
                inventory: Some(n.inventory.clone()),
                faith: Some(n.faith),
                hunger: n.hunger,
                relations: n
                    .relations
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v != 0)
                    .map(|(j, v)| (names[j].to_string(), *v))
                    .collect(),
            })
            .collect(),
    };
    std::fs::create_dir_all(path.parent().ok_or("save path")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn load(path: &std::path::Path, npcs: &mut [Npc]) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let data: Save = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !(1..=4).contains(&data.version) {
        return Err("Неизвестная версия сохранения".into());
    }
    let names = names(npcs);
    for npc in npcs {
        if let Some(saved) = data.npcs.iter().find(|s| s.name == npc.name) {
            npc.trust = saved.trust.clamp(-100, 100);
            npc.health = saved.health.clamp(1, 100);
            npc.fear = if saved.fear.is_finite() {
                saved.fear.clamp(0., 1.)
            } else {
                0.
            };
            npc.thought = saved.thought.chars().take(180).collect();
            npc.opinion = saved.opinion.chars().take(160).collect();
            npc.goal = saved.goal.chars().take(140).collect();
            if let Some(inv) = &saved.inventory {
                npc.inventory = inv.clone();
            }
            if let Some(faith) = saved.faith {
                npc.faith = faith.clamp(0, 100);
            }
            npc.hunger = if saved.hunger.is_finite() {
                saved.hunger.clamp(0., 1.)
            } else {
                0.
            };
            npc.relations.fill(0);
            for (name, v) in &saved.relations {
                if let Some(j) = names.iter().position(|n| n == name) {
                    npc.relations[j] = (*v).clamp(-100, 100);
                }
            }
            npc.talk = saved
                .talk
                .iter()
                .rev()
                .take(TALK_LINES)
                .rev()
                .map(|l| Line {
                    player: l.player,
                    text: l.text.chars().take(260).collect(),
                })
                .collect();
            npc.path.clear();
            npc.dest = None;
            npc.action = Action::Idle;
            npc.target = None;
            npc.item = None;
            npc.place = None;
            npc.heard_from = None;
            npc.next_plan = 0.;
            npc.heard = None;
            npc.threat_until = 0.;
            npc.needs_think = true;
            npc.epoch += 1;
            npc.memory = saved
                .memories
                .iter()
                .rev()
                .take(16)
                .rev()
                .map(|s| s.chars().take(600).collect())
                .collect();
        }
    }
    Ok(())
}

#[cfg(test)]
mod save_tests {
    use super::*;

    #[test]
    fn loading_restores_zero_relations_instead_of_current_values() {
        let town = Town::new();
        let mut people = population(&town);
        let path =
            std::env::temp_dir().join(format!("veligrad-relations-{}.json", std::process::id()));
        people[0].relations[1] = 0;
        save(&path, &people).unwrap();
        people[0].relations[1] = 90;
        load(&path, &mut people).unwrap();
        assert_eq!(people[0].relations[1], 0);
        std::fs::remove_file(path).unwrap();
    }
}
