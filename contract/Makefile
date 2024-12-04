setup:
	bash scripts/setup.sh

build:
	scarb build

test:
	scarb test

localnet:
	starknet-devnet --host 127.0.0.1 --port 5050 --seed 1337
declare:
	starkli declare ./target/dev/bankai_BankaiContract.contract_class.json --rpc http://127.0.0.1:5050 --account account.json --private-key 0x00000000000000000000000000000000ed744265ce4c723fc93dc990842d0d3b