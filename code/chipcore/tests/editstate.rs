use cvmath::Vec2i;

fn base_level() -> chipty::LevelDto {
	chipty::LevelDto {
		name: "Edit".to_string(),
		author: Some("Author".to_string()),
		hint: Some("Hint".to_string()),
		password: Some("PASS".to_string()),
		required_chips: 2,
		time_limit: 300,
		map: chipty::FieldDto {
			width: 3,
			height: 3,
			legend: vec![chipty::Terrain::Blank, chipty::Terrain::Floor, chipty::Terrain::Wall, chipty::Terrain::Water],
			data: vec![1, 2, 3, 0, 1, 2, 3, 0, 1],
		},
		entities: vec![
			chipty::EntityArgs { kind: chipty::EntityKind::Player, pos: Vec2i::new(0, 0), face_dir: None },
			chipty::EntityArgs { kind: chipty::EntityKind::Tank, pos: Vec2i::new(2, 1), face_dir: Some(chipty::Compass::Left) },
		],
		connections: vec![chipty::FieldConn { src: Vec2i::new(1, 0), dest: Vec2i::new(2, 2) }],
		camera_triggers: vec![chipty::CameraFocusTrigger {
			player_pos: Vec2i::new(0, 1),
			entity_index: 1,
			entity_kind: chipty::EntityKind::Tank,
		}],
		replays: None,
		trophies: None,
	}
}

#[test]
fn from_level_dto_expands_field_data() {
	let state = chipcore::EditState::from_level_dto(&base_level());

	assert_eq!(state.width, 3);
	assert_eq!(state.height, 3);
	assert_eq!(state.get_terrain(Vec2i::new(0, 0)), chipty::Terrain::Floor);
	assert_eq!(state.get_terrain(Vec2i::new(1, 0)), chipty::Terrain::Wall);
	assert_eq!(state.get_terrain(Vec2i::new(2, 0)), chipty::Terrain::Water);
	assert_eq!(state.get_terrain(Vec2i::new(0, 1)), chipty::Terrain::Blank);
	assert_eq!(state.get_terrain(Vec2i::new(-1, 0)), chipty::Terrain::Wall);
	assert_eq!(state.entity_order.len(), 2);
	assert_eq!(state.next_entity_id, 2);
}

#[test]
fn empty_field_data_loads_as_floor() {
	let mut level = base_level();
	level.map.data.clear();

	let state = chipcore::EditState::from_level_dto(&level);

	assert!(state.terrain.iter().all(|&terrain| terrain == chipty::Terrain::Floor));
}

#[test]
fn to_level_dto_preserves_metadata_and_terrain() {
	let mut state = chipcore::EditState::from_level_dto(&base_level());
	state.set_terrain(Vec2i::new(1, 1), chipty::Terrain::Fire);

	let level = state.to_level_dto();
	let round_trip = chipcore::EditState::from_level_dto(&level);

	assert_eq!(level.name, "Edit");
	assert_eq!(level.author.as_deref(), Some("Author"));
	assert_eq!(level.hint.as_deref(), Some("Hint"));
	assert_eq!(level.password.as_deref(), Some("PASS"));
	assert_eq!(level.required_chips, 2);
	assert_eq!(level.time_limit, 300);
	assert_eq!(level.map.legend[0], chipty::Terrain::Blank);
	assert_eq!(level.map.legend[1], chipty::Terrain::Floor);
	assert_eq!(round_trip.terrain, state.terrain);
}

#[test]
fn entity_ids_stay_stable_across_edit_operations() {
	let mut state = chipcore::EditState::default();
	let a = state.create_entity(chipty::EntityArgs { kind: chipty::EntityKind::Player, pos: Vec2i::new(0, 0), face_dir: None });
	let b = state.create_entity(chipty::EntityArgs { kind: chipty::EntityKind::Tank, pos: Vec2i::new(1, 0), face_dir: Some(chipty::Compass::Up) });

	assert!(state.move_entity(a, Vec2i::new(2, 2)));
	assert!(state.rotate_entity(b));
	assert!(state.swap_entity_order(a, b));

	assert_eq!(state.entity_at(Vec2i::new(2, 2)), Some(a));
	assert_eq!(state.entity(b).unwrap().face_dir, Some(chipty::Compass::Right));
	assert_eq!(state.entities_in_order().map(|(id, _)| id).collect::<Vec<_>>(), vec![b, a]);
}

