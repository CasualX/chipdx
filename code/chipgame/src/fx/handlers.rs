use super::*;

fn ent_pos(kind: chipty::EntityKind, pos: Vec2i) -> Vec3f {
	// Give sprites an offset based on their priority
	let z = match kind {
		chipty::EntityKind::Socket => 2.0,
		chipty::EntityKind::Thief => 1.0,
		_ => 0.0,
	};
	Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, z)
}

pub fn entity_created(fx: &mut FxState, ehandle: chipcore::EntityHandle, kind: chipty::EntityKind) {
	let Some(ent) = fx.game.ents.get(ehandle) else { return };

	let pos = ent_pos(ent.kind, ent.pos);
	let sprite = sprite_for_ent(ent, &fx.game);
	let model = if pos.z >= 20.0 { chipty::ModelId::FloorSprite } else { model_for_kind(ent.kind) };
	let mut obj = render::Object {
		data: render::ObjectData {
			pos,
			sprite,
			frame: 0,
			model,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};
	if matches!(kind, chipty::EntityKind::Bomb) {
		obj.data.sprite = chipty::SpriteId::BombA;
		obj.anim.anims.push(render::AnimState::AnimLoop(render::SpriteAnimLoop {
			start_time: fx.time + fx.random.next_f64() * 10.0,
			frame_rate: 16.0,
		}));
	}

	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.game_objects.insert(ehandle, handle);

	if let Some(follow) = fx.camera.controller.follow_entity_mut() {
		if follow.master == ehandle {
			follow.move_src = ent.pos;
			follow.move_dest = ent.pos;
			follow.move_time = fx.time;
			follow.move_spd = ent.base_spd as f32 / chipcore::FPS as f32;
		}
	}
}

fn template_pos(kind: chipty::EntityKind, pos: Vec2i) -> Vec3f {
	let z = match kind {
		chipty::EntityKind::Block | chipty::EntityKind::IceBlock => 0.0,
		_ => 20.0,
	};
	let pos = Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, z);
	return pos;
}

