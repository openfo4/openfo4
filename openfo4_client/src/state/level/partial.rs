#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartialEntityFlags {
    /// Graphics related components.
    pub graphics: bool,
    /// Physics related components.
    pub physics: bool,
    pub actor: bool,
    pub scene: bool,
    pub script: bool,
}
