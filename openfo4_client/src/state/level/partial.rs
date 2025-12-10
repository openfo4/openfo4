use bitflags::bitflags;

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct PartialEntityFlags: u32 {
        /// Graphics related components.
        const GRAPHICS = 1 << 1;
        /// Physics related components.
        const PHYSICS = 1 << 2;
        const ACTOR = 1 << 3;
        const SCENE = 1 << 4;
    }
}
