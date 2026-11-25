//! Platform dashboards from the stored snapshot, against a synced clone of examples/platform-repo.

mod common;

use common::Fixture;
use pharos_engine::{Engine, Origin, PlacementStatus};

#[test]
fn home_placements_have_status_and_reads() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let dashboards = engine.dashboards().unwrap();

    let home = &dashboards[0];
    assert_eq!(home.metadata.name, "home");
    let names: Vec<_> = home.placements.iter().map(|p| p.widget.as_str()).collect();
    assert_eq!(
        names,
        ["service-catalog", "k8s-context", "welcome", "my-pipelines"]
    );
    let statuses: Vec<_> = home.placements.iter().map(|p| p.status.clone()).collect();
    assert_eq!(
        statuses,
        [
            PlacementStatus::Runnable,
            PlacementStatus::Runnable,
            PlacementStatus::Static,
            PlacementStatus::NeedsSetup {
                inputs: vec!["team".into()],
                secrets: vec!["GITLAB_TOKEN".into()],
            },
        ]
    );
    let reads: Vec<_> = home.placements.iter().map(|p| p.reads.as_deref()).collect();
    assert_eq!(reads, [Some("catalog/services/*.yaml"), None, None, None]);
    assert!(home.placements.iter().all(|p| p.manifest.is_some()));
}

#[test]
fn home_comes_first_then_by_name() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    for name in ["zulu", "alpha"] {
        f.write(
            &format!("dashboards/{name}.yaml"),
            &format!(
                "apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: {{ name: {name} }}\n\
                 spec:\n  widgets:\n    - {{ widget: welcome, x: 0, y: 0, w: 4, h: 2 }}\n"
            ),
        );
    }
    f.commit("more dashboards");
    engine.plan_sync().unwrap();
    engine.apply_sync(false).unwrap();

    let names: Vec<_> = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .map(|d| d.metadata.name)
        .collect();
    assert_eq!(names, ["home", "alpha", "zulu"]);
}

#[test]
fn nothing_before_first_apply() {
    let f = Fixture::new();
    let mut engine = Engine::open(&f.data).unwrap();
    engine.add_source(&f.url(), "main").unwrap();

    assert!(engine.dashboards().unwrap().is_empty());
    assert!(engine.platform().unwrap().is_none());
}

#[test]
fn platform_after_apply() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let platform = engine.platform().unwrap().unwrap();
    assert_eq!(
        platform.metadata.title.as_deref(),
        Some("ACME Developer Portal")
    );
}

fn leaf(name: &str) -> String {
    format!(
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: {name} }}\n\
         spec:\n  source: {{ type: command, executable: kubectl }}\n  view: {{ type: raw }}\n"
    )
}

#[test]
fn platform_composite_expands_children() {
    let f = Fixture::new();
    f.write("widgets/leaf-a/widget.yaml", &leaf("leaf-a"));
    f.write("widgets/leaf-b/widget.yaml", &leaf("leaf-b"));
    f.write(
        "widgets/pair/widget.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: { name: pair }\n\
         spec:\n  widgets:\n    - { widget: leaf-a, x: 0, y: 0, w: 6, h: 2 }\n    \
         - { widget: leaf-b, x: 6, y: 0, w: 6, h: 2 }\n",
    );
    let home = std::fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    f.write(
        "dashboards/home.yaml",
        &format!("{home}    - {{ widget: pair, x: 0, y: 8, w: 12, h: 2 }}\n"),
    );
    let engine = {
        f.commit("composite");
        f.synced_engine()
    };

    let dashboards = engine.dashboards().unwrap();

    let home = &dashboards[0];
    assert_eq!(home.id, "p:home");
    assert_eq!(home.origin, Origin::Platform);
    assert!(!home.editable);
    let pair = home.placements.iter().find(|p| p.widget == "pair").unwrap();
    assert_eq!(pair.status, PlacementStatus::Runnable);
    assert_eq!(pair.children.len(), 2);
    for (child, name) in pair.children.iter().zip(["leaf-a", "leaf-b"]) {
        assert_eq!(child.widget, name);
        assert_eq!(child.status, PlacementStatus::Runnable);
        assert!(child.children.is_empty());
        assert!(child.manifest.is_some());
    }
}

