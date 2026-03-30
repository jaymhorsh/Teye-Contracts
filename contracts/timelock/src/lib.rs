#![no_std]
#![allow(clippy::too_many_arguments)]

pub mod events;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short,
    Address, Bytes, BytesN, Env, Symbol, Vec,
};

// ── Serialization-safety limits ───────────────────────────────────────────────

/// Maximum caller-supplied nesting depth value accepted per `PayloadNode`.
/// Nodes reporting a depth above this are rejected before storage to prevent
/// unbounded deserialization costs.
pub const MAX_PAYLOAD_DEPTH: u32 = 8;

/// Maximum number of `PayloadNode` entries allowed in a single `NestedPayload`.
pub const MAX_PAYLOAD_WIDTH: u32 = 16;

/// Maximum number of `LeafData` entries per `PayloadNode`.
pub const MAX_LEAF_COUNT: u32 = 32;

/// Maximum number of transactions held in the on-chain queue at once.
pub const MAX_QUEUE_SIZE: u32 = 64;

// ── Storage keys ─────────────────────────────────────────────────────────────

const ADMIN: Symbol = symbol_short!("ADMIN");
const INIT: Symbol = symbol_short!("INIT");
const QUEUE: Symbol = symbol_short!("QUEUE");

// ── Error catalogue ───────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TimelockError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    TxNotFound = 4,
    TooEarlyToExecute = 5,
    /// A `PayloadNode` reported a `depth` value that exceeds `MAX_PAYLOAD_DEPTH`.
    PayloadTooDeep = 6,
    /// The node count or leaf count exceeds the configured width/count limits.
    PayloadTooWide = 7,
    TxAlreadyQueued = 8,
    QueueFull = 9,
    InvalidDelay = 10,
}

// ── Nested payload types ──────────────────────────────────────────────────────

/// Atomic key-value pair at the innermost level of a nested payload.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeafData {
    pub key: Symbol,
    pub value: BytesN<32>,
}

/// A single node in the payload tree.  `depth` is the caller-reported nesting
/// level (0 = root); the contract enforces `depth <= MAX_PAYLOAD_DEPTH`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadNode {
    pub depth: u32,
    pub data: Bytes,
    pub leaves: Vec<LeafData>,
}

/// Top-level nested-payload container attached to each queued transaction.
/// The contract validates width/depth/leaf-count before persisting.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NestedPayload {
    pub version: u32,
    pub nodes: Vec<PayloadNode>,
}

/// A transaction queued in the timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockEntry {
    pub id: BytesN<32>,
    pub target: Address,
    pub execute_after: u64,
    pub priority: bool,
    pub payload: NestedPayload,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct TimelockContract;

#[contractimpl]
impl TimelockContract {
    /// Initialise the timelock. Must be called exactly once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), TimelockError> {
        if env.storage().instance().has(&INIT) {
            return Err(TimelockError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&INIT, &true);
        Ok(())
    }

    /// Queue a transaction with an arbitrarily nested payload.
    ///
    /// The contract validates that every node in `payload` respects
    /// `MAX_PAYLOAD_DEPTH`, `MAX_PAYLOAD_WIDTH`, and `MAX_LEAF_COUNT` before
    /// writing to ledger storage.  Payloads that exceed these limits are
    /// rejected with [`TimelockError::PayloadTooDeep`] or
    /// [`TimelockError::PayloadTooWide`], preventing stack overflows or
    /// excessive gas consumption during serialization.
    pub fn queue_tx(
        env: Env,
        caller: Address,
        id: BytesN<32>,
        target: Address,
        delay: u64,
        payload: NestedPayload,
    ) -> Result<TimelockEntry, TimelockError> {
        Self::require_init(&env)?;
        caller.require_auth();

        if delay == 0 {
            return Err(TimelockError::InvalidDelay);
        }

        Self::validate_payload(&payload)?;

        let mut queue: Vec<TimelockEntry> = env
            .storage()
            .instance()
            .get(&QUEUE)
            .unwrap_or_else(|| Vec::new(&env));

        for entry in queue.iter() {
            if entry.id == id {
                return Err(TimelockError::TxAlreadyQueued);
            }
        }

        if queue.len() >= MAX_QUEUE_SIZE {
            return Err(TimelockError::QueueFull);
        }

        let execute_after = env.ledger().timestamp().saturating_add(delay);
        let entry = TimelockEntry {
            id: id.clone(),
            target,
            execute_after,
            priority: false,
            payload,
        };
        queue.push_back(entry.clone());
        env.storage().instance().set(&QUEUE, &queue);

        events::emit_tx_queued(&env, &id, execute_after);
        Ok(entry)
    }

