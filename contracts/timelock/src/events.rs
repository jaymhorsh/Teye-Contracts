use soroban_sdk::{symbol_short, BytesN, Env};

/// Emitted when a transaction is successfully queued.
pub fn emit_tx_queued(env: &Env, id: &BytesN<32>, execute_after: u64) {
    env.events()
        .publish((symbol_short!("queued"), id.clone()), execute_after);
}

/// Emitted when a transaction is successfully executed.
pub fn emit_tx_executed(env: &Env, id: &BytesN<32>, executed_at: u64) {
    env.events()
        .publish((symbol_short!("executed"), id.clone()), executed_at);
}

/// Emitted when a transaction is promoted to priority execution.
pub fn emit_tx_prioritized(env: &Env, id: &BytesN<32>) {
    env.events()
        .publish((symbol_short!("prio"), id.clone()), true);
}