pub fn create_clone_template(fx: &mut FxState, args: chipty::EntityArgs) {
	let pos = template_pos(args.kind, args.pos);
	let sprite = sprite_for_args(args, &fx.game);
	let model = if pos.z >= 20.0 { chipty::ModelId::FloorSprite } else { model_for_kind(args.kind) };
	let obj = render::Object {
		data: render::ObjectData {
			pos,
			sprite,
			frame: 0,
			model,
			alpha: 1.0,
			greyscale: true,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};

	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
}

pub fn entity_removed(fx: &mut FxState, ehandle: chipcore::EntityHandle, kind: chipty::EntityKind) {
	entity_hidden(fx, ehandle, false);

	let Some(obj_handle) = fx.game_objects.remove(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	// Object rises, fades and is removed
	let rises = matches!(kind, chipty::EntityKind::Chip
		| chipty::EntityKind::BlueKey | chipty::EntityKind::RedKey | chipty::EntityKind::GreenKey | chipty::EntityKind::YellowKey
		| chipty::EntityKind::Flippers | chipty::EntityKind::FireBoots | chipty::EntityKind::IceSkates | chipty::EntityKind::SuctionBoots);

	// Object fades and is removed
	let faded = matches!(kind, chipty::EntityKind::Socket);

	if rises {
		obj.data.alpha = 1.0;
		obj.anim.anims.push(render::AnimState::FadeOut(render::FadeOut { atime: 0.0 }));
		obj.anim.anims.push(render::AnimState::MoveZ(render::MoveZ { target_z: 50.0, move_spd: 200.0 }));
	}
	else if faded {
		obj.anim.anims.push(render::AnimState::FadeOut(render::FadeOut { atime: 0.0 }));
	}
	else if matches!(kind, chipty::EntityKind::Bomb) {
		obj.anim.anims.clear();
	}
	obj.anim.unalive_after_anim = true;
}

pub fn entity_step(fx: &mut FxState, ehandle: chipcore::EntityHandle) {
	let Some(&obj_handle) = fx.game_objects.get(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };
	let Some(ent) = fx.game.ents.get(ehandle) else { return };

	let src = ent.pos - match ent.step_dir { Some(step_dir) => step_dir.to_vec(), None => Vec2::ZERO };

	let start_pos = ent_pos(ent.kind, src);
	let end_pos = ent_pos(ent.kind, ent.pos);
	obj.data.pos = start_pos;

	obj.data.sprite = animated_sprite_for_ent(ent, &fx.game);
	obj.data.frame = 0;

	// Ensure the previous step animation is cleared...
	// See [MoveStep::animate] setting obj.pos when the animation completes.
	obj.anim.anims.clear();

	let jump_height = match ent.kind {
		chipty::EntityKind::PinkBall => 6.0,
		chipty::EntityKind::Walker => 6.0,
		_ => 0.0,
	};

	obj.anim.anims.push(render::AnimState::MoveStep(render::MoveStep {
		start_pos,
		end_pos,
		move_time: fx.time,
		duration: ent.step_spd as f32 / chipcore::FPS as f32,
		jump_height,
		sprite_after: None,
	}));
	obj.anim.anims.push(render::AnimState::AnimSeq(render::SpriteAnimSeq {
		start_time: fx.time,
		frame_count: 4,
		frame_rate: 16.0,
	}));

	// Quick hack to flatten sprites on top of walls
	// obj.data.model = if end_pos.z >= 20.0 { chipty::ModelId::FloorSprite } else { model_for_kind(ent.kind) };

	if let Some(follow) = fx.camera.controller.follow_entity_mut() {
		if follow.master == ehandle {
			follow.move_src = src;
			follow.move_dest = ent.pos;
			follow.move_time = fx.time;
			follow.move_spd = ent.step_spd as f32 / chipcore::FPS as f32;
		}
	}
}

pub fn entity_teleport(fx: &mut FxState, ehandle: chipcore::EntityHandle) {
	// Step out of the teleport
	entity_step(fx, ehandle);
}

pub fn entity_face_dir(fx: &mut FxState, ehandle: chipcore::EntityHandle) {
	let Some(&obj_handle) = fx.game_objects.get(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };
	let Some(ent) = fx.game.ents.get(ehandle) else { return };

	obj.data.sprite = sprite_for_ent(ent, &fx.game);
}

pub fn player_game_over(fx: &mut FxState, ehandle: chipcore::EntityHandle, reason: chipcore::GameOverReason) {
	let Some(&obj_handle) = fx.game_objects.get(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };
	let Some(ent) = fx.game.ents.get(ehandle) else { return };

	// Update the player sprite
	let sprite = match reason {
		chipcore::GameOverReason::LevelComplete => chipty::SpriteId::PlayerCheer,
		chipcore::GameOverReason::Drowned => chipty::SpriteId::WaterSplash,
		chipcore::GameOverReason::Burned => chipty::SpriteId::PlayerBurned,
		chipcore::GameOverReason::Bombed => chipty::SpriteId::PlayerBurned,
		chipcore::GameOverReason::Collided => chipty::SpriteId::PlayerBurned,
		chipcore::GameOverReason::Eaten => chipty::SpriteId::PlayerBurned,
		chipcore::GameOverReason::TimeOut => chipty::SpriteId::PlayerBurned,
		chipcore::GameOverReason::NotOkay => chipty::SpriteId::PlayerBurned,
	};

	// If Step animation is playing, set it as the sprite update field
	if let Some(move_step) = obj.anim.anims.iter_mut().find_map(|anim| match anim { render::AnimState::MoveStep(move_step) => Some(move_step), _ => None }) {
		move_step.sprite_after = Some(sprite);
	}
	else {
		obj.data.sprite = sprite;
	}

	if matches!(reason, chipcore::GameOverReason::LevelComplete) {
		handlers::effect(fx, ent.pos, render::EffectType::Fireworks);
	}
}

pub fn player_activity(fx: &mut FxState, ehandle: chipcore::EntityHandle) {
	// PlayerActivity is fired after PlayerGameOver in case of drowning and burning...
	if !fx.game.is_game_over() {
		entity_face_dir(fx, ehandle);
	}
}

pub fn bomb_explode(fx: &mut FxState, pos: Vec2i) {
	effect(fx, pos, render::EffectType::Sparkles);

	let bomb_center = Vec3::new(pos.x as f32 * 32.0 + 16.0, pos.y as f32 * 32.0 + 16.0, 0.0);
	fx.camera.add_shake_at(bomb_center, 50.0, 1.0);
}

pub fn player_push(fx: &mut FxState, ehandle: chipcore::EntityHandle) {
	let Some(&obj_handle) = fx.game_objects.get(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };
	let Some(ent) = fx.game.ents.get(ehandle) else { return };

	if !matches!(ent.kind, chipty::EntityKind::Player) {
		return;
	}

	// No pushing animation in water
	let terrain = fx.game.field.get_terrain(ent.pos);
	if matches!(terrain, chipty::Terrain::Water) {
		return;
	}

	obj.data.sprite = match ent.face_dir.or(ent.step_dir) {
		Some(chipty::Compass::Up) => chipty::SpriteId::PlayerPushN,
		Some(chipty::Compass::Down) => chipty::SpriteId::PlayerPushS,
		Some(chipty::Compass::Left) => chipty::SpriteId::PlayerPushW,
		Some(chipty::Compass::Right) => chipty::SpriteId::PlayerPushE,
		_ => chipty::SpriteId::PlayerWalkIdle,
	};
}

pub fn entity_hidden(fx: &mut FxState, ehandle: chipcore::EntityHandle, hidden: bool) {
	let Some(&obj_handle) = fx.game_objects.get(&ehandle) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	if hidden {
		fx.hidden_objects.insert(ehandle, obj_handle);
		// Instantly on game start, gradually otherwise
		if fx.game.time <= 1 {
			obj.data.alpha = 0.0;
		}
	}
	else {
		fx.hidden_objects.remove(&ehandle);
		obj.data.alpha = 1.0;
		obj.anim.remove_fade_anim();
	}

	obj.data.depth_test = !hidden;
}

pub fn terrain_updated(fx: &mut FxState, pos: Vec2i, old: chipty::Terrain, new: chipty::Terrain) {
	fx.render.field.set_terrain(pos, new);
	match (old, new) {
		(chipty::Terrain::ToggleFloor, chipty::Terrain::ToggleWall) => handlers::toggle_wall(fx, pos),
		(chipty::Terrain::ToggleWall, chipty::Terrain::ToggleFloor) => handlers::toggle_wall(fx, pos),
		(chipty::Terrain::ToggleFloor, _) => handlers::remove_toggle_wall(fx, pos),
		(chipty::Terrain::ToggleWall, _) => handlers::remove_toggle_wall(fx, pos),
		(_, chipty::Terrain::ToggleFloor) => handlers::create_toggle_wall(fx, pos, false),
		(_, chipty::Terrain::ToggleWall) => handlers::create_toggle_wall(fx, pos, true),
		(chipty::Terrain::Fire, _) => handlers::remove_fire(fx, pos),
		(_, chipty::Terrain::Fire) => handlers::create_fire(fx, pos),
		(chipty::Terrain::RecessedWall, chipty::Terrain::Wall) => handlers::recessed_wall_raised(fx, pos),

		(chipty::Terrain::InvisibleWall, chipty::Terrain::HiddenWall) => {},
		(chipty::Terrain::HiddenWall, chipty::Terrain::InvisibleWall) => {},
		(chipty::Terrain::InvisibleWall | chipty::Terrain::HiddenWall, _) => handlers::remove_wall_mirage(fx, pos),
		(_, chipty::Terrain::InvisibleWall | chipty::Terrain::HiddenWall) => handlers::create_wall_mirage(fx, pos),

		(chipty::Terrain::WaterHazard, _) => handlers::remove_water_hazard(fx, pos),
		(_, chipty::Terrain::WaterHazard) => handlers::create_water_hazard(fx, pos),

		(_, chipty::Terrain::FakeBlueWall) => handlers::create_fake_blue_wall(fx, pos),
		(chipty::Terrain::FakeBlueWall, _) => handlers::remove_fake_blue_wall(fx, pos),

		(_, chipty::Terrain::BearTrap) => handlers::create_bear_trap(fx, pos),
		(chipty::Terrain::BearTrap, _) => handlers::remove_bear_trap(fx, pos),
		_ => {}
	}
}

pub fn fire_hidden(fx: &mut FxState, pos: Vec2i, hidden: bool) {
	let Some(&obj_handle) = fx.fire_sprites.get(&pos) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	if hidden {
		fx.hidden_fire.insert(pos, obj_handle);
		// Instantly on game start, gradually otherwise
		if fx.game.time <= 1 {
			obj.data.alpha = 0.0;
		}
	}
	else {
		fx.hidden_fire.remove(&pos);
		obj.data.alpha = 1.0;
	}

	obj.data.depth_test = !hidden;
}
pub fn create_fire(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0 - 2.0, 0.0), // Make fire appear below other sprites
			sprite: chipty::SpriteId::FireA,
			frame: 0,
			model: chipty::ModelId::Sprite,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: vec![render::AnimState::AnimLoop(render::SpriteAnimLoop {
				start_time: fx.time + fx.random.next_f64() * 10.0,
				frame_rate: 8.0,
			})],
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.fire_sprites.insert(pos, handle);
}
pub fn remove_fire(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(obj_handle) = fx.fire_sprites.remove(&pos) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	obj.anim.anims.push(render::AnimState::FadeOut(render::FadeOut { atime: 0.0 }));
	obj.anim.unalive_after_anim = true;
}

fn model_for_kind(kind: chipty::EntityKind) -> chipty::ModelId {
	match kind {
		chipty::EntityKind::Block => chipty::ModelId::Wall,
		chipty::EntityKind::IceBlock => chipty::ModelId::Wall,
		chipty::EntityKind::Tank => chipty::ModelId::Tank,
		chipty::EntityKind::Bug => chipty::ModelId::FlatSprite,
		chipty::EntityKind::Blob => chipty::ModelId::ReallyFlatSprite,
		chipty::EntityKind::Paramecium => chipty::ModelId::ReallyFlatSprite,
		_ => chipty::ModelId::Sprite,
	}
}

fn sprite_for_player(face_dir: Option<chipty::Compass>, terrain: chipty::Terrain) -> chipty::SpriteId {
	if matches!(terrain, chipty::Terrain::Water) {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerSwimN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerSwimS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerSwimW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerSwimE,
			_ => chipty::SpriteId::PlayerSwimIdle,
		}
	}
	else {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerWalkN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerWalkS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerWalkW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerWalkE,
			_ => chipty::SpriteId::PlayerWalkIdle,
		}
	}
}

