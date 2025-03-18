use std::env;

// Checking status of env vars
pub fn check_env_vars() -> Result<(), String> {
    let required_vars = [
        "BEACON_RPC_URL",
        "STARKNET_RPC_URL",
        "STARKNET_ADDRESS",
        "STARKNET_PRIVATE_KEY",
        "ATLANTIC_API_KEY",
        "PROOF_REGISTRY",
        "POSTGRESQL_HOST",
        "POSTGRESQL_USER",
        "POSTGRESQL_PASSWORD",
        "POSTGRESQL_DB_NAME",
        "RPC_LISTEN_HOST",
        "RPC_LISTEN_PORT",
        "TRANSACTOR_API_KEY",
    ];

    for &var in &required_vars {
        if env::var(var).is_err() {
            return Err(format!("Environment variable `{}` is not set", var));
        }
    }

    Ok(())
}