fn user_ids(engine: &Engine) -> Vec<String> {
    engine
        .dashboards()
        .unwrap()
        .into_iter()
        .filter(|d| d.origin == Origin::User)
        .map(|d| d.id)
        .collect()
}

#[test]
fn compose_and_persist() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let created = engine.create_dashboard("My Home").unwrap();
    assert_eq!(created.id, "u:1");
    assert_eq!(created.metadata.name, "my-home");
    assert_eq!(created.metadata.title.as_deref(), Some("My Home"));
    assert!(created.editable);
    engine.add_widget("u:1", "service-catalog", None).unwrap();
    drop(engine);

    let engine = Engine::open(&f.data).unwrap();
    let mine: Vec<_> = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .filter(|d| d.origin == Origin::User)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].placements.len(), 1);
    assert_eq!(mine[0].placements[0].widget, "service-catalog");
    assert_eq!(mine[0].placements[0].status, PlacementStatus::Runnable);
    assert_eq!((mine[0].placements[0].x, mine[0].placements[0].y), (0, 0));
}

#[test]
fn names_slugify_and_suffix_on_collision() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let names: Vec<_> = ["My Home", "my  home!", "My Home", "***"]
        .iter()
        .map(|t| engine.create_dashboard(t).unwrap().metadata.name)
        .collect();
    assert_eq!(names, ["my-home", "my-home-2", "my-home-3", "dashboard"]);
}

#[test]
fn add_widget_places_below_with_default_size() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.add_widget("u:1", "service-catalog", None).unwrap();
    let view = engine.add_widget("u:1", "welcome", None).unwrap();
    let [first, second] = &view.placements[..] else {
        panic!("two placements")
    };
    assert_eq!((first.y, first.h), (0, 4));
    assert_eq!((second.x, second.y), (0, 4));
    assert!(matches!(
        engine.add_widget("u:1", "nope", None),
        Err(pharos_engine::EngineError::UnknownWidget(_))
    ));
}

#[test]
fn missing_widget_in_user_dashboard_is_missing() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.add_widget("u:1", "welcome", None).unwrap();
    // Remove the widget from the repo (and from home, which places it).
    f.remove("widgets/welcome");
    let home = std::fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    let kept: Vec<_> = home.lines().filter(|l| !l.contains("welcome")).collect();
    f.write("dashboards/home.yaml", &(kept.join("\n") + "\n"));
    f.resync(&mut engine, "drop welcome");

    let dashboards = engine.dashboards().unwrap();

    let mine = dashboards.iter().find(|d| d.id == "u:1").unwrap();
    assert_eq!(mine.placements.len(), 1);
    assert_eq!(mine.placements[0].status, PlacementStatus::Missing);
    assert!(mine.placements[0].manifest.is_none());
}

#[test]
fn stale_landing_falls_back() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.set_landing(Some("u:999")).unwrap();

    let ids: Vec<_> = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .map(|d| d.id)
        .collect();

    assert_eq!(ids, ["p:home", "u:1"]);
}

#[test]
fn landing_moves_a_valid_dashboard_first_and_clears() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.create_dashboard("B").unwrap();
    let ids =
        |e: &Engine| -> Vec<String> { e.dashboards().unwrap().into_iter().map(|d| d.id).collect() };
    assert_eq!(ids(&engine), ["p:home", "u:1", "u:2"]);

    engine.set_landing(Some("u:2")).unwrap();
    assert_eq!(ids(&engine), ["u:2", "p:home", "u:1"]);
    engine.set_landing(Some("p:home")).unwrap();
    assert_eq!(ids(&engine), ["p:home", "u:1", "u:2"]);
    engine.set_landing(Some("u:2")).unwrap();
    engine.set_landing(None).unwrap();
    assert_eq!(ids(&engine), ["p:home", "u:1", "u:2"]);
    assert!(engine.set_landing(Some("garbage")).is_err());
}

#[test]
fn remove_widget_returns_fresh_view() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.add_widget("u:1", "service-catalog", None).unwrap();
    let view = engine.add_widget("u:1", "welcome", None).unwrap();
    let doomed = view.placements[0].id.unwrap();

    let fresh = engine.remove_widget("u:1", doomed).unwrap();

    assert_eq!(fresh.placements.len(), 1);
    assert_eq!(fresh.placements[0].widget, "welcome");
    assert!(engine.remove_widget("u:1", doomed).is_err());
}