fn sprite_for_playernpc(face_dir: Option<chipty::Compass>, terrain: chipty::Terrain) -> chipty::SpriteId {
	if matches!(terrain, chipty::Terrain::Water) {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerSwimN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerSwimS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerSwimW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerSwimE,
			_ => chipty::SpriteId::PlayerSwimIdle,
		}
	}
	else if matches!(terrain, chipty::Terrain::Fire) {
		chipty::SpriteId::PlayerBurned
	}
	else {
		match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::PlayerWalkN,
			Some(chipty::Compass::Down) => chipty::SpriteId::PlayerWalkS,
			Some(chipty::Compass::Left) => chipty::SpriteId::PlayerWalkW,
			Some(chipty::Compass::Right) => chipty::SpriteId::PlayerWalkE,
			_ => chipty::SpriteId::PlayerBurned,
		}
	}
}

fn animated_sprite_for_ent(ent: &chipcore::Entity, game: &chipcore::GameState) -> chipty::SpriteId {
	match ent.kind {
		chipty::EntityKind::Player => sprite_for_player(ent.face_dir, game.field.get_terrain(ent.pos)),
		chipty::EntityKind::PlayerNPC => sprite_for_playernpc(ent.face_dir, game.field.get_terrain(ent.pos)),
		chipty::EntityKind::Chip => chipty::SpriteId::Chip,
		chipty::EntityKind::Socket => chipty::SpriteId::Socket,
		chipty::EntityKind::Block => chipty::SpriteId::DirtBlock,
		chipty::EntityKind::IceBlock => chipty::SpriteId::IceBlock,
		chipty::EntityKind::Flippers => chipty::SpriteId::Flippers,
		chipty::EntityKind::FireBoots => chipty::SpriteId::FireBoots,
		chipty::EntityKind::IceSkates => chipty::SpriteId::IceSkates,
		chipty::EntityKind::SuctionBoots => chipty::SpriteId::SuctionBoots,
		chipty::EntityKind::BlueKey => chipty::SpriteId::BlueKey,
		chipty::EntityKind::RedKey => chipty::SpriteId::RedKey,
		chipty::EntityKind::GreenKey => chipty::SpriteId::GreenKey,
		chipty::EntityKind::YellowKey => chipty::SpriteId::YellowKey,
		chipty::EntityKind::Thief => chipty::SpriteId::Thief,
		chipty::EntityKind::Bug => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::BugNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::BugSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::BugWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::BugEA,
			_ => chipty::SpriteId::BugN,
		},
		chipty::EntityKind::Tank => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::TankN,
			Some(chipty::Compass::Down) => chipty::SpriteId::TankS,
			Some(chipty::Compass::Left) => chipty::SpriteId::TankW,
			Some(chipty::Compass::Right) => chipty::SpriteId::TankE,
			_ => chipty::SpriteId::TankN,
		},
		chipty::EntityKind::PinkBall => chipty::SpriteId::PinkBall,
		chipty::EntityKind::FireBall => chipty::SpriteId::FireballA,
		chipty::EntityKind::Glider => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::GliderNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::GliderSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::GliderWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::GliderEA,
			_ => chipty::SpriteId::GliderN,
		},
		chipty::EntityKind::Walker => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::WalkerNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::WalkerSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::WalkerWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::WalkerEA,
			_ => chipty::SpriteId::WalkerN,
		},
		chipty::EntityKind::Teeth => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::TeethNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::TeethSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::TeethWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::TeethEA,
			_ => chipty::SpriteId::TeethN,
		},
		chipty::EntityKind::Blob => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::BlobNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::BlobSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::BlobWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::BlobEA,
			_ => chipty::SpriteId::Blob,
		},
		chipty::EntityKind::Paramecium => match ent.face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::ParameciumNA,
			Some(chipty::Compass::Down) => chipty::SpriteId::ParameciumSA,
			Some(chipty::Compass::Left) => chipty::SpriteId::ParameciumWA,
			Some(chipty::Compass::Right) => chipty::SpriteId::ParameciumEA,
			_ => chipty::SpriteId::ParameciumN,
		},
		chipty::EntityKind::Bomb => chipty::SpriteId::BombA,
	}
}

