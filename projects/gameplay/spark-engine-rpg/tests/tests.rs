//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine_rpg::*;

#[test]
fn party_inv_quest_turn() {
    let mut rpg = RpgEngine::new(".");
    let a = rpg.party.add("hero");
    rpg.party.get_mut(a).unwrap().stats.set("hp", 100.0);
    assert!(rpg.inventory.add("potion", 3).is_ok());
    assert_eq!(rpg.inventory.count("potion"), 3);
    rpg.quests.upsert(QuestId("q1".into()), QuestStatus::Active);
    assert_eq!(rpg.quests.status(&QuestId("q1".into())), Some(QuestStatus::Active));
    rpg.turns.enqueue(a);
    rpg.turns.enqueue(ActorId(99));
    assert_eq!(rpg.turns.current(), Some(a));
    rpg.turns.advance();
    assert_eq!(rpg.turns.current(), Some(ActorId(99)));
}