#[test]
fn delete_dashboard_cascades() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    engine.add_widget("u:1", "welcome", None).unwrap();
    engine.delete_dashboard("u:1").unwrap();
    assert!(user_ids(&engine).is_empty());
    let db = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    let rows: i64 = db
        .query_row("SELECT COUNT(*) FROM placements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn platform_dashboards_cannot_be_mutated_and_rename_works() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    assert!(engine.rename_dashboard("p:home", "x").is_err());
    assert!(engine.delete_dashboard("p:home").is_err());
    assert!(engine.add_widget("p:home", "welcome", None).is_err());
    assert!(engine.remove_widget("p:home", 1).is_err());
    assert!(engine.delete_dashboard("u:42").is_err());

    engine.rename_dashboard("u:1", "Renamed").unwrap();

    let d = engine.dashboards().unwrap();
    let mine = d.iter().find(|d| d.id == "u:1").unwrap();
    assert_eq!(mine.metadata.title.as_deref(), Some("Renamed"));
    let catalog = engine.catalog().unwrap();
    assert_eq!(catalog.len(), 5);
    let containers: Vec<_> = catalog
        .iter()
        .filter(|w| w.container)
        .map(|w| w.name.as_str())
        .collect();
    assert_eq!(containers, ["on-call-overview"]);
}

#[test]
fn over_deep_container_chain_is_not_available_not_a_stack_overflow() {
    const CHAIN: usize = 40; // deeper than the engine's expansion cap (32)
    let f = Fixture::new();
    f.write("widgets/chain-leaf/widget.yaml", &leaf("chain-leaf"));
    for i in 0..CHAIN {
        let child = if i + 1 == CHAIN {
            "chain-leaf".to_string()
        } else {
            format!("chain-{}", i + 1)
        };
        f.write(
            &format!("widgets/chain-{i}/widget.yaml"),
            &format!(
                "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: chain-{i} }}\n\
                 spec:\n  widgets:\n    - {{ widget: {child}, x: 0, y: 0, w: 12, h: 2 }}\n"
            ),
        );
    }
    let home = std::fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    f.write(
        "dashboards/home.yaml",
        &format!("{home}    - {{ widget: chain-0, x: 0, y: 8, w: 12, h: 2 }}\n"),
    );
    f.commit("deep chain");
    let engine = f.synced_engine();

    let dashboards = engine.dashboards().unwrap();

    let mut node = dashboards[0]
        .placements
        .iter()
        .find(|p| p.widget == "chain-0")
        .unwrap();
    let mut depth = 0;
    while let Some(child) = node.children.first() {
        node = child;
        depth += 1;
    }
    assert_eq!(depth, 32, "expansion stops at the cap");
    assert_eq!(node.widget, "chain-32");
    assert!(matches!(node.status, PlacementStatus::NotAvailable { .. }));
    assert!(node.children.is_empty());
}

/// Inserts a group (no widget) and optionally a widget child under it into user
/// dashboard 1 straight through SQLite (no engine API creates groups yet).
fn insert_group(f: &Fixture, with_member: bool) {
    let conn = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    conn.execute(
        "INSERT INTO placements (dashboard_id, parent_id, widget, title, x, y, w, h, position, inputs)
         VALUES (1, NULL, NULL, 'My K8s', 0, 0, 12, 4, 0, '{}')",
        [],
    )
    .unwrap();
    if with_member {
        let gid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO placements (dashboard_id, parent_id, widget, title, x, y, w, h, position, inputs)
             VALUES (1, ?1, 'service-catalog', NULL, 0, 0, 6, 2, 0, '{}')",
            [gid],
        )
        .unwrap();
    }
}

#[test]
fn a_user_group_becomes_a_titled_container_with_children_and_ids() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    insert_group(&f, true);
    let view = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .find(|d| d.id == "u:1")
        .unwrap();
    let group = &view.placements[0];
    assert!(group.container);
    assert_eq!(group.title.as_deref(), Some("My K8s"));
    assert!(group.id.is_some());
    assert!(group.manifest.is_none());
    assert_eq!(group.children.len(), 1);
    assert!(group.children[0].id.is_some());
    assert_eq!(group.status, PlacementStatus::Runnable);
}

