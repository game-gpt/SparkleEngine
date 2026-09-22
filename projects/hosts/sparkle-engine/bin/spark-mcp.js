#!/usr/bin/env node
"use strict";

/**
 * 极简 MCP stdio 适配：只暴露 `spark_script` 一个工具，内部转调 `spark-shell`。
 * 不引入 MCP SDK；用 JSON-RPC 2.0 子集（initialize / tools/list / tools/call）。
 *
 * 用法：node projects/hosts/sparkle-engine/bin/spark-mcp.js
 * 或配置 Cursor MCP command 指向本文件。
 */

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function engineRoot() {
    return path.resolve(__dirname, "../../../..");
}

function findShellBinary() {
    const env = (typeof process.env.SPARK_SHELL_BIN === "string" && process.env.SPARK_SHELL_BIN.trim()) || "";
    if (env) return path.resolve(env);
    const root = engineRoot();
    const name = process.platform === "win32" ? "spark-shell.exe" : "spark-shell";
    for (const profile of ["release", "debug"]) {
        const candidate = path.join(root, "target", profile, name);
        if (fs.existsSync(candidate)) return candidate;
    }
    return null;
}

function send(msg) {
    const body = JSON.stringify(msg);
    const frame = `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
    process.stdout.write(frame);
}

function runSparkShell(args) {
    const bin = findShellBinary();
    if (!bin) {
        return { ok: false, error: "spark-shell binary missing; run cargo build -p spark-edit" };
    }
    const result = spawnSync(bin, args, {
        encoding: "utf8",
        windowsHide: true,
        env: process.env,
        cwd: process.cwd(),
    });
    if (result.error) {
        return { ok: false, error: String(result.error) };
    }
    return {
        ok: (result.status ?? 1) === 0,
        status: result.status,
        stdout: result.stdout || "",
        stderr: result.stderr || "",
    };
}

function handle(msg) {
    if (!msg || typeof msg !== "object") return;
    const { id, method, params } = msg;

    if (method === "initialize") {
        send({
            jsonrpc: "2.0",
            id,
            result: {
                protocolVersion: "2024-11-05",
                capabilities: { tools: {} },
                serverInfo: { name: "sparkle-mcp", version: "0.0.0" },
            },
        });
        return;
    }

    if (method === "notifications/initialized" || method === "initialized") {
        return;
    }

    if (method === "tools/list") {
        send({
            jsonrpc: "2.0",
            id,
            result: {
                tools: [
                    {
                        name: "spark_script",
                        description: "Run a Spark Edit Runtime VON plan (check / dry-run / apply). Prefer dry-run before apply.",
                        inputSchema: {
                            type: "object",
                            properties: {
                                code: { type: "string", description: "VON edit plan or spark-edit-1 calls" },
                                path: { type: "string", description: "Path to .edit.von plan file" },
                                mode: {
                                    type: "string",
                                    enum: ["check", "dry-run", "apply"],
                                    default: "dry-run",
                                },
                                cwd: { type: "string", description: "Project root" },
                                capabilities: {
                                    type: "array",
                                    items: { type: "string" },
                                    description: "Defaults to read-only. apply requires project-edit or write-assets.",
                                    default: ["read-project"],
                                },
                            },
                        },
                    },
                ],
            },
        });
        return;
    }

    if (method === "tools/call") {
        const name = params && params.name;
        const args = (params && params.arguments) || {};
        if (name !== "spark_script") {
            send({
                jsonrpc: "2.0",
                id,
                error: { code: -32601, message: `unknown tool: ${name}` },
            });
            return;
        }
        const mode = args.mode || "dry-run";
        const caps = Array.isArray(args.capabilities) ? args.capabilities.map(String) : ["read-project"];
        const canWrite = caps.some((c) => {
            const t = c.toLowerCase();
            return t === "project-edit" || t === "write-assets" || t === "project_edit" || t === "write_assets";
        });
        if (mode === "apply" && !canWrite) {
            send({
                jsonrpc: "2.0",
                id,
                result: {
                    content: [
                        {
                            type: "text",
                            text: JSON.stringify({
                                ok: false,
                                diagnostics: [
                                    {
                                        severity: "error",
                                        code: "spark.edit.capability_denied",
                                        message: "apply requires capabilities including project-edit or write-assets",
                                    },
                                ],
                                changes: [],
                                transaction: "rolled_back",
                            }),
                        },
                    ],
                    isError: true,
                },
            });
            return;
        }
        const shellArgs = ["--mode", mode, "--json"];
        for (const c of caps) {
            shellArgs.push("--capability", c);
        }
        if (mode === "apply" && canWrite && caps.length === 0) {
            shellArgs.push("--capability", "project-edit");
        }
        if (args.cwd) {
            shellArgs.push("--cwd", String(args.cwd));
        }
        if (args.code) {
            shellArgs.push("--code", String(args.code));
        } else if (args.path) {
            shellArgs.push("--path", String(args.path));
        } else {
            send({
                jsonrpc: "2.0",
                id,
                result: {
                    content: [{ type: "text", text: JSON.stringify({ ok: false, error: "provide code or path" }) }],
                    isError: true,
                },
            });
            return;
        }
        const out = runSparkShell(shellArgs);
        const text = out.stdout || out.stderr || JSON.stringify(out);
        send({
            jsonrpc: "2.0",
            id,
            result: {
                content: [{ type: "text", text }],
                isError: !out.ok,
            },
        });
        return;
    }

    if (id !== undefined) {
        send({
            jsonrpc: "2.0",
            id,
            error: { code: -32601, message: `method not found: ${method}` },
        });
    }
}

function main() {
    let buffer = Buffer.alloc(0);
    process.stdin.on("data", (chunk) => {
        buffer = Buffer.concat([buffer, chunk]);
        while (true) {
            const headerEnd = buffer.indexOf("\r\n\r\n");
            if (headerEnd < 0) break;
            const header = buffer.slice(0, headerEnd).toString("utf8");
            const match = /Content-Length:\s*(\d+)/i.exec(header);
            if (!match) {
                buffer = buffer.slice(headerEnd + 4);
                continue;
            }
            const len = Number(match[1]);
            const bodyStart = headerEnd + 4;
            if (buffer.length < bodyStart + len) break;
            const body = buffer.slice(bodyStart, bodyStart + len).toString("utf8");
            buffer = buffer.slice(bodyStart + len);
            try {
                handle(JSON.parse(body));
            } catch (err) {
                // ignore malformed
            }
        }
    });
}

main();
