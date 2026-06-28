use super::*;

const BASE_SPD: i32 = 12;

mod blob;
mod block;
mod bomb;
mod bug;
mod fireball;
mod glider;
mod iceblock;
mod paramecium;
mod pickup;
mod player;
mod playernpc;
mod pinkball;
mod socket;
mod tank;
mod teeth;
mod thief;
mod walker;

pub use chipty::EntityArgs;

impl GameState {
	/// Creates an entity from arguments.
	pub fn entity_create(&mut self, data: &EntityArgs) -> EntityHandle {
		// Don't create entities outside the field
		if data.pos.x < 0 || data.pos.x >= self.field.width || data.pos.y < 0 || data.pos.y >= self.field.height {
			return EntityHandle::INVALID;
		}

		let s = self;
		let create_fn = match data.kind {
			EntityKind::Player => player::create,
			EntityKind::PlayerNPC => playernpc::create,
			EntityKind::Chip => pickup::create,
			EntityKind::Socket => socket::create,
			EntityKind::Block => block::create,
			EntityKind::IceBlock => iceblock::create,
			EntityKind::Flippers => pickup::create,
			EntityKind::FireBoots => pickup::create,
			EntityKind::IceSkates => pickup::create,
			EntityKind::SuctionBoots => pickup::create,
			EntityKind::BlueKey => pickup::create,
			EntityKind::RedKey => pickup::create,
			EntityKind::GreenKey => pickup::create,
			EntityKind::YellowKey => pickup::create,
			EntityKind::Thief => thief::create,
			EntityKind::Bomb => bomb::create,
			EntityKind::Bug => bug::create,
			EntityKind::FireBall => fireball::create,
			EntityKind::PinkBall => pinkball::create,
			EntityKind::Tank => tank::create,
			EntityKind::Glider => glider::create,
			EntityKind::Teeth => teeth::create,
			EntityKind::Walker => walker::create,
			EntityKind::Blob => blob::create,
			EntityKind::Paramecium => paramecium::create,
		};
		let ehandle = create_fn(s, data);
		s.events.fire(GameEvent::EntityCreated { entity: ehandle, kind: data.kind });

		return ehandle;
	}

	/// Removes an entity from the game.
	pub fn entity_remove(&mut self, ehandle: EntityHandle) -> Option<EntityArgs> {
		let s = self;
		let ent = s.ents.remove(ehandle)?;
		s.qt.remove(ehandle, ent.pos);
		s.events.fire(GameEvent::EntityRemoved { entity: ehandle, kind: ent.kind });
		Some(ent.to_entity_args())
	}
}