fn sprite_for_ent(ent: &chipcore::Entity, game: &chipcore::GameState) -> chipty::SpriteId {
	sprite_for_parts(ent.kind, ent.face_dir, game.field.get_terrain(ent.pos))
}

fn sprite_for_args(args: chipty::EntityArgs, game: &chipcore::GameState) -> chipty::SpriteId {
	sprite_for_parts(args.kind, args.face_dir, game.field.get_terrain(args.pos))
}

fn sprite_for_parts(kind: chipty::EntityKind, face_dir: Option<chipty::Compass>, terrain: chipty::Terrain) -> chipty::SpriteId {
	match kind {
		chipty::EntityKind::Player => sprite_for_player(face_dir, terrain),
		chipty::EntityKind::PlayerNPC => sprite_for_playernpc(face_dir, terrain),
		chipty::EntityKind::Chip => chipty::SpriteId::Chip,
		chipty::EntityKind::Socket => chipty::SpriteId::Socket,
		chipty::EntityKind::Block => chipty::SpriteId::DirtBlock,
		chipty::EntityKind::IceBlock => chipty::SpriteId::IceBlock,
		chipty::EntityKind::Flippers => chipty::SpriteId::Flippers,
		chipty::EntityKind::FireBoots => chipty::SpriteId::FireBoots,
		chipty::EntityKind::IceSkates => chipty::SpriteId::IceSkates,
		chipty::EntityKind::SuctionBoots => chipty::SpriteId::SuctionBoots,
		chipty::EntityKind::BlueKey => chipty::SpriteId::BlueKey,
		chipty::EntityKind::RedKey => chipty::SpriteId::RedKey,
		chipty::EntityKind::GreenKey => chipty::SpriteId::GreenKey,
		chipty::EntityKind::YellowKey => chipty::SpriteId::YellowKey,
		chipty::EntityKind::Thief => chipty::SpriteId::Thief,
		chipty::EntityKind::Bug => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::BugN,
			Some(chipty::Compass::Down) => chipty::SpriteId::BugS,
			Some(chipty::Compass::Left) => chipty::SpriteId::BugW,
			Some(chipty::Compass::Right) => chipty::SpriteId::BugE,
			_ => chipty::SpriteId::BugN,
		},
		chipty::EntityKind::Tank => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::TankN,
			Some(chipty::Compass::Down) => chipty::SpriteId::TankS,
			Some(chipty::Compass::Left) => chipty::SpriteId::TankW,
			Some(chipty::Compass::Right) => chipty::SpriteId::TankE,
			_ => chipty::SpriteId::TankN,
		},
		chipty::EntityKind::PinkBall => chipty::SpriteId::PinkBall,
		chipty::EntityKind::FireBall => chipty::SpriteId::Fireball,
		chipty::EntityKind::Glider => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::GliderN,
			Some(chipty::Compass::Down) => chipty::SpriteId::GliderS,
			Some(chipty::Compass::Left) => chipty::SpriteId::GliderW,
			Some(chipty::Compass::Right) => chipty::SpriteId::GliderE,
			_ => chipty::SpriteId::GliderN,
		},
		chipty::EntityKind::Walker => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::WalkerN,
			Some(chipty::Compass::Down) => chipty::SpriteId::WalkerS,
			Some(chipty::Compass::Left) => chipty::SpriteId::WalkerW,
			Some(chipty::Compass::Right) => chipty::SpriteId::WalkerE,
			_ => chipty::SpriteId::WalkerN,
		},
		chipty::EntityKind::Teeth => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::TeethN,
			Some(chipty::Compass::Down) => chipty::SpriteId::TeethS,
			Some(chipty::Compass::Left) => chipty::SpriteId::TeethW,
			Some(chipty::Compass::Right) => chipty::SpriteId::TeethE,
			_ => chipty::SpriteId::TeethN,
		},
		chipty::EntityKind::Blob => chipty::SpriteId::Blob,
		chipty::EntityKind::Paramecium => match face_dir {
			Some(chipty::Compass::Up) => chipty::SpriteId::ParameciumN,
			Some(chipty::Compass::Down) => chipty::SpriteId::ParameciumS,
			Some(chipty::Compass::Left) => chipty::SpriteId::ParameciumW,
			Some(chipty::Compass::Right) => chipty::SpriteId::ParameciumE,
			_ => chipty::SpriteId::ParameciumN,
		}
		chipty::EntityKind::Bomb => chipty::SpriteId::Bomb,
	}
}

