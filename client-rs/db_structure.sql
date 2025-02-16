CREATE TABLE jobs (
    job_uuid UUID PRIMARY KEY,
    job_status TEXT NOT NULL,
    atlantic_proof_generate_batch_id TEXT NULL,
    atlantic_proof_wrapper_batch_id TEXT NULL,
    slot BIGINT NOT NULL, -- Slot associated with the job
    batch_range_begin_epoch BIGINT NULL,
    batch_range_end_epoch BIGINT NULL,
    type TEXT NOT NULL,
    tx_hash TEXT NULL,
    failed_at_step TEXT NULL,
    retries_count BIGINT NULL,
    last_failure_time TIMESTAMP NULL,
    updated_at TIMESTAMP DEFAULT NOW (),
    created_at TIMESTAMP DEFAULT NOW ()
);

CREATE TABLE epoch_merkle_paths (
    epoch_id BIGINT NOT NULL,
    path_index BIGINT NOT NULL,
    merkle_path TEXT NOT NULL,
    PRIMARY KEY (epoch_id, path_index) -- Ensures uniqueness of the combination
);

CREATE TABLE verified_epoch (
    epoch_id BIGINT PRIMARY KEY,
    beacon_header_root TEXT NOT NULL, -- Header root hash of the Beacon chain header
    beacon_state_root TEXT NOT NULL, -- State root hash of the Beacon chain state
    slot BIGINT NOT NULL, -- The number of slot at which this epoch was verified
    committee_hash TEXT NOT NULL, -- Sync committee hash of the sync commitee related to this epoch
    n_signers BIGINT NOT NULL, -- Number of epoch signers
    execution_header_hash TEXT NOT NULL, -- Execution layer blockhash
    execution_header_height BIGINT NOT NULL, -- Execution layer height
    epoch_index BIGINT NOT NULL, -- `Index of the epoch inside the batch
    batch_root TEXT NOT NULL, -- Epochs batch root hash
);

CREATE TABLE verified_sync_committee (
    sync_committee_id BIGINT PRIMARY KEY, -- Unique identifier for sync committee  (slot number/0x2000)
    sync_committee_hash TEXT NOT NULL -- Sync committee hash that we are creating inside bankai
);

CREATE TABLE daemon_state (
    latest_known_beacon_slot BIGINT NOT NULL,
    latest_known_beacon_block BYTEA NOT NULL,
    updated_at TIMESTAMP DEFAULT NOW ()
);
