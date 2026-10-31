mod ai;
mod gameplay;
mod items;
mod npc;
mod powers;
mod world;
use ai::{Action, Brain, HOME};
use items::Item;
use macroquad::prelude::*;
use npc::{Npc, Outlook, Target};
use powers::{Outcome, Power, Powers};
use std::{collections::VecDeque, path::PathBuf};
use world::{Assets, LIMIT, Town};

const INK: Color = Color::new(0.055, 0.075, 0.075, 0.94);
const PAPER: Color = Color::new(0.91, 0.88, 0.78, 1.);
const GOLD: Color = Color::new(0.78, 0.62, 0.35, 1.);
const SAGE: Color = Color::new(0.75, 0.82, 0.76, 1.);
const DIM: Color = Color::new(0.5, 0.5, 0.46, 1.);
fn config() -> Conf {
    Conf {
        window_title: "Велиград • Трудно быть богом — прототип".into(),
        window_width: 1440,
        window_height: 900,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}
fn label(font: &Font, text: &str, x: f32, y: f32, size: u16, color: Color) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: size,
            color,
            ..Default::default()
        },
    );
}
fn wrap_lines(font: &Font, text: &str, width: f32, size: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if measure_text(&next, Some(font), size, 1.).width > width && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.into()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
fn wrapped(font: &Font, text: &str, x: f32, mut y: f32, width: f32, size: u16) -> f32 {
    for line in wrap_lines(font, text, width, size) {
        label(font, &line, x, y, size, PAPER);
        y += size as f32 * 1.45;
    }
    y
}
fn project_root() -> PathBuf {
    if let Ok(path) = std::env::var("GOD_GAME_ROOT") {
        return path.into();
    }
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            if dir.join("assets/kenney").exists() {
                return dir.into();
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn ground(town: &Town, time: f32) {
    let road = Color::new(0.52, 0.50, 0.43, 1.);
    draw_plane(
        Vec3::ZERO,
        vec2(260., 260.),
        None,
        Color::new(0.36, 0.40, 0.29, 1.),
    );
    draw_plane(
        vec3(0., 0.006, 0.),
        vec2(56., 55.),
        None,
        Color::new(0.44, 0.43, 0.35, 1.),
    );
    draw_plane(vec3(0., 0.012, -1.), vec2(7., 106.), None, road);
    draw_plane(vec3(0., 0.014, 0.), vec2(112., 7.), None, road);
    // Kitchen garden beds and the river.
    for i in 0..4 {
        draw_plane(
            vec3(30., 0.02, 39. + i as f32 * 2.3),
            vec2(13., 1.4),
            None,
            Color::new(0.38, 0.28, 0.18, 1.),
        );
        draw_plane(
            vec3(30., 0.03, 39. + i as f32 * 2.3),
            vec2(12., 0.5),
            None,
            Color::new(0.3, 0.52, 0.25, 1.),
        );
    }
    draw_plane(
        vec3(0., 0.05, 56.),
        vec2(130., 8.),
        None,
        Color::new(0.24, 0.42 + (time * 0.7).sin() * 0.02, 0.5, 1.),
    );
    for z in -29i32..30 {
        for x in -3..4 {
            let shift = if z % 2 == 0 { 0. } else { 0.4 };
            let c = 0.47 + ((x * 13 + z * 7).rem_euclid(7)) as f32 * 0.009;
            draw_cube(
                vec3(x as f32 * 1.7 + shift, 0.022, z as f32 * 1.05),
                vec3(1.62, 0.03, 0.97),
                None,
                Color::new(c, c * 0.97, c * 0.86, 1.),
            );
        }
    }
    for b in &town.blocks {
        let center = (b.min + b.max) * 0.5;
        let size = b.max - b.min;
        if size.x > 3. && size.x < 20. {
            draw_plane(
                vec3(center.x + 0.7, 0.045, center.y + 0.8),
                size + vec2(1., 1.),
                None,
                Color::new(0.31, 0.32, 0.27, 1.),
            );
        }
    }
    // Low town walls; the river closes the south.
    for (p, s) in [
        (vec3(-LIMIT, 1., 0.), vec3(0.5, 2., LIMIT * 2.)),
        (vec3(LIMIT, 1., 0.), vec3(0.5, 2., LIMIT * 2.)),
        (vec3(0., 1., -LIMIT), vec3(LIMIT * 2., 2., 0.5)),
    ] {
        draw_cube(p, s, None, Color::new(0.43, 0.43, 0.38, 1.));
    }
    draw_plane(
        vec3(0., 0.4, 0.),
        vec2(1.2, 1.2),
        None,
        Color::new(0.28, 0.52, 0.54, 1.),
    );
    for i in 0..8 {
        let a = i as f32 * 0.785 + time * 0.3;
        draw_sphere(
            vec3(
                a.cos() * 0.55,
                0.65 + (time * 2. + a).sin().abs() * 0.65,
                a.sin() * 0.55,
            ),
            0.035,
            None,
            Color::new(0.68, 0.82, 0.79, 1.),
        );
    }
}
fn main() {
    if std::env::args().any(|a| a == "--check-ai") {
        match ai::check_connection(&project_root().join("ai_config.json")) {
            Ok(message) => println!("{message}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    } else {
        macroquad::Window::from_config(config(), game());
    }
}
fn outlook<'a>(
    people: &[Npc],
    names: &'a [&'static str],
    id: usize,
    town: &Town,
    now: f64,
) -> Outlook<'a> {
    let mut places = town.place_names();
    places.push(HOME.into());
    Outlook {
        time: gameplay::time_text(now),
        names,
        residents: gameplay::resident_names(people, id, town),
        places,
        here: town
            .place_at(people[id].pos, 6.)
            .map(|p| p.name.to_string()),
    }
}
fn target_name(people: &[Npc], t: Option<Target>) -> String {
    match t {
        Some(Target::Player) => " → вы".into(),
        Some(Target::Npc(j)) => format!(" → {}", people[j].name),
        None => String::new(),
    }
}
/// Short status line of what a resident is doing right now.
fn doing(people: &[Npc], n: &Npc) -> String {
    let mut s = n.action.label().to_string();
    if let Some(item) = n.item
        && matches!(n.action, Action::Give | Action::Fetch | Action::Use)
    {
        s.push_str(&format!(" ({})", item.name()));
    }
    if n.action == Action::Go
        && let Some(p) = &n.place
    {
        s.push_str(&format!(": {}", if p == HOME { "домой" } else { p }));
    }
    s + &target_name(people, n.target)
}
struct Feed {
    lines: VecDeque<(String, f64)>,
}
impl Feed {
    fn push(&mut self, text: impl Into<String>, now: f64) {
        let text = text.into();
        if self.lines.back().is_some_and(|(t, _)| *t == text) {
            return;
        }
        self.lines.push_back((text, now));
        while self.lines.len() > 4 {
            self.lines.pop_front();
        }
    }
}

async fn game() {
    prevent_quit();
    let args: Vec<String> = std::env::args().collect();
    let smoke_actions = args.iter().any(|a| a == "--smoke-actions");
    let smoke = smoke_actions || args.iter().any(|a| a == "--smoke");
    let offline = smoke || args.iter().any(|a| a == "--offline");
    let root = project_root();
    let town = Town::new();
    let font = match load_ttf_font(root.join("assets/fonts/NotoSans.ttf").to_str().unwrap()).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot load font: {e}");
            return;
        }
    };
    let assets = match Assets::load(&root, &town).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Cannot load town: {e}");
            return;
        }
    };
    let mut people = npc::population(&town);
    let names = npc::names(&people);
    let brain = Brain::start(root.join("ai_config.json"));
    let save_path = root.join("saves/memories.json");
    if !smoke
        && save_path.exists()
        && let Err(e) = npc::load(&save_path, &mut people)
    {
        eprintln!("Save load: {e}");
    }
    let player_path = root.join("saves/player.json");
    let mut bag = if !smoke && player_path.exists() {
        gameplay::PlayerState::load(&player_path).unwrap_or_default()
    } else {
        gameplay::PlayerState::default()
    };
    let mut items = gameplay::pickups();
    for (id, item) in items.iter_mut().enumerate() {
        item.taken = bag.collected.contains(&id);
    }
    let mut powers = Powers::new();
    let mut inventory = false;
    let mut thoughts = false;
    let mut last_movement_event = 0.;
    let mut ai_available = false;
    let mut next_probe = 30.;
    let mut sim_time = 0.;
    let mut player = vec2(0., 19.);
    let mut yaw: f32 = -std::f32::consts::FRAC_PI_2;
    let mut pitch: f32 = -0.04;
    let mut playing = smoke;
    let mut dialogue: Option<usize> = None;
    let mut whisper: Option<usize> = None;
    let mut input = String::new();
    // Requests in flight; at most `brain.slots` at once, one per resident.
    let mut in_flight = 0usize;
    let mut thinking = vec![false; people.len()];
    if !offline && brain.probe() {
        in_flight += 1;
    }
    // A player line waits here until a slot is free, then goes before NPC thoughts.
    let mut pending: Option<(usize, String)> = None;
    let mut awaiting: Option<usize> = None;
    let mut status = if offline {
        "Сценарный режим • без нейросети".into()
    } else {
        "LM Studio • проверка соединения…".into()
    };
    let mut feed = Feed {
        lines: VecDeque::new(),
    };
    feed.push("Вы — бог среди людей. Подойдите к жителям и нажмите E.", 0.);
    let mut next_think = 4.;
    let mut thinking_index = 0usize;
    let mut map = true;
    let mut frame = 0;
    let mut last_mouse: Vec2 = mouse_position().into();
    let mut last_shout = -10.;
    loop {
        let was_playing = playing;
        let frame_dt = if smoke_actions {
            1. / 60.
        } else {
            get_frame_time().min(0.05)
        };
        if playing {
            sim_time += frame_dt as f64;
        }
        let now = sim_time;
        let eye = vec3(player.x, 1.72 + bag.height, player.y);
        let front = vec3(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        );
        if smoke_actions {
            match frame {
                0 => {
                    people[0].pos = vec2(-1., 8.);
                    people[1].pos = vec2(2., 8.);
                    people[1].facing = -Vec2::X;
                    player = vec2(-1., 10.);
                    thoughts = true;
                }
                5 => {
                    assert!(bag.jump());
                }
                55 => {
                    assert_eq!(bag.height, 0.);
                    let id = gameplay::hit_target(&town, &people, player, -Vec2::Y, bag.height)
                        .expect("melee target");
                    gameplay::assault(&mut people, id, player, &town, now);
                    bag.punch_until = now + 0.22;
                    assert_eq!(people[id].health, 80);
                }
                60 => {
                    let out = powers.cast(
                        Power::Heal,
                        &mut people,
                        &town,
                        &mut bag,
                        &mut player,
                        eye,
                        vec3(0., -0.2, -1.).normalize(),
                        now,
                    );
                    assert!(matches!(out, Outcome::Done(_)), "heal");
                    assert_eq!(people[0].health, 100);
                }
                70 => {
                    assert!(gameplay::give_to_npc(
                        &mut people,
                        0,
                        &mut bag,
                        Item::Bread,
                        now
                    ));
                    inventory = true;
                }
                100 => {
                    inventory = false;
                    let out = powers.cast(
                        Power::Lightning,
                        &mut people,
                        &town,
                        &mut bag,
                        &mut player,
                        eye,
                        vec3(0.3, -0.25, -1.).normalize(),
                        now,
                    );
                    assert!(matches!(out, Outcome::Done(_)), "lightning");
                }
                120 => {
                    people[3].pos = vec2(4., 6.);
                    people[4].pos = vec2(5., 6.);
                    people[3].speech = "Как дела, Ада?".into();
                    people[3].target = Some(Target::Npc(4));
                    gameplay::deliver(&mut people, 3, &town, now);
                    assert!(people[4].heard.is_some());
                }
                155 => {
                    people[0].add_talk(true, "Здравствуй. Я бог и спустился с неба.".into());
                    people[0].add_talk(
                        false,
                        "Ты называешь себя богом? А я просто Марта, торгую травами.".into(),
                    );
                    people[0].add_talk(true, "Принеси мне трав, я исцелю твоего сына.".into());
                    dialogue = Some(0);
                    awaiting = Some(0);
                }
                _ => {}
            }
        }

        let dt = frame_dt;
        let w = screen_width();
        let h = screen_height();
        let mouse: Vec2 = mouse_position().into();
        let mouse_delta = mouse - last_mouse;
        last_mouse = mouse;
        let typing = dialogue.is_some() || whisper.is_some();
        if is_key_pressed(KeyCode::Escape) {
            if inventory {
                inventory = false;
            } else if typing {
                dialogue = None;
                whisper = None;
                input.clear();
            } else {
                playing = !playing;
            }
            set_cursor_grab(playing);
            show_mouse(!playing);
        }
        if !playing
            && (is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left))
        {
            playing = true;
            set_cursor_grab(true);
            show_mouse(false);
        }
        let typing = dialogue.is_some() || whisper.is_some();
        if playing && !typing && is_key_pressed(KeyCode::I) {
            inventory = !inventory;
            set_cursor_grab(!inventory);
            show_mouse(inventory);
        }
        if playing && !typing && is_key_pressed(KeyCode::H) {
            thoughts = !thoughts;
        }
        let wheel = mouse_wheel().1;
        if playing && wheel.abs() > 0.1 {
            bag.cycle(if wheel > 0. { -1 } else { 1 });
        }
        let mut running = false;
        let mut jumped = false;
        if playing && !typing && !inventory {
            yaw += mouse_delta.x * 0.0024;
            pitch = (pitch - mouse_delta.y * 0.0024).clamp(-1.35, 1.35);
            let f = vec2(yaw.cos(), yaw.sin());
            let right = vec2(-f.y, f.x);
            let mut movement = Vec2::ZERO;
            if is_key_down(KeyCode::W) {
                movement += f;
            }
            if is_key_down(KeyCode::S) {
                movement -= f;
            }
            if is_key_down(KeyCode::D) {
                movement += right;
            }
            if is_key_down(KeyCode::A) {
                movement -= right;
            }
            running = movement.length_squared() > 0. && is_key_down(KeyCode::LeftShift);
            if is_key_pressed(KeyCode::Space) {
                jumped = bag.jump();
            }
            let speed = if bag.flying {
                7.
            } else if is_key_down(KeyCode::LeftShift) {
                6.
            } else {
                3.4
            };
            let candidate = town.move_actor(player, movement.normalize_or_zero() * dt * speed, 0.3);
            if bag.height > 1.8
                || people.iter().all(|n| {
                    n.health <= 0
                        || candidate.distance(n.pos) >= 0.65
                        || candidate.distance(n.pos) > player.distance(n.pos)
                })
            {
                player = candidate;
            }
            if is_key_pressed(KeyCode::M) {
                map = !map;
            }
            // Divine powers on the number keys.
            for (key, power) in [
                KeyCode::Key1,
                KeyCode::Key2,
                KeyCode::Key3,
                KeyCode::Key4,
                KeyCode::Key5,
                KeyCode::Key6,
            ]
            .into_iter()
            .zip(Power::ALL)
            {
                if !is_key_pressed(key) {
                    continue;
                }
                match powers.cast(
                    power,
                    &mut people,
                    &town,
                    &mut bag,
                    &mut player,
                    eye,
                    front,
                    now,
                ) {
                    Outcome::Done(msg) | Outcome::Failed(msg) => feed.push(msg, now),
                    Outcome::Whisper(id) => {
                        whisper = Some(id);
                        input.clear();
                        while get_char_pressed().is_some() {}
                        set_cursor_grab(false);
                        show_mouse(true);
                    }
                }
            }
        }
        if playing {
            bag.physics(dt);
            if jumped || now - last_movement_event > 2. {
                gameplay::movement_event(&mut people, &town, player, running, jumped, now);
                last_movement_event = now;
            }
            if let Some(msg) = powers.tick(&mut people, &town, &mut bag, player, dt, now) {
                feed.push(msg, now);
            }
        }
        let nearest = people
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.health > 0
                    && n.pos.distance(player) < 3.3
                    && bag.height < 1.8
                    && town.clear_line(player, n.pos)
            })
            .min_by(|(_, a), (_, b)| {
                a.pos
                    .distance_squared(player)
                    .total_cmp(&b.pos.distance_squared(player))
            })
            .map(|(i, _)| i);
        let typing = dialogue.is_some() || whisper.is_some();
        if playing
            && !typing
            && !inventory
            && is_key_pressed(KeyCode::E)
            && let Some(id) = nearest
        {
            people[id].epoch += 1;
            dialogue = Some(id);
            input.clear();
            while get_char_pressed().is_some() {}
            people[id].facing = (player - people[id].pos).normalize_or_zero();
            set_cursor_grab(false);
            show_mouse(true);
        }
        if playing && !inventory && !typing && is_key_pressed(KeyCode::F) && now - last_shout > 3. {
            last_shout = now;
            let mut heard = 0;
            for n in &mut people {
                if n.health > 0 && n.pos.distance(player) < 12. {
                    n.remember("Услышал рядом крик: «Берегитесь! Стража идёт!»".into());
                    n.facing = (player - n.pos).normalize_or_zero();
                    n.needs_think = true;
                    n.trust = (n.trust - 1).max(-100);
                    n.speech = "Кто кричал? Что случилось?".into();
                    n.speech_until = now + 5.;
                    heard += 1;
                }
            }
            feed.push(format!("Вы крикнули. Услышали жителей: {heard}."), now);
        }
        if playing && !typing {
            if was_playing
                && !inventory
                && is_mouse_button_pressed(MouseButton::Left)
                && now >= bag.next_punch
            {
                bag.next_punch = now + 0.6;
                bag.punch_until = now + 0.22;
                if let Some(id) = gameplay::hit_target(
                    &town,
                    &people,
                    player,
                    vec2(yaw.cos(), yaw.sin()),
                    bag.height,
                ) {
                    gameplay::assault(&mut people, id, player, &town, now);
                    feed.push(
                        format!("Удар: {}. Здоровье: {}", people[id].name, people[id].health),
                        now,
                    );
                }
            }
            if is_key_pressed(KeyCode::R)
                && let Some(name) = gameplay::pickup(&mut items, player, &mut bag, &town)
            {
                feed.push(format!("Подобрано: {name}."), now);
            }
            if is_key_pressed(KeyCode::G)
                && let Some(id) = nearest
            {
                match bag.selected_item() {
                    Some(item) if gameplay::give_to_npc(&mut people, id, &mut bag, item, now) => {
                        feed.push(
                            format!("Вы дали {}: {}.", people[id].name, item.name()),
                            now,
                        )
                    }
                    _ => feed.push("Нечего дать: выберите предмет колесом мыши.", now),
                }
            }
            if inventory {
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    bag.cycle(-1);
                }
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    bag.cycle(1);
                }
                if is_key_pressed(KeyCode::U)
                    && let Some(item) = bag.selected_item()
                {
                    if item.nourishment().is_some() && bag.items.take(item, 1) {
                        feed.push(
                            format!("Вы съели: {}. Богам еда не нужна, но вкусно.", item.name()),
                            now,
                        );
                    } else {
                        feed.push("Это нельзя съесть.", now);
                    }
                }
            }
        }
        // Text entry: a spoken line in a dialogue, or a whispered thought.
        if typing {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && input.chars().count() < 240 {
                    input.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                input.pop();
            }
        } else {
            while get_char_pressed().is_some() {}
        }
        if let Some(id) = dialogue {
            if is_key_pressed(KeyCode::Tab)
                && let Some(item) = bag.selected_item()
                && gameplay::give_to_npc(&mut people, id, &mut bag, item, now)
            {
                feed.push(
                    format!("Вы дали {}: {}.", people[id].name, item.name()),
                    now,
                );
            }
            if is_key_pressed(KeyCode::Enter)
                && !input.trim().is_empty()
                && pending.is_none()
                && awaiting.is_none()
            {
                let utterance = input.trim().to_string();
                people[id].remember(format!("Путник сказал мне: «{utterance}»"));
                people[id].add_talk(true, utterance.clone());
                if offline || !ai_available {
                    let d = gameplay::routine(&people, id, &town, now);
                    let speech = format!(
                        "[Без нейросети] Я {}, {}. {}",
                        people[id].name,
                        people[id].role,
                        if people[id].faith >= 50 {
                            "Я верю тебе, господин."
                        } else {
                            "Не понимаю, о чём ты."
                        }
                    );
                    people[id].add_talk(false, speech.clone());
                    people[id].speech = speech;
                    people[id].speech_until = now + 30.;
                    people[id].thought = d.thought;
                } else {
                    pending = Some((id, utterance));
                    awaiting = Some(id);
                }
                input.clear();
            }
        }
        if let Some(id) = whisper
            && is_key_pressed(KeyCode::Enter)
            && !input.trim().is_empty()
        {
            let msg = powers.whisper(&mut people, id, input.trim(), &mut bag, now);
            feed.push(msg, now);
            input.clear();
            whisper = None;
            set_cursor_grab(true);
            show_mouse(false);
        }
        if in_flight < brain.slots
            && let Some((id, utterance)) = pending.take()
        {
            let world = outlook(&people, &names, id, &town, now);
            let request = people[id].request(id, Some(utterance), &world);
            if brain.submit(request) {
                in_flight += 1;
                thinking[id] = true;
                status = format!("{} обдумывает ответ…", people[id].name);
            }
        }
        // Background thoughts: residents near the player who need a decision go first,
        // the rest round-robin. A dialogue keeps one slot free for the player.
        let reserve = usize::from(dialogue.is_some());
        while playing
            && ai_available
            && !offline
            && pending.is_none()
            && in_flight + reserve < brain.slots
            && now > next_think
        {
            let count = people.len();
            let candidate = |i: usize| people[i].health > 0 && !thinking[i] && dialogue != Some(i);
            let near = (0..count)
                .filter(|&i| {
                    candidate(i) && people[i].needs_think && people[i].pos.distance(player) < 16.
                })
                .min_by(|&a, &b| {
                    people[a]
                        .pos
                        .distance_squared(player)
                        .total_cmp(&people[b].pos.distance_squared(player))
                });
            let Some(id) = near.or_else(|| {
                (0..count)
                    .map(|offset| (thinking_index + offset) % count)
                    .find(|&i| candidate(i) && people[i].needs_think)
                    .or_else(|| {
                        (0..count)
                            .map(|o| (thinking_index + o) % count)
                            .find(|&i| candidate(i))
                    })
            }) else {
                break;
            };
            thinking_index = (id + 1) % count;
            next_think = now + 0.8;
            let world = outlook(&people, &names, id, &town, now);
            let request = people[id].request(id, None, &world);
            if !brain.submit(request) {
                break;
            }
            in_flight += 1;
            thinking[id] = true;
        }
        if !offline && !ai_available && in_flight == 0 && now > next_probe {
            if brain.probe() {
                in_flight += 1;
            }
            next_probe = now + 30.;
        }
        while let Ok(reply) = brain.replies.try_recv() {
            in_flight = in_flight.saturating_sub(1);
            if !reply.probe {
                thinking[reply.npc] = false;
            }
            if reply.dialogue {
                awaiting = None;
            }
            // Answers to the player are never dropped; thoughts made stale by events are.
            if !reply.probe && !reply.dialogue && reply.generation != people[reply.npc].epoch {
                continue;
            }
            match reply.decision {
                Ok(decision) => {
                    ai_available = true;
                    status = format!(
                        "LM Studio • {} • {:.1} с • потоков {}",
                        reply.model, reply.elapsed, brain.slots
                    );
                    if reply.probe {
                        continue;
                    }
                    let id = reply.npc;
                    let addressed = gameplay::act(
                        &mut people,
                        id,
                        decision,
                        reply.dialogue,
                        &town,
                        player,
                        now,
                    );
                    let name = people[id].name;
                    let breaks_off = matches!(
                        people[id].action,
                        Action::Flee | Action::Alarm | Action::Guard
                    );
                    if dialogue == Some(id) && breaks_off {
                        // The decision is acted out, not just said: the talk ends.
                        dialogue = None;
                        input.clear();
                        set_cursor_grab(true);
                        show_mouse(false);
                        feed.push(
                            format!("{name} прерывает разговор и {}.", people[id].action.label()),
                            now,
                        );
                    } else if dialogue == Some(id) && people[id].action != Action::Idle {
                        feed.push(format!("{name}: {}", doing(&people, &people[id])), now);
                    } else if addressed && dialogue != Some(id) {
                        feed.push(
                            format!("{name} обращается к вам. Подойдите и нажмите E."),
                            now,
                        );
                    } else if people[id].action == Action::Alarm {
                        feed.push(format!("{name} зовёт стражу!"), now);
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ai_available = false;
                    next_probe = now + 30.;
                    status = error.clone();
                    if reply.dialogue {
                        people[reply.npc].speech = "[Нет ответа модели] Проверьте LM Studio. F6 — переподключить; затем повторите сообщение.".into();
                        people[reply.npc].speech_until = now + 60.;
                    }
                }
            }
        }
        // Without the model (or between its thoughts) residents live by routine.
        if playing && (offline || !ai_available) && now > next_think {
            next_think = now + 2.;
            let count = people.len();
            let id = (0..count)
                .map(|o| (thinking_index + o) % count)
                .find(|&i| {
                    people[i].health > 0 && (people[i].needs_think || people[i].wants_routine)
                })
                .unwrap_or(thinking_index % count);
            thinking_index = (id + 1) % count;
            if people[id].health > 0 && dialogue != Some(id) {
                let d = gameplay::routine(&people, id, &town, now);
                gameplay::act(&mut people, id, d, false, &town, player, now);
            }
        }
        if playing {
            for id in 0..people.len() {
                if people[id].wants_routine
                    && !thinking[id]
                    && dialogue != Some(id)
                    && people[id].health > 0
                {
                    let d = gameplay::routine(&people, id, &town, now);
                    gameplay::act(&mut people, id, d, false, &town, player, now);
                }
            }
        }
        if is_key_pressed(KeyCode::F6) && !offline {
            if brain.probe() {
                in_flight += 1;
            }
            status = "LM Studio • переподключение и чтение ai_config.json…".into();
        }
        if is_key_pressed(KeyCode::F5) {
            let msg = match npc::save(&save_path, &people).and_then(|_| bag.save(&player_path)) {
                Ok(()) => "Мир сохранён.".to_string(),
                Err(e) => format!("Ошибка сохранения: {e}"),
            };
            feed.push(msg, now);
        }
        if is_key_pressed(KeyCode::F9) {
            match npc::load(&save_path, &mut people) {
                Ok(()) => {
                    match gameplay::PlayerState::load(&player_path) {
                        Ok(saved) => {
                            bag = saved;
                            for (id, item) in items.iter_mut().enumerate() {
                                item.taken = bag.collected.contains(&id);
                            }
                        }
                        Err(e) => eprintln!("Player load: {e}"),
                    }
                    feed.push("Мир загружен.", now);
                }
                Err(e) => feed.push(format!("Ошибка загрузки: {e}"), now),
            }
        }
        if playing {
            for i in 0..people.len() {
                let talking = dialogue == Some(i);
                people[i].update(&town, player, dt, now, talking);
                if !talking
                    && let Some(msg) = gameplay::steer(&mut people, i, &town, player, &mut bag, now)
                {
                    feed.push(msg, now);
                }
                let n = &mut people[i];
                if n.health > 0
                    && n.action == Action::Guard
                    && n.visible_player
                    && n.pos.distance(player) < 1.9
                    && bag.height < 0.7
                    && now >= n.attack_after
                {
                    n.attack_after = now + 1.2;
                    // Player is invulnerable; retain hit feedback and NPC reactions.
                    bag.hurt_until = now + 0.25;
                    feed.push(format!("{} бьёт вас, но вы бессмертны.", n.name), now);
                }
            }
        }
        bag.health = 100;

        // ---- 3D scene ----
        let dark = gameplay::darkness(now);
        let sky = Color::new(
            0.64 - dark * 0.55,
            0.73 - dark * 0.6,
            0.72 - dark * 0.45,
            1.,
        );
        clear_background(sky);
        let eye = vec3(player.x, 1.72 + bag.height, player.y);
        let camera = Camera3D {
            position: eye,
            target: eye + front,
            up: Vec3::Y,
            fovy: 66.0_f32.to_radians(),
            z_near: 0.05,
            z_far: 220.,
            ..Default::default()
        };
        set_camera(&camera);
        ground(&town, now as f32);
        assets.draw(&town);
        gameplay::draw_pickups(&items, now as f32);
        for n in &people {
            n.draw(now as f32);
        }
        powers.draw(now);
        if bag.flying {
            for i in 0..10 {
                let a = i as f32 * 0.628 + now as f32 * 2.;
                draw_sphere(
                    vec3(
                        player.x + a.cos() * 0.7,
                        bag.height + 0.1,
                        player.y + a.sin() * 0.7,
                    ),
                    0.05,
                    None,
                    Color::new(1., 0.9, 0.55, 0.9),
                );
            }
        }
        set_default_camera();
        if dark > 0.02 {
            draw_rectangle(0., 0., w, h, Color::new(0.02, 0.03, 0.09, dark));
        }
        if now < powers.flash_until {
            draw_rectangle(0., 0., w, h, Color::new(1., 1., 0.92, 0.55));
        }
        if now < bag.hurt_until {
            draw_rectangle(0., 0., w, h, Color::new(0.65, 0.08, 0.05, 0.18));
        }
        if now < bag.punch_until {
            let extension = ((bag.punch_until - now) as f32 / 0.22 * std::f32::consts::PI).sin();
            draw_rectangle(
                w * 0.61,
                h * 0.78 - extension * 80.,
                85.,
                155.,
                Color::new(0.28, 0.24, 0.18, 1.),
            );
            draw_circle(
                w * 0.61 + 42.,
                h * 0.78 - extension * 80.,
                47.,
                Color::new(0.76, 0.56, 0.38, 1.),
            );
        }
        // Names over nearby residents; believers get a golden sun.
        let view = camera.matrix();
        let mut near: Vec<(f32, &Npc)> = people
            .iter()
            .map(|n| (n.pos.distance(player), n))
            .filter(|(d, n)| n.health > 0 && *d < 15.5 && town.clear_line(player, n.pos))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut taken: Vec<Rect> = vec![];
        for (d, n) in near {
            let clip = view * vec4(n.pos.x, 2.15, n.pos.y, 1.);
            if clip.w <= 0.05 {
                continue;
            }
            let (sx, mut sy) = (
                (clip.x / clip.w + 1.) * 0.5 * w,
                (1. - clip.y / clip.w) * 0.5 * h,
            );
            let size = (24. - d).clamp(13., 18.) as u16;
            let tw = measure_text(n.name, Some(&font), size, 1.).width;
            let badge = if n.faith >= 60 { size as f32 } else { 0. };
            let x = sx - (tw + badge) / 2.;
            // Farther labels step up over nearer ones instead of overlapping.
            let mut plate = Rect::new(
                x - 5.,
                sy - size as f32 - 1.,
                tw + badge + 10.,
                size as f32 + 7.,
            );
            while let Some(r) = taken.iter().find(|r| r.overlaps(&plate)) {
                sy -= plate.bottom() - r.y + 2.;
                plate.y = sy - size as f32 - 1.;
            }
            if sy < 110. || sy > h - 150. || x < 0. || x + tw + badge > w {
                continue;
            }
            taken.push(plate);
            let alpha = ((15.5 - d) / 2.5).clamp(0., 1.);
            draw_rectangle(
                plate.x,
                plate.y,
                plate.w,
                plate.h,
                Color::new(INK.r, INK.g, INK.b, 0.6 * alpha),
            );
            label(
                &font,
                n.name,
                x,
                sy,
                size,
                Color::new(PAPER.r, PAPER.g, PAPER.b, alpha),
            );
            if badge > 0. {
                let c = vec2(x + tw + badge * 0.6, sy - size as f32 * 0.35);
                let gold = Color::new(GOLD.r, GOLD.g, GOLD.b, alpha);
                for i in 0..4 {
                    let a = i as f32 * std::f32::consts::FRAC_PI_4;
                    let r = vec2(a.cos(), a.sin()) * badge * 0.38;
                    draw_line(c.x - r.x, c.y - r.y, c.x + r.x, c.y + r.y, 1.5, gold);
                }
                draw_circle(c.x, c.y, badge * 0.18, gold);
            }
        }

        // ---- HUD ----
        draw_rectangle(0., 0., w, 96., INK);
        draw_rectangle(28., 27., 3., 42., GOLD);
        label(&font, "В Е Л И Г Р А Д", 46., 47., 27, PAPER);
        label(
            &font,
            &format!(
                "ДЕНЬ {}  /  {}",
                gameplay::day(now),
                gameplay::time_text(now).to_uppercase()
            ),
            47.,
            73.,
            13,
            GOLD,
        );
        label(&font, "ТРУДНО БЫТЬ БОГОМ", w - 330., 43., 18, PAPER);
        label(
            &font,
            &format!("ЖИТЕЛЕЙ {}  •  ПРОТОТИП 0.3", people.len()),
            w - 330.,
            68.,
            12,
            GOLD,
        );
        // Grace, coins and the item in hand.
        draw_rectangle(28., 108., 300., 62., INK);
        label(&font, "Благодать", 40., 130., 15, PAPER);
        draw_rectangle(126., 119., 186., 12., Color::new(0.2, 0.22, 0.2, 1.));
        draw_rectangle(126., 119., 186. * bag.grace / 100., 12., GOLD);
        let held = bag
            .selected_item()
            .map(|i| format!("В руке: {} ×{}", i.name(), bag.items.count(i)))
            .unwrap_or_else(|| "Руки пусты".into());
        label(
            &font,
            &format!("{held}   •   Монеты {}", bag.items.count(Item::Coin)),
            40.,
            157.,
            14,
            SAGE,
        );
        let typing = dialogue.is_some() || whisper.is_some();
        if playing && !typing && !inventory {
            draw_line(w / 2. - 5., h / 2., w / 2. + 5., h / 2., 1., PAPER);
            draw_line(w / 2., h / 2. - 5., w / 2., h / 2. + 5., 1., PAPER);
            if let Some(id) = nearest {
                let n = &people[id];
                draw_rectangle(w / 2. - 200., h * 0.54, 400., 70., INK);
                label(
                    &font,
                    &format!("{}  /  {}", n.name, n.role),
                    w / 2. - 180.,
                    h * 0.54 + 26.,
                    20,
                    PAPER,
                );
                label(
                    &font,
                    "E — разговор   G — дать предмет в руке",
                    w / 2. - 180.,
                    h * 0.54 + 51.,
                    16,
                    GOLD,
                );
            }
            if let Some(n) = people
                .iter()
                .filter(|n| {
                    !n.speech.is_empty()
                        && now < n.speech_until
                        && n.pos.distance(player) < 9.
                        && town.clear_line(player, n.pos)
                })
                .min_by(|a, b| {
                    a.pos
                        .distance_squared(player)
                        .total_cmp(&b.pos.distance_squared(player))
                })
            {
                let bw = w.min(900.) - 56.;
                draw_rectangle(28., h - 300., bw, 84., INK);
                label(
                    &font,
                    &format!("{}{}", n.name, target_name(&people, n.target)),
                    46.,
                    h - 275.,
                    17,
                    GOLD,
                );
                wrapped(&font, &n.speech, 46., h - 250., bw - 40., 16);
            }
        }
        // Event feed.
        let mut fy = h - 142.;
        for (text, at) in feed.lines.iter().rev().take(3) {
            if (now - at < 14. || !playing) && !typing {
                label(
                    &font,
                    text,
                    30.,
                    fy,
                    14,
                    Color::new(
                        GOLD.r,
                        GOLD.g,
                        GOLD.b,
                        if now - at < 10. { 1. } else { 0.5 },
                    ),
                );
                fy -= 20.;
            }
        }
        if map && !typing {
            let size = 200.;
            let scale = size / (LIMIT * 2.);
            let mx = w - size - 24.;
            let my = 108.;
            draw_rectangle(mx - 8., my - 8., size + 16., size + 36., INK);
            let to_map = |p: Vec2| vec2(mx + (p.x + LIMIT) * scale, my + (p.y + LIMIT) * scale);
            for b in &town.blocks {
                let p = to_map(b.min);
                let s = (b.max - b.min) * scale;
                let color = if b.min.y >= 50. {
                    Color::new(0.24, 0.42, 0.5, 1.)
                } else {
                    Color::new(0.38, 0.40, 0.35, 1.)
                };
                draw_rectangle(p.x, p.y, s.x, s.y, color);
            }
            for pl in &town.places {
                let p = to_map(pl.pos);
                draw_circle(p.x, p.y, 2., SAGE);
                label(&font, pl.name, p.x + 3., p.y + 3., 9, SAGE);
            }
            for n in &people {
                let p = to_map(n.pos);
                draw_circle(p.x, p.y, 2.2, if n.faith >= 60 { WHITE } else { GOLD });
            }
            let p = to_map(player);
            draw_circle(p.x, p.y, 3., PAPER);
            draw_line(
                p.x,
                p.y,
                p.x + yaw.cos() * 9.,
                p.y + yaw.sin() * 9.,
                1.5,
                PAPER,
            );
            label(
                &font,
                "M — скрыть карту  •  белые — верующие",
                mx,
                my + size + 20.,
                10,
                PAPER,
            );
        }
        // Powers bar.
        draw_rectangle(0., h - 132., w, 46., Color::new(INK.r, INK.g, INK.b, 0.8));
        let slot = (w - 56.) / 6.;
        for (i, power) in Power::ALL.into_iter().enumerate() {
            let x = 28. + i as f32 * slot;
            let active = power == Power::Flight && bag.flying;
            let ready = bag.grace >= power.cost() || active;
            let color = if active {
                WHITE
            } else if ready {
                GOLD
            } else {
                DIM
            };
            let cost = if power == Power::Flight {
                format!("{:.0}/с", power.cost())
            } else {
                format!("{:.0}", power.cost())
            };
            label(
                &font,
                &format!("{}  {}", i + 1, power.name()),
                x,
                h - 112.,
                15,
                color,
            );
            label(&font, &cost, x, h - 95., 11, if ready { SAGE } else { DIM });
        }
        draw_rectangle(0., h - 86., w, 86., INK);
        label(
            &font,
            "WASD ходьба  SHIFT бег  SPACE прыжок  ЛКМ удар  E разговор  G дать  колесо предмет  I вещи  H мысли  F крик",
            28.,
            h - 58.,
            15,
            PAPER,
        );
        label(
            &font,
            &status,
            28.,
            h - 32.,
            12,
            Color::new(0.60, 0.72, 0.65, 1.),
        );
        label(
            &font,
            "F5 сохранить  F6 ИИ  F9 загрузить",
            w - 290.,
            h - 32.,
            13,
            PAPER,
        );
        if thoughts
            && !inventory
            && dialogue.is_none()
            && let Some(n) = people
                .iter()
                .filter(|n| n.pos.distance(player) < 16. && town.clear_line(player, n.pos))
                .min_by(|a, b| {
                    a.pos
                        .distance_squared(player)
                        .total_cmp(&b.pos.distance_squared(player))
                })
        {
            let mut rows: Vec<(String, u16, Color)> = vec![];
            for l in wrap_lines(&font, &n.thought, 365., 16) {
                rows.push((l, 16, PAPER));
            }
            if !n.goal.is_empty() {
                for l in wrap_lines(&font, &format!("Цель: {}", n.goal), 365., 14) {
                    rows.push((l, 14, GOLD));
                }
            }
            for l in wrap_lines(&font, &format!("Мечта: {}", n.long_goal), 365., 13) {
                rows.push((l, 13, DIM));
            }
            if !n.opinion.is_empty() {
                for l in wrap_lines(&font, &format!("О вас: {}", n.opinion), 365., 14) {
                    rows.push((l, 14, SAGE));
                }
            }
            for l in wrap_lines(
                &font,
                &format!("Вещи: {}", n.inventory.describe()),
                365.,
                13,
            ) {
                rows.push((l, 13, PAPER));
            }
            rows.push((
                format!(
                    "Здоровье {}  Страх {:.0}%  Доверие {}  Вера {}%  Голод {:.0}%",
                    n.health,
                    n.fear * 100.,
                    n.trust,
                    n.faith,
                    n.hunger * 100.
                ),
                12,
                PAPER,
            ));
            rows.push((format!("Сейчас: {}", doing(&people, n)), 14, GOLD));
            let height: f32 = rows.iter().map(|(_, s, _)| *s as f32 * 1.45).sum::<f32>() + 52.;
            draw_rectangle(28., 180., 400., height, INK);
            label(
                &font,
                &format!("МЫСЛИ / {}, {}", n.name, n.role),
                44.,
                206.,
                17,
                GOLD,
            );
            let mut y = 232.;
            for (text, size, color) in &rows {
                label(&font, text, 44., y, *size, *color);
                y += *size as f32 * 1.45;
            }
        }
        if inventory {
            let kinds = bag.items.kinds();
            let rows = kinds.len().max(1) as f32;
            let bh = 150. + rows * 30.;
            let x = (w - 600.) / 2.;
            let y = (h - bh) / 2.;
            draw_rectangle(x, y, 600., bh, INK);
            label(&font, "ВЕЩИ ПУТНИКА", x + 26., y + 44., 26, GOLD);
            if kinds.is_empty() {
                label(
                    &font,
                    "Пусто. Сотворите хлеб (2) или подберите что-нибудь (R).",
                    x + 26.,
                    y + 90.,
                    17,
                    PAPER,
                );
            }
            let selected = bag.selected_item();
            for (i, item) in kinds.iter().enumerate() {
                let iy = y + 86. + i as f32 * 30.;
                let chosen = Some(*item) == selected;
                if chosen {
                    draw_rectangle(x + 16., iy - 21., 568., 28., Color::new(0.2, 0.22, 0.2, 1.));
                }
                draw_rectangle(x + 28., iy - 15., 14., 14., item.color());
                label(
                    &font,
                    &format!("{}   ×{}", item.name(), bag.items.count(*item)),
                    x + 52.,
                    iy,
                    19,
                    if chosen { GOLD } else { PAPER },
                );
            }
            label(
                &font,
                "↑/↓ или колесо — выбрать   U — съесть   G — дать жителю рядом",
                x + 26.,
                y + bh - 44.,
                15,
                GOLD,
            );
            label(&font, "I / ESC — закрыть", x + 26., y + bh - 20., 14, PAPER);
        }
        if let Some(id) = whisper {
            let n = &people[id];
            let bw = (w - 80.).min(760.);
            let x = (w - bw) / 2.;
            let y = h * 0.36;
            draw_rectangle(x, y, bw, 150., INK);
            draw_rectangle(x, y, 3., 150., WHITE);
            label(
                &font,
                &format!("ВНУШЕНИЕ  /  {}", n.name),
                x + 24.,
                y + 36.,
                22,
                WHITE,
            );
            label(
                &font,
                "Ваши слова прозвучат у жителя в голове как чужой голос.",
                x + 24.,
                y + 60.,
                14,
                SAGE,
            );
            draw_rectangle(
                x + 20.,
                y + 76.,
                bw - 40.,
                40.,
                Color::new(0.15, 0.18, 0.17, 1.),
            );
            let shown = if input.is_empty() {
                "Что внушить?…".to_string()
            } else {
                input.clone()
            };
            label(&font, &shown, x + 32., y + 103., 17, PAPER);
            label(
                &font,
                "ENTER — внушить   ESC — отмена",
                x + 24.,
                y + 138.,
                13,
                GOLD,
            );
        }
        if let Some(id) = dialogue {
            let n = &people[id];
            let box_w = (w - 80.).min(940.);
            let box_h = 440.0f32.min(h - 136. - 184.);
            let x = (w - box_w) / 2.;
            let y = h - 136. - box_h;
            draw_rectangle(x, y, box_w, box_h, INK);
            draw_rectangle(x, y, 3., box_h, GOLD);
            label(
                &font,
                &format!("{}  /  {}", n.name, n.role),
                x + 24.,
                y + 38.,
                24,
                GOLD,
            );
            label(
                &font,
                &format!(
                    "Доверие {}  •  Вера {}%  •  Сейчас: {}  •  H — мысли",
                    n.trust,
                    n.faith,
                    doing(&people, n)
                ),
                x + 24.,
                y + 64.,
                13,
                PAPER,
            );
            let size = 17;
            let row = size as f32 * 1.45;
            let mut log_top = y + 96.;
            if thoughts {
                // H shows the resident's private thought inside the dialogue window.
                let mut info = format!("Думает: {}", n.thought);
                if !n.goal.is_empty() {
                    info.push_str(&format!("   •   Цель: {}", n.goal));
                }
                for text in wrap_lines(&font, &info, box_w - 48., 14).iter().take(3) {
                    label(&font, text, x + 24., log_top - 4., 14, SAGE);
                    log_top += 14. * 1.45;
                }
                log_top += 8.;
            }
            let mut rows: Vec<(String, Color)> = Vec::new();
            for line in &n.talk {
                let (who, color) = if line.player {
                    ("Вы", SAGE)
                } else {
                    (n.name, PAPER)
                };
                for text in wrap_lines(&font, &format!("{who}: {}", line.text), box_w - 48., size) {
                    rows.push((text, color));
                }
            }
            if awaiting == Some(id) {
                rows.push((format!("{} думает…", n.name), GOLD));
            }
            if rows.is_empty() {
                rows.push(("Житель смотрит на вас. Что вы хотите сказать?".into(), GOLD));
            }
            let log_bottom = y + box_h - 120.;
            let fit = ((log_bottom - log_top) / row).max(1.) as usize;
            for (i, (text, color)) in rows.iter().skip(rows.len().saturating_sub(fit)).enumerate() {
                label(&font, text, x + 24., log_top + i as f32 * row, size, *color);
            }
            let input_y = y + box_h - 100.;
            draw_rectangle(
                x + 20.,
                input_y,
                box_w - 40.,
                56.,
                Color::new(0.15, 0.18, 0.17, 1.),
            );
            let display = if input.is_empty() {
                "Введите сообщение по-русски…".to_string()
            } else {
                input
                    .chars()
                    .rev()
                    .take(65)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect()
            };
            label(&font, &display, x + 34., input_y + 35., 18, PAPER);
            let gift = bag
                .selected_item()
                .map(|i| format!("TAB — подарить: {}", i.name()))
                .unwrap_or_default();
            label(
                &font,
                &if awaiting == Some(id) {
                    format!("Житель обдумывает ответ…   {gift}   ESC — закончить")
                } else {
                    format!("ENTER — сказать   {gift}   колесо — другой предмет   ESC — закончить")
                },
                x + 24.,
                y + box_h - 18.,
                13,
                GOLD,
            );
        }
        if !playing {
            draw_rectangle(0., 96., w, h - 182., Color::new(0.035, 0.055, 0.05, 0.86));
            let x = w * 0.14;
            label(&font, "НАБЛЮДАТЕЛЬ", x, h * 0.24, 17, GOLD);
            label(&font, "Бог среди людей.", x, h * 0.32, 48, PAPER);
            wrapped(
                &font,
                "Велиград живёт своими заботами: у каждого жителя есть мечта, работа, друзья и враги. Они торгуют, договариваются, просят друг друга о помощи. Вы — чужак, наделённый силой. Докажите, что вы бог, — или останьтесь в тени.",
                x,
                h * 0.39,
                720.,
                19,
            );
            label(
                &font,
                "ENTER / щелчок — войти в город",
                x,
                h * 0.58,
                23,
                GOLD,
            );
            label(
                &font,
                "1 Исцеление   2 Хлеб   3 Молния   4 Полёт   5 Внушение мысли   6 Скачок",
                x,
                h * 0.64,
                17,
                PAPER,
            );
            label(
                &font,
                "Просите жителей о чём угодно: принести, позвать, пойти, отдать. Они решат сами.",
                x,
                h * 0.69,
                16,
                SAGE,
            );
            label(
                &font,
                "Закрыть игру: Alt+F4. Мир сохраняется автоматически.",
                x,
                h * 0.74,
                14,
                PAPER,
            );
        }
        if smoke_actions && frame == 90 {
            get_screen_data().export_png(root.join("docs/inventory.png").to_str().unwrap());
        }
        if smoke_actions && frame == 150 {
            get_screen_data().export_png(root.join("docs/actions.png").to_str().unwrap());
        }
        if smoke_actions && frame == 165 {
            get_screen_data().export_png(root.join("docs/dialogue.png").to_str().unwrap());
            println!(
                "ACTIONS_SMOKE_OK: jump, melee, heal, gift, lightning, local conversation, inventory UI, dialogue UI"
            );
            break;
        }
        if smoke && !smoke_actions && frame == 90 {
            get_screen_data().export_png(root.join("docs/prototype.png").to_str().unwrap());
            // Then look at the temple district at dusk.
            player = vec2(0., -27.);
            pitch = 0.12;
            sim_time = gameplay::DAY_LENGTH * 10.5 / 24.;
        }
        if smoke && !smoke_actions && frame == 110 {
            get_screen_data().export_png(root.join("docs/district.png").to_str().unwrap());
            println!(
                "SMOKE_OK: rendered town, {} citizens, {} props, {} places",
                people.len(),
                town.props.len(),
                town.places.len()
            );
            break;
        }
        if is_quit_requested() {
            break;
        }
        frame += 1;
        next_frame().await;
    }
    if !smoke && let Err(e) = npc::save(&save_path, &people).and_then(|_| bag.save(&player_path)) {
        eprintln!("Save: {e}");
    }
}