pub fn lock_opened(fx: &mut FxState, pos: Vec2<i32>, key: chipcore::KeyColor) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: match key {
				chipcore::KeyColor::Red => chipty::SpriteId::RedLock,
				chipcore::KeyColor::Green => chipty::SpriteId::GreenLock,
				chipcore::KeyColor::Blue => chipty::SpriteId::BlueLock,
				chipcore::KeyColor::Yellow => chipty::SpriteId::YellowLock,
			},
			frame: 0,
			model: chipty::ModelId::Wall,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: vec![render::AnimState::MoveZ(render::MoveZ {
				target_z: -21.0,
				move_spd: 200.0,
			})],
			unalive_after_anim: true,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);

	// Visual feedback: the key was consumed to open the lock.
	let key_obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: match key {
				chipcore::KeyColor::Red => chipty::SpriteId::RedKey,
				chipcore::KeyColor::Green => chipty::SpriteId::GreenKey,
				chipcore::KeyColor::Blue => chipty::SpriteId::BlueKey,
				chipcore::KeyColor::Yellow => chipty::SpriteId::YellowKey,
			},
			frame: 0,
			model: chipty::ModelId::Sprite,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: vec![
				render::AnimState::FadeOut(render::FadeOut { atime: 0.0 }),
				render::AnimState::MoveZ(render::MoveZ {
					target_z: 50.0,
					move_spd: 200.0,
				}),
			],
			unalive_after_anim: true,
		},
	};
	let key_handle = fx.render.objects.alloc();
	fx.render.objects.insert(key_handle, key_obj);
}

