use super::*;

#[derive(Debug)]
pub struct HiddenData {
	/// Whether the entity should be hidden on dirt terrain.
	pub dirt: bool,
}

pub(super) fn update_hidden_flag(s: &mut GameState, ent: &mut Entity) {
	let mut hidden = false;
	for ehandle in s.qt.get(ent.pos) {
		let Some(other) = s.ents.get(ehandle) else { continue };
		if matches!(other.kind, EntityKind::Block | EntityKind::IceBlock) {
			hidden = true;
			break;
		}
	}

	// Hide entities on wall-like terrain
	let terrain = s.field.get_terrain(ent.pos);
	if matches!(terrain,
		| Terrain::Wall | Terrain::BlueLock | Terrain::RedLock | Terrain::GreenLock | Terrain::YellowLock
		| Terrain::DirtBlock | Terrain::ToggleWall | Terrain::HiddenWall | Terrain::InvisibleWall
		| Terrain::RealBlueWall | Terrain::FakeBlueWall
	) {
		hidden = true;
	}

	if ent.data.hidden.dirt && matches!(terrain, Terrain::Dirt) {
		hidden = true;
	}

	let hidden_changed = (ent.flags & EF_HIDDEN != 0) != hidden;

	if hidden {
		ent.flags |= EF_HIDDEN;
	}
	else {
		ent.flags &= !EF_HIDDEN;
	}

	if hidden_changed {
		s.events.fire(GameEvent::EntityHidden { entity: ent.handle, hidden });
	}
}

pub(super) fn update_hidden_fire(s: &mut GameState, pos: Vec2i, hidden: bool) {
	let terrain = s.field.get_terrain(pos);
	if matches!(terrain, Terrain::Fire) {
		s.events.fire(GameEvent::FireHidden { pos, hidden });
	}
}