    /// Execute a queued transaction once its delay has elapsed (or it is
    /// marked as priority).  Removes the entry from the queue on success.
    pub fn execute_tx(
        env: Env,
        caller: Address,
        id: BytesN<32>,
    ) -> Result<TimelockEntry, TimelockError> {
        Self::require_init(&env)?;
        caller.require_auth();

        let queue: Vec<TimelockEntry> = env
            .storage()
            .instance()
            .get(&QUEUE)
            .unwrap_or_else(|| Vec::new(&env));

        let now = env.ledger().timestamp();
        let mut found: Option<TimelockEntry> = None;

        for entry in queue.iter() {
            if entry.id == id {
                if !entry.priority && now < entry.execute_after {
                    return Err(TimelockError::TooEarlyToExecute);
                }
                found = Some(entry);
                break;
            }
        }

        let entry = found.ok_or(TimelockError::TxNotFound)?;

        let mut new_queue: Vec<TimelockEntry> = Vec::new(&env);
        for e in queue.iter() {
            if e.id != id {
                new_queue.push_back(e);
            }
        }
        env.storage().instance().set(&QUEUE, &new_queue);

        events::emit_tx_executed(&env, &id, now);
        Ok(entry)
    }

    /// Admin-only: promote a queued transaction to priority so it can be
    /// executed immediately regardless of its delay.
    pub fn prioritize_tx(
        env: Env,
        admin: Address,
        id: BytesN<32>,
    ) -> Result<(), TimelockError> {
        Self::require_init(&env)?;
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN)
            .ok_or(TimelockError::NotInitialized)?;
        if admin != stored_admin {
            return Err(TimelockError::Unauthorized);
        }
        admin.require_auth();

        let queue: Vec<TimelockEntry> = env
            .storage()
            .instance()
            .get(&QUEUE)
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        let mut new_queue: Vec<TimelockEntry> = Vec::new(&env);
        for entry in queue.iter() {
            if entry.id == id {
                found = true;
                let mut prioritized = entry.clone();
                prioritized.priority = true;
                new_queue.push_back(prioritized);
            } else {
                new_queue.push_back(entry);
            }
        }

        if !found {
            return Err(TimelockError::TxNotFound);
        }

        env.storage().instance().set(&QUEUE, &new_queue);
        events::emit_tx_prioritized(&env, &id);
        Ok(())
    }

    /// Return all currently queued transactions.
    pub fn get_queue(env: Env) -> Vec<TimelockEntry> {
        env.storage()
            .instance()
            .get(&QUEUE)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Return a single queued transaction by its ID.
    pub fn get_tx(env: Env, id: BytesN<32>) -> Result<TimelockEntry, TimelockError> {
        let queue: Vec<TimelockEntry> = env
            .storage()
            .instance()
            .get(&QUEUE)
            .unwrap_or_else(|| Vec::new(&env));

        for entry in queue.iter() {
            if entry.id == id {
                return Ok(entry);
            }
        }
        Err(TimelockError::TxNotFound)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Validate that a `NestedPayload` stays within all serialization-safety
    /// limits before it is written to ledger storage.
    fn validate_payload(payload: &NestedPayload) -> Result<(), TimelockError> {
        if payload.nodes.len() > MAX_PAYLOAD_WIDTH {
            return Err(TimelockError::PayloadTooWide);
        }
        for node in payload.nodes.iter() {
            if node.depth > MAX_PAYLOAD_DEPTH {
                return Err(TimelockError::PayloadTooDeep);
            }
            if node.leaves.len() > MAX_LEAF_COUNT {
                return Err(TimelockError::PayloadTooWide);
            }
        }
        Ok(())
    }

    fn require_init(env: &Env) -> Result<(), TimelockError> {
        if !env.storage().instance().has(&INIT) {
            return Err(TimelockError::NotInitialized);
        }
        Ok(())
    }
}