pub fn items_thief(fx: &mut FxState, boots: chipcore::Boots) {
	if boots == chipcore::Boots::NONE {
		return;
	}

	let Some(player) = fx.game.ents.get(fx.game.ps.master) else {
		return;
	};
	let pos = player.pos;
	let base = Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 10.0);

	let mut sprites: Vec<chipty::SpriteId> = Vec::new();
	if boots.has(chipcore::Boots::FLIPPERS) {
		sprites.push(chipty::SpriteId::Flippers);
	}
	if boots.has(chipcore::Boots::FIRE_BOOTS) {
		sprites.push(chipty::SpriteId::FireBoots);
	}
	if boots.has(chipcore::Boots::ICE_SKATES) {
		sprites.push(chipty::SpriteId::IceSkates);
	}
	if boots.has(chipcore::Boots::SUCTION_BOOTS) {
		sprites.push(chipty::SpriteId::SuctionBoots);
	}

	for (i, sprite) in sprites.into_iter().enumerate() {
		let x_off = 6.0 * i as f32;
		let y_off = -2.0 * i as f32;
		let obj = render::Object {
			data: render::ObjectData {
				pos: base + Vec3::new(x_off, y_off, 0.0),
				sprite,
				frame: 0,
				model: chipty::ModelId::Sprite,
				alpha: 1.0,
				greyscale: false,
				depth_test: true,
			},
			anim: render::Animation {
				anims: vec![
					render::AnimState::FadeOut(render::FadeOut { atime: 0.0 }),
					render::AnimState::MoveZ(render::MoveZ {
						target_z: base.z + 50.0,
						move_spd: 200.0,
					}),
				],
				unalive_after_anim: true,
			},
		};
		let handle = fx.render.objects.alloc();
		fx.render.objects.insert(handle, obj);
	}
}

