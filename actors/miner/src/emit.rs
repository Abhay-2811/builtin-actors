// A namespace for helpers that build and emit provider Actor events.
use fil_actors_runtime::runtime::Runtime;
use fil_actors_runtime::{ActorError, EventBuilder};
use fvm_shared::sector::SectorNumber;

/// Indicates a sector has been pre-committed.
pub fn sector_precommitted(rt: &impl Runtime, sector: SectorNumber) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new().typ("sector-precommitted").field_indexed("sector", &sector).build()?,
    )
}

/// Indicates a sector has been activated.
pub fn sector_activated(rt: &impl Runtime, sector: SectorNumber) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new().typ("sector-activated").field_indexed("sector", &sector).build()?,
    )
}

/// Indicates a sector has been updated.
pub fn sector_updated(rt: &impl Runtime, sector: SectorNumber) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new().typ("sector-updated").field_indexed("sector", &sector).build()?,
    )
}

/// Indicates a sector has been terminated.
pub fn sector_terminated(rt: &impl Runtime, sector: SectorNumber) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new().typ("sector-terminated").field_indexed("sector", &sector).build()?,
    )
}
