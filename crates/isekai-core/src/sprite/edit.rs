use super::{Animation, Sprite, SpriteSheet};

impl SpriteSheet {
    fn allocate_editor_id(&mut self) -> Result<u64, String> {
        if self.next_id >= i64::MAX as u64 {
            return Err("No sprite or animation IDs remain in this sheet".into());
        }
        Ok(self.allocate_id())
    }
    pub fn add_sprite(&mut self, mut sprite: Sprite, now: u64) -> Result<u64, String> {
        sprite.id = self.allocate_editor_id()?;
        sprite.modified = Some(now);
        let id = sprite.id;
        self.sprites.push(sprite);
        Ok(id)
    }
    pub fn put_sprite(
        &mut self,
        id: u64,
        mut sprite: Sprite,
        now: u64,
        size: (u32, u32),
    ) -> Result<bool, String> {
        if sprite.id != id {
            return Err("sprite id cannot change".into());
        }
        let index = self
            .sprites
            .iter()
            .position(|s| s.id == id)
            .ok_or("sprite no longer exists")?;
        self.validate_sprite_edit(&sprite, size.0, size.1)
            .map_err(|e| e.join("; "))?;
        sprite.modified = self.sprites[index].modified;
        if self.sprites[index] == sprite {
            return Ok(false);
        }
        sprite.modified = Some(now);
        self.sprites[index] = sprite;
        Ok(true)
    }
    pub fn add_animation(&mut self, mut animation: Animation, now: u64) -> Result<u64, String> {
        animation.id = self.allocate_editor_id()?;
        animation.modified = Some(now);
        let id = animation.id;
        self.animations.push(animation);
        Ok(id)
    }
    pub fn put_animation(
        &mut self,
        id: u64,
        mut animation: Animation,
        now: u64,
    ) -> Result<bool, String> {
        if animation.id != id {
            return Err("animation id cannot change".into());
        }
        let index = self
            .animations
            .iter()
            .position(|a| a.id == id)
            .ok_or("animation no longer exists")?;
        self.validate_animation_edit(&animation)
            .map_err(|e| e.join("; "))?;
        animation.modified = self.animations[index].modified;
        if self.animations[index] == animation {
            return Ok(false);
        }
        animation.modified = Some(now);
        self.animations[index] = animation;
        Ok(true)
    }
    pub fn rename_sprite(
        &mut self,
        id: u64,
        name: &str,
        now: u64,
        size: (u32, u32),
    ) -> Result<bool, String> {
        let mut sprite = self
            .sprites
            .iter()
            .find(|s| s.id == id)
            .ok_or("sprite no longer exists")?
            .clone();
        sprite.name = name.into();
        self.put_sprite(id, sprite, now, size)
    }
    pub fn remove_sprite(&mut self, id: u64) -> Result<(), String> {
        let index = self
            .sprites
            .iter()
            .position(|s| s.id == id)
            .ok_or("sprite no longer exists")?;
        let references: Vec<_> = self
            .animations
            .iter()
            .filter(|c| c.frames.iter().any(|f| f.sprite_id == id))
            .map(|c| c.name.as_str())
            .collect();
        if !references.is_empty() {
            return Err(format!(
                "sprite {:?} is used by animations: {}",
                self.sprites[index].name,
                references.join(", ")
            ));
        }
        self.sprites.remove(index);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::format::Shape;
    #[test]
    fn rename_and_remove_preserve_reference_and_name_guards() {
        let (_, mut sheet) = crate::sprite::example();
        let before = sheet.clone();
        assert!(sheet.rename_sprite(1, "b", 1, (64, 16)).is_err());
        assert_eq!(sheet, before, "duplicate names must not change the sheet");
        let error = sheet.remove_sprite(1).unwrap_err();
        assert!(error.contains("cycle"), "{error}");
        assert_eq!(sheet, before, "referenced sprites must not be removed");
        sheet.rename_sprite(1, "renamed", 1, (64, 16)).unwrap();
        assert_eq!(sheet.animations[0].frames[0].sprite_id, 1);
        assert!(sheet.remove_sprite(1).unwrap_err().contains("\"renamed\""));
        assert!(sheet.remove_sprite(999).is_err());
        sheet.rename_sprite(1, "a", 1, (64, 16)).unwrap();
        sheet.sprites[0].modified = None;
        assert_eq!(
            crate::sprite::store::to_json(&sheet),
            crate::sprite::example::EXAMPLE_JSON
        );
        sheet.animations.clear();
        sheet.remove_sprite(1).unwrap();
        assert_eq!(sheet.sprites.len(), 3);
    }
    #[test]
    fn exhausted_id_space_rejects_creation_without_changing_the_sheet() {
        let (_, mut sheet) = crate::sprite::example();
        sheet.next_id = i64::MAX as u64 - 1;
        let sprite = sheet.sprites[0].clone();
        assert_eq!(
            sheet.add_sprite(sprite.clone(), 1).unwrap(),
            i64::MAX as u64 - 1
        );
        assert!(
            sheet.validate().is_err(),
            "duplicate name is unrelated to ID exhaustion"
        );
        sheet.sprites.last_mut().unwrap().name = "last".into();
        sheet.validate().unwrap();
        let before = sheet.clone();
        assert!(sheet.add_sprite(sprite, 2).is_err());
        assert!(sheet.add_animation(sheet.animations[0].clone(), 2).is_err());
        assert_eq!(sheet, before);
        let json = crate::sprite::store::to_json(&sheet);
        assert!(crate::sprite::store::parse(&json).is_ok());
    }
    #[test]
    fn edits_track_real_changes_and_validate_only_the_item() {
        let (_, mut sheet) = crate::sprite::example();
        let sprite = Sprite {
            id: 0,
            props: Default::default(),
            modified: None,
            name: "new".into(),
            shape: Shape::Set {
                canvas: [1, 1],
                parts: vec![crate::sprite::format::Part::default()],
            },
            pivot: [8.0, 16.0],
        };
        let id = sheet.add_sprite(sprite, 10).unwrap();
        assert_eq!(id, 6);
        assert_eq!(sheet.next_id, 7);
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(10));
        let animation = crate::sprite::Animation {
            id: 0,
            name: "empty".into(),
            mode: crate::sprite::Mode::Loop,
            tags: vec![],
            props: Default::default(),
            frames: vec![],
            modified: None,
        };
        let aid = sheet.add_animation(animation, 11).unwrap();
        assert_eq!(aid, 7);
        assert_eq!(sheet.animations.last().unwrap().modified, Some(11));
        let old_anim = sheet.animations[0].clone();
        let mut sprite = sheet.sprites.iter().find(|s| s.id == id).unwrap().clone();
        sprite.modified = Some(999);
        assert!(!sheet.put_sprite(id, sprite.clone(), 20, (64, 16)).unwrap());
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(10));
        sprite.pivot = [2.0, 3.0];
        sprite.props.insert("key".into(), "value".into());
        assert!(sheet.put_sprite(id, sprite.clone(), 21, (64, 16)).unwrap());
        assert_eq!(sheet.sprites.last().unwrap().modified, Some(21));
        assert!(sheet.rename_sprite(id, "renamed", 22, (64, 16)).unwrap());
        assert_eq!(sheet.animations[0], old_anim);
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.id = 1;
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.name = "a".into();
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut bad = sheet.sprites.last().unwrap().clone();
        bad.shape = Shape::Rect {
            col: 99,
            row: 0,
            cols: 1,
            rows: 1,
        };
        assert!(sheet.put_sprite(id, bad, 23, (64, 16)).is_err());
        let mut animation = sheet.animations.last().unwrap().clone();
        assert!(!sheet.put_animation(aid, animation.clone(), 30).unwrap());
        animation
            .frames
            .push(crate::sprite::format::AnimationFrame {
                sprite_id: id,
                ms: 100,
            });
        animation.tags.push("walk".into());
        assert!(sheet.put_animation(aid, animation.clone(), 31).unwrap());
        assert_eq!(sheet.animations.last().unwrap().modified, Some(31));
        animation.frames[0].ms = 0;
        assert!(sheet.put_animation(aid, animation.clone(), 32).is_err());
        animation.frames[0].ms = 100;
        animation.frames[0].sprite_id = 999;
        assert!(sheet.put_animation(aid, animation, 32).is_err());
        // Saving, reading and saving again keeps ids, next_id, modified and the bytes.
        let saved = crate::sprite::store::to_json(&sheet);
        let read = crate::sprite::store::parse(&saved).unwrap();
        assert_eq!(read, sheet);
        assert_eq!(crate::sprite::store::to_json(&read), saved);
    }
}
