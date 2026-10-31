#[cfg(test)]
mod audit_repros {
    use super::*;
    use crate::{npc, items::{Inventory, Item}};
    #[test]
    fn audit_remote_delivery_at_last_seen_position() {
        let town = Town::new();
        let mut ns = npc::population(&town);
        let mut bag = PlayerState::default();
        ns[0].pos = vec2(0., 10.);
        ns[0].last_seen_player = Some(vec2(0., 11.));
        ns[0].visible_player = false;
        ns[0].action = Action::Give;
        ns[0].target = Some(Target::Player);
        ns[0].item = Some(Item::Bread);
        ns[0].inventory = Inventory::of(&[(Item::Bread, 1)]);
        let before = bag.items.count(Item::Bread);
        steer(&mut ns, 0, &town, vec2(40., 40.), &mut bag, 10.);
        assert_eq!(bag.items.count(Item::Bread), before + 1, "observed defect: remote gift");
    }
    #[test]
    fn audit_repeated_work_resets_production() {
        let town = Town::new();
        let mut ns = npc::population(&town);
        let mut bag = PlayerState::default();
        let id = ns.iter().position(|n| n.name == "Мила").unwrap();
        ns[id].pos = town.place(ns[id].work).unwrap().pos;
        ns[id].inventory = Inventory::of(&[(Item::Flour, 5)]);
        for now in [1., 11., 21., 31., 41., 51.] {
            act(&mut ns, id, Decision::new("", "", Action::Work), false, &town, vec2(40.,40.), now);
            steer(&mut ns, id, &town, vec2(40.,40.), &mut bag, now);
            steer(&mut ns, id, &town, vec2(40.,40.), &mut bag, now+9.);
        }
        assert_eq!(ns[id].inventory.count(Item::Bread), 0, "observed defect: no progress in 59 seconds");
        steer(&mut ns, id, &town, vec2(40.,40.), &mut bag, 77.);
        assert_eq!(ns[id].inventory.count(Item::Bread), 2, "control: uninterrupted task produces bread");
    }
    #[test]
    fn audit_loading_does_not_restore_zero_relation() {
        let town = Town::new();
        let mut ns = npc::population(&town);
        ns[0].relations[1] = 0;
        let path = std::env::temp_dir().join(format!("htbg-audit-{}.json", std::process::id()));
        npc::save(&path, &ns).unwrap();
        ns[0].relations[1] = 55;
        npc::load(&path, &mut ns).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(ns[0].relations[1], 55, "observed defect: changed relation survives load");
    }
    #[test]
    fn audit_denial_of_divinity_increases_faith() {
        let town = Town::new();
        let mut ns = npc::population(&town);
        ns[0].pos = vec2(0., 10.);
        ns[1].pos = vec2(1., 10.);
        ns[0].faith = 60;
        ns[1].faith = 0;
        ns[0].speech = "Он не бог, никаких чудес не было.".into();
        deliver(&mut ns, 0, &town, 10.);
        assert!(ns[1].faith > 0, "observed defect: negation raises faith");
    }
}


