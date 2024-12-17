include .env
export

setup:
	bash scripts/setup.sh

build:
	scarb build

test:
	scarb test

localnet:
	source .env && starknet-devnet --host 127.0.0.1 --port 5050 --seed 1337 --fork-network ${RPC_URL}

account-add-local:
	source .env && \
	sncast \
		account import \
		--url http://127.0.0.1:5050 \
		--name devnet \
		--private-key 0x00000000000000000000000000000000ed744265ce4c723fc93dc990842d0d3b \
		--address 0x46d40ee9ddf64f6a92b04f26902f67a76c93692b8637afd43daeeeebc836609 \
		--type oz

account-add-sepolia:
	source .env && \
	sncast \
		account import \
		--url ${RPC_URL} \
		--name sepolia \
		--address ${ACCOUNT_ADDRESS} \
		--private-key ${PRIV_KEY} \
		--type argent

declare:
	sncast --account devnet declare --url http://127.0.0.1:5050 --fee-token eth --contract-name BankaiContract

declare-sepl:
	source .env && sncast --account sepolia declare --url ${RPC_URL} --fee-token eth --contract-name BankaiContract