#[test]
fn entities_serialize_in_entity_order_after_normalize() {
	let mut state = chipcore::EditState::default();
	state.width = 3;
	state.height = 3;
	state.terrain = vec![chipty::Terrain::Floor; 9];
	let chip = state.create_entity(chipty::EntityArgs { kind: chipty::EntityKind::Chip, pos: Vec2i::new(1, 0), face_dir: None });
	let player = state.create_entity(chipty::EntityArgs { kind: chipty::EntityKind::Player, pos: Vec2i::new(0, 0), face_dir: None });
	state.required_chips = -1;
	assert_eq!(state.entities_in_order().map(|(id, _)| id).collect::<Vec<_>>(), vec![chip, player]);

	let level = state.to_level_dto();

	assert_eq!(level.required_chips, 0);
	assert_eq!(level.entities[0].kind, chipty::EntityKind::Chip);
	assert_eq!(level.entities[1].kind, chipty::EntityKind::Player);
}

#[test]
fn removing_entity_updates_storage_and_order() {
	let mut state = chipcore::EditState::from_level_dto(&base_level());
	let id = state.entity_order[0];

	let removed = state.remove_entity(id).unwrap();

	assert_eq!(removed.kind, chipty::EntityKind::Player);
	assert!(state.entity(id).is_none());
	assert!(!state.entity_order.contains(&id));
}

#[test]
fn resize_preserves_shifted_contents_and_rejects_invalid_sizes() {
	let mut state = chipcore::EditState::from_level_dto(&base_level());
	state.set_terrain(Vec2i::new(0, 0), chipty::Terrain::Fire);
	let player = state.entity_at(Vec2i::new(0, 0)).unwrap();

	assert!(state.resize(1, 1, 0, 0, chipty::Terrain::Blank));

	assert_eq!(state.width, 4);
	assert_eq!(state.height, 4);
	assert_eq!(state.get_terrain(Vec2i::new(1, 1)), chipty::Terrain::Fire);
	assert_eq!(state.entity(player).unwrap().pos, Vec2i::new(1, 1));
	assert_eq!(state.connections[0], chipty::FieldConn { src: Vec2i::new(2, 1), dest: Vec2i::new(3, 3) });
	assert!(!state.resize(-2, 0, 0, 0, chipty::Terrain::Blank));
}

#[test]
fn brush_create_captures_level_data() {
	let state = chipcore::EditState::from_level_dto(&base_level());

	let brush = state.brush_create();

	assert_eq!(brush.width, state.width);
	assert_eq!(brush.height, state.height);
	assert_eq!(brush.terrain[0], Some(chipty::Terrain::Floor));
	assert_eq!(brush.entities.len(), 2);
	assert_eq!(brush.connections, state.connections);
}

#[test]
fn brush_apply_offsets_internal_connections_only() {
	let mut state = chipcore::EditState {
		width: 5,
		height: 5,
		terrain: vec![chipty::Terrain::Floor; 25],
		..chipcore::EditState::default()
	};
	let brush = chipty::LevelBrush {
		width: 2,
		height: 2,
		terrain: vec![Some(chipty::Terrain::Fire), None, None, Some(chipty::Terrain::Water)],
		entities: vec![chipty::EntityArgs { kind: chipty::EntityKind::Block, pos: Vec2i::new(1, 1), face_dir: None }],
		connections: vec![
			chipty::FieldConn { src: Vec2i::new(0, 0), dest: Vec2i::new(1, 1) },
			chipty::FieldConn { src: Vec2i::new(0, 0), dest: Vec2i::new(4, 4) },
		],
	};

	state.brush_apply(Vec2i::new(2, 2), &brush);

	assert_eq!(state.get_terrain(Vec2i::new(2, 2)), chipty::Terrain::Fire);
	assert_eq!(state.get_terrain(Vec2i::new(3, 3)), chipty::Terrain::Water);
	assert_eq!(state.entity_at(Vec2i::new(3, 3)).and_then(|id| state.entity(id)).unwrap().kind, chipty::EntityKind::Block);
	assert!(state.connections.contains(&chipty::FieldConn { src: Vec2i::new(2, 2), dest: Vec2i::new(3, 3) }));
	assert!(state.connections.contains(&chipty::FieldConn { src: Vec2i::new(2, 2), dest: Vec2i::new(4, 4) }));
}

#[test]
fn toggle_connection_adds_and_removes_exact_connection() {
	let mut state = chipcore::EditState::default();
	let conn = chipty::FieldConn { src: Vec2i::new(0, 0), dest: Vec2i::new(1, 1) };

	state.toggle_connection(conn);
	assert_eq!(state.connections, vec![conn]);

	state.toggle_connection(conn);
	assert!(state.connections.is_empty());
}
