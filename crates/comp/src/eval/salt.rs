//! Expression salts: what keeps `wiggle` on one property from moving in lockstep with `wiggle` on another.
//!
//! A salt is a hash of the property's path (`"rotation.0"`, `"light.intensity"`), not a number somebody picked. Every
//! property is declared once, in the two lists below, and the test checks the whole list for collisions, so a new
//! property cannot slip in unchecked. A property keeps its salt for good: renaming a path changes the random stream
//! of its expressions, adding a property changes nothing else.

const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

/// Continues a 32-bit FNV-1a hash (offset basis and prime from the FNV specification) over `bytes`.
const fn fnv(mut h: u32, bytes: &[u8]) -> u32 {
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u32;
        h = h.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    h
}

const fn of(path: &str) -> u32 {
    fnv(FNV_OFFSET, path.as_bytes())
}

/// The salt of component `i` (0, 1 or 2) of a vector property: the hash of `"<path>.<i>"`.
///
/// FNV-1a's state is its running hash, so continuing from the property's own salt hashes the longer path.
pub const fn axis(property: u32, i: usize) -> u32 {
    assert!(i < AXES, "a vector property has three components");
    fnv(property, &[b'.', b'0' + i as u8])
}

/// The salt of a named property of something that has its own salt (`owner`): the hash of `"<owner path>.<name>"`.
pub const fn named(owner: u32, name: &str) -> u32 {
    fnv(fnv(owner, b"."), name.as_bytes())
}

/// The salt of the effect at `index` in a chain; its parameters and composite options are [`named`] under it.
pub fn effect(index: usize) -> u32 {
    of(&format!("fx.{index}"))
}

/// The most components any vector property has.
const AXES: usize = 3;

macro_rules! salts {
    (scalars { $($scalar:ident = $spath:literal,)* } vectors { $($vector:ident = $vpath:literal,)* }) => {
        $(pub const $scalar: u32 = of($spath);)*
        $(pub const $vector: u32 = of($vpath);)*
        /// Every declared property path with its salt, vector components expanded.
        #[cfg(test)]
        fn all() -> Vec<(String, u32)> {
            let mut out = vec![$(($spath.to_string(), $scalar),)*];
            $(out.extend((0..AXES).map(|i| (format!("{}.{i}", $vpath), axis($vector, i))));)*
            out
        }
    };
}

salts! {
    scalars {
        ALONG_T = "along.t",
        SKEW = "skew",
        OPACITY = "opacity",
        TEXTURE_PLUGIN = "texture.plugin",
        CAMERA_ZOOM = "camera.zoom",
        CAMERA_FOV = "camera.fov",
        CAMERA_DOF_FOCUS = "camera.dof.focus",
        CAMERA_DOF_BLUR = "camera.dof.blur",
        LIGHT_COLOR = "light.color",
        LIGHT_INTENSITY = "light.intensity",
        PARALLAX_STRENGTH = "image.parallax.strength",
        PARALLAX_LOOK = "image.parallax.look",
        MESH_COLOR = "mesh.color",
        MESH_SIDE = "mesh.side",
        MESH_BACK = "mesh.back",
        TEXT_BOX_PADDING_X = "text.box.padding.x",
        TEXT_BOX_PADDING_Y = "text.box.padding.y",
        TEXT_BOX_FILL = "text.box.fill",
        SWATCH = "swatch",
        SHAPE_REPEAT_PALETTE = "shape.repeat.palette",
        SHAPE_TAPER = "shape.taper",
    }
    vectors {
        POS = "pos",
        ANCHOR = "anchor",
        SCALE = "scale",
        AIM = "aim",
        ROTATION = "rotation",
        ORIENTATION = "orientation",
        CAMERA_POI = "camera.poi",
        LIGHT_TARGET = "light.target",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn distinct_properties_get_distinct_salts() {
        let mut seen: HashMap<u32, String> = HashMap::new();
        for (path, salt) in all() {
            if let Some(other) = seen.insert(salt, path.clone()) {
                panic!("{other} and {path} share the salt {salt}");
            }
        }
    }

    #[test]
    fn every_property_of_every_effect_in_a_chain_has_its_own_salt() {
        let properties = ["amount", "size", "composite.mix", "composite.mask.lo", "composite.mask.hi", "composite.mask.softness", "composite.mask.strength", "composite.mask.width"];
        let mut seen: HashMap<u32, String> = HashMap::new();
        for index in 0..16 {
            for property in properties {
                let path = format!("fx.{index}.{property}");
                if let Some(other) = seen.insert(named(effect(index), property), path.clone()) {
                    panic!("{other} and {path} share a salt");
                }
            }
        }
    }

    #[test]
    fn a_named_salt_is_the_hash_of_the_whole_path() {
        assert_eq!(named(of("fx.3"), "amount"), of("fx.3.amount"));
        assert_eq!(effect(3), of("fx.3"));
    }

    #[test]
    fn a_component_salt_is_the_hash_of_its_own_path() {
        // FNV-1a 32-bit of "a" is 0xe40c292c (published test vector)
        assert_eq!(of("a"), 0xe40c_292c);
        assert_eq!(axis(of("rotation"), 2), of("rotation.2"));
    }
}