pub fn recessed_wall_raised(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, -20.0),
			sprite: chipty::SpriteId::Wall,
			frame: 0,
			model: chipty::ModelId::Wall,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: vec![render::AnimState::MoveZ(render::MoveZ {
				target_z: 0.0,
				move_spd: 200.0,
			})],
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);

	// Keep the terrain as RecessedWall so that the wall object is drawn on top
	fx.render.field.set_terrain(pos, chipty::Terrain::RecessedWall);
}

pub fn create_toggle_wall(fx: &mut FxState, pos: Vec2<i32>, raised: bool) {
	let z = if raised { 0.0 } else { -21.0 };
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, z),
			sprite: chipty::SpriteId::Wall,
			frame: 0,
			model: chipty::ModelId::ToggleWall,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: vec![
				render::AnimState::MoveZ(render::MoveZ {
					target_z: if raised { 0.0 } else { -21.0 },
					move_spd: 200.0,
				}),
			],
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.toggle_walls.insert(pos, handle);
}
pub fn remove_toggle_wall(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(obj_handle) = fx.toggle_walls.remove(&pos) else { return };
	fx.render.objects.remove(obj_handle);
}
pub fn toggle_wall(fx: &mut FxState, pos: Vec2i) {
	let Some(&obj_handle) = fx.toggle_walls.get(&pos) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	let terrain = fx.game.field.get_terrain(pos);
	obj.anim.anims.clear();
	obj.anim.anims.push(render::AnimState::MoveZ(render::MoveZ {
		target_z: if matches!(terrain, chipty::Terrain::ToggleWall) { 0.0 } else { -21.0 },
		move_spd: 200.0,
	}));
}

