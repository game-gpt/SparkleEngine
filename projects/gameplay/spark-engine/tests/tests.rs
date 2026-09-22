//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;
use spark_script::{HostPhase, HostSchema, ScriptCompiler, ScriptLanguage};
use spark_vm::StdHost;
use std::path::PathBuf;

#[test]
fn registry_and_hooks_shared() {
    let eng = SparkEngine::new(".");
    {
        let mut s = eng.shared().borrow_mut();
        s.registry.set("meta", "version", RegValue::Number(1.0));
        s.hooks.register("init", "demo", "on_init");
    }
    assert_eq!(eng.shared().borrow().registry.get("meta", "version").and_then(|v| v.as_number()), Some(1.0));
    assert_eq!(eng.shared().borrow().hooks.list("init").len(), 1);
}

#[test]
fn manifest_roundtrip_von() {
    let raw = r#"
id = "demo"
name = "Demo"
version = "0.0.0"
entry = "main.vk"
dependencies = ["core"]
"#;
    let m = spark_engine::manifest::parse_mod_von(raw).unwrap();
    assert_eq!(m.id, "demo");
    assert_eq!(m.dependencies, vec!["core"]);
}

#[test]
fn vfs_stays_inside_root() {
    let dir = std::env::temp_dir().join("spark_engine_vfs_test");
    let _ = std::fs::create_dir_all(&dir);
    let vfs = ModVfs::new("t", dir.clone());
    let ok = vfs.resolve("a/b.txt").unwrap();
    assert!(ok.starts_with(&dir));
    assert!(vfs.resolve("../x").is_err());
}

#[test]
fn hook_calls_script_function() {
    let host = HostSchema::new(1);
    let package = ScriptCompiler::new()
        .compile_source(
            ScriptLanguage::Valkyrie,
            r#"
            micro on_init() {
                return null
            }
            return 0
            "#,
            &host,
        )
        .unwrap();
    let domain = ScriptDomain::from_image("hand", &package.image, &host, ScriptBudget::default()).unwrap();
    let mut eng = SparkEngine::new(".");
    eng.shared().borrow_mut().hooks.register("init", "hand", "on_init");
    eng.insert_mod(
        "hand",
        LoadedMod {
            manifest: ModManifest {
                id: "hand".into(),
                name: "Hand".into(),
                version: "0.0.1".into(),
                entry: None,
                artifact: None,
                language: None,
                dependencies: vec![],
            },
            root: PathBuf::from("."),
            vfs: ModVfs::new("hand", PathBuf::from(".")),
            domain: Some(domain),
            enabled: true,
        },
    );
    eng.fire_hook_std("init", &[]).unwrap();
}

#[test]
fn load_mod_dir_shares_host_schema() {
    let root = std::env::temp_dir().join("spark_engine_mod_schema");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "schema_demo"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            return 1
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    let id = eng.load_mod_dir(&root).unwrap();
    assert_eq!(id, "schema_demo");
    let m = eng.get_mod(&id).unwrap();
    let domain = m.domain.as_ref().unwrap();
    assert!(domain.enabled);
    assert!(domain.has_lifecycle("on_load"));
    assert_eq!(domain.runtime.vm.step_limit, ScriptBudget::default().instruction_limit);
}

#[test]
fn load_mod_writes_and_reuses_spkx_cache() {
    let root = std::env::temp_dir().join("spark_engine_mod_spkx_cache");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "cache_demo"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            return 42
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let cache_dir = root.join(".spark-cache");
    assert!(cache_dir.is_dir());
    let spkx: Vec<_> = std::fs::read_dir(&cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("spkx"))
        .collect();
    assert_eq!(spkx.len(), 1);
    let published = root.join("published.spkx");
    std::fs::copy(spkx[0].path(), &published).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "cache_demo"
version = "0.0.0"
artifact = "published.spkx"
"#,
    )
    .unwrap();
    // 删掉源码：显式 artifact 路径不得再依赖入口编译。
    let _ = std::fs::remove_file(root.join("main.vk"));
    let mut eng2 = SparkEngine::new(root.parent().unwrap());
    let id = eng2.load_mod_dir(&root).unwrap();
    assert!(eng2.get_mod(&id).unwrap().domain.as_ref().unwrap().has_lifecycle("on_load"));
}

