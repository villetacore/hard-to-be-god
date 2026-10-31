use crate::{
    ai::{Action, Decision, HOME, PLAYER},
    items::{Inventory, Item},
    npc::{Npc, Target},
    world::{Town, sees},
};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

/// Real seconds in one game day.
pub const DAY_LENGTH: f64 = 480.;
pub fn hour(now: f64) -> f32 {
    ((8. + now / DAY_LENGTH * 24.) % 24.) as f32
}
pub fn day(now: f64) -> u32 {
    1 + ((8. + now / DAY_LENGTH * 24.) / 24.) as u32
}
pub fn phase(now: f64) -> &'static str {
    match hour(now) {
        h if !(5. ..22.).contains(&h) => "ночь",
        h if h < 10. => "утро",
        h if h < 18. => "день",
        _ => "вечер",
    }
}
pub fn time_text(now: f64) -> String {
    let h = hour(now);
    format!(
        "{}, {:02}:{:02}",
        phase(now),
        h as u32,
        ((h.fract()) * 60.) as u32
    )
}
/// How dark the scene is, 0 at noon to ~0.6 at midnight.
pub fn darkness(now: f64) -> f32 {
    let h = hour(now);
    let from_noon = ((h - 13.).abs() / 11.).min(1.);
    (from_noon.powf(2.5) * 0.62).clamp(0., 0.62)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerState {
    pub health: i32,
    pub items: Inventory,
    pub collected: Vec<usize>,
    pub grace: f32,
    // Saves before inventories existed stored these separately.
    #[serde(skip_serializing)]
    bread: Option<u32>,
    #[serde(skip_serializing)]
    medicine: Option<u32>,
    #[serde(skip_serializing)]
    coins: Option<u32>,
    #[serde(skip)]
    pub selected: usize,
    #[serde(skip)]
    pub flying: bool,
    #[serde(skip)]
    pub height: f32,
    #[serde(skip)]
    pub velocity: f32,
    #[serde(skip)]
    pub punch_until: f64,
    #[serde(skip)]
    pub next_punch: f64,
    #[serde(skip)]
    pub hurt_until: f64,
}
impl Default for PlayerState {
    fn default() -> Self {
        Self {
            health: 100,
            items: Inventory::of(&[(Item::Bread, 2), (Item::Medicine, 1), (Item::Coin, 8)]),
            collected: vec![],
            grace: 100.,
            bread: None,
            medicine: None,
            coins: None,
            selected: 0,
            flying: false,
            height: 0.,
            velocity: 0.,
            punch_until: 0.,
            next_punch: 0.,
            hurt_until: 0.,
        }
    }
}
impl PlayerState {
    pub fn jump(&mut self) -> bool {
        if self.height <= 0. && self.velocity <= 0. && !self.flying {
            self.velocity = 5.2;
            true
        } else {
            false
        }
    }
    pub fn physics(&mut self, dt: f32) {
        if self.flying {
            self.velocity = 0.;
            self.height += (3.2 - self.height) * (dt * 1.5).min(1.);
            return;
        }
        self.velocity -= 14. * dt;
        self.height += self.velocity * dt;
        if self.height <= 0. {
            self.height = 0.;
            self.velocity = 0.;
        }
    }
    pub fn selected_item(&self) -> Option<Item> {
        let kinds = self.items.kinds();
        if kinds.is_empty() {
            None
        } else {
            Some(kinds[self.selected % kinds.len()])
        }
    }
    pub fn cycle(&mut self, step: i32) {
        let n = self.items.kinds().len().max(1) as i32;
        self.selected = (self.selected as i32 + step).rem_euclid(n) as usize;
    }
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let tmp = path.with_extension("tmp");
        std::fs::create_dir_all(path.parent().ok_or("save path")?).map_err(|e| e.to_string())?;
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let mut p: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if p.bread.is_some() || p.medicine.is_some() || p.coins.is_some() {
            p.items = Inventory::of(&[
                (Item::Bread, p.bread.take().unwrap_or(0)),
                (Item::Medicine, p.medicine.take().unwrap_or(0)),
                (Item::Coin, p.coins.take().unwrap_or(0)),
            ]);
        }
        p.health = p.health.clamp(1, 100);
        p.grace = if p.grace.is_finite() {
            p.grace.clamp(0., 100.)
        } else {
            100.
        };
        Ok(p)
    }
}
pub struct Pickup {
    pub pos: Vec2,
    pub item: Item,
    pub taken: bool,
}
pub fn pickups() -> Vec<Pickup> {
    [
        (vec2(-2., 16.), Item::Bread),
        (vec2(2., 15.), Item::Medicine),
        (vec2(-4., 4.), Item::Coin),
        (vec2(4., -7.), Item::Bread),
        (vec2(-30., 2.), Item::Tool),
        (vec2(31., -2.), Item::Ale),
        (vec2(2., -36.), Item::Candle),
        (vec2(-33., -20.), Item::Book),
        (vec2(26., 39.), Item::Herbs),
        (vec2(-6., 47.), Item::Fish),
        (vec2(-34., 33.), Item::Flour),
        (vec2(20., -32.), Item::Coin),
        (vec2(-20., 30.), Item::Coin),
        (vec2(44., 18.), Item::Medicine),
    ]
    .into_iter()
    .map(|(pos, item)| Pickup {
        pos,
        item,
        taken: false,
    })
    .collect()
}
pub fn pickup(
    items: &mut [Pickup],
    player: Vec2,
    bag: &mut PlayerState,
    town: &Town,
) -> Option<&'static str> {
    for (id, p) in items.iter_mut().enumerate() {
        if !p.taken && p.pos.distance(player) < 2. && town.clear_line(player, p.pos) {
            p.taken = true;
            bag.collected.push(id);
            let n = if p.item == Item::Coin { 3 } else { 1 };
            bag.items.add(p.item, n);
            return Some(p.item.name());
        }
    }
    None
}
pub fn draw_pickups(items: &[Pickup], time: f32) {
    for p in items.iter().filter(|i| !i.taken) {
        let y = 0.28 + (time * 2.).sin() * 0.05;
        draw_cube(
            vec3(p.pos.x, y, p.pos.y),
            vec3(0.35, 0.3, 0.25),
            None,
            p.item.color(),
        );
    }
}
pub fn hit_target(
    town: &Town,
    npcs: &[Npc],
    position: Vec2,
    front: Vec2,
    height: f32,
) -> Option<usize> {
    if height > 0.7 {
        return None;
    }
    aimed(town, npcs, position, front, 2.3, 0.72, false)
}
/// The resident in the player's sights, if any.
pub fn aimed(
    town: &Town,
    npcs: &[Npc],
    position: Vec2,
    front: Vec2,
    range: f32,
    cone: f32,
    include_down: bool,
) -> Option<usize> {
    npcs.iter()
        .enumerate()
        .filter(|(_, n)| include_down || n.health > 0)
        .filter(|(_, n)| {
            let d = n.pos - position;
            d.length() < range
                && (d.length() < 0.8 || front.dot(d.normalize_or_zero()) > cone)
                && town.clear_line(position, n.pos)
        })
        .min_by(|(_, a), (_, b)| {
            a.pos
                .distance_squared(position)
                .total_cmp(&b.pos.distance_squared(position))
        })
        .map(|(i, _)| i)
}
pub fn assault(npcs: &mut [Npc], victim: usize, player: Vec2, town: &Town, now: f64) {
    let victim_name = npcs[victim].name;
    let victim_pos = npcs[victim].pos;
    npcs[victim].health = (npcs[victim].health - 20).max(0);
    npcs[victim].hurt_until = now + 0.4;
    if npcs[victim].health == 0 {
        npcs[victim].down_until = now + 25.;
    }
    for (i, n) in npcs.iter_mut().enumerate() {
        if n.health == 0 && i != victim {
            continue;
        }
        let witnessed = i == victim
            || (sees(town, n.pos, n.facing, player) && town.clear_line(n.pos, victim_pos));
        if witnessed {
            n.epoch += 1;
            n.last_seen_player = Some(player);
            n.remember(if i == victim {
                "Путник ударил меня!".to_string()
            } else {
                format!("Путник ударил {victim_name}. Я это видел.")
            });
            n.trust = (n.trust - if i == victim { 30 } else { 15 }).max(-100);
            n.faith = (n.faith - 5).max(0);
            n.fear = (n.fear + 0.65).min(1.);
            n.threat_until = now + 25.;
            n.needs_think = true;
            n.thought = if n.role == "стражник" {
                "Нужно остановить нападающего."
            } else {
                "Мне опасно оставаться рядом. Надо уйти."
            }
            .into();
            n.speech = if n.role == "стражник" {
                "Стоять! Прекрати драку!"
            } else {
                "На помощь! Он бьёт людей!"
            }
            .into();
            n.speech_until = now + 7.;
            n.action = if n.role == "стражник" {
                Action::Guard
            } else {
                Action::Flee
            };
            n.dest = None;
            n.next_plan = 0.;
        }
    }
}
pub fn movement_event(
    npcs: &mut [Npc],
    town: &Town,
    player: Vec2,
    running: bool,
    jumped: bool,
    now: f64,
) {
    for n in npcs.iter_mut().filter(|n| n.health > 0) {
        let visible = sees(town, n.pos, n.facing, player);
        if visible && jumped {
            n.remember("Видел, как путник подпрыгнул рядом.".into());
            n.needs_think = true;
        } else if running && n.pos.distance(player) < 6. {
            n.remember(
                if visible {
                    "Вижу бегущего путника."
                } else {
                    "Слышу быстрые шаги неподалёку, но не вижу кто это."
                }
                .into(),
            );
            n.facing = (player - n.pos).normalize_or_zero();
            n.needs_think = true;
        }
        if visible && n.pos.distance(player) < 2. && now > n.personal_space_until {
            n.personal_space_until = now + 8.;
            n.remember("Путник подошёл ко мне вплотную.".into());
            n.needs_think = true;
            if n.trust < -20 && n.faith < 40 {
                n.fear = (n.fear + 0.15).min(1.);
            }
        }
    }
}
/// The player hands an item to a resident.
pub fn give_to_npc(
    npcs: &mut [Npc],
    id: usize,
    bag: &mut PlayerState,
    item: Item,
    now: f64,
) -> bool {
    if npcs[id].health <= 0 || !bag.items.take(item, 1) {
        return false;
    }
    let n = &mut npcs[id];
    n.inventory.add(item, 1);
    n.trust = (n.trust + item.gift_value()).min(100);
    n.fear = (n.fear - 0.2).max(0.);
    n.remember(format!("Путник дал мне: {}.", item.name()));
    n.heard = Some(format!("Путник только что дал мне: {}.", item.name()));
    n.thought = "Он мне что-то дал. Что ему от меня нужно?".into();
    n.speech = format!("Это мне? Спасибо за {}.", item.name());
    n.speech_until = now + 6.;
    n.needs_think = true;
    true
}
/// Residents `id` can see and talk to, nearest first.
pub fn visible_residents(npcs: &[Npc], id: usize, town: &Town) -> Vec<usize> {
    let mut found: Vec<usize> = (0..npcs.len())
        .filter(|&j| {
            j != id
                && npcs[j].health > 0
                && npcs[j].pos.distance(npcs[id].pos) < 8.
                && town.clear_line(npcs[id].pos, npcs[j].pos)
        })
        .collect();
    found.sort_by(|&a, &b| {
        npcs[a]
            .pos
            .distance_squared(npcs[id].pos)
            .total_cmp(&npcs[b].pos.distance_squared(npcs[id].pos))
    });
    found
}
pub fn resident_names(npcs: &[Npc], id: usize, town: &Town) -> Vec<String> {
    visible_residents(npcs, id, town)
        .into_iter()
        .map(|j| npcs[j].name.to_string())
        .collect()
}
/// Carries out a decision: resolves its target, raises alarms and lets listeners hear
/// the speech. Returns true when the NPC spoke to the player.
pub fn act(
    npcs: &mut [Npc],
    id: usize,
    d: Decision,
    dialogue: bool,
    town: &Town,
    player: Vec2,
    now: f64,
) -> bool {
    let target = if d.target == PLAYER {
        Some(Target::Player)
    } else {
        npcs.iter()
            .position(|n| n.name == d.target)
            .filter(|&j| j != id)
            .map(Target::Npc)
    };
    npcs[id].visible_player = sees(town, npcs[id].pos, npcs[id].facing, player);
    npcs[id].apply(d, target, dialogue, town, player, now);
    match npcs[id].action {
        Action::Alarm => raise_alarm(npcs, id, now),
        Action::Use => eat(&mut npcs[id], now),
        Action::Talk | Action::Ask => {
            if let Some(Target::Npc(j)) = npcs[id].target
                && npcs[j].pos.distance(npcs[id].pos) > 5.
                && !npcs[id].speech.is_empty()
            {
                // Too far to be heard: walk over and say it there.
                let n = &mut npcs[id];
                n.queued_speech = Some(std::mem::take(&mut n.speech));
            }
        }
        _ => {}
    }
    deliver(npcs, id, town, now);
    npcs[id].target == Some(Target::Player) && !npcs[id].speech.is_empty()
}
fn eat(n: &mut Npc, now: f64) {
    let Some(item) = n.item else { return };
    if let Some((relief, health)) = item.nourishment()
        && n.inventory.take(item, 1)
    {
        n.hunger = (n.hunger - relief).max(0.);
        n.health = (n.health + health).min(100);
        n.remember(format!("Я съел: {}.", item.name()));
    } else {
        n.remember(format!("Хотел съесть {}, но не вышло.", item.name()));
        n.needs_think = true;
    }
    n.action = Action::Idle;
    n.next_plan = now + 6.;
}
fn raise_alarm(npcs: &mut [Npc], caller: usize, now: f64) {
    let name = npcs[caller].name;
    let from = npcs[caller].pos;
    let seen = npcs[caller].last_seen_player;
    if npcs[caller].speech.is_empty() {
        npcs[caller].speech = "Стража! Сюда, скорее!".into();
        npcs[caller].speech_until = now + 8.;
    }
    for (i, n) in npcs.iter_mut().enumerate() {
        if i == caller || n.health <= 0 || n.role != "стражник" || n.pos.distance(from) > 30.
        {
            continue;
        }
        n.remember(format!("{name} звал(а) стражу на помощь против путника."));
        n.heard = Some(format!("{name} зовёт меня на помощь: путник опасен."));
        n.heard_from = Some(caller);
        n.thought = format!("{name} зовёт на помощь. Нужно разобраться с путником.");
        n.last_seen_player = seen.or(n.last_seen_player);
        n.action = Action::Guard;
        n.threat_until = n.threat_until.max(now + 12.);
        n.next_plan = 0.;
        n.needs_think = true;
    }
}
const HOLY_WORDS: [&str; 7] = ["чуд", "бог", "свят", "молни", "исцел", "знамен", "божеств"];
/// Deliver actual speech only to nearby listeners. Only the addressee is asked to answer.
pub fn deliver(npcs: &mut [Npc], speaker: usize, town: &Town, now: f64) {
    let name = npcs[speaker].name;
    let pos = npcs[speaker].pos;
    let speech = npcs[speaker].speech.clone();
    let target = npcs[speaker].target;
    let asking = npcs[speaker].action == Action::Ask;
    let preaching = npcs[speaker].faith >= 50 && {
        let lower = speech.to_lowercase();
        HOLY_WORDS.iter().any(|w| lower.contains(w))
    };
    if speech.is_empty() {
        return;
    }
    let addressee = match target {
        Some(Target::Player) => Some("путнику".to_string()),
        Some(Target::Npc(j)) => Some(format!("жителю {}", npcs[j].name)),
        None => None,
    };
    for (id, n) in npcs.iter_mut().enumerate() {
        if id == speaker
            || n.health <= 0
            || n.pos.distance(pos) >= 7.
            || !town.clear_line(pos, n.pos)
        {
            continue;
        }
        // Rumours of miracles spread belief, but hearsay alone only goes so far.
        if preaching && n.faith < 60 {
            n.faith = (n.faith + 4).min(60);
        }
        if target == Some(Target::Npc(id)) {
            let verb = if asking {
                "просит меня"
            } else {
                "обращается ко мне"
            };
            n.remember(format!("{name} {verb}: «{speech}»"));
            n.befriend(speaker, 1);
            if n.heard.is_none() && (asking || now >= n.social_after) {
                n.heard = Some(format!(
                    "{name} {verb}: «{speech}». Я могу ответить (target: {name})."
                ));
                n.heard_from = Some(speaker);
                n.social_after = now + 12.;
                n.needs_think = true;
            }
        } else if let Some(to) = &addressee {
            n.remember(format!("{name} обращается к {to}: «{speech}»"));
        } else {
            n.remember(format!("{name} говорит: «{speech}»"));
        }
    }
}
/// Moves an item from a resident to the player or another resident.
fn hand_over(
    npcs: &mut [Npc],
    from: usize,
    to: Target,
    item: Item,
    bag: &mut PlayerState,
) -> Option<String> {
    if !npcs[from].inventory.take(item, 1) {
        return None;
    }
    let giver = npcs[from].name;
    match to {
        Target::Player => {
            bag.items.add(item, 1);
            npcs[from].remember(format!("Я отдал путнику: {}.", item.name()));
            Some(format!("{giver} отдаёт вам: {}.", item.name()))
        }
        Target::Npc(j) => {
            let receiver = npcs[j].name;
            npcs[j].inventory.add(item, 1);
            npcs[j].befriend(from, item.gift_value());
            npcs[j].remember(format!("{giver} дал(а) мне: {}.", item.name()));
            npcs[j].heard = Some(format!("{giver} только что дал(а) мне: {}.", item.name()));
            npcs[j].heard_from = Some(from);
            npcs[j].needs_think = true;
            npcs[from].remember(format!("Я отдал {receiver}: {}.", item.name()));
            npcs[from].befriend(j, 2);
            None
        }
    }
}
fn finish(n: &mut Npc, now: f64, memory: Option<String>) {
    if let Some(m) = memory {
        n.remember(m);
    }
    n.action = Action::Idle;
    n.dest = None;
    n.busy_until = 0.;
    n.needs_think = true;
    n.next_plan = now + 12.;
}
/// Executes the current action of resident `id` for this frame: picks where to walk and
/// completes errands. Returns a message for the player when something concerns them.
pub fn steer(
    npcs: &mut [Npc],
    id: usize,
    town: &Town,
    player: Vec2,
    bag: &mut PlayerState,
    now: f64,
) -> Option<String> {
    if npcs[id].health <= 0 {
        return None;
    }
    let me = npcs[id].pos;
    let target_pos = match npcs[id].target {
        Some(Target::Player) => Some(if npcs[id].visible_player {
            player
        } else {
            npcs[id].last_seen_player.unwrap_or(player)
        }),
        Some(Target::Npc(j)) => Some(npcs[j].pos),
        None => None,
    };
    // A point next to the target rather than on top of it.
    let beside = |t: Vec2, gap: f32| t + (me - t).normalize_or_zero() * gap;
    // Close enough to `t`: within reach, or the walk ended as near as the streets allow.
    let settled = npcs[id].path.is_empty() && npcs[id].dest.is_some_and(|d| d.distance(me) <= 1.);
    let reached =
        |t: Vec2, reach: f32| t.distance(me) <= reach || (settled && t.distance(me) < reach + 2.);
    let action = npcs[id].action;
    let mut notice = None;
    match action {
        Action::Idle | Action::Alarm | Action::Use => {
            npcs[id].dest = None;
            if let Some(t) = target_pos
                && t.distance(me) < 8.
            {
                npcs[id].facing = (t - me).normalize_or_zero();
            }
        }
        Action::Bow => {
            npcs[id].dest = None;
            if let Some(t) = target_pos {
                npcs[id].facing = (t - me).normalize_or_zero();
            }
        }
        Action::Wander => {
            if npcs[id].dest.is_none_or(|d| d.distance(me) < 1.) {
                let i = (id * 7 + (now / 11.) as usize) % town.destinations.len();
                npcs[id].dest = Some(town.destinations[i]);
            }
        }
        Action::Guard => {
            npcs[id].dest = npcs[id].last_seen_player.map(|t| beside(t, 1.2));
        }
        Action::Flee => {}
        Action::Follow => match target_pos {
            Some(t) if t.distance(me) > 2.4 => npcs[id].dest = Some(beside(t, 2.)),
            Some(t) => {
                npcs[id].dest = None;
                npcs[id].facing = (t - me).normalize_or_zero();
            }
            None => npcs[id].dest = None,
        },
        Action::Approach | Action::Talk | Action::Ask => match target_pos {
            Some(t) if !reached(t, 2.) => npcs[id].dest = Some(beside(t, 1.5)),
            Some(t) => {
                npcs[id].dest = None;
                npcs[id].facing = (t - me).normalize_or_zero();
                if let Some(Target::Npc(j)) = npcs[id].target {
                    npcs[j].facing = (me - t).normalize_or_zero();
                }
                if let Some(words) = npcs[id].queued_speech.take() {
                    npcs[id].speech = words;
                    npcs[id].speech_until = now + 12.;
                    deliver(npcs, id, town, now);
                    if action == Action::Ask {
                        npcs[id].action = Action::Idle;
                        npcs[id].next_plan = now + 10.;
                    }
                }
            }
            None => npcs[id].dest = None,
        },
        Action::Go => {
            let place = npcs[id].place.clone().unwrap_or_default();
            match npcs[id].home_or(town, &place) {
                Some(p) if p.distance(me) > 1.6 => npcs[id].dest = Some(p),
                Some(_) => {
                    let label = if place == HOME {
                        "домой"
                    } else {
                        place.as_str()
                    };
                    finish(&mut npcs[id], now, Some(format!("Я пришёл: {label}.")));
                    npcs[id].next_plan = now + 25.;
                }
                None => finish(&mut npcs[id], now, None),
            }
        }
        Action::Pray => {
            let temple = town.place("храм").map(|p| p.pos).unwrap_or(Vec2::ZERO);
            if temple.distance(me) > 1.6 {
                npcs[id].dest = Some(temple);
            } else {
                npcs[id].dest = None;
                npcs[id].facing = vec2(0., -1.);
                if npcs[id].busy_until == 0. {
                    npcs[id].busy_until = now + 20.;
                } else if now >= npcs[id].busy_until {
                    finish(
                        &mut npcs[id],
                        now,
                        Some("Я помолился в храме, на душе спокойнее.".into()),
                    );
                    npcs[id].fear = (npcs[id].fear - 0.3).max(0.);
                }
            }
        }
        Action::Work => {
            let work = npcs[id].work;
            let spot = town.place(work).map(|p| p.pos).unwrap_or(npcs[id].home);
            if spot.distance(me) > 2.5 {
                npcs[id].dest = Some(spot);
            } else {
                npcs[id].dest = None;
                if npcs[id].busy_until == 0. {
                    npcs[id].busy_until = now + 25.;
                } else if now >= npcs[id].busy_until {
                    npcs[id].busy_until = now + 25.;
                    produce(&mut npcs[id], town);
                }
            }
        }
        Action::Give | Action::Fetch => {
            let Some(item) = npcs[id].item else {
                finish(&mut npcs[id], now, None);
                return None;
            };
            let Some(to) = npcs[id].target else {
                finish(&mut npcs[id], now, None);
                return None;
            };
            if npcs[id].inventory.has(item) {
                let t = target_pos.unwrap_or(me);
                // Search uses the remembered location; transfer requires the actual recipient.
                let recipient = match to {
                    Target::Player => player,
                    Target::Npc(j) => npcs[j].pos,
                };
                if me.distance(recipient) > 1.9 || !town.clear_line(me, recipient) {
                    npcs[id].dest = Some(beside(t, 1.3));
                } else {
                    notice = hand_over(npcs, id, to, item, bag);
                    finish(&mut npcs[id], now, None);
                    npcs[id].next_plan = now + 8.;
                }
            } else if action == Action::Give {
                finish(
                    &mut npcs[id],
                    now,
                    Some(format!(
                        "Хотел отдать {}, но у меня этого нет.",
                        item.name()
                    )),
                );
            } else if let Some(source) = town
                .places
                .iter()
                .find(|p| p.source == Some(item) && (p.public || p.name == npcs[id].work))
            {
                if source.pos.distance(me) > 2. {
                    npcs[id].dest = Some(source.pos);
                } else if npcs[id].busy_until == 0. {
                    npcs[id].dest = None;
                    npcs[id].busy_until = now + 4.;
                } else if now >= npcs[id].busy_until {
                    npcs[id].busy_until = 0.;
                    if !make(&mut npcs[id], item) {
                        let missing = recipe(item).map(|(i, _)| i.name()).unwrap_or("сырья");
                        finish(
                            &mut npcs[id],
                            now,
                            Some(format!(
                                "Не из чего сделать {}: нет {missing}.",
                                item.name()
                            )),
                        );
                    }
                }
            } else {
                let owners: Vec<&str> = npcs
                    .iter()
                    .enumerate()
                    .filter(|(j, n)| *j != id && n.inventory.has(item))
                    .map(|(_, n)| n.name)
                    .take(3)
                    .collect();
                let hint = if owners.is_empty() {
                    "ни у кого".to_string()
                } else {
                    format!("у {}", owners.join(", "))
                };
                finish(
                    &mut npcs[id],
                    now,
                    Some(format!(
                        "Мне негде взять {}. Он есть {hint} — можно попросить.",
                        item.name()
                    )),
                );
            }
        }
    }
    notice
}
/// What a workplace needs to make one batch: (input, output count).
fn recipe(item: Item) -> Option<(Item, u32)> {
    match item {
        Item::Bread => Some((Item::Flour, 2)),
        Item::Medicine => Some((Item::Herbs, 1)),
        _ => None,
    }
}
/// Makes or gathers `item`, consuming inputs. False when inputs are missing.
fn make(n: &mut Npc, item: Item) -> bool {
    match recipe(item) {
        Some((input, out)) => {
            if !n.inventory.take(input, 1) {
                return false;
            }
            n.inventory.add(item, out);
        }
        None => n.inventory.add(item, 1),
    }
    true
}
/// One shift of work: craft the workplace's goods, or earn a coin selling.
fn produce(n: &mut Npc, town: &Town) {
    match town.place(n.work).and_then(|p| p.source) {
        Some(item) if n.inventory.count(item) < 8 => {
            if !make(n, item) {
                let missing = recipe(item).map(|(i, _)| i.name()).unwrap_or("сырья");
                let note = format!("Кончилось: {missing}. Без этого работа стоит.");
                if n.memory.back() != Some(&note) {
                    n.remember(note);
                    n.needs_think = true;
                }
            }
        }
        Some(_) => {}
        None if n.inventory.count(Item::Coin) < 40 => n.inventory.add(Item::Coin, 1),
        None => {}
    }
}
fn food(n: &Npc) -> Option<Item> {
    [Item::Bread, Item::Fish, Item::Ale]
        .into_iter()
        .find(|i| n.inventory.has(*i))
}
/// Rule-based behaviour: used without the model and whenever a plan runs out.
pub fn routine(npcs: &[Npc], id: usize, town: &Town, now: f64) -> Decision {
    let n = &npcs[id];
    let neighbors = visible_residents(npcs, id, town);
    if n.threat_until > now && n.fear > 0.4 {
        return if n.role == "стражник" {
            Decision::new(
                "Нужно защитить жителей.",
                "Я слежу за тобой. Без драки!",
                Action::Guard,
            )
        } else {
            Decision::new(
                "Держаться подальше от опасности.",
                "Здесь небезопасно!",
                Action::Flee,
            )
            .to(PLAYER)
        };
    }
    if n.hunger > 0.6 {
        if let Some(item) = food(n) {
            return Decision::new("Я проголодался, надо поесть.", "", Action::Use).with_item(item);
        }
        if let Some(&j) = neighbors.iter().find(|&&j| food(&npcs[j]).is_some()) {
            return Decision::new(
                "Живот сводит от голода. Попрошу поесть.",
                "Не поделишься едой? Я совсем голоден.",
                Action::Ask,
            )
            .to(npcs[j].name);
        }
    }
    if let Some(from) = n.heard_from.filter(|&j| npcs[j].health > 0) {
        return Decision::new(
            "Выслушаю соседа и отвечу.",
            if n.trust < 0 {
                "Я тоже не доверяю этому путнику."
            } else {
                "Понимаю. Давай обсудим это спокойно."
            },
            Action::Talk,
        )
        .to(npcs[from].name);
    }
    if n.visible_player && n.faith >= 60 && n.action != Action::Bow {
        return Decision::new(
            "Это он — тот, кто творит чудеса!",
            "Благослови меня, господин.",
            Action::Bow,
        )
        .to(PLAYER);
    }
    let h = hour(now);
    if !(5. ..22.).contains(&h) {
        return Decision::new("Поздно. Пора домой спать.", "", Action::Go).at(HOME);
    }
    if h >= 18. {
        return if id.is_multiple_of(3) || n.work == "трактир" {
            Decision::new("Вечер. Загляну в трактир.", "", Action::Go).at("трактир")
        } else {
            Decision::new("Вечереет, пора домой.", "", Action::Go).at(HOME)
        };
    }
    if let Some(&j) = neighbors.first()
        && (now as usize / 7 + id).is_multiple_of(4)
    {
        let line = match n.role {
            "торговка" | "жена кузнеца" | "купец" => {
                "Как торговля? Говорят, цены опять растут."
            }
            "стражник" => "Всё спокойно? Капитан велел смотреть в оба.",
            "лекарь" | "повитуха" => "Если кому нужна помощь, приходите в лекарню.",
            "кузнец" => "Работы много, а стража опять дерёт поборы.",
            "священник" | "проповедник" => {
                "Не забывайте о молитве, добрые люди."
            }
            "трактирщик" | "служанка" => "Заходите вечером в трактир!",
            _ => "Слышали последние новости?",
        };
        return Decision::new("Узнаю у соседей, что нового.", line, Action::Talk).to(npcs[j].name);
    }
    Decision::new("Пора за работу.", "", Action::Work)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Town, Vec<Npc>) {
        let town = Town::new();
        let people = crate::npc::population(&town);
        (town, people)
    }
    #[test]
    fn inventory_save_roundtrip_and_legacy_migration() {
        let path =
            std::env::temp_dir().join(format!("veligrad-player-test-{}.json", std::process::id()));
        let original = PlayerState {
            items: Inventory::of(&[(Item::Medicine, 2), (Item::Coin, 17)]),
            collected: vec![0, 2],
            ..Default::default()
        };
        original.save(&path).unwrap();
        let restored = PlayerState::load(&path).unwrap();
        assert_eq!(restored.items, original.items);
        assert_eq!(restored.collected, vec![0, 2]);
        std::fs::write(
            &path,
            r#"{"health":90,"bread":3,"medicine":0,"coins":5,"collected":[]}"#,
        )
        .unwrap();
        let old = PlayerState::load(&path).unwrap();
        assert_eq!(old.items.count(Item::Bread), 3);
        assert_eq!(old.items.count(Item::Coin), 5);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn jump_lands_and_cannot_double_jump() {
        let mut p = PlayerState::default();
        assert!(p.jump());
        p.physics(0.05);
        assert!(!p.jump());
        for _ in 0..100 {
            p.physics(0.016);
        }
        assert_eq!(p.height, 0.);
        assert!(p.jump());
    }
    #[test]
    fn pickups_are_taken_once() {
        let town = Town::new();
        let mut p = PlayerState::default();
        let mut items = pickups();
        assert!(pickup(&mut items, vec2(-2., 16.), &mut p, &town).is_some());
        assert!(pickup(&mut items, vec2(-2., 16.), &mut p, &town).is_none());
        assert_eq!(p.items.count(Item::Bread), 3);
    }
    #[test]
    fn melee_respects_direction_range_and_walls() {
        let (t, mut ns) = setup();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[0].pos = vec2(0., 8.);
        assert_eq!(hit_target(&t, &ns, vec2(0., 10.), -Vec2::Y, 0.), Some(0));
        assert!(hit_target(&t, &ns, vec2(0., 10.), Vec2::Y, 0.).is_none());
        ns[0].pos = vec2(-12., 0.);
        assert!(hit_target(&t, &ns, vec2(-15.5, 0.), Vec2::X, 0.).is_none());
    }
    #[test]
    fn assault_changes_victim_and_visible_witnesses() {
        let (t, mut ns) = setup();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[0].pos = vec2(0., 8.);
        ns[1].pos = vec2(2., 9.);
        ns[1].facing = -Vec2::X;
        assault(&mut ns, 0, vec2(0., 9.), &t, 10.);
        assert_eq!(ns[0].health, 80);
        assert_eq!(ns[0].action, Action::Flee);
        assert_eq!(ns[1].action, Action::Guard);
        assert!(ns[2].memory.is_empty());
    }
    #[test]
    fn conversation_is_local() {
        let (t, mut ns) = setup();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[0].pos = vec2(0., 10.);
        ns[1].pos = vec2(2., 10.);
        ns[2].pos = vec2(-40., -30.);
        ns[0].speech = "Добрый день".into();
        ns[0].target = Some(Target::Npc(1));
        deliver(&mut ns, 0, &t, 20.);
        assert!(ns[1].heard.is_some());
        assert_eq!(ns[1].heard_from, Some(0));
        assert!(ns[2].heard.is_none());
    }
    fn run(
        ns: &mut [Npc],
        t: &Town,
        bag: &mut PlayerState,
        player: Vec2,
        from: f64,
        secs: f64,
    ) -> Vec<String> {
        let mut notes = vec![];
        let steps = (secs / 0.05) as usize;
        for s in 0..steps {
            let now = from + s as f64 * 0.05;
            for i in 0..ns.len() {
                ns[i].update(t, player, 0.05, now, false);
                if let Some(n) = steer(ns, i, t, player, bag, now) {
                    notes.push(n);
                }
            }
        }
        notes
    }
    #[test]
    fn fetch_gathers_and_delivers_to_the_player() {
        let (t, mut ns) = setup();
        let mut bag = PlayerState::default();
        let player = vec2(0., 44.);
        // Фома the fisherman: no fish left, but the pier is a public source.
        let fisher = ns.iter().position(|n| n.name == "Фома").unwrap();
        ns[fisher].inventory = Inventory::default();
        ns[fisher].pos = vec2(4., 46.);
        act(
            &mut ns,
            fisher,
            Decision::new("Принесу.", "Сейчас!", Action::Fetch)
                .to(PLAYER)
                .with_item(Item::Fish),
            false,
            &t,
            player,
            1.,
        );
        let before = bag.items.count(Item::Fish);
        let notes = run(&mut ns, &t, &mut bag, player, 1., 30.);
        assert_eq!(bag.items.count(Item::Fish), before + 1, "{notes:?}");
        assert!(notes.iter().any(|n| n.contains("Фома")));
    }
    #[test]
    fn baking_needs_flour_and_failure_is_remembered() {
        let (t, mut ns) = setup();
        let baker = ns.iter().position(|n| n.name == "Мила").unwrap();
        ns[baker].inventory = Inventory::default();
        produce(&mut ns[baker], &t);
        assert!(!ns[baker].inventory.has(Item::Bread));
        assert!(ns[baker].memory.back().unwrap().contains("мука"));
        ns[baker].inventory.add(Item::Flour, 1);
        produce(&mut ns[baker], &t);
        assert_eq!(ns[baker].inventory.count(Item::Bread), 2);
    }
    #[test]
    fn delivery_waits_for_the_actual_player() {
        let (town, mut ns) = setup();
        let mut bag = PlayerState::default();
        let id = 9;
        ns[id].pos = vec2(0., 10.);
        ns[id].action = Action::Give;
        ns[id].target = Some(Target::Player);
        ns[id].item = Some(Item::Bread);
        ns[id].inventory = Inventory::of(&[(Item::Bread, 1)]);
        ns[id].visible_player = false;
        ns[id].last_seen_player = Some(ns[id].pos);
        let before = bag.items.count(Item::Bread);
        steer(&mut ns, id, &town, vec2(40., 40.), &mut bag, 1.);
        assert_eq!(bag.items.count(Item::Bread), before);
        assert_eq!(ns[id].inventory.count(Item::Bread), 1);
        assert_eq!(ns[id].action, Action::Give);
        steer(&mut ns, id, &town, vec2(0., 10.5), &mut bag, 2.);
        assert_eq!(bag.items.count(Item::Bread), before + 1);
        assert_eq!(ns[id].inventory.count(Item::Bread), 0);
        assert_eq!(ns[id].action, Action::Idle);
    }
    #[test]
    fn residents_hand_goods_to_each_other() {
        let (t, mut ns) = setup();
        let mut bag = PlayerState::default();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[9].pos = vec2(0., 10.);
        ns[8].pos = vec2(6., 10.);
        let mila = ns[9].name;
        assert_eq!(mila, "Мила");
        act(
            &mut ns,
            9,
            Decision::new("Угощу Ждана.", "Держи хлеб!", Action::Give)
                .to("Ждан")
                .with_item(Item::Bread),
            false,
            &t,
            vec2(-40., -40.),
            1.,
        );
        run(&mut ns, &t, &mut bag, vec2(-40., -40.), 1., 8.);
        assert_eq!(ns[8].inventory.count(Item::Bread), 1);
        assert!(ns[8].relations[9] > 60);
        assert_eq!(ns[8].heard_from, Some(9));
    }
    #[test]
    fn far_requests_are_said_on_arrival() {
        let (t, mut ns) = setup();
        let mut bag = PlayerState::default();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
        }
        ns[0].pos = vec2(0., 10.);
        ns[2].pos = vec2(0., 20.);
        act(
            &mut ns,
            0,
            Decision::new("Попрошу Илью.", "Илья, дай лекарство.", Action::Ask).to("Илья"),
            false,
            &t,
            vec2(-40., -40.),
            1.,
        );
        assert!(ns[2].heard.is_none(), "too far to hear yet");
        run(&mut ns, &t, &mut bag, vec2(-40., -40.), 1., 12.);
        assert!(ns[2].heard.as_deref().unwrap().contains("просит меня"));
    }
    #[test]
    fn alarm_brings_the_guard() {
        let (t, mut ns) = setup();
        ns[0].pos = vec2(0., 10.);
        ns[0].last_seen_player = Some(vec2(0., 12.));
        ns[1].pos = vec2(6., 10.);
        act(
            &mut ns,
            0,
            Decision::new("Зову стражу.", "", Action::Alarm).to(PLAYER),
            false,
            &t,
            vec2(0., 12.),
            5.,
        );
        assert_eq!(ns[0].action, Action::Alarm);
        assert_eq!(ns[1].action, Action::Guard);
        assert_eq!(ns[1].last_seen_player, Some(vec2(0., 12.)));
    }
    #[test]
    fn expired_plan_asks_for_a_new_thought_before_routine() {
        let (t, mut ns) = setup();
        act(
            &mut ns,
            0,
            Decision::new("Моя цель.", "", Action::Idle),
            false,
            &t,
            vec2(30., 30.),
            1.,
        );
        assert!(!ns[0].needs_think);
        ns[0].update(&t, vec2(30., 30.), 0.016, 20., false);
        assert!(ns[0].needs_think);
        assert_eq!(ns[0].thought, "Моя цель.");
        ns[0].update(&t, vec2(30., 30.), 0.016, 33., false);
        assert!(ns[0].wants_routine);
    }
    #[test]
    fn routine_follows_the_clock_and_hunger() {
        let (t, mut ns) = setup();
        for n in ns.iter_mut() {
            n.pos = vec2(40., 40.);
            n.hunger = 0.;
        }
        let night = DAY_LENGTH * 15. / 24.; // 23:00
        assert_eq!(routine(&ns, 1, &t, night).action, Action::Go);
        assert_eq!(routine(&ns, 1, &t, 10.).action, Action::Work);
        ns[5].hunger = 0.9;
        let d = routine(&ns, 5, &t, 10.);
        assert_eq!((d.action, d.item.as_str()), (Action::Use, "хлеб"));
    }
    #[test]
    fn dialogue_reply_is_kept_as_npc_line() {
        let (t, mut ns) = setup();
        ns[0].add_talk(true, "Я бог.".into());
        let mut d = Decision::new("Странно.", "Ты? Бог?", Action::Idle).to(PLAYER);
        d.trust_delta = -2;
        act(&mut ns, 0, d, true, &t, vec2(0., 9.), 1.);
        let lines: Vec<_> = ns[0].talk.iter().map(|l| l.player).collect();
        assert_eq!(lines, vec![true, false]);
        assert_eq!(ns[0].trust, -2);
    }
}
