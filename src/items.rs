use macroquad::prelude::Color;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    Bread,
    Herbs,
    Medicine,
    Coin,
    Ale,
    Tool,
    Book,
    Candle,
    Flour,
    Fish,
}
impl Item {
    pub const ALL: [Item; 10] = [
        Item::Bread,
        Item::Herbs,
        Item::Medicine,
        Item::Coin,
        Item::Ale,
        Item::Tool,
        Item::Book,
        Item::Candle,
        Item::Flour,
        Item::Fish,
    ];
    /// Russian name; the model uses these words in `item`.
    pub fn name(self) -> &'static str {
        match self {
            Item::Bread => "хлеб",
            Item::Herbs => "травы",
            Item::Medicine => "лекарство",
            Item::Coin => "монета",
            Item::Ale => "эль",
            Item::Tool => "инструмент",
            Item::Book => "книга",
            Item::Candle => "свеча",
            Item::Flour => "мука",
            Item::Fish => "рыба",
        }
    }
    pub fn from_name(name: &str) -> Option<Item> {
        let name = name.trim().to_lowercase();
        Item::ALL
            .into_iter()
            .find(|i| i.name() == name || (name.len() > 4 && name.starts_with(&i.name()[..4])))
    }
    /// (hunger relief, health gain) when eaten or drunk.
    pub fn nourishment(self) -> Option<(f32, i32)> {
        match self {
            Item::Bread => Some((0.45, 5)),
            Item::Fish => Some((0.4, 5)),
            Item::Ale => Some((0.12, 0)),
            Item::Medicine => Some((0., 45)),
            Item::Herbs => Some((0.05, 10)),
            _ => None,
        }
    }
    /// How much a gift of this item warms the receiver.
    pub fn gift_value(self) -> i32 {
        match self {
            Item::Coin => 4,
            Item::Bread | Item::Fish => 7,
            Item::Medicine => 12,
            Item::Ale | Item::Herbs | Item::Candle => 5,
            Item::Tool | Item::Book => 8,
            Item::Flour => 4,
        }
    }
    pub fn color(self) -> Color {
        match self {
            Item::Bread => Color::new(0.72, 0.46, 0.22, 1.),
            Item::Herbs => Color::new(0.34, 0.6, 0.28, 1.),
            Item::Medicine => Color::new(0.32, 0.65, 0.51, 1.),
            Item::Coin => Color::new(0.85, 0.7, 0.25, 1.),
            Item::Ale => Color::new(0.55, 0.36, 0.16, 1.),
            Item::Tool => Color::new(0.45, 0.47, 0.5, 1.),
            Item::Book => Color::new(0.42, 0.18, 0.18, 1.),
            Item::Candle => Color::new(0.93, 0.9, 0.78, 1.),
            Item::Flour => Color::new(0.9, 0.88, 0.82, 1.),
            Item::Fish => Color::new(0.5, 0.6, 0.68, 1.),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Inventory(pub BTreeMap<Item, u32>);
impl Inventory {
    pub fn of(items: &[(Item, u32)]) -> Self {
        let mut inv = Self::default();
        for &(item, n) in items {
            inv.add(item, n);
        }
        inv
    }
    pub fn count(&self, item: Item) -> u32 {
        self.0.get(&item).copied().unwrap_or(0)
    }
    pub fn has(&self, item: Item) -> bool {
        self.count(item) > 0
    }
    pub fn add(&mut self, item: Item, n: u32) {
        if n > 0 {
            let c = self.0.entry(item).or_insert(0);
            *c = (*c + n).min(999);
        }
    }
    pub fn take(&mut self, item: Item, n: u32) -> bool {
        match self.0.get_mut(&item) {
            Some(c) if *c >= n => {
                *c -= n;
                if *c == 0 {
                    self.0.remove(&item);
                }
                true
            }
            _ => false,
        }
    }
    pub fn kinds(&self) -> Vec<Item> {
        self.0.keys().copied().collect()
    }
    pub fn describe(&self) -> String {
        if self.0.is_empty() {
            return "ничего".into();
        }
        self.0
            .iter()
            .map(|(i, n)| format!("{} ×{n}", i.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_counts_and_names() {
        let mut inv = Inventory::of(&[(Item::Bread, 2), (Item::Coin, 3)]);
        assert!(inv.take(Item::Bread, 2));
        assert!(!inv.has(Item::Bread));
        assert!(!inv.take(Item::Bread, 1));
        assert_eq!(inv.describe(), "монета ×3");
        assert_eq!(Item::from_name("Хлеб"), Some(Item::Bread));
        assert_eq!(Item::from_name("монеты"), Some(Item::Coin));
        assert_eq!(Item::from_name(""), None);
    }
}