#[test]
fn load_mod_registers_lifecycle_systems() {
    let root = std::env::temp_dir().join("spark_engine_mod_systems");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "sys_demo"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            return 1
        }
        micro update() {
            return 2
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    assert!(eng.script_systems().len() >= 2);
    assert!(eng.script_systems().for_phase(spark_script::HostPhase::Update).any(|s| s.mod_id.as_ref() == "sys_demo"));
}

#[test]
fn apply_script_commands_spawns_in_world() {
    let root = std::env::temp_dir().join("spark_engine_mod_apply");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "apply_demo"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            queue_spawn("rock")
            return 1
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let mut world = spark_ecs::World::new();
    let report = eng.apply_script_commands_to_world(&mut world).unwrap();
    assert_eq!(report.spawned.len(), 1);
    let e = report.spawned[0];
    assert_eq!(world.get::<ScriptArchetypeTag>(e).map(|t| t.name.as_ref()), Some("rock"));
    assert_eq!(eng.shared().borrow().query.count("rock"), 1);
}

#[test]
fn query_natives_read_refreshed_snapshot() {
    let root = std::env::temp_dir().join("spark_engine_mod_query");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "query_demo"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            queue_spawn("rock")
            return 0
        }
        micro update() {
            return query_archetype_count("rock")
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let mut world = spark_ecs::World::new();
    eng.apply_script_commands_to_world(&mut world).unwrap();
    let mut host = StdHost;
    let v = eng.get_mod_mut("query_demo").unwrap().domain.as_mut().unwrap().call("update", &[], &mut host).unwrap();
    assert_eq!(v.as_number(), Some(1.0));
    let bits = eng.shared().borrow().query.entity_at("rock", 0).unwrap();
    assert!(world.is_alive(spark_ecs::Entity::from_bits(bits)));
}

#[test]
fn run_script_systems_applies_spawn_from_update() {
    let root = std::env::temp_dir().join("spark_engine_mod_run_sys");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "run_sys"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            return 0
        }
        micro update() {
            queue_spawn("npc")
            return 1
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let mut world = spark_ecs::World::new();
    let mut hooks = StdHost;
    let report = eng.run_script_systems(HostPhase::Update, &mut world, &mut hooks).unwrap();
    assert_eq!(report.spawned.len(), 1);
    let view = ScriptQueryView::new(&world);
    assert_eq!(view.entities_with_archetype("npc").len(), 1);
}

#[test]
fn render_prepare_denies_queue_spawn() {
    let root = std::env::temp_dir().join("spark_engine_mod_phase_deny");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "phase_deny"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro render_prepare() {
            queue_spawn("ghost")
            return 0
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    eng.script_systems_mut().register(ScriptSystemDescriptor::new("phase_deny", "draw", "render_prepare", HostPhase::RenderPrepare));
    let mut world = spark_ecs::World::new();
    let mut hooks = StdHost;
    let err = eng.run_script_systems(HostPhase::RenderPrepare, &mut world, &mut hooks).unwrap_err();
    assert!(err.code().contains("script") || format!("{err:?}").contains("HostDenied"), "{err:?}");
}

#[test]
fn system_without_write_access_denies_add_component() {
    let root = std::env::temp_dir().join("spark_engine_mod_access_deny");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "access_deny"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            queue_spawn("rock")
            return 0
        }
        micro update() {
            queue_add_component(query_entity_at("rock", 0), "Transform")
            return 0
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    // 覆盖默认空访问生命周期：显式 Declared 且无写集。
    eng.script_systems_mut().register(ScriptSystemDescriptor::new("access_deny", "update", "update", HostPhase::Update).read("Transform"));
    let mut world = spark_ecs::World::new();
    eng.apply_script_commands_to_world(&mut world).unwrap();
    let mut hooks = StdHost;
    let err = eng.run_script_systems(HostPhase::Update, &mut world, &mut hooks).unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("host_access_denied") || msg.contains("HostDenied") || msg.contains("script"), "{msg}");
}

