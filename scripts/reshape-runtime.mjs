/**
 * Spark 运行时硬切断：删除 GameHost / EcsHost 兼容层，收敛到 WindowPump + SparkRuntime。
 *
 * 本脚本记录一次性 reshape 的删除清单与模块搬迁；已执行项会跳过。
 * 参考 CounterStrike/scripts/reshape-crates.mjs（粗粒度搬迁，不逐下游修补）。
 *
 * node scripts/reshape-runtime.mjs
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

/** @type {{ action: "delete" | "move", from: string, to?: string }[]} */
const plan = [
    { action: "delete", from: "projects/gameplay/spark-engine/src/ecs_host.rs" },
    { action: "delete", from: "projects/gameplay/spark-engine/src/app.rs" },
    { action: "delete", from: "projects/gameplay/spark-engine/src/app3d.rs" },
    { action: "delete", from: "projects/gameplay/spark-engine/tests/ecs_host.rs" },
    { action: "delete", from: "projects/gameplay/spark-engine/tests/app.rs" },
    { action: "delete", from: "projects/gameplay/spark-engine/tests/app3d.rs" },
    { action: "delete", from: "projects/examples/ping-pong/src/game.rs" },
    {
        action: "move",
        from: "projects/gameplay/spark-engine/src/ecs_host.rs",
        to: "projects/gameplay/spark-engine/src/frame_state.rs",
    },
];

function abs(rel) {
    return path.join(root, rel);
}

let deleted = 0;
let skipped = 0;

for (const step of plan) {
    const fromPath = abs(step.from);
    if (step.action === "delete") {
        if (!fs.existsSync(fromPath)) {
            skipped += 1;
            continue;
        }
        fs.unlinkSync(fromPath);
        console.log(`deleted ${step.from}`);
        deleted += 1;
        continue;
    }
    if (step.action === "move") {
        if (!fs.existsSync(fromPath)) {
            skipped += 1;
            continue;
        }
        const toPath = abs(step.to);
        if (fs.existsSync(toPath)) {
            skipped += 1;
            continue;
        }
        fs.mkdirSync(path.dirname(toPath), { recursive: true });
        fs.renameSync(fromPath, toPath);
        console.log(`moved ${step.from} -> ${step.to}`);
    }
}

console.log(`reshape-runtime: deleted=${deleted} skipped=${skipped}`);
console.log("手动后续：run.rs 仅 run_runtime / run_window_2d；RuntimeHost2d: WindowPump2d；下游游戏仓自行迁移。");