#[test]
fn an_empty_user_group_is_a_static_container() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    insert_group(&f, false);
    let view = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .find(|d| d.id == "u:1")
        .unwrap();
    let group = &view.placements[0];
    assert!(group.container && group.children.is_empty());
    assert_eq!(group.status, PlacementStatus::Static);
}

#[test]
fn create_group_adds_an_empty_titled_container() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let view = engine.create_group("u:1", "Infra", None).unwrap();
    let [group] = &view.placements[..] else {
        panic!("one placement")
    };
    assert!(group.container && group.children.is_empty());
    assert_eq!(group.title.as_deref(), Some("Infra"));
    assert!(group.id.is_some());
    assert!(engine.create_group("u:1", "  ", None).is_err());
}

#[test]
fn add_widget_into_a_group_nests_it() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let view = engine.create_group("u:1", "Infra", None).unwrap();
    let gid = view.placements[0].id.unwrap();
    engine
        .add_widget("u:1", "service-catalog", Some(gid))
        .unwrap();
    let view = engine.add_widget("u:1", "welcome", Some(gid)).unwrap();
    assert_eq!(view.placements.len(), 1);
    let kids = &view.placements[0].children;
    assert_eq!(kids.len(), 2);
    assert!(kids.iter().all(|k| k.id.is_some()));
    assert_eq!((kids[0].y, kids[0].h), (0, 4));
    assert_eq!(kids[1].y, 4);
}

#[test]
fn rename_placement_changes_a_group_title() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let view = engine.create_group("u:1", "Infra", None).unwrap();
    let gid = view.placements[0].id.unwrap();
    let view = engine.rename_placement("u:1", gid, "K8s").unwrap();
    assert_eq!(view.placements[0].title.as_deref(), Some("K8s"));
    assert!(engine.rename_placement("u:1", 9999, "x").is_err());
}

#[test]
fn remove_widget_removes_a_nested_group_and_members() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let view = engine.create_group("u:1", "Infra", None).unwrap();
    let gid = view.placements[0].id.unwrap();
    engine.add_widget("u:1", "welcome", Some(gid)).unwrap();
    let view = engine.remove_widget("u:1", gid).unwrap();
    assert!(view.placements.is_empty());
    let db = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    let rows: i64 = db
        .query_row("SELECT COUNT(*) FROM placements", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0, "cascade removed the member row too");
    // And a nested member can be removed on its own.
    let view = engine.create_group("u:1", "Again", None).unwrap();
    let gid = view.placements[0].id.unwrap();
    let view = engine.add_widget("u:1", "welcome", Some(gid)).unwrap();
    let member = view.placements[0].children[0].id.unwrap();
    let view = engine.remove_widget("u:1", member).unwrap();
    assert_eq!(view.placements.len(), 1);
    assert!(view.placements[0].children.is_empty());
}

#[test]
fn add_widget_rejects_an_unknown_parent() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    assert!(matches!(
        engine.add_widget("u:1", "welcome", Some(9999)),
        Err(pharos_engine::EngineError::UnknownPlacement { .. })
    ));
}

#[test]
fn widget_placements_cannot_be_parents_or_renamed() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let view = engine.add_widget("u:1", "welcome", None).unwrap();
    let wid = view.placements[0].id.unwrap();
    assert!(matches!(
        engine.add_widget("u:1", "welcome", Some(wid)),
        Err(pharos_engine::EngineError::InvalidValue { .. })
    ));
    assert!(matches!(
        engine.create_group("u:1", "G", Some(wid)),
        Err(pharos_engine::EngineError::InvalidValue { .. })
    ));
    assert!(matches!(
        engine.rename_placement("u:1", wid, "x"),
        Err(pharos_engine::EngineError::InvalidValue { .. })
    ));
}