pub fn create_wall_mirage(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: chipty::SpriteId::Wall,
			frame: 0,
			model: chipty::ModelId::Wall,
			alpha: 0.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.mirage_walls.insert(pos, handle);
}
pub fn remove_wall_mirage(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(obj_handle) = fx.mirage_walls.remove(&pos) else { return };

	fx.render.objects.remove(obj_handle);
}

pub fn create_water_hazard(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: chipty::SpriteId::WaterSplash,
			frame: 0,
			model: chipty::ModelId::Sprite,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.water_hazards.insert(pos, handle);
	fx.render.field.set_terrain(pos, chipty::Terrain::Water);
}
pub fn remove_water_hazard(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(obj_handle) = fx.water_hazards.remove(&pos) else { return };

	fx.render.objects.remove(obj_handle);
}

pub fn create_fake_blue_wall(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: fx.render.tiles[chipty::Terrain::FakeBlueWall as usize].sprite,
			frame: 0,
			model: chipty::ModelId::Wall,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.fake_blue_walls.insert(pos, handle);
	fx.render.field.set_terrain(pos, chipty::Terrain::Floor);
}
pub fn remove_fake_blue_wall(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(obj_handle) = fx.fake_blue_walls.remove(&pos) else { return };
	let Some(obj) = fx.render.objects.get_mut(obj_handle) else { return };

	// Walls are rendered first and Sprites just disabled depth testing
	// So disable depth testing for the wall while fading out to ensure sprites get rendered
	obj.data.depth_test = false;

	obj.anim.anims.push(render::AnimState::FadeTo(render::FadeTo { target_alpha: 0.0, fade_spd: 8.0 }));
	obj.anim.unalive_after_anim = true;
}

pub fn create_bear_trap(fx: &mut FxState, pos: Vec2<i32>) {
	let obj = render::Object {
		data: render::ObjectData {
			pos: Vec3::new(pos.x as f32 * 32.0, pos.y as f32 * 32.0, 0.0),
			sprite: chipty::SpriteId::BearTrap,
			frame: 0,
			model: chipty::ModelId::Floor,
			alpha: 1.0,
			greyscale: false,
			depth_test: true,
		},
		anim: render::Animation {
			anims: Vec::new(),
			unalive_after_anim: false,
		},
	};
	let handle = fx.render.objects.alloc();
	fx.render.objects.insert(handle, obj);
	fx.traps.insert(pos, BearTrapState { state: chipcore::TrapState::Active, handle });
	fx.render.field.set_terrain(pos, chipty::Terrain::Floor);
}
pub fn remove_bear_trap(fx: &mut FxState, pos: Vec2<i32>) {
	let Some(trap) = fx.traps.remove(&pos) else { return };
	fx.render.objects.remove(trap.handle);
}

pub fn game_over(fx: &mut FxState, reason: chipcore::GameOverReason) {
	fx.game_realtime = (fx.time - fx.game_start_time) as f32;
	fx.next_level_load = fx.time + 2.0;
	fx.game_over = Some(reason);
}

pub fn effect(fx: &mut FxState, pos: Vec2i, ty: render::EffectType) {
	let pos = Vec3::new(pos.x as f32 * 32.0 + 16.0, pos.y as f32 * 32.0 + 16.0, 10.0);
	let start = fx.time;
	fx.render.effects.push(render::Effect { ty, pos, start });
}
