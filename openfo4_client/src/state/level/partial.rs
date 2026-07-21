#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartialEntityFlags {
    /// Graphics related components.
    pub graphics: bool,
    /// Physics related components.
    pub physics: bool,
    pub actor: bool,
    pub scene: bool,
    pub script: bool,
    /// Only build this entity if it has the `HAS_DISTANT_LOD` flag set.
    pub require_distant_lod: bool,
}