#[test]
fn a_group_nested_past_the_depth_limit_is_not_available() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    engine.create_dashboard("A").unwrap();
    let mut parent = None;
    let mut view = None;
    for _ in 0..=pharos_engine::MAX_PLACEMENT_DEPTH {
        let v = engine.create_group("u:1", "G", parent).unwrap();
        // The newest group is the only one without children; find its id by walking down.
        let mut node = &v.placements[0];
        while let Some(c) = node.children.first() {
            node = c;
        }
        parent = node.id;
        view = Some(v);
    }
    let mut node = &view.unwrap().placements[0];
    while let Some(c) = node.children.first() {
        node = c;
    }
    assert_eq!(
        node.status,
        PlacementStatus::NotAvailable {
            feature: "groups nested this deeply".to_string()
        }
    );
    assert!(node.children.is_empty());
}

#[test]
fn customize_seeds_a_user_dashboard_from_a_platform_one() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let platform = engine.dashboards().unwrap().remove(0);
    assert_eq!(platform.id, "p:home");

    let view = engine.customize_dashboard("p:home").unwrap();

    assert_eq!(view.origin, Origin::User);
    assert!(view.editable);
    assert_eq!(view.metadata.title, platform.metadata.title);
    assert_eq!(view.placements.len(), platform.placements.len());
    for (copy, orig) in view.placements.iter().zip(&platform.placements) {
        assert_eq!(copy.widget, orig.widget);
        assert_eq!(
            (copy.x, copy.y, copy.w, copy.h),
            (orig.x, orig.y, orig.w, orig.h)
        );
        assert!(copy.id.is_some());
    }
    // The platform dashboard is untouched and the copy is listed.
    assert!(engine.dashboards().unwrap().iter().any(|d| d.id == view.id));
}

#[test]
fn customize_twice_makes_two_independent_dashboards() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let a = engine.customize_dashboard("p:home").unwrap();
    let b = engine.customize_dashboard("p:home").unwrap();
    assert_ne!(a.id, b.id);
    assert_ne!(a.metadata.name, b.metadata.name);
    engine.delete_dashboard(&a.id).unwrap();
    let after = engine.dashboards().unwrap();
    let kept = after.iter().find(|d| d.id == b.id).unwrap();
    assert_eq!(kept.placements.len(), b.placements.len());
}

#[test]
fn customize_an_unknown_or_empty_platform_dashboard() {
    let f = Fixture::new();
    f.write(
        "dashboards/empty.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: { name: empty }\n\
         spec:\n  widgets: []\n",
    );
    f.commit("empty dashboard");
    let mut engine = f.synced_engine();

    assert!(matches!(
        engine.customize_dashboard("p:does-not-exist"),
        Err(pharos_engine::EngineError::UnknownDashboard(_))
    ));
    assert!(engine.customize_dashboard("u:1").is_err());
    assert!(engine.customize_dashboard("garbage").is_err());
    let view = engine.customize_dashboard("p:empty").unwrap();
    assert_eq!(view.origin, Origin::User);
    assert!(view.placements.is_empty());
    assert_eq!(view.metadata.title.as_deref(), Some("empty"));
}

#[test]
fn customize_copies_a_container_as_one_row() {
    let f = Fixture::new();
    f.write("widgets/leaf-a/widget.yaml", &leaf("leaf-a"));
    f.write("widgets/leaf-b/widget.yaml", &leaf("leaf-b"));
    f.write(
        "widgets/pair/widget.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: { name: pair }\n\
         spec:\n  widgets:\n    - { widget: leaf-a, x: 0, y: 0, w: 6, h: 2 }\n    \
         - { widget: leaf-b, x: 6, y: 0, w: 6, h: 2 }\n",
    );
    f.write(
        "dashboards/duo.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: { name: duo, title: Duo }\n\
         spec:\n  widgets:\n    - { widget: pair, x: 0, y: 0, w: 12, h: 2 }\n",
    );
    f.commit("duo");
    let mut engine = f.synced_engine();

    let view = engine.customize_dashboard("p:duo").unwrap();

    assert_eq!(view.placements.len(), 1);
    let pair = &view.placements[0];
    assert_eq!(pair.widget, "pair");
    // Re-expanded from the manifest, so the children carry no row ids of their own.
    assert_eq!(pair.children.len(), 2);
    assert!(pair.children.iter().all(|c| c.id.is_none()));
}
