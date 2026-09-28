# spark-engine-rpg

RPG skeleton: party, inventory, stats, quests, turn order.

```rust
use spark_engine_rpg::{ActorId, QuestId, QuestStatus, RpgEngine};

let mut rpg = RpgEngine::new(".");
let hero = rpg.party.add("hero");
rpg.party.get_mut(hero).unwrap().stats.set("hp", 100.0);
rpg.inventory.add("potion", 3).unwrap();
rpg.quests.upsert(QuestId("q1".into()), QuestStatus::Active);
rpg.turns.enqueue(hero);
rpg.turns.advance();
```

Classes and skill data are provided by the game.

```bash
cargo test -p spark-engine-rpg
```