#[test]
fn system_without_world_access_denies_query() {
    let root = std::env::temp_dir().join("spark_engine_mod_query_deny");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "query_deny"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro update() {
            return query_archetype_count("rock")
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    // 空访问 Declared：禁止 query_*
    eng.script_systems_mut().register(ScriptSystemDescriptor::new("query_deny", "update", "update", HostPhase::Update));
    let mut world = spark_ecs::World::new();
    let mut hooks = StdHost;
    let err = eng.run_script_systems(HostPhase::Update, &mut world, &mut hooks).unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("read_world") || msg.contains("HostDenied") || msg.contains("script"), "{msg}");
}

#[test]
fn system_query_archetype_filters_snapshot() {
    let root = std::env::temp_dir().join("spark_engine_mod_query_filter");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mod.von"),
        r#"id = "query_filter"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("main.vk"),
        r#"
        micro on_load() {
            queue_spawn("rock")
            queue_spawn("tree")
            return 0
        }
        micro update() {
            return query_archetype_count("rock") + query_archetype_count("tree")
        }
        return 0
        "#,
    )
    .unwrap();
    let mut eng = SparkEngine::new(root.parent().unwrap());
    eng.load_mod_dir(&root).unwrap();
    let desc = ScriptSystemDescriptor::new("query_filter", "update", "update", HostPhase::Update).read("Transform").query_archetype("rock");
    eng.script_systems_mut().register(desc.clone());
    let mut world = spark_ecs::World::new();
    eng.apply_script_commands_to_world(&mut world).unwrap();
    assert_eq!(eng.shared().borrow().query_base.count("rock"), 1);
    assert_eq!(eng.shared().borrow().query_base.count("tree"), 1);

    eng.shared().borrow_mut().begin_script_call(HostPhase::Update, Some(&desc));
    assert_eq!(eng.shared().borrow().query.count("rock"), 1);
    assert_eq!(eng.shared().borrow().query.count("tree"), 0);
    let mut hooks = StdHost;
    let v =
        eng.get_mod_mut("query_filter").unwrap().domain.as_mut().unwrap().call_in_phase("update", &[], HostPhase::Update, &mut hooks).unwrap();
    eng.shared().borrow_mut().end_script_call();
    assert_eq!(v.as_number(), Some(1.0));
    assert_eq!(eng.shared().borrow().query.count("tree"), 1);
}

#[test]
fn tick_scripts_calls_update_lifecycle() {
    let source = r#"
        micro update() {
            return 9
        }
        return 0
        "#;
    let host = HostSchema::new(1);
    let mut compiler = ScriptCompiler::new();
    let package = compiler.compile_source(ScriptLanguage::Valkyrie, source, &host).unwrap();
    let domain = ScriptDomain::from_image("tick.mod", &package.image, &host, ScriptBudget::default()).unwrap();
    domain.command_buffer.borrow_mut().spawn("marker");
    let mut eng = SparkEngine::new(".");
    eng.insert_mod(
        "tick.mod",
        LoadedMod {
            manifest: ModManifest {
                id: "tick.mod".into(),
                name: "Tick".into(),
                version: "0.0.1".into(),
                entry: None,
                artifact: None,
                language: None,
                dependencies: vec![],
            },
            root: PathBuf::from("."),
            vfs: ModVfs::new("tick.mod", PathBuf::from(".")),
            domain: Some(domain),
            enabled: true,
        },
    );
    let mut hooks = StdHost;
    let cmds = eng.tick_scripts(false, &mut hooks).unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].0, "tick.mod");
    assert_eq!(cmds[0].1.len(), 1);
}

#[test]
fn topo_deps_order() {
    let root = std::env::temp_dir().join("spark_engine_mods_topo");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("base")).unwrap();
    std::fs::create_dir_all(root.join("child")).unwrap();
    std::fs::write(
        root.join("base/mod.von"),
        r#"id = "base"
version = "1.0.0"
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("child/mod.von"),
        r#"id = "child"
version = "1.0.0"
dependencies = ["base"]
"#,
    )
    .unwrap();
    let ordered = discover_and_order(&root).unwrap();
    assert_eq!(ordered[0].id, "base");
    assert_eq!(ordered[1].id, "child");
}
