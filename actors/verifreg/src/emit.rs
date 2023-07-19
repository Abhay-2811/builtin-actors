use crate::{Allocation, AllocationID, Claim, ClaimID, DataCap};
use fil_actors_runtime::runtime::Runtime;
use fil_actors_runtime::{ActorError, EventBuilder};
use fvm_shared::ActorID;

// A namespace for helpers that build and emit verified registry events.

/// Indicates a new value for a verifier's datacap balance.
/// Note that receiving this event does not necessarily mean the balance has changed.
/// The value is in datacap whole units (not TokenAmount).
pub fn verifier_balance(
    rt: &impl Runtime,
    verifier: ActorID,
    new_balance: &DataCap,
) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new()
            .label("verifier-balance")
            .field_indexed("verifier", &verifier)
            .field("balance", new_balance)
            .build()?,
    )
}

/// Indicates a new allocation has been made.
pub fn allocation(
    rt: &impl Runtime,
    id: AllocationID,
    alloc: &Allocation,
) -> Result<(), ActorError> {
    rt.emit_event(&EventBuilder::new().label("allocation").with_allocation(id, alloc).build()?)
}

/// Indicates an expired allocation has been removed.
pub fn allocation_removed(
    rt: &impl Runtime,
    id: AllocationID,
    alloc: &Allocation,
) -> Result<(), ActorError> {
    rt.emit_event(
        &EventBuilder::new().label("allocation-removed").with_allocation(id, alloc).build()?,
    )
}

/// Indicates an allocation has been claimed.
pub fn claim(rt: &impl Runtime, id: ClaimID, claim: &Claim) -> Result<(), ActorError> {
    rt.emit_event(&EventBuilder::new().label("claim").with_claim(id, claim).build()?)
}

/// Indicates an existing claim has been updated (e.g. with a longer term).
pub fn claim_updated(rt: &impl Runtime, id: ClaimID, claim: &Claim) -> Result<(), ActorError> {
    rt.emit_event(&EventBuilder::new().label("claim-updated").with_claim(id, claim).build()?)
}

/// Indicates an expired claim has been removed.
pub fn claim_removed(rt: &impl Runtime, id: ClaimID, claim: &Claim) -> Result<(), ActorError> {
    rt.emit_event(&EventBuilder::new().label("claim-removed").with_claim(id, claim).build()?)
}

// Private helpers //
trait WithAllocation {
    fn with_allocation(self, id: AllocationID, alloc: &Allocation) -> EventBuilder;
}

impl WithAllocation for EventBuilder {
    fn with_allocation(self, id: AllocationID, alloc: &Allocation) -> EventBuilder {
        self.field_indexed("id", &id)
            .field_indexed("client", &alloc.client)
            .field_indexed("provider", &alloc.provider)
            .field_indexed("data-cid", &alloc.data)
            .field("data-size", &alloc.size)
            .field("term-min", &alloc.term_min)
            .field("term-max", &alloc.term_max)
            .field("expiration", &alloc.expiration)
    }
}

trait WithClaim {
    fn with_claim(self, id: ClaimID, claim: &Claim) -> EventBuilder;
}

impl WithClaim for EventBuilder {
    fn with_claim(self, id: ClaimID, claim: &Claim) -> EventBuilder {
        self.field_indexed("id", &id)
            .field_indexed("provider", &claim.provider)
            .field_indexed("client", &claim.client)
            .field_indexed("data-cid", &claim.data)
            .field("data-size", &claim.size)
            .field("term-min", &claim.term_min)
            .field("term-max", &claim.term_max)
            .field("term-start", &claim.term_start)
            .field("sector", &claim.sector)
    }
}
