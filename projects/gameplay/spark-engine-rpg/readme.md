# spark-engine-rpg

RPG 骨架：队伍、背包、属性、任务、回合序。

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

职业与技能数据由游戏提供。

```bash
cargo test -p spark-engine-rpg
```
